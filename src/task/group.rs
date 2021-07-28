use super::error::TaskError;
use super::task;
use task::{Task, TaskHandle};

use std::{mem, ops::Deref};
use std::{
    ops::DerefMut,
    sync::{atomic::AtomicUsize, atomic::Ordering, Arc, Mutex, MutexGuard, Weak},
};

/// Raw type for `Group` instance.
///
/// Stores actual informations for controlling local tasks and dependency.
pub struct GroupRaw {
    /// the name of group. This name can be duplicated to other groups.
    name: String,
    /// Unique id of the group.
    id: usize,
    /// Empty task for processing next group in topology.
    empty_task: Task,
    /// Local task handle list for calling tasks in batch.
    pub(crate) tasks: Vec<TaskHandle>,
    /// Stores chaining information to other groups.
    pub(crate) chains: GroupChains,
}

impl GroupRaw {
    /// Check group has successor groups.
    pub fn has_successors(&self) -> bool {
        self.chains.success_groups.is_empty()
    }

    /// Check group has predecessor groups.
    pub fn has_predecessors(&self) -> bool {
        self.chains.precede_groups.is_empty()
    }

    /// Remove invalidated task from list and rearrange them.
    pub(crate) fn rearrange_tasks(&mut self) {
        self.tasks.retain(|t| !t.is_released());
    }

    /// Get the handle of empty task.
    pub(crate) fn handle_of_empty_task(&self) -> TaskHandle {
        self.empty_task.handle()
    }

    /// Create new group.
    ///
    /// Every heap allocation in inside must be successful.
    /// Given `name` instance must be valid and not empty. It's okay to be duplicated to other
    /// group's name.
    ///
    /// Given `id` must be valid and not duplicated to other group's id, so must be unique.
    /// This function is not be called directly, but usually from `Group::new` method.
    fn new(name: &str, id: usize) -> Self {
        Self {
            name: name.to_string(),
            id,
            empty_task: Task::empty_task(),
            tasks: vec![],
            chains: GroupChains::default(),
        }
    }

    /// Check any group which has given id is exist in this group's chain list.
    fn is_contains_id(&self, id: usize) -> bool {
        let this_predeces = &self.chains.precede_groups;
        if this_predeces.iter().any(|x| x.id == id) {
            return true;
        }
        let this_successors = &self.chains.success_groups;
        if this_successors.iter().any(|x| x.id == id) {
            return true;
        }

        return false;
    }
}

/// Stores chaining informations to other groups.
#[derive(Default)]
pub(crate) struct GroupChains {
    ///
    pub(crate) precede_groups: Vec<GroupHandle>,
    ///
    pub(crate) success_groups: Vec<GroupHandle>,
}

/// Task group unit.
pub struct Group {
    raw: Arc<Mutex<GroupRaw>>,
}

impl Group {
    /// Get the name of the group.
    pub fn name(&self) -> String {
        self.raw.lock().unwrap().name.clone()
    }

    /// Get new handle item of the group.
    pub fn handle(&self) -> GroupHandle {
        GroupHandle {
            value: Arc::downgrade(&self.raw),
            id: self.raw.lock().unwrap().id,
        }
    }

    /// Create task which is binding lambda closure.
    ///
    /// Given name must be valid and not empty. It's ok to be duplicated with other task's name.
    #[must_use]
    pub fn create_task(
        &mut self,
        name: &str,
        f: impl Fn() + Sync + Send + 'static,
    ) -> Result<Task, TaskError> {
        if name.is_empty() {
            Err(TaskError::InvalidItemName)
        } else {
            let task = Task::from_closure(name, f);
            let task_handle = task.handle();

            let mut raw = self.raw.lock().unwrap();
            raw.tasks.push(task_handle);

            Ok(task)
        }
    }

    /// Create task which is binding item's pointer and valid immutable method from the item.
    ///
    /// Given name must be valid and not empty. It's ok to be duplicated with other task's name.
    /// Being binded item should not be invalidated, or moved state.
    /// Otherwise, calling invalidated item's method will be undefined behavior.
    ///
    /// Calling method of task may not invalidate borrowing rule, but care about synchronization
    /// and data race manually in the logic.
    #[must_use]
    pub fn create_task_method<T, F>(&mut self, name: &str, t: &T, f: F) -> Result<Task, TaskError>
    where
        T: 'static,
        F: Fn(&T) + Sync + Send + 'static,
    {
        if name.is_empty() {
            Err(TaskError::InvalidItemName)
        } else {
            let task = Task::from_method(name, t, f);
            let task_handle = task.handle();

            let mut raw = self.raw.lock().unwrap();
            raw.tasks.push(task_handle);

            Ok(task)
        }
    }

