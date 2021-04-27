#![allow(dead_code)]
#![feature(allocator_api)]
mod error;
mod group;
mod task;

use group::*;

struct Manager {
    groups: GroupList,
}

impl Manager {
    ///
    ///
    ///
    pub fn new() -> Self {
        Self {
            groups: GroupList::default(),
        }
    }

    ///
    ///
    ///
    #[must_use]
    pub fn create_group(&mut self, group_name: &str) -> Option<Group> {
        Some(create_group(&mut self.groups, group_name))
    }

    ///
    ///
    ///
    pub fn call_all(&self) {
        for group in &self.groups {
            match group.value_as_ref() {
                None => continue,
                Some(accessor) => accessor.call_all(),
            }
        }
    }

    ///
    ///
    ///
    pub fn cleanup_all(&mut self) {
        todo!("Not yet implemented");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn simple_test() {
        use crate::*;
        use std::sync::{Arc, Mutex};

        let mut manager = Manager::new();
        let mut group = manager.create_group("Group name").unwrap();
        let _task = group.create_task_as_closure(
            "Task1",
            Box::new(|| {
                println!("Hello world! from Task1.");
            }),
        );
        manager.call_all();

        let from_outside: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
        let mut group2 = manager.create_group("Group Name2").unwrap();

        let cloned = Arc::clone(&from_outside);
        let _task2 = group2.create_task_as_closure(
            "Task2",
            Box::new(move || {
                println!("Hello world! from Task2.");
                let mut cloned_guard = cloned.lock().unwrap();
                *cloned_guard += 180;
            }),
        );
        manager.call_all();

        let cloned = Arc::clone(&from_outside);
        let _task3 = group2.create_task_as_closure(
            "Task3",
            Box::new(move || {
                let cloned_guard = cloned.lock().unwrap();
                println!("From Task3, value is {}", *cloned_guard);
            }),
        );

        manager.call_all();
    }
}
