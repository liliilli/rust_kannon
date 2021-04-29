use std::mem;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

///
///
///
enum EItemType {
    Closure(Box<dyn Fn()>),
}

/// Raw type for `Task` instance.
///
/// Stores actual informations for task.
pub struct TaskRaw {
    pub name: String,
    func: EItemType,
}

impl TaskRaw {
    fn from_closure(name: &str, func: Box<dyn Fn()>) -> Self {
        Self {
            name: name.to_string(),
            func: EItemType::Closure(func),
        }
    }

    pub fn call(&self) {
        match &self.func {
            EItemType::Closure(func) => func(),
        }
    }
}

/// Task instance which callable in any thread context in the system.
pub struct Task {
    raw: Arc<Mutex<TaskRaw>>,
}

impl Task {
    ///
    ///
    ///
    pub fn from_closure(task_name: &str, func: Box<dyn Fn()>) -> Self {
        let raw = TaskRaw::from_closure(task_name, func);
        Self {
            raw: Arc::new(Mutex::new(raw)),
        }
    }

    /// Get the name of the task.
    pub fn name(&self) -> String {
        self.raw.lock().unwrap().name.clone()
    }

    ///
    ///
    ///
    pub fn handle(&self) -> TaskHandle {
        TaskHandle {
            value: Arc::downgrade(&self.raw),
        }
    }

    /// Call task's function.
    ///
    /// # Notes
    ///
    /// Maybe performance down by locking whenever calling callbacks.
    pub fn call(&self) {
        self.raw.lock().unwrap().call();
    }
}

///
///
///
#[derive(Clone)]
pub struct TaskHandle {
    value: Weak<Mutex<TaskRaw>>,
}

impl TaskHandle {
    ///
    ///
    ///
    pub fn value_as_ref<'a>(&'a self) -> Option<TaskAccessor<'a>> {
        let task = self.value.upgrade()?;
        let task_lock = task.lock();
        if let Ok(task_guard) = task_lock {
            // Warning!
            let task_guard: MutexGuard<'a, TaskRaw> = unsafe { mem::transmute(task_guard) };
            Some(TaskAccessor { task_guard })
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
}

///
///
///
pub struct TaskAccessor<'a> {
    task_guard: MutexGuard<'a, TaskRaw>,
}

impl<'a> Deref for TaskAccessor<'a> {
    type Target = TaskRaw;

    fn deref(&self) -> &Self::Target {
        self.task_guard.deref()
    }
}
