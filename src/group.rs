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
    tasks: Vec<TaskHandle>,
    /// Stores chaining information to other groups.
    chains: GroupChains,
}

impl GroupRaw {
    fn new(name: &str, id: usize) -> Self {
        Self {
            name: name.to_string(),
            id,
            tasks: vec![],
            chains: GroupChains::default(),
        }
    }

    ///
    ///
    ///
    pub fn call_all(&self) {
        for task in &self.tasks {
            match task.value_as_ref() {
                None => continue,
                Some(accessor) => accessor.call(),
            }
        }
    }
}

/// Stores chaining informations to other groups.
#[derive(Default)]
struct GroupChains {
    ///
    precede_group_list: Vec<GroupHandle>,
    ///
    success_group_list: Vec<GroupHandle>,
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
    pub fn call_all(&self) {
        let guard = self.raw.lock().unwrap();
        guard.call_all();
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

impl<'a> MutGroupAccessor<'a> {
    ///
    ///
    ///
    fn precede_group(&mut self, group: GroupHandle) -> Result<(), error::TaskError> {
        if self.guard.id == group.id {
            return Err(error::TaskError::InvalidChaining);
        }

        // Chain each other.
        //self.success_group_list.push(group);
        //group.value_as_mut().unwrap().precede_group_list.push();
        Ok(())
    }
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
pub type GroupList = Vec<GroupHandle>;

///
///
///
#[must_use]
pub fn create_group(group_list: &mut GroupList, group_name: &str) -> Group {
    let group = Group::new(group_name);
    let group_handle = group.handle();

    group_list.push(group_handle);
    group
}

#[cfg(test)]
mod tests {
    #[test]
    fn testest() {
        use crate::*;
        let mut group_list = group::GroupList::default();

        let mut group1 = create_group(&mut group_list, "Hello world! group 1");
        println!("{}", group1.name());

        let group2 = create_group(&mut group_list, "Bye world! group 2");
        println!("{}", group2.name());

        let task1 = group1.create_task_as_closure(
            "Closure1",
            Box::new(|| {
                println!("Hello world! from group1");
            }),
        );
        println!("{}", task1.name());

        group1.call_all();
        task1.call();
    }
}
