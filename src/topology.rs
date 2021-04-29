use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex, Weak,
};

use super::group;
use super::task;

pub struct Topology {
    group_nodes: Vec<Arc<Mutex<GroupNode>>>,
    pub(crate) root_groups: Vec<GroupNodeHandle>,
}

impl Topology {
    ///
    ///
    ///
    fn create_group_nodes(group_list: &group::GroupList) -> Vec<Arc<Mutex<GroupNode>>> {
        let mut group_nodes = vec![];
        group_list.iter().for_each(|x| {
            // Setup local nodes.
            let group_node = Arc::new(Mutex::new(GroupNode::new_empty(x.clone())));

            // Make group's local task nodes.
            let (task_nodes, task_count) = {
                let mut nodes = vec![];
                let mut count = 0u32;
                match x.value_as_ref() {
                    // Critical section
                    None => return,
                    Some(accessor) => accessor
                        .tasks
                        .iter()
                        .filter(|&task| task.is_released() == false)
                        .for_each(|task| {
                            let group_node_handle = Arc::downgrade(&group_node);
                            let node = TaskNode::new(task.clone(), group_node_handle);
                            // Insert node into list.
                            nodes.push(node);
                            count += 1;
                        }),
                }

                (nodes, count)
            };

            // Update group node's list and counter.
            {
                let mut group_node_guard = group_node.lock();
                let group_node_ref = group_node_guard.as_mut().unwrap();
                group_node_ref.task_nodes = task_nodes;
                group_node_ref
                    .remained_task_cnt
                    .store(task_count, Ordering::Relaxed);
            }

            // Insert group into list.
            group_nodes.push(group_node);
        });

        group_nodes
    }

    ///
    ///
    pub fn try_from(group_list: &group::GroupList) -> Option<Self> {
        // Check there is a any validated group and not empty.
        if group_list.is_empty() || group_list.iter().all(|group| group.is_released()) {
            return None;
        }

        // Make topology item and fill it.
        let group_nodes = Self::create_group_nodes(group_list);

        // Make chain to each groups.
        for group in group_nodes.iter() {
            let successor_nodes = {
                let group = group.lock().unwrap();
                let accessor = group.handle.value_as_ref().unwrap();
                let successors = &accessor.chains.success_group_list;

                let mut successor_nodes: Vec<_> = group_nodes
                    .iter()
                    .filter(|&g| match g.try_lock() {
                        Err(_) => false,
                        Ok(g) => {
                            let g_id = g.handle.id();
                            successors
                                .iter()
                                .filter(|&s| s.is_released() == false)
                                .any(|s| s.id() == g_id)
                        }
                    })
                    .collect();
                successor_nodes.iter_mut().for_each(|&mut s| {
                    s.lock()
                        .unwrap()
                        .remained_predecessor_cnt
                        .fetch_add(1, Ordering::Relaxed);
                });

                let mut v = vec![];
                for node in successor_nodes.into_iter() {
                    v.push(Arc::downgrade(node));
                }
                v
            };

            // Intended cloning for avoiding borrow rule violation.
            let group = group.clone();
            group.lock().unwrap().successor_nodes = successor_nodes;
        }

        let root_groups = {
            let mut v = vec![];
            let root_groups_iter = group_nodes.iter().filter(|&g| {
                let guard = g.lock().unwrap();
                guard.remained_predecessor_cnt.load(Ordering::Relaxed) == 0
            });
            for root_group in root_groups_iter {
                v.push(Arc::downgrade(root_group));
            }
            v
        };

        Some(Self {
            group_nodes,
            root_groups,
        })
    }
}

pub(crate) type GroupNodeHandle = Weak<Mutex<GroupNode>>;

pub struct GroupNode {
    handle: group::GroupHandle,
    pub(crate) task_nodes: Vec<TaskNode>,
    pub(crate) remained_task_cnt: AtomicU32,
    pub(crate) successor_nodes: Vec<GroupNodeHandle>,
    pub(crate) remained_predecessor_cnt: AtomicU32,
}

impl GroupNode {
    ///
    ///
    ///
    fn new_empty(handle: group::GroupHandle) -> Self {
        Self {
            handle,
            task_nodes: vec![],
            remained_task_cnt: AtomicU32::new(0),
            successor_nodes: vec![],
            remained_predecessor_cnt: AtomicU32::new(0),
        }
    }
}

#[derive(Clone)]
pub struct TaskNode {
    pub(crate) handle: task::TaskHandle,
    pub(crate) group_node: Weak<Mutex<GroupNode>>,
}

impl TaskNode {
    ///
    ///
    ///
    fn new(handle: task::TaskHandle, group_node: GroupNodeHandle) -> Self {
        Self { handle, group_node }
    }
}
