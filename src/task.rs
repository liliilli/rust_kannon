use std::cell::RefCell;
use std::mem;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

/// Internal trait
trait Functor {
    /// Call binded function.
    fn call(&self);
}

///
///
///
struct TaskClosure {
    f: Box<dyn Fn()>,
}

impl Functor for TaskClosure {
    fn call(&self) {
        (self.f)()
    }
}

///
///
///
struct TaskMethod<T, F> {
    t: NonNull<T>,
    f: F,
}

impl<T, F> Functor for TaskMethod<T, F>
where
    F: Fn(&T),
{
    fn call(&self) {
        (self.f)(unsafe { self.t.as_ref() })
    }
}

struct TaskMethodMut<T, F> {
    t: RefCell<NonNull<T>>,
    f: F,
}

impl<T, F> Functor for TaskMethodMut<T, F>
where
    F: Fn(&mut T),
{
    fn call(&self) {
        (self.f)(unsafe { self.t.borrow_mut().as_mut() })
    }
}

/// Raw type for `Task` instance.
///
/// Stores actual informations for task.
pub struct TaskRaw {
    pub name: String,
    func: Box<dyn Functor>,
}

impl TaskRaw {
    ///
    ///
    ///
    fn from_closure(name: &str, f: Box<dyn Fn()>) -> Self {
        Self {
            name: name.to_string(),
            func: Box::new(TaskClosure { f }),
        }
    }

    ///
    ///
    ///
    fn from_method<T, F>(name: &'_ str, t: &T, f: F) -> Self
    where
        T: 'static,
        F: Fn(&T) + 'static,
    {
        let t = NonNull::new(t as *const _ as *mut T).unwrap();

        Self {
            name: name.to_string(),
            func: Box::new(TaskMethod { t, f }),
        }
    }

    ///
    ///
    ///
    fn from_method_mut<T, F>(name: &'_ str, t: &mut T, f: F) -> Self
    where
        T: 'static,
        F: Fn(&mut T) + 'static,
    {
        let t = RefCell::new(NonNull::new(t as *mut T).unwrap());

        Self {
            name: name.to_string(),
            func: Box::new(TaskMethodMut { t, f }),
        }
    }

    /// Call binded function (closure, or methods).
    pub(crate) fn call(&self) {
        self.func.call()
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
    pub(crate) fn from_closure(name: &str, f: Box<dyn Fn()>) -> Self {
        let raw = TaskRaw::from_closure(name, f);
        Self {
            raw: Arc::new(Mutex::new(raw)),
        }
    }

    ///
    ///
    ///
    pub(crate) fn from_method<T, F>(name: &'_ str, t: &T, f: F) -> Self
    where
        T: 'static,
        F: Fn(&T) + 'static,
    {
        let raw = TaskRaw::from_method(name, t, f);
        Self {
            raw: Arc::new(Mutex::new(raw)),
        }
    }

    ///
    ///
    ///
    pub(crate) fn from_method_mut<T, F>(name: &'_ str, t: &mut T, f: F) -> Self
    where
        T: 'static,
        F: Fn(&mut T) + 'static,
    {
        let raw = TaskRaw::from_method_mut(name, t, f);
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
    pub(crate) fn call(&self) {
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
