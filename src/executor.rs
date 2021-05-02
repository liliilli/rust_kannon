use std::cell::Cell;

use super::error::TaskError;
use super::topology::Topology;
use super::worker::Worker;

/// The type which can execute created topology using inserted worker.
pub struct Executor {
    /// Stores topology instance to process.
    topology: Option<Topology>,
    /// Stores worker instance.
    worker: Option<Box<dyn Worker>>,
    /// Check flag for executor is executed now or not.
    is_executed: Cell<bool>,
}

impl Executor {
    /// Create new executor which can process tasks of topology `topology::Topology` using worker which implement
    /// `worker::Worker` trait.
    pub fn new() -> Self {
        Self {
            topology: None,
            worker: None,
            is_executed: Cell::new(false),
        }
    }

    /// Exchange `topology` with new moved `topology`.
    ///
    /// This function does nothing when this executor is being executed but return with error.
    /// Exchanged old topology will be returned when successful.
    pub fn exchange_topology(&mut self, topology: Topology) -> Result<Option<Topology>, TaskError> {
        if self.is_executed() {
            Err(TaskError::AlreadyExecuted)
        } else {
            let old_topology = self.topology.take();
            self.topology = Some(topology);
            Ok(old_topology)
        }
    }

    /// Detach `topology` and leave this executor empty state.
    ///
    /// If executor is already being executed and not finished,
    /// this function do nothing but return with error value.
    pub fn detach_topology(&mut self) -> Result<Option<Topology>, TaskError> {
        if self.is_executed() {
            Err(TaskError::AlreadyExecuted)
        } else {
            Ok(self.topology.take())
        }
    }

    /// Exchange `worker` with new moved `worker`.
    ///
    /// If executor is already being executed and not finished,
    /// do nothing but return with error value.
    pub fn exchange_worker(
        &mut self,
        worker: Box<dyn Worker>,
    ) -> Result<Option<Box<dyn Worker>>, TaskError> {
        if self.is_executed() {
            Err(TaskError::AlreadyExecuted)
        } else {
            let old_worker = self.worker.take();
            self.worker = Some(worker);
            Ok(old_worker)
        }
    }

    /// Detach 'worker' and leave this executor empty state.
    ///
    /// If executor is already being executed and not finished,
    /// this function do nothing but return with error value.
    pub fn detach_worker(&mut self) -> Result<Option<Box<dyn Worker>>, TaskError> {
        if self.is_executed() {
            Err(TaskError::AlreadyExecuted)
        } else {
            Ok(self.worker.take())
        }
    }

    /// Check executor is being executed.
    pub fn is_executed(&self) -> bool {
        self.is_executed.get()
    }

    /// Execute topology with set worker.
    ///
    /// If executed, user should check that execution is finished using `wait_finish` function.
    pub fn execute(&self) -> Result<(), TaskError> {
        // Check this executor is already executed.
        if self.is_executed() {
            return Err(TaskError::AlreadyExecuted);
        }

        // Check topology and worker are set..
        if self.topology.is_none() {
            return Err(TaskError::InvalidGroupHandle);
        }
        if self.worker.is_none() {
            return Err(TaskError::EmptyWorker);
        }

        let worker = self.worker.as_ref().unwrap();
        worker.ready(self.topology.as_ref().unwrap()).unwrap();
        worker.execute()?;

        self.is_executed.set(true);
        Ok(())
    }

    /// Wait until execution is finished.
    pub fn wait_finish(&self) -> Result<(), TaskError> {
        // Check this executor is idle.
        if !self.is_executed() {
            return Err(TaskError::AlreadyIdle);
        }

        // Check worker is exist.
        if self.worker.is_none() {
            return Err(TaskError::EmptyWorker);
        }

        let worker = self.worker.as_ref().unwrap();
        worker.wait_finish();

        self.is_executed.set(false);
        Ok(())
    }
}
