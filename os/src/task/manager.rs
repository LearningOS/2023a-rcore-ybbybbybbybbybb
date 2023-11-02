//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // self.ready_queue.pop_front()
        let mut len = self.ready_queue.len();
        if len == 0 {
            return None;
        }
        let mut tmp = self.ready_queue.pop_front().unwrap();
        while len > 1 {
            let mut now = self.ready_queue.pop_front().unwrap();
            if tmp.inner_exclusive_access().stride > now.inner_exclusive_access().stride {
                unsafe {
                    core::ptr::swap(&mut tmp as *mut _, &mut now as *mut _);
                }
            }
            self.ready_queue.push_back(now);
            len -= 1;
        }
        Some(tmp)
        // self.ready_queue
        //     .iter()
        //     .min_by(|&x, &y| {
        //         let x_inner = x.inner_exclusive_access();
        //         let y_inner = y.inner_exclusive_access();
        //         x_inner.stride.cmp(&y_inner.stride)
        //     })
        //     .cloned()
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
