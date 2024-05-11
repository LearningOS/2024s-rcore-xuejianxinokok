//! Implementation of  [`ProcessControlBlock`]

use super::id::RecycleAllocator;
use super::manager::insert_into_pid2process;
use super::TaskControlBlock;
use super::{add_task, SignalFlags};
use super::{pid_alloc, PidHandle};
use crate::fs::{File, Stdin, Stdout};
use crate::mm::{translated_refmut, MemorySet, KERNEL_SPACE};
use crate::sync::{Condvar, Mutex, Semaphore, UPSafeCell};
use crate::trap::{trap_handler, TrapContext};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefMut;

/// Process Control Block
pub struct ProcessControlBlock {
    /// immutable
    pub pid: PidHandle,
    /// mutable
    inner: UPSafeCell<ProcessControlBlockInner>,
}

/// Inner of Process Control Block
pub struct ProcessControlBlockInner {
    /// is zombie?
    pub is_zombie: bool,
    /// memory set(address space)
    pub memory_set: MemorySet,
    /// parent process
    pub parent: Option<Weak<ProcessControlBlock>>,
    /// children process
    pub children: Vec<Arc<ProcessControlBlock>>,
    /// exit code
    pub exit_code: i32,
    /// file descriptor table
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    /// signal flags
    pub signals: SignalFlags,
    /// tasks(also known as threads)
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    /// task resource allocator
    pub task_res_allocator: RecycleAllocator,
    /// mutex list
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    /// semaphore list
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    /// condvar list
    pub condvar_list: Vec<Option<Arc<Condvar>>>,

    /// 是否启用死锁检测
    pub deadlock_detect: bool,
    /// 简便起见可对 mutex 和 semaphore 分别进行检测，无需考虑二者 (以及 waittid 等) 混合使用导致的死锁
    pub mutex_detector: BankerDeadlockDetector,
    pub semaphore_detector: BankerDeadlockDetector,
}

impl ProcessControlBlockInner {
    #[allow(unused)]
    /// get the address of app's page table
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// allocate a new file descriptor
    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
    /// allocate a new task id
    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }
    /// deallocate a task id
    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }
    /// the count of tasks(threads) in this process
    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }
    /// get a task with tid in this process
    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }
}

impl ProcessControlBlock {
    /// inner_exclusive_access
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }
    /// new process from elf file
    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        trace!("kernel: ProcessControlBlock::new");
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        // allocate a pid
        let pid_handle = pid_alloc();
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: vec![
                        // 0 -> stdin
                        Some(Arc::new(Stdin)),
                        // 1 -> stdout
                        Some(Arc::new(Stdout)),
                        // 2 -> stderr
                        Some(Arc::new(Stdout)),
                    ],
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    // -------死锁检测相关
                    deadlock_detect: false,
                    mutex_detector: BankerDeadlockDetector::new(),
                    semaphore_detector: BankerDeadlockDetector::new(),
                })
            },
        });
        // create a main thread, we should allocate ustack and trap_cx here
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            ustack_base,
            true,
        ));
        // prepare trap_cx of main thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            ustack_top,
            KERNEL_SPACE.exclusive_access().token(),
            kstack_top,
            trap_handler as usize,
        );
        // add main thread to the process
        let mut process_inner = process.inner_exclusive_access();
        process_inner.tasks.push(Some(Arc::clone(&task)));

        // -------死锁检测相关,在结构体已经初始化
        //process_inner.mutex_detector.allocation.push(Vec::with_capacity(0));
        //process_inner.mutex_detector.need.push(Vec::with_capacity(0));
        //process_inner.semaphore_detector.allocation.push(Vec::with_capacity(0));
        //process_inner.semaphore_detector.need.push(Vec::with_capacity(0));

        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }

    /// Only support processes with a single thread.
    pub fn exec(self: &Arc<Self>, elf_data: &[u8], args: Vec<String>) {
        trace!("kernel: exec");
        assert_eq!(self.inner_exclusive_access().thread_count(), 1);
        // memory_set with elf program headers/trampoline/trap context/user stack
        trace!("kernel: exec .. MemorySet::from_elf");
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        let new_token = memory_set.token();
        // substitute memory_set
        trace!("kernel: exec .. substitute memory_set");
        self.inner_exclusive_access().memory_set = memory_set;
        // then we alloc user resource for main thread again
        // since memory_set has been changed
        trace!("kernel: exec .. alloc user resource for main thread again");
        let task = self.inner_exclusive_access().get_task(0);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.res.as_mut().unwrap().ustack_base = ustack_base;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();
        // push arguments on user stack
        trace!("kernel: exec .. push arguments on user stack");
        let mut user_sp = task_inner.res.as_mut().unwrap().ustack_top();
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    new_token,
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0;
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(new_token, p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(new_token, p as *mut u8) = 0;
        }
        // make the user_sp aligned to 8B for k210 platform
        user_sp -= user_sp % core::mem::size_of::<usize>();
        // initialize trap_cx
        trace!("kernel: exec .. initialize trap_cx");
        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            task.kstack.get_top(),
            trap_handler as usize,
        );
        trap_cx.x[10] = args.len();
        trap_cx.x[11] = argv_base;
        *task_inner.get_trap_cx() = trap_cx;
    }

    /// Only support processes with a single thread.
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        trace!("kernel: fork");
        let mut parent = self.inner_exclusive_access();
        assert_eq!(parent.thread_count(), 1);
        // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
        let memory_set = MemorySet::from_existed_user(&parent.memory_set);
        // alloc a pid
        let pid = pid_alloc();
        // copy fd table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
        // create child process pcb
        let child = Arc::new(Self {
            pid,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    // -------死锁检测相关
                    deadlock_detect: false,
                    mutex_detector: BankerDeadlockDetector::new(),
                    semaphore_detector: BankerDeadlockDetector::new(),
                })
            },
        });
        // add child
        parent.children.push(Arc::clone(&child));
        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task(0)
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));
        // attach task to child process
        let mut child_inner = child.inner_exclusive_access();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);
        // modify kstack_top in trap_cx of this thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        trap_cx.kernel_sp = task.kstack.get_top();
        drop(task_inner);
        insert_into_pid2process(child.getpid(), Arc::clone(&child));
        // add this thread to scheduler
        add_task(task);
        child
    }
    /// get pid
    pub fn getpid(&self) -> usize {
        self.pid.0
    }

    /// 设置死锁检测
    pub fn set_deadlock_detect(&self, enable: bool) -> isize {
        let mut inner = self.inner_exclusive_access();
        inner.deadlock_detect = enable;
        // if enable {
        //     let mutex=BankerDeadlockDetector::new();
        //     //按照task长度填充
        //     for  i in 0..inner.tasks.len()  {
        //     }
        //     inner.mutex_detector = Some(mutex);

        //     inner.semaphore_detector = Some(BankerDeadlockDetector::new());

        // } else {
        //     inner.mutex_detector = None;
        //     inner.semaphore_detector = None;
        // }
        0
    }
}

