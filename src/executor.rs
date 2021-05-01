use super::error::TaskError;
use super::topology::Topology;
use super::worker::Worker;

///
///
///
pub struct Executor {
    topology: Option<Topology>,
    worker: Option<Box<dyn Worker>>,
}

impl Executor {
    ///
    ///
    ///
    pub fn new() -> Self {
        Self {
            topology: None,
            worker: None,
        }
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
    pub fn exchange_worker(&mut self, worker: Box<dyn Worker>) -> Option<Box<dyn Worker>> {
        let old_worker = self.worker.take();
        self.worker = Some(worker);
        old_worker
    }

    ///
    ///
    ///
    pub fn detach_worker(&mut self) -> Option<Box<dyn Worker>> {
        self.worker.take()
    }

    ///
    ///
    ///
    pub fn execute(&self) -> Result<(), TaskError> {
        // Check topology is set.
        if self.topology.is_none() {
            return Err(TaskError::InvalidGroupHandle);
        }
        if self.worker.is_none() {}

        let worker = self.worker.as_ref().unwrap();
        worker.ready(self.topology.as_ref().unwrap()).unwrap();
        worker.execute().unwrap();
        worker.wait_finish();

        Ok(())
    }
}
