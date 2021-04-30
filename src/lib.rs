#![allow(dead_code)]
#![feature(allocator_api)]
mod error;
mod executor;
mod group;
mod task;
mod topology;

use error::TaskError;
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

    /// Create new topology which can be executable tasks or return failure value.
    pub fn create_topology(&self) -> Result<Topology, TaskError> {
        Topology::try_from(&self.groups)
    }

    ///
    ///
    ///
    pub fn is_cyclic(&self) -> bool {
        todo!("Not yet implemented");
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
    pub fn cleanup_all(&mut self) {
        todo!("Not yet implemented");
    }
}

#[cfg(test)]
mod tests {
    trait TestTrait {
        fn print_something(&self);
    }

    struct TestStruct {
        phrase: String,
    }

    impl TestStruct {
        fn print_mutable(&mut self) {
            self.phrase += "Additional string!";
        }
    }

    impl TestTrait for TestStruct {
        fn print_something(&self) {
            println!("{}", self.phrase);
        }
    }

    #[test]
    fn simple_test() {
        use crate::*;
        use std::sync::{Arc, Mutex};

        let mut manager = Manager::new();
        let mut group = manager.create_group("Group name").unwrap();
        let _task = group.create_task("Task1", || {
            println!("Hello world! from Task1 of group1.");
        });

        let from_outside: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
        let mut group2 = manager.create_group("Group Name2").unwrap();

        let _task21 = {
            let cloned = Arc::clone(&from_outside);
            group2.create_task("Task21", move || {
                println!("Hello world! from Task21 of group2.");
                *cloned.lock().unwrap() += 180;
            })
        };

        let _task22 = {
            let cloned = Arc::clone(&from_outside);
            group2.create_task("Task22", move || {
                println!(
                    "From Task22 of group2, value is {}",
                    *cloned.lock().unwrap()
                );
            })
        };

        // Group2 should be called before Group.
        group2.precede(group.handle()).unwrap();

        let mut group3 = manager.create_group("Group Name3").unwrap();
        let _task31 = {
            let cloned = Arc::clone(&from_outside);
            group3.create_task("Task31", move || {
                println!("Call closure of Task31 of group3.");
                *cloned.lock().unwrap() -= 45;
            })
        };
        // Group3 should be called before Group2, so Group3 => Group2 => Group0.
        group3.precede(group2.handle()).unwrap();

        let mut test_item = TestStruct {
            phrase: "Hello from struct".to_string(),
        };
        let _task32 = group3.create_task_method("Task1", &test_item, TestStruct::print_something);
        let _task33 =
            group3.create_task_method_mut("Task1", &mut test_item, TestStruct::print_mutable);

        // Create topology and execute it.
        match manager.create_topology() {
            Err(_) => return,
            Ok(topology) => {
                let mut executor = executor::Executor::new();
                executor.set_topology(topology);
                executor.execute().unwrap();
                println!("\n\n");
            }
        }

        // Create topology and execute it.
        match manager.create_topology() {
            Err(_) => return,
            Ok(topology) => {
                let mut executor = executor::Executor::new();
                executor.set_topology(topology);
                executor.execute().unwrap();
                println!("\n\n");
            }
        }
    }
}