/// 银行家死锁检测器
/// 参考 https://learningos.cn/rCore-Tutorial-Guide-2024S/chapter8/5exercise.html
pub struct BankerDeadlockDetector {
    /// 可利用资源向量 Available ：含有 m 个元素的一维数组，每个元素代表可利用的某一类资源的数目， 其初值是该类资源的全部可用数目，其值随该类资源的分配和回收而动态地改变。 Available[j] = k，表示第 j 类资源的可用数量为 k。
    pub available: Vec<isize>,
    /// 分配矩阵 Allocation：n * m 矩阵，表示每类资源已分配给每个线程的资源数。 Allocation[i,j] = g，则表示线程 i 当前己分得第 j 类资源的数量为 g。
    pub allocation: Vec<Vec<isize>>,
    /// 需求矩阵 Need：n * m 的矩阵，表示每个线程还需要的各类资源数量。 Need[i,j] = d，则表示线程 i 还需要第 j 类资源的数量为 d 。
    pub need: Vec<Vec<isize>>,
}

impl BankerDeadlockDetector {
    pub fn new() -> Self {
        Self {
            available: vec![],
            allocation: vec![vec![]],
            need: vec![vec![]],
        }
    }

    /// 死锁检测算法不同于银行家算法
    /// 死锁检测算法,检测是否安全状态
    pub fn is_safe_state(&self) -> (bool, Vec<usize>) {

       

        let n = self.allocation.len();
        let m = self.available.len();

        // 1.设置两个向量
        // work表示操作系统可提供给线程继续运行所需的各类资源数目 初始时，Work = Available
        let mut work = self.available.clone();
        // Finish，表示系统是否有足够的资源分配给线程， 使之运行完成.初始时 Finish[0..n-1] = false表示所有线程都没结束
        let mut finish = vec![false; n];
        // println!("work      : {:?}", &work);
        // 用于记录安全序列
        let mut safe_sequence = Vec::new();

        loop {
            let mut found = false;
            for i in 0..n {
                // 2. 从线程集合中找到一个能满足下述条件的线程
                // Finish[i] == false;
                // Need[i,j] ≤ Work[j];
                // 若找到，执行步骤 3
                if !finish[i] && (0..m).all(|j| self.need[i][j] <= work[j]) {

                    // 3.当线程 thr[i] 获得资源后，可顺利执行，直至完成，并释放出分配给它的资源，故应执行:
                    // Work[j] = Work[j] + Allocation[i, j];
                    // Finish[i] = true; //当有足够资源分配给线程时
                    for j in 0..m {
                        work[j] += self.allocation[i][j];
                    }
                    finish[i] = true;

                    safe_sequence.push(i);
                    found = true;
                    break;
                }
            }

            if !found {
                // 没有找到符合条件的线程，退出循环
                break;
            }
        }

        // println!("workend   : {:?}", &work);
        // println!("available : {:?}", &self.available);
        // println!("allocation: {:?}", &self.allocation,);
        // println!("need      : {:?}", &self.need);
        // println!("finish    : {:?}", &finish);
        // println!("---------------------------------");

        // 4.如果 Finish[0..n-1] 都为 true，则表示系统处于安全状态；否则表示系统处于不安全状态，即出现死锁。
        // 检查是否所有线程都已完成
        if finish.iter().all(|&x| x) {
            (true, safe_sequence) // 安全序列
        } else {
            (false, vec![]) // 存在死锁
        }
    }
}