    /// Create task which is binding item's pointer and valid mutable method from the item.
    ///
    /// Given name must be valid and not empty. It's ok to be duplicated with other task's name.
    /// Being binded item should not be invalidated, or moved state.
    /// Otherwise, calling invalidated item's immutable method will be undefined behavior.
    ///
    /// Calling method of task may not invalidate borrowing rule, but care about synchronization
    /// and data race manually in the logic.
    #[must_use]
    pub fn create_task_method_mut<T, F>(
        &mut self,
        name: &str,
        t: &mut T,
        f: F,
    ) -> Result<Task, TaskError>
    where
        T: 'static,
        F: Fn(&mut T) + Sync + Send + 'static,
    {
        if name.is_empty() {
            Err(TaskError::InvalidItemName)
        } else {
            let task = Task::from_method_mut(name, t, f);
            let task_handle = task.handle();

            let mut raw = self.raw.lock().unwrap();
            raw.tasks.push(task_handle);

            Ok(task)
        }
    }

    /// Let this group precede given other group.
    ///
    /// If function is successful, this group will be processed before other group.
    pub fn precede(&mut self, handle: GroupHandle) -> Result<(), TaskError> {
        let this_handle = self.handle();
        let mut guard = self.raw.lock().unwrap();
        if guard.id == handle.id {
            // Same group can not be chain each other.
            Err(TaskError::InvalidChaining)
        } else {
            // Check given handle is already inserted into the lists (precede and success).
            if guard.is_contains_id(handle.id) {
                return Err(TaskError::InvalidChaining);
            }
            // Check other is still validated.
            let mut other_handle = handle.clone();
            let mut other_group = match other_handle.value_as_mut() {
                None => return Err(TaskError::InvalidGroupHandle),
                Some(accessor) => accessor,
            };

            // Make chain relation.
            guard.chains.success_groups.push(handle);
            other_group.chains.precede_groups.push(this_handle);
            Ok(())
        }
    }

    /// Let this group succeeds given other group.
    ///
    /// If function is successful, this group will follow after other group.
    pub fn succeed(&mut self, handle: GroupHandle) -> Result<(), TaskError> {
        let this_handle = self.handle();
        let mut guard = self.raw.lock().unwrap();
        if guard.id == handle.id {
            // Same group can not be chain each other.
            Err(TaskError::InvalidChaining)
        } else {
            // Check given handle is already inserted into the lists (precede and success).
            if guard.is_contains_id(handle.id) {
                return Err(TaskError::InvalidChaining);
            }
            // this_predeces and this_successors will not be used anymore.
            // Check other is still validated.
            let mut other_handle = handle.clone();
            let mut other_group = match other_handle.value_as_mut() {
                None => return Err(TaskError::InvalidGroupHandle),
                Some(accessor) => accessor,
            };

            // Make chain relation.
            guard.chains.precede_groups.push(handle);
            other_group.chains.success_groups.push(this_handle);
            Ok(())
        }
    }
}

/// Handle type for the group.
#[derive(Clone)]
pub struct GroupHandle {
    value: Weak<Mutex<GroupRaw>>,
    id: usize,
}

impl GroupHandle {
    /// Access to the group execusively and return accessor `GroupAccessor` item.
    ///
    /// If actual group item is invalidated, do nothing just return `None` value.
    /// When group is already locked by other context, it waits until locking is end.
    /// Be careful not causing dead-lock.
    pub fn value_as_ref<'a>(&'a self) -> Option<GroupAccessor<'a>> {
        let group = self.value.upgrade()?;
        let group_lock = group.lock();
        if let Ok(guard) = group_lock {
            let guard: MutexGuard<'a, GroupRaw> = unsafe { mem::transmute(guard) };
            Some(GroupAccessor { guard })
        } else {
            None
        }
    }

    /// Access to the group execusively and return accessor `GroupAccessorMut` item.
    ///
    /// If actual group item is invalidated, do nothing just return `None` value.
    /// When group is already locked by other context, it waits until locking is end.
    /// Be careful not causing dead-lock.
    pub fn value_as_mut<'a>(&'a mut self) -> Option<GroupAccessorMut<'a>> {
        let group = self.value.upgrade()?;
        let group_lock = group.lock();
        if let Ok(guard) = group_lock {
            let guard: MutexGuard<'a, GroupRaw> = unsafe { mem::transmute(guard) };
            Some(GroupAccessorMut { guard })
        } else {
            None
        }
    }

    /// Check this group is released or not.
    pub fn is_released(&self) -> bool {
        self.value.strong_count() == 0
    }

    /// Return unique id of group.
    pub fn id(&self) -> usize {
        self.id
    }
}

