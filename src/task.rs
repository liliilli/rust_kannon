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

/// Task type that stores lambda function closure.
struct TaskClosure {
    f: Box<dyn Fn()>,
}

impl Functor for TaskClosure {
    /// Call inside closure.
    fn call(&self) {
        (self.f)()
    }
}

/// Task type that stores valid item's pointer and valid method reference of item.
///
/// This only can store `&T` const method, use `TaskMethodMut` if using mutable method of `&mut T`.
/// Binded item's pointer should not be invalidated, or moved.
/// Calling moved item's method may be occur undefined behavior by following additional logics.
struct TaskMethod<T, F> {
    t: NonNull<T>,
    f: F,
}

impl<T, F> Functor for TaskMethod<T, F>
where
    F: Fn(&T),
{
    // Call const method.
    fn call(&self) {
        (self.f)(unsafe { self.t.as_ref() })
    }
}

/// Task type that stores valid item's pointer and valid muable method reference of item.
///
/// This only can store `&mut T` mutable method, use `TaskMethod` if want to use immutable method of `&T`.
/// Binded item's pointer should not be invalidated, or moved.
/// Calling moved item's method may be occur undefined behavior by following additional logics.
struct TaskMethodMut<T, F> {
    t: RefCell<NonNull<T>>,
    f: F,
}

impl<T, F> Functor for TaskMethodMut<T, F>
where
    F: Fn(&mut T),
{
    // Call mutable method.
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
    /// Create task which is binding lambda closure.
    ///
    /// Given name must be valid and not empty. It's ok to be duplicated with other task's name.
    fn from_closure(name: &str, f: Box<dyn Fn()>) -> Self {
        assert!(name.is_empty() == false, "Task name must not be empty.");
        Self {
            name: name.to_string(),
            func: Box::new(TaskClosure { f }),
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
    fn from_method<T, F>(name: &'_ str, t: &T, f: F) -> Self
    where
        T: 'static,
        F: Fn(&T) + 'static,
    {
        assert!(name.is_empty() == false, "Task name must not be empty.");
        let t = NonNull::new(t as *const _ as *mut T).unwrap();

        Self {
            name: name.to_string(),
            func: Box::new(TaskMethod { t, f }),
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
    /// Create task which is binding lambda closure.
    ///
    /// Given name must be valid and not empty. It's ok to be duplicated with other task's name.
    pub(crate) fn from_closure(name: &str, f: Box<dyn Fn()>) -> Self {
        let raw = TaskRaw::from_closure(name, f);
        Self {
            raw: Arc::new(Mutex::new(raw)),
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

    /// Create task which is binding item's pointer and valid mutable method from the item.
    ///
    /// Given name must be valid and not empty. It's ok to be duplicated with other task's name.
    /// Being binded item should not be invalidated, or moved state.
    /// Otherwise, calling invalidated item's immutable method will be undefined behavior.
    ///
    /// Calling method of task may not invalidate borrowing rule, but care about synchronization
    /// and data race manually in the logic.
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

    /// Get new handle of the task.
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

/// Handle type for the task in arbitrary group.
///
///
#[derive(Clone)]
pub struct TaskHandle {
    value: Weak<Mutex<TaskRaw>>,
}

impl TaskHandle {
    /// Access to the task execusively and return accessor `TaskAccssor` item.
    ///
    /// If actual task item is invalidated, do nothing just return `None` value.
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

    /// Check task is released or not.
    pub fn is_released(&self) -> bool {
        self.value.strong_count() == 0
    }
}

/// Accessor item type for task.
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
