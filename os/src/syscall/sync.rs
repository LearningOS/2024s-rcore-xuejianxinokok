use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;

        // -------死锁检测相关
        //可利用资源向量
        process_inner.mutex_detector.available[id] = 1;
        process_inner.mutex_detector.allocation[tid][id] = 0;
        process_inner.mutex_detector.need[tid][id] = 0;

        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        let id = process_inner.mutex_list.len() - 1;
        // -------死锁检测相关
        //可利用资源向量
        process_inner.mutex_detector.available.push(1);
        process_inner.mutex_detector.allocation[tid].push(0);
        process_inner.mutex_detector.need[tid].push(0);

        id as isize
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    // 设置需求矩阵
    //Need[i,j] ≤ Work[j]
    process_inner.mutex_detector.need.get_mut(tid).unwrap()[mutex_id] += 1;

    // 如果开启了死锁检测
    if process_inner.deadlock_detect {
        //这里进行死锁检测
        let (safe, _) = process_inner.mutex_detector.is_safe_state();
        if !safe {
            //开启死锁检测功能后， mutex_lock 和 semaphore_down 如果检测到死锁， 应拒绝相应操作并返回 -0xDEAD (十六进制值)
            return -0xdead;
        }
    }
    // 设置 分配矩阵,确保tid 所在vec 存在
    process_inner
        .mutex_detector
        .allocation
        .get_mut(tid)
        .unwrap()[mutex_id] += 1;

    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());

    drop(process_inner);
    drop(process);
    mutex.lock();

    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    // 如果开启了死锁检测
    //if  process_inner.deadlock_detect{
    // 设置需求矩阵
    process_inner.mutex_detector.need.get_mut(tid).unwrap()[mutex_id] -= 1;

    // 设置 分配矩阵,确保tid 所在vec 存在
    process_inner
        .mutex_detector
        .allocation
        .get_mut(tid)
        .unwrap()[mutex_id] -= 1;
    //}
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));

        // -------死锁检测相关
        //可利用资源向量,如果id是复用之前的
        process_inner.semaphore_detector.available[id] = res_count as isize;
        for i in 0..process_inner.tasks.len() {
            process_inner.semaphore_detector.allocation[i][id] = 0;
            process_inner.semaphore_detector.need[i][id] = 0;
        }

        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));

        let id = process_inner.semaphore_list.len() - 1;

        // -------死锁检测相关
        //可利用资源向量
        process_inner
            .semaphore_detector
            .available
            .push(res_count as isize);
        for i in 0..process_inner.tasks.len() {
            process_inner.semaphore_detector.allocation[i].push(0);
            process_inner.semaphore_detector.need[i].push(0);
        }

        id
    };
    drop(process_inner);
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner); // 一定要在up之前drop
    sem.up();

    // 如果开启了死锁检测
    //if  process_inner.deadlock_detect{
    // 设置需求矩阵
    // process_inner.semaphore_detector.need.get_mut(tid).unwrap()[sem_id] = 0;
    let mut process_inner = process.inner_exclusive_access();
    process_inner.semaphore_detector.available[sem_id] += 1;
    // 设置 分配矩阵,确保tid 所在vec 存在
    process_inner
        .semaphore_detector
        .allocation
        .get_mut(tid)
        .unwrap()[sem_id] -= 1;
    //}
    drop(process_inner);

    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();

    let mut process_inner = process.inner_exclusive_access();
    // 请求个数
    let request: isize = 1;
    // 需求个数
    process_inner.semaphore_detector.need.get_mut(tid).unwrap()[sem_id] += request;


    if process_inner.deadlock_detect {
        //这里进行死锁检测
        let (safe, _) = process_inner.semaphore_detector.is_safe_state();
        if !safe {
            process_inner.semaphore_detector.need.get_mut(tid).unwrap()[sem_id] -= request;
            drop(process_inner);
            //开启死锁检测功能后， mutex_lock 和 semaphore_down 如果检测到死锁， 应拒绝相应操作并返回 -0xDEAD (十六进制值)
            return -0xdead; // 整数是 -57005
        }
    }



    // 1. 请求个数<=需求个数

    // 2. 如果request 小于 available，
    if request <= process_inner.semaphore_detector.available[sem_id] {
        // 3.尝试分配资源
        process_inner.semaphore_detector.available[sem_id] -= request;
        //死锁检测通过后,设置分配矩阵allocation,确保tid 所在vec 存在
        process_inner
            .semaphore_detector
            .allocation
            .get_mut(tid)
            .unwrap()[sem_id] += request;
        process_inner.semaphore_detector.need.get_mut(tid).unwrap()[sem_id] -= request;

        //4.  执行安全检测
        if process_inner.deadlock_detect {
            //这里进行死锁检测
            let (safe, _) = process_inner.semaphore_detector.is_safe_state();
            if safe {
                let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
                drop(process_inner);
                sem.down();
            } else {
                //不安全

                // 恢复之前状态
                process_inner.semaphore_detector.available[sem_id] += request;
                //死锁检测通过后,设置分配矩阵allocation,确保tid 所在vec 存在
                process_inner
                    .semaphore_detector
                    .allocation
                    .get_mut(tid)
                    .unwrap()[sem_id] -= request;
                process_inner.semaphore_detector.need.get_mut(tid).unwrap()[sem_id] += request;
                drop(process_inner);
                //开启死锁检测功能后， mutex_lock 和 semaphore_down 如果检测到死锁， 应拒绝相应操作并返回 -0xDEAD (十六进制值)
                return -0xdead; // 整数是 -57005
            }
        }
    } else {
        // 资源不足 则等待
        let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
        drop(process_inner);
        sem.down();
    }

  

    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    let state = if enabled == 1 {
        true
    } else if enabled == 0 {
        false
    } else {
        // 参数不合法
        return -1;
    };
    current_process().set_deadlock_detect(state)
}