impl std::cmp::PartialEq for GroupHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Accessor item type for the group.
pub struct GroupAccessor<'a> {
    guard: MutexGuard<'a, GroupRaw>,
}

impl<'a> Deref for GroupAccessor<'a> {
    type Target = GroupRaw;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

/// Mutable accessor item type for the group.
pub struct GroupAccessorMut<'a> {
    guard: MutexGuard<'a, GroupRaw>,
}

impl<'a> Deref for GroupAccessorMut<'a> {
    type Target = GroupRaw;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a> DerefMut for GroupAccessorMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

/// Alias
pub(crate) type GroupList = Vec<GroupHandle>;

/// Create group which can include task items that can be executed simutaneously by `executor::Executor`.
///
/// Given `name` must be not empty and validated. Group's name does not have to be unique.
/// This function is only called from `GroupManager::create_group` method.
#[must_use]
pub(crate) fn create_group(groups: &mut GroupList, name: &str) -> Result<Group, TaskError> {
    if name.is_empty() {
        Err(TaskError::InvalidItemName)
    } else {
        static mut ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

        let id = unsafe { ID_COUNTER.fetch_add(1, Ordering::Relaxed) };
        let group = Group {
            raw: Arc::new(Mutex::new(GroupRaw::new(name, id))),
        };
        let group_handle = group.handle();

        groups.push(group_handle);
        Ok(group)
    }
}

///
///
///
pub struct GroupManager {
    groups: GroupList,
}

impl GroupManager {
    ///
    ///
    ///
    pub fn new() -> Self {
        Self {
            groups: GroupList::default(),
        }
    }

    /// Create new group that can contain tasks and executable in topology.
    ///
    /// If name was empty or any following internal logic, function would be failed and group item
    /// can not be created.
    ///
    /// # Arguments
    ///
    /// * `name` - Not empty, valid group name.
    #[must_use]
    pub fn create_group(&mut self, name: &str) -> Result<Group, TaskError> {
        create_group(&mut self.groups, name)
    }

    ///
    ///
    ///
    pub fn groups(&self) -> &GroupList {
        &self.groups
    }

    ///
    ///
    ///
    pub fn is_cyclic(&self) -> bool {
        todo!("Not implemented yet.");
        let pred: &dyn Fn(&&GroupHandle) -> bool = &|x: &&GroupHandle| match (*x).value_as_ref() {
            None => false,
            Some(accessor) => accessor.has_predecessors(),
        };
        let groups_len = self.groups.len();

        let _visiteds = {
            let mut vec = Vec::<bool>::with_capacity(groups_len);
            vec.resize(groups_len, false);
            vec
        };

        let _root_group_iter = self.groups.iter().filter(pred);
        true
    }

    /// Remove invalidated group from list and rerrange them.
    pub fn rearrange_groups(&mut self) {
        // Get removal candidate groups.
        let (released_groups, mut remained_groups): (Vec<GroupHandle>, Vec<GroupHandle>) = self
            .groups
            .iter() // Could not use into_iter because internal type does not implement Copy.
            .map(|g| g.clone())
            .partition(|g| g.is_released());

        // Remove all chains from remained_groups.
        let released_group_ids: Vec<_> = released_groups.iter().map(|g| g.id()).collect();
        remained_groups.iter_mut().for_each(|g| {
            let chains = &mut g.value_as_mut().unwrap().chains;
            chains
                .precede_groups
                .retain(|h| !released_group_ids.iter().any(|&i| i == h.id()));
            chains
                .success_groups
                .retain(|h| !released_group_ids.iter().any(|&i| i == h.id()));
        });

        // End.
        self.groups = remained_groups;
    }

    /// Remove invalidated tasks from list of valid groups.
    pub fn rearrange_tasks(&mut self) {
        for group in &mut self.groups {
            if let Some(mut group) = group.value_as_mut() {
                group.rearrange_tasks();
            }
        }
    }
}
