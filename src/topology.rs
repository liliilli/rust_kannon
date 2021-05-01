use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex, Weak,
};

use super::error::TaskError;
use super::group;
use super::task;

/// Represents executable task group unit.
///
///
pub struct Topology {
    group_nodes: Vec<Arc<Mutex<GroupNode>>>,
    pub(crate) task_count: usize,
    pub(crate) root_groups: Vec<GroupNodeHandle>,
}

impl Topology {
    /// Create group node list and total count of tasks to process from the input, but not chained list.
    ///
    /// Internal function.
    /// Called from `Self::try_from`.
    fn create_group_nodes(group_list: &group::GroupList) -> (Vec<Arc<Mutex<GroupNode>>>, usize) {
        let mut group_nodes = vec![];
        let mut total_task_count = 0usize;

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

                // If count is 0, we have to insert empty node of local group to proceed to next
                // group.
                if count == 0 {
                    // Critical section
                    match x.value_as_ref() {
                        None => return,
                        Some(accessor) => {
                            let task_node_handle = accessor.handle_of_empty_task();
                            let group_node_handle = Arc::downgrade(&group_node);
                            let node = TaskNode::new(task_node_handle, group_node_handle);
                            // Insert node into list.
                            nodes.push(node);
                            count += 1;
                        }
                    }
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
                total_task_count += task_count as usize;
            }

            // Insert group into list.
            group_nodes.push(group_node);
        });

        (group_nodes, total_task_count)
    }

    /// Try to create topology instance from group list.
    ///
    /// Successfully created topology instance can be executable and have validated group and
    /// tasks.
    ///
    /// If failed, library error code will be returned.
    pub(crate) fn try_from(groups: &group::GroupList) -> Result<Self, TaskError> {
        // Check there is a any validated group and not empty.
        if groups.is_empty() || groups.iter().all(|group| group.is_released()) {
            return Err(TaskError::NoValidatedGroups);
        }

        // Make topology item and fill it.
        let (group_nodes, task_count) = Self::create_group_nodes(groups);

        // Make chain to each groups.
        for group_node in group_nodes.iter() {
            let successor_nodes = {
                let group_node = group_node.lock().unwrap();
                let successors = &group_node
                    .handle
                    .value_as_ref()
                    .unwrap()
                    .chains
                    .success_groups;

                // Find successor nodes from actual group's successors.
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

                // Downgrade successors.
                let mut weaks = vec![];
                for node in successor_nodes.into_iter() {
                    weaks.push(Arc::downgrade(node));
                }
                weaks
            };

            // Intended cloning for avoiding borrow rule violation.
            let group_node = group_node.clone();
            group_node.lock().unwrap().successor_nodes = successor_nodes;
        }

        // Make root group node list which items does not have any predeceed group nodes.
        let root_group_nodes = {
            let mut nodes = vec![];
            group_nodes
                .iter()
                .filter(|&g| g.lock().unwrap().is_ready())
                .for_each(|g| nodes.push(Arc::downgrade(g)));
            nodes
        };

        Ok(Self {
            group_nodes,
            task_count,
            root_groups: root_group_nodes,
        })
    }
}

/// Alias of weaked synchronized group node.
///
///
pub(crate) type GroupNodeHandle = Weak<Mutex<GroupNode>>;

/// The node.
///
///
pub(crate) struct GroupNode {
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

    /// Check group node is ready to being processed.
    ///
    ///
    pub fn is_ready(&self) -> bool {
        self.remained_predecessor_cnt.load(Ordering::Acquire) == 0
    }
}

#[derive(Clone)]
pub(crate) struct TaskNode {
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
