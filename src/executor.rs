use std::sync::atomic::Ordering;
use std::sync::mpsc;

use super::error::TaskError;
use super::topology::{TaskNode, Topology};

///
///
///
pub struct Executor {
    topology: Option<Topology>,
}

impl Executor {
    ///
    ///
    ///
    pub fn new() -> Self {
        Self { topology: None }
    }

    ///
    ///
    ///
    pub fn set_topology(&mut self, topology: Topology) {
        self.topology = Some(topology);
    }

    ///
    ///
    ///
    pub fn reset_topology(&mut self) {
        self.topology = None;
    }

    ///
    ///
    ///
    pub fn execute(&self) -> Result<(), TaskError> {
        // Check topology is set.
        if self.topology.is_none() {
            return Err(TaskError::InvalidGroupHandle);
        }

        // Execute anyway.
        let (tx, rx) = mpsc::channel::<TaskNode>();

        // Insert root group's task into tx.
        let topology = self.topology.as_ref().unwrap();
        for root_group in &topology.root_groups {
            let root_group = root_group.upgrade().unwrap();

            for task in root_group.lock().unwrap().task_nodes.iter() {
                tx.send(task.clone()).unwrap();
            }
        }

        // Process tasks.
        loop {
            let task = rx.try_recv();
            if task.is_err() {
                // Check hitotu karano item wo irete, karano guru-pudemo tsugihe susumeru youni
                // suru.
                break;
            }

            // Execute task's closure if can.
            let task = task.unwrap();
            if let Some(accessor) = task.handle.value_as_ref() {
                accessor.call();
            };

            // Decrease group task counter by 1.
            let group = task.group_node.upgrade().unwrap();
            let group_lock = group.lock().unwrap();
            let last_count = group_lock.remained_task_cnt.fetch_sub(1, Ordering::Relaxed);

            // If last count is 1, we have to decrease counter of successing all groups as a signal.
            if last_count == 1 {
                for successor in &group_lock.successor_nodes {
                    let successor = successor.upgrade().unwrap();
                    let successor = successor.lock().unwrap();

                    // If decreasing group is ready, insert new tasks to tx.
                    let last_count = successor
                        .remained_predecessor_cnt
                        .fetch_sub(1, Ordering::Release);
                    if last_count == 1 {
                        for task in &successor.task_nodes {
                            tx.send(task.clone()).unwrap();
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
