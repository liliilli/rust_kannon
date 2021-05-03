use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex, Weak,
};

use super::error::TaskError;
use super::group;
use super::task;

/// Represents executable task group unit.
pub struct Topology {
    group_nodes: Vec<Arc<Mutex<GroupNode>>>,
    pub(crate) task_count: usize,
    pub(crate) root_groups: Vec<GroupNodeHandle>,
}

impl Topology {
    /// Create group node list and total count of tasks to process from the input, but not chained list.
    ///
    /// Internal function.
    /// Called from `Self::fill_from_list`.
    fn create_group_nodes(
        group_list: &group::GroupList,
        out: &mut Vec<Arc<Mutex<GroupNode>>>,
    ) -> usize {
        let mut total_task_count = 0usize;
        out.clear();

        for x in group_list {
            // Setup local nodes.
            let group_node = Arc::new(Mutex::new(GroupNode::new(x.clone())));

            // Make group's local task nodes.
            let (task_nodes, task_count) = {
                let mut nodes = vec![];
                let mut count = 0u32;
                match x.value_as_ref() {
                    // Critical section
                    None => continue,
                    Some(accessor) => {
                        for task in accessor.tasks.iter().filter(|&task| !task.is_released()) {
                            let group_node_handle = Arc::downgrade(&group_node);
                            let node = TaskNode::new(task.clone(), group_node_handle);
                            // Insert node into list.
                            nodes.push(node);
                            count += 1;
                        }

                        // If count is 0, we have to insert empty node of local group to proceed to next group.
                        if count == 0 {
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
            out.push(group_node);
        }

        total_task_count
    }

    ///
    ///
    /// Internal function.
    /// Called from `Self::new_from`.
    /// Called from `Self::rearrange_from`.
    fn fill_from_list(groups: &group::GroupList, out: &mut Vec<Arc<Mutex<GroupNode>>>) -> usize {
        let task_count = Self::create_group_nodes(groups, out);

        // Make chain to each groups.
        for group_node in out.iter() {
            let successor_nodes: Vec<_> = {
                let group_node = group_node.lock().unwrap();
                let successors = &group_node
                    .handle
                    .value_as_ref()
                    .unwrap()
                    .chains
                    .success_groups;

                // Find successor nodes from actual group's successors.
                out.iter()
                    .filter(|&g| match g.try_lock() {
                        Err(_) => false,
                        Ok(g) => {
                            let g_id = g.handle.id();
                            successors
                                .iter()
                                .filter(|&s| !s.is_released())
                                .any(|s| s.id() == g_id)
                        }
                    })
                    .map(|s| {
                        s.lock().unwrap().increase_predecessor_count();
                        Arc::downgrade(s)
                    })
                    .collect()
            };

            // Intended cloning for avoiding borrow rule violation.
            let group_node = group_node.clone();
            group_node.lock().unwrap().successor_nodes = successor_nodes;
        }

        task_count
    }

    /// Try to create topology instance from group list.
    ///
    /// Successfully created topology instance can be executable and have validated group and
    /// tasks.
    ///
    /// If failed, library error code will be returned.
    pub(crate) fn new_from(groups: &group::GroupList) -> Result<Self, TaskError> {
        // Check there is a any validated group and not empty.
        if groups.is_empty() || groups.iter().all(|group| group.is_released()) {
            return Err(TaskError::NoValidatedGroups);
        }

        // Make topology item and fill it.
        let mut group_nodes = vec![];
        let task_count = Self::fill_from_list(groups, &mut group_nodes);

        // Make root group node list which items does not have any predeceed group nodes.
        let root_groups: Vec<_> = group_nodes
            .iter()
            .filter(|&g| g.lock().unwrap().is_ready())
            .map(|g| Arc::downgrade(g))
            .collect();

        Ok(Self {
            group_nodes,
            task_count,
            root_groups,
        })
    }

    /// Create empty topology.
    pub fn new() -> Self {
        Self {
            group_nodes: vec![],
            task_count: 0,
            root_groups: vec![],
        }
    }

    /// Rearrange topology with given group list.
    pub fn rearrange_from(&mut self, groups: &group::GroupList) {
        self.root_groups.clear();
        self.task_count = Self::fill_from_list(groups, &mut self.group_nodes);

        // Make root group node list which items does not have any predeceed group nodes.
        for root_node in self
            .group_nodes
            .iter()
            .filter(|&g| g.lock().unwrap().is_ready())
        {
            self.root_groups.push(Arc::downgrade(root_node));
        }
    }
}

/// Alias of weaked synchronized group node.
pub(crate) type GroupNodeHandle = Weak<Mutex<GroupNode>>;

/// The group node.
pub(crate) struct GroupNode {
    handle: group::GroupHandle,
    pub(crate) task_nodes: Vec<TaskNode>,
    remained_task_cnt: AtomicU32,
    pub(crate) successor_nodes: Vec<GroupNodeHandle>,
    remained_predecessor_cnt: AtomicU32,
}

impl GroupNode {
    /// Create new group node.
    fn new(handle: group::GroupHandle) -> Self {
        Self {
            handle,
            task_nodes: vec![],
            remained_task_cnt: AtomicU32::new(0),
            successor_nodes: vec![],
            remained_predecessor_cnt: AtomicU32::new(0),
        }
    }

    /// Check group node is ready to being processed.
    fn is_ready(&self) -> bool {
        self.remained_predecessor_cnt.load(Ordering::Acquire) == 0
    }

    /// Increase remained predecessor count by 1 and return last value.
    fn increase_predecessor_count(&self) -> u32 {
        self.remained_predecessor_cnt.fetch_add(1, Ordering::SeqCst)
    }

    /// Decrease remained predecessor count by 1 and return last value.
    pub(super) fn decrease_predecessor_count(&self) -> u32 {
        self.remained_predecessor_cnt
            .fetch_sub(1, Ordering::Release)
    }

    /// Get remained task count.
    pub(super) fn task_count(&self) -> u32 {
        self.remained_task_cnt.load(Ordering::Relaxed)
    }

    /// Decrease remained task count by 1 and return last value.
    pub(super) fn decrease_task_count(&self) -> u32 {
        self.remained_task_cnt.fetch_sub(1, Ordering::Relaxed)
    }
}

#[derive(Clone)]
pub(crate) struct TaskNode {
    pub(crate) handle: task::TaskHandle,
    pub(crate) group_node: Weak<Mutex<GroupNode>>,
}

impl TaskNode {
    /// Create new task node.
    fn new(handle: task::TaskHandle, group_node: GroupNodeHandle) -> Self {
        Self { handle, group_node }
    }
}
