use super::error;
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
    /// Local task handle list for calling tasks in batch.
    pub(crate) tasks: Vec<TaskHandle>,
    /// Stores chaining information to other groups.
    pub(crate) chains: GroupChains,
}

impl GroupRaw {
    ///
    ///
    ///
    fn new(name: &str, id: usize) -> Self {
        Self {
            name: name.to_string(),
            id,
            tasks: vec![],
            chains: GroupChains::default(),
        }
    }

    /// Check group has successor groups.
    pub fn has_successors(&self) -> bool {
        self.chains.success_group_list.is_empty()
    }

    /// Check group has predecessor groups.
    pub fn has_predecessors(&self) -> bool {
        self.chains.precede_group_list.is_empty()
    }
}

/// Stores chaining informations to other groups.
#[derive(Default)]
pub(crate) struct GroupChains {
    ///
    pub(crate) precede_group_list: Vec<GroupHandle>,
    ///
    pub(crate) success_group_list: Vec<GroupHandle>,
}

/// Task group unit.
pub struct Group {
    raw: Arc<Mutex<GroupRaw>>,
}

impl Group {
    /// Creat new group with valid informatons.
    fn new(group_name: &str) -> Self {
        static mut ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

        let id = unsafe { ID_COUNTER.fetch_add(1, Ordering::Relaxed) };
        Self {
            raw: Arc::new(Mutex::new(GroupRaw::new(group_name, id))),
        }
    }

    /// Get the name of the group.
    pub fn name(&self) -> String {
        self.raw.lock().unwrap().name.clone()
    }

    /// Get handle item of this group.
    pub fn handle(&self) -> GroupHandle {
        GroupHandle {
            value: Arc::downgrade(&self.raw),
            id: self.raw.lock().unwrap().id,
        }
    }

    /// Create task which has closure instance.
    ///
    ///
    #[must_use]
    pub fn create_task_as_closure(&mut self, task_name: &str, closure: Box<dyn Fn()>) -> Task {
        let task = Task::from_closure(task_name, closure);
        let task_handle = task.handle();

        let mut raw = self.raw.lock().unwrap();
        raw.tasks.push(task_handle);

        task
    }

    ///
    ///
    ///
    ///
    #[must_use]
    pub fn create_task(
        &mut self,
        name: &str,
        f: impl Fn() + 'static,
    ) -> Result<Task, error::TaskError> {
        if name.is_empty() {
            Err(error::TaskError::InvalidItemName)
        } else {
            let f = Box::new(f);
            let task = Task::from_closure(name, f);
            let task_handle = task.handle();

            let mut raw = self.raw.lock().unwrap();
            raw.tasks.push(task_handle);

            Ok(task)
        }
    }

    ///
    ///
    ///
    #[must_use]
    pub fn create_task_method<T, F>(
        &mut self,
        name: &str,
        t: &T,
        f: F,
    ) -> Result<Task, error::TaskError>
    where
        T: 'static,
        F: Fn(&T) + 'static,
    {
        if name.is_empty() {
            Err(error::TaskError::InvalidItemName)
        } else {
            let task = Task::from_method(name, t, f);
            let task_handle = task.handle();

            let mut raw = self.raw.lock().unwrap();
            raw.tasks.push(task_handle);

            Ok(task)
        }
    }

    ///
    ///
    ///
    #[must_use]
    pub fn create_task_method_mut<T, F>(
        &mut self,
        name: &str,
        t: &mut T,
        f: F,
    ) -> Result<Task, error::TaskError>
    where
        T: 'static,
        F: Fn(&mut T) + 'static,
    {
        if name.is_empty() {
            Err(error::TaskError::InvalidItemName)
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
    pub fn precede(&mut self, handle: GroupHandle) -> Result<(), error::TaskError> {
        let this_handle = self.handle();
        let mut guard = self.raw.lock().unwrap();
        if guard.id == handle.id {
            // Same group can not be chain each other.
            Err(error::TaskError::InvalidChaining)
        } else {
            // Check given handle is already inserted into the lists (precede and success).
            let other_id = handle.id;
            let this_predeces = &guard.chains.precede_group_list;
            if this_predeces.iter().any(|x| x.id == other_id) {
                return Err(error::TaskError::InvalidChaining);
            }
            let this_successors = &guard.chains.success_group_list;
            if this_successors.iter().any(|x| x.id == other_id) {
                return Err(error::TaskError::InvalidChaining);
            }
            // this_predeces and this_successors will not be used anymore.

            let mut other_handle = handle.clone();
            let mut other_group = match other_handle.value_as_mut() {
                None => return Err(error::TaskError::InvalidGroupHandle),
                Some(accessor) => accessor,
            };

            // Make chain relation.
            guard.chains.success_group_list.push(handle);
            other_group.chains.precede_group_list.push(this_handle);
            Ok(())
        }
    }
}

///
///
///
#[derive(Clone)]
pub struct GroupHandle {
    value: Weak<Mutex<GroupRaw>>,
    id: usize,
}

impl GroupHandle {
    ///
    ///
    ///
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

    ///
    ///
    ///
    pub fn value_as_mut<'a>(&'a mut self) -> Option<MutGroupAccessor<'a>> {
        let group = self.value.upgrade()?;
        let group_lock = group.lock();
        if let Ok(guard) = group_lock {
            let guard: MutexGuard<'a, GroupRaw> = unsafe { mem::transmute(guard) };
            Some(MutGroupAccessor { guard })
        } else {
            None
        }
    }

    ///
    ///
    ///
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

///
///
///
pub struct GroupAccessor<'a> {
    guard: MutexGuard<'a, GroupRaw>,
}

impl<'a> Deref for GroupAccessor<'a> {
    type Target = GroupRaw;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

///
///
///
pub struct MutGroupAccessor<'a> {
    guard: MutexGuard<'a, GroupRaw>,
}

impl<'a> Deref for MutGroupAccessor<'a> {
    type Target = GroupRaw;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a> DerefMut for MutGroupAccessor<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

///
///
///
pub(crate) type GroupList = Vec<GroupHandle>;

///
///
///
#[must_use]
pub(crate) fn create_group(groups: &mut GroupList, name: &str) -> Group {
    let group = Group::new(name);
    let group_handle = group.handle();

    groups.push(group_handle);
    group
}
