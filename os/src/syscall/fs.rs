//! File and filesystem-related syscalls
use crate::fs::{open_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    trace!("kernel:pid[{}] sys_fstat", current_task().unwrap().pid.0);

    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        // let file = file.clone();
        // if !file.readable() {
        //     return -1;
        // }
        // release current task TCB manually to avoid multi-borrow
        trace!("kernel: sys_fstat .. file.stat");
        //获取元数据
        let stat = file.stat();
        drop(inner);

        //获取数据长度
        let len: usize = core::mem::size_of::<Stat>();
        let buf: *const u8 = st as *const u8;
        let mut buffers = translated_byte_buffer(current_user_token(), buf, len);
        let src_ptr = &stat as *const Stat;
        // 注意此处不能获取buffers.as_mut_ptr()
        let dst_ptr: *mut Stat = buffers[0].as_mut_ptr() as *mut Stat;
        unsafe {
            //第三个参数应该是1 还是len ??
            core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, 1);
        }

        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_linkat", current_task().unwrap().pid.0);

    let token = current_user_token();
    let old_path = translated_str(token, old_name);
    let new_path = translated_str(token, new_name);
    //可能的错误 链接同名文件
    if old_path == new_path {
        return -11;
    }
    //获取 文件描述符
    let old_fd = sys_open(old_name, OpenFlags::RDONLY.bits()) as usize;
    // let a=true;
    // if a{
    //     return -13;
    // }
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    if let Some(file) = &inner.fd_table[old_fd] {
        // if a{
        //     return -14;
        // }
        file.link(new_path.as_str())
    } else {
        -12
    }
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_unlinkat", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, name);

    //获取 文件描述符
    let fd = sys_open(name, OpenFlags::RDONLY.bits()) as usize;

    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    if let Some(file) = &inner.fd_table[fd] {
        file.unlink(path.as_str())
    } else {
        -12
    }
}
