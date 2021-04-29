#![allow(dead_code)]
#![feature(allocator_api)]
mod error;
mod executor;
mod group;
mod task;
mod topology;

use group::*;
use topology::Topology;

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
    pub fn is_cyclic(&self) -> bool {
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

    ///
    ///
    ///
    pub fn call_all2(&self) {
        match Topology::try_from(&self.groups) {
            None => return,
            Some(topology) => {
                let mut executor = executor::Executor::new();
                executor.set_topology(topology);
                executor.execute().unwrap();
                println!("good!");
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

        let _task2 = {
            let cloned = Arc::clone(&from_outside);
            group2.create_task_as_closure(
                "Task2",
                Box::new(move || {
                    println!("Hello world! from Task2.");
                    let mut cloned_guard = cloned.lock().unwrap();
                    *cloned_guard += 180;
                }),
            )
        };
        manager.call_all();

        let _task3 = {
            let cloned = Arc::clone(&from_outside);
            group2.create_task_as_closure(
                "Task3",
                Box::new(move || {
                    let cloned_guard = cloned.lock().unwrap();
                    println!("From Task3, value is {}", *cloned_guard);
                }),
            )
        };

        // Group2 should be called before Group.
        group2.precede(group.handle()).unwrap();

        // We want to bind
        manager.call_all2();
    }
}
