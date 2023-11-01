//! Process management syscalls

use crate::{
    config::MAX_SYSCALL_NUM,
    task::*,
    timer::get_time_us,
    mm::{modify_usize_address, VirtAddr, MapPermission},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// implement for task info
impl TaskInfo {
    /// new a task info
    pub fn new(status: TaskStatus, syscall_times: [u32; MAX_SYSCALL_NUM], time: usize) -> Self {
        TaskInfo {
            status,
            syscall_times,
            time,
        }
    }
}


/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us(); 
    unsafe {
        modify_usize_address(current_user_token(), &(*ts).sec  as *const _ as usize, us / 1_000_000);
        modify_usize_address(current_user_token(), &(*ts).usec as *const _ as usize, us % 1_000_000);
    }
    0
    // -1
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    // trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    trace!("kernel: sys_task_info");
    get_current_task_info(ti)
}

// use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};
/// sys mmap
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);
    /// 与 范天奇，叶可禾交流port转MapPermission
    let permission = MapPermission::from_bits_truncate((port << 1) as u8) | MapPermission::U;
    if !start_va.aligned() {
        return -1;
    } else if port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    // let permission = MapPermission::R | MapPermission::W;
    current_task_mmap(start_va, end_va, permission)
}

// munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);
    if !start_va.aligned() {
        return -1;
    }
    current_task_munmap(start_va, end_va)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
