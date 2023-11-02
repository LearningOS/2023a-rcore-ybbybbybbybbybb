//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::{MAX_SYSCALL_NUM, BIG_STRIDE},
    loader::get_app_data_by_name,
    timer::*,
    mm::{VirtAddr, MapPermission, StepByOne, translated_byte_buffer, translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus, TaskControlBlock,
    },
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

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    // trace!(
    //     "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
    //     current_task().unwrap().pid.0
    // );
    // -1
    if ts.is_null() {
        return -1;
    }
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *translated_refmut(current_user_token(), &mut (*ts).sec  as *mut usize) = us / 1000000;
        *translated_refmut(current_user_token(), &mut (*ts).usec as *mut usize) = us % 1000000;
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if ti.is_null() {
        return -1;
    }
    let current_task = current_task().unwrap();
    let task_inner = current_task.inner_exclusive_access();
    let task_info = TaskInfo{
        status: task_inner.task_status,
        syscall_times: task_inner.task_syscall_times,
        time: get_time_ms() - task_inner.task_first_start_time,
    };
    let task_info_ptr = &task_info as *const _ as *const u8;
    let task_info_len = core::mem::size_of::<TaskInfo>();
    let task_info_slice = unsafe { core::slice::from_raw_parts(task_info_ptr, task_info_len) };
    let mut buffers = translated_byte_buffer(current_user_token(), ti as *const u8, task_info_len);
    buffers[0][..task_info_len].copy_from_slice(task_info_slice);
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // -1
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);
    let permission = MapPermission::from_bits_truncate((port << 1) as u8) | MapPermission::U;
    if !start_va.aligned() {
        return -1;
    } else if port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    let current_task = current_task().unwrap();
    let mut task_inner = current_task.inner_exclusive_access();
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();
    let mut vpn = start_vpn;
    while vpn < end_vpn {
        println!("now:{}", vpn.0);
        if let Some(pte) = task_inner.memory_set.translate(vpn) {
            if pte.is_valid() {
                return -1;
            }
        }
        vpn.step();
    }
    task_inner.memory_set.insert_framed_area(start_va, end_va, permission);
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // -1
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);
    if !start_va.aligned() {
        return -1;
    }
    let current_task = current_task().unwrap();
    let mut task_inner = current_task.inner_exclusive_access();
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();
    let mut vpn = start_vpn;
    println!("2 statr:{}, end:{}", start_vpn.0, end_vpn.0);
    while vpn < end_vpn {
        println!("2 now:{}", vpn.0);
        match task_inner.memory_set.translate(vpn) {
            Some(pte) => {
                if !pte.is_valid() {
                    return -1;
                }
            }, 
            None => {
                return -1;
            },
        }
        vpn.step();
    }
    task_inner.memory_set.shrink_to(start_va, start_va);
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    // trace!(
    //     "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
    //     current_task().unwrap().pid.0
    // );
    // -1
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(elf_data) = get_app_data_by_name(path.as_str()) {
        let new_task = Arc::new(TaskControlBlock::new(elf_data));
        let new_pid = new_task.pid.0;
    
        let current_task = current_task().unwrap();
        // let cloned_new_task = new_task.clone();
        let mut new_task_inner = new_task.inner_exclusive_access();
        new_task_inner.parent = Some(Arc::downgrade(&current_task));
        drop(new_task_inner);
        let mut parent_inner = current_task.inner_exclusive_access();
        parent_inner.children.push(new_task.clone());
        add_task(new_task);
    
        return new_pid as isize;
    } else {
        return -1;
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if prio <= 1 {
        return -1;
    }
    let current_task = current_task().unwrap();
    let mut task_inner = current_task.inner_exclusive_access();
    task_inner.pass = BIG_STRIDE / prio as usize;
    prio
}

/// 
pub fn sys_add_syscall_times(syscall_id: usize) {
    let current_task = current_task().unwrap();
    let mut task_inner = current_task.inner_exclusive_access();
    task_inner.add_syscall_times(syscall_id);
}
