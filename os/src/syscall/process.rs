//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    mm::translated_byte_buffer,
    task::current_user_token,
    task::{
        change_program_brk, exit_current_and_run_next, get_current_task_info,
        inc_current_syscall_times, map_memory, suspend_current_and_run_next, unmap_memory,
        TaskStatus,
    },
    timer::{get_time_ms, get_time_us},
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
    let src_ptr = &time as *const TimeVal;
    // 注意此处不能获取buffers.as_mut_ptr()
    let dst_ptr: *mut TimeVal = buffers[0].as_mut_ptr() as *mut TimeVal;
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
    trace!("kernel: sys_task_info");
    // 获取当前毫秒
    let current_ms: usize = get_time_ms();
    let (status, syscall_times, start_time) = get_current_task_info();
    let info = TaskInfo {
        status: status,
        syscall_times: syscall_times,
        time: current_ms - start_time,
    };

    //获取数据长度
    let len: usize = core::mem::size_of::<TaskInfo>();
    let buf: *const u8 = ti as *const u8;
    let mut buffers = translated_byte_buffer(current_user_token(), buf, len);
    let src_ptr = &info as *const TaskInfo;
    let dst_ptr: *mut TaskInfo = buffers[0].as_mut_ptr() as *mut TaskInfo;
    unsafe {
        core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, 1);
    }
    0
}

/// 增加系统调用统计
pub fn inc_syscall_times(syscall_id: usize) -> () {
    inc_current_syscall_times(syscall_id);
}

/// 申请长度为 len 字节的物理内存（不要求实际物理内存位置，可以随便找一块），将其映射到 start 开始的虚存，内存页属性为 port
/// - start 需要映射的虚存起始地址，要求按页对齐
/// - len 映射字节长度，可以为 0
/// - port：第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0
/// - 返回值：执行成功则返回 0，错误返回 -1
///
/// 可能的错误：
/// - start 没有按页大小对齐  (start & 0xFFF)!= 0
/// - port & !0x7 != 0 (port 其余位必须为0)
/// - port & 0x7 == 0 (这样的内存无意义)
/// - [start, start + len) 中存在已经被映射的页
/// - 物理内存不足
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    // 判断前3种错误情况
    if (start & 0xFFF) != 0 || (port & !0x7) != 0 || (port & 0x7) == 0 {
        return -1;
    }
    map_memory(start, len, port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    //start 没有按页大小对齐
    if (start & 0xFFF) != 0 {
        return -1;
    }
    unmap_memory(start, len)
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
