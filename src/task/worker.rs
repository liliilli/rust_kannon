use std::{
    cmp,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

extern crate crossbeam_deque;
extern crate crossbeam_utils;

use super::{
    error::TaskError,
    topology::{TaskNode, Topology},
};

/// Default worker trait for executing tasks in the various ways.
pub trait Worker {
    /// Ready worker with given topology `topology::Topology`.
    ///
    /// If ready is failed, return error code.
    fn ready(&self, topology: &Topology) -> Result<(), TaskError>;

    /// Execute worker and process tasks.
    ///
    /// If ready is failed, return error code.
    fn execute(&self) -> Result<(), TaskError>;

    ///
    ///
    ///
    fn wait_finish(&self);
}

/// Worker variation type which process tasks sequentially.
pub struct SequentialWorker {
    tx: mpsc::Sender<TaskNode>,
    rx: mpsc::Receiver<TaskNode>,
    task_count: AtomicUsize,
}

impl SequentialWorker {
    /// Create new sequential worker.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<TaskNode>();
        Self {
            tx,
            rx,
            task_count: AtomicUsize::new(0),
        }
    }
}

impl Worker for SequentialWorker {
    fn ready(&self, topology: &Topology) -> Result<(), TaskError> {
        // Insert root group's task into tx.
        for root_group in &topology.root_groups {
            let root_group = root_group.upgrade().unwrap();

            for task in &root_group.lock().unwrap().task_nodes {
                self.tx.send(task.clone()).unwrap();
            }
        }
        self.task_count
            .store(topology.task_count, Ordering::Relaxed);

        Ok(())
    }

