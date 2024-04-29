//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
        inc_current_syscall_times,get_current_task_info,
    },
    timer::{get_time_ms, get_time_us},
    mm::translated_byte_buffer,
    task::current_user_token,
  
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
    let time = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    //获取数据长度
    let len: usize = core::mem::size_of::<TimeVal>();
    let buf: *const u8 = ts as *const u8;
    let mut buffers = translated_byte_buffer(current_user_token(), buf, len);
    let src_ptr=&time as * const TimeVal;
    // 注意此处不能获取buffers.as_mut_ptr()
    let dst_ptr: *mut TimeVal =buffers[0].as_mut_ptr() as * mut TimeVal;
    unsafe {
        //第三个参数应该是1 还是len ??
        core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, 1);
    }

    0

}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    // 获取当前毫秒
    let current_ms: usize=get_time_ms();
    let (status,syscall_times,start_time)=get_current_task_info();
    let info=TaskInfo {
        status: status,
        syscall_times: syscall_times,
        time: current_ms-start_time,
    };

    //获取数据长度
    let len: usize = core::mem::size_of::<TaskInfo>();
    let buf: *const u8 = ti as *const u8;
    let mut buffers = translated_byte_buffer(current_user_token(), buf, len);
    let src_ptr=&info as * const TaskInfo;
    let dst_ptr: *mut TaskInfo =buffers[0].as_mut_ptr() as * mut TaskInfo;
    unsafe {
        core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, 1);
    }
    0
}



/// 增加系统调用统计
pub  fn inc_syscall_times(syscall_id: usize)->(){
    inc_current_syscall_times(syscall_id);
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
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
