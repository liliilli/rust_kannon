use dy_tasksys;

trait TestTrait {
    fn print_something(&self);
}

struct TestStruct {
    phrase: String,
}

impl TestStruct {
    fn print_mutable(&mut self) {
        self.phrase += "!";
    }
}

impl TestTrait for TestStruct {
    fn print_something(&self) {
        println!("{}", self.phrase);
    }
}

#[test]
fn integration() {
    use dy_tasksys::task::{
        executor::Executor, group::GroupManager, topology::Topology, worker::ThreadingWorker,
    };
    use std::sync::{Arc, Mutex};

    let mut manager = GroupManager::new();
    let mut executor = Executor::new();
    executor
        .exchange_worker(Box::new(ThreadingWorker::try_new_automatic().unwrap()))
        .unwrap();
    let mut topology = Topology::new();

    let mut group = manager.create_group("Group name").unwrap();
    let _task = group.create_task("Task1", || {
        println!("Hello world! from Task1 of group1.");
    });

    let from_outside = Arc::new(Mutex::new(0i32));
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
    let _t32 = group3.create_task_method("Task1", &test_item, TestStruct::print_something);
    let _t33 = group3.create_task_method_mut("Task1", &mut test_item, TestStruct::print_mutable);

    for i in 0..100 {
        println!("Trial 1 {}", i);
        // Rearrange secion
        manager.rearrange_groups();
        manager.rearrange_tasks();
        topology.rearrange_from(manager.groups());

        // Execution section
        executor.exchange_topology(topology).unwrap();
        executor.execute().unwrap();
        executor.wait_finish().unwrap();

        topology = executor.detach_topology().unwrap().unwrap();
        println!("\n");
    }
}