    fn execute(&self) -> Result<(), TaskError> {
        // Process tasks.
        loop {
            let task = self.rx.try_recv();
            if task.is_err() {
                assert!(
                    self.task_count.load(Ordering::Relaxed) == 0,
                    "Topology's total task count must be matched."
                );
                break;
            }

            // Execute task's closure if can.
            let task = task.unwrap();
            if let Some(accessor) = task.handle.value_as_ref() {
                accessor.call();
            };

            // Decrease group task counter by 1.
            self.task_count.fetch_sub(1, Ordering::Relaxed);
            let group = task.group_node.upgrade().unwrap();
            let group_lock = group.lock().unwrap();
            let last_count = group_lock.decrease_task_count();

            // If last count is 1, we have to decrease counter of successing all groups as a signal.
            if last_count == 1 {
                for successor in &group_lock.successor_nodes {
                    let successor = successor.upgrade().unwrap();
                    let successor = successor.lock().unwrap();

                    // If decreasing group is ready, insert new tasks to tx.
                    let last_count = successor.decrease_predecessor_count();
                    if last_count == 1 {
                        for task in &successor.task_nodes {
                            self.tx.send(task.clone()).unwrap();
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn wait_finish(&self) {
        let backoff = crossbeam_utils::Backoff::new();
        while self.task_count.load(Ordering::Relaxed) != 0 {
            backoff.spin();
        }
    }
}

///
///
///
struct BlockedThreads {
    ///
    list: Vec<thread::Thread>,
    ///
    insertable: bool,
}

impl BlockedThreads {
    pub fn new() -> Self {
        Self {
            list: vec![],
            insertable: true,
        }
    }

    ///
    pub fn is_insertable(&self) -> bool {
        self.insertable
    }

    ///
    pub fn push(&mut self, thread: thread::Thread) {
        assert!(
            self.insertable == true,
            "This function must only be called when is_insertable() is true."
        );
        self.list.push(thread);
    }

    ///
    ///
    ///
    pub fn try_unparks_of(&mut self, count: usize) {
        self.list
            .drain(0..count)
            .into_iter()
            .for_each(|t| t.unpark());
    }

    ///
    ///
    ///
    pub fn unpark_all(&mut self) {
        self.list.drain(..).into_iter().for_each(|t| t.unpark());
    }
}

///
///
///
pub struct ThreadingWorker {
    ///
    global_fifo: Arc<crossbeam_deque::Injector<TaskNode>>,
    ///
    threads: Vec<JoinHandle<()>>,
    ///
    blocked_threads: Arc<Mutex<BlockedThreads>>,
    ///
    is_worker_terminated: Arc<AtomicBool>,
    ///
    task_count: Arc<AtomicUsize>,
}

impl ThreadingWorker {
    /// Create new parallel processing worker item with hardware_concurrency thread count.
    pub fn try_new_automatic() -> Option<Self> {
        let available_concurrency = thread::available_concurrency()
            .map(|n| n.get())
            .unwrap_or(1);
        Self::try_new(available_concurrency)
    }

    ///
    ///
    ///
    pub fn try_new(hardware_concurrency: usize) -> Option<Self> {
        if hardware_concurrency == 0 {
            return None;
        }

        let is_worker_terminated = Arc::new(AtomicBool::new(false));
        let global_fifo = Arc::new(crossbeam_deque::Injector::<TaskNode>::new());
        let blocked_threads = Arc::new(Mutex::new(BlockedThreads::new()));
        let task_count = Arc::new(AtomicUsize::new(0));

        // Create threads and related data.
        let threads: Vec<_> = (0..hardware_concurrency)
            .into_iter()
            .map(|id| {
                // Clone items.
                let is_worker_terminated = is_worker_terminated.clone();
                let global_fifo = global_fifo.clone();
                let blocked_threads = blocked_threads.clone();
                let task_count = task_count.clone();
                let backoff = crossbeam_utils::Backoff::new();

                // Build thread.
                thread::Builder::new()
                    .name(format!("ThreadingWorker thread_index:{}", id).into())
                    .spawn(move || loop {
                        // If workers are terminated, we have to exit.
                        if is_worker_terminated.load(Ordering::Acquire) {
                            return ();
                        }

                        // Get task except for received termination signal.
                        let task = loop {
                            let t = global_fifo.steal();
                            if t.is_success() {
                                backoff.reset();
                                break t.success().unwrap();
                            }
                            if t.is_empty() {
                                let is_inserted = {
                                    let mut guard = blocked_threads.lock().unwrap();
                                    if guard.is_insertable() {
                                        guard.push(thread::current());
                                        true
                                    } else {
                                        false
                                    }
                                };
                                if is_inserted {
                                    thread::park();
                                }

                                if is_worker_terminated.load(Ordering::SeqCst) {
                                    return ();
                                }
                            }

                            // We have to wait thread for a while for retrying stealing.
                            backoff.spin();
                        };
                        if let Some(accessor) = task.handle.value_as_ref() {
                            accessor.call();
                        };

                        // Decrease group task counter by 1.
                        task_count.fetch_sub(1, Ordering::AcqRel);
                        let group = task.group_node.upgrade().unwrap();
                        let group = group.lock().unwrap();
                        let cnt = group.decrease_task_count();

                        // If last count is 1, we have to decrease counter of successing all groups as a signal.
                        // This is thread-safe and one more thread can not be proceeded in.
                        if cnt == 1 {
                            for successor in &group.successor_nodes {
                                let successor = successor.upgrade().unwrap();
                                let successor = successor.lock().unwrap();

                                // If decreasing group is ready, insert new tasks to tx.
                                // This is thread-safe and one more thread can not be proceed in.
                                let last_count = successor.decrease_predecessor_count();
                                if last_count == 1 {
                                    let wake_count = cmp::min(
                                        successor.task_count() as usize,
                                        hardware_concurrency,
                                    );
                                    for task in &successor.task_nodes {
                                        global_fifo.push(task.clone());
                                    }

                                    // Weak up list.
                                    let mut guard = blocked_threads.lock().unwrap();
                                    guard.try_unparks_of(wake_count);
                                }
                            }
                        }
                    })
                    .unwrap()
            })
            .collect();

        Some(Self {
            global_fifo,
            threads,
            blocked_threads,
            is_worker_terminated,
            task_count,
        })
    }
}

impl Worker for ThreadingWorker {
    fn ready(&self, topology: &Topology) -> Result<(), TaskError> {
        // Set task count.
        // Counter mut be set before insertion of tasks.
        self.task_count.store(topology.task_count, Ordering::SeqCst);

        // Insert root group's task into tx.
        for root_group in &topology.root_groups {
            let root_group = root_group.upgrade().unwrap();

            for task in &root_group.lock().unwrap().task_nodes {
                self.global_fifo.push(task.clone());
            }
        }

        Ok(())
    }

    fn execute(&self) -> Result<(), TaskError> {
        let mut threads = self.blocked_threads.lock().unwrap();
        threads.unpark_all();

        Ok(())
    }

    fn wait_finish(&self) {
        let backoff = crossbeam_utils::Backoff::new();
        while self.task_count.load(Ordering::Relaxed) != 0 {
            backoff.spin();
        }
    }
}

impl Drop for ThreadingWorker {
    fn drop(&mut self) {
        self.is_worker_terminated.store(true, Ordering::SeqCst);
        self.wait_finish();
        {
            let mut threads = self.blocked_threads.lock().unwrap();
            threads.insertable = false;
            threads.unpark_all();
        }

        self.threads.drain(..).for_each(|h| h.join().unwrap());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn thread_test1() {
        use std::thread;

        let available_concurrency = thread::available_concurrency()
            .map(|n| n.get())
            .unwrap_or(1);

        let threads = (0..available_concurrency)
            .into_iter()
            .map(|cnt| {
                thread::spawn(move || {
                    println!("This is thread {}.", cnt);
                })
            })
            .collect::<Vec<_>>();

        for thread in threads.into_iter() {
            thread.join().unwrap();
        }
    }
}
