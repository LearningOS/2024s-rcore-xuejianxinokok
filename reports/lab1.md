


os/src/task/task.rs

```rust
use crate::config::MAX_SYSCALL_NUM;


pub struct TaskControlBlock {
    ...
    ///  启动时间(单位ms)
    pub start_time: usize,
    /// 系统调用次数
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
}
```



解决编译错误

os/src/task/mod.rs

```rust

use crate::config::{MAX_APP_NUM,MAX_SYSCALL_NUM};

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        let mut tasks = [TaskControlBlock {
            ...
            start_time :0,
            syscall_times:[0; MAX_SYSCALL_NUM]

        }; MAX_APP_NUM];

```


在启动时 设置值

```rust
impl TaskManager {
   fn run_first_task(&self) -> ! {
           ...
           task0.task_status = TaskStatus::Running;
           // 记录开始时间
           task0.start_time = get_time_ms();
           ...
       }
   
   fn run_next_task(&self) {
           if let Some(next) = self.find_next_task() {
               ...
               inner.tasks[next].task_status = TaskStatus::Running;
   			
               // 只在第一次运行时记录开始时间
               if inner.tasks[next].start_time<=0 {
                 // 记录开始时间
                  inner.tasks[next].start_time = get_time_ms();
               }
               ....
       }
   
    /// 增加当前系统调用次数
    fn inc_current_syscall_times(&self,syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times[syscall_id] +=1;
    }
    /// 获取当前进程信息
    fn get_current_task_info(&self)->(TaskStatus,[u32; MAX_SYSCALL_NUM],usize){
        let  inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (inner.tasks[current].task_status,inner.tasks[current].syscall_times,inner.tasks[current].start_time)
    }

}

/// 增加当前进程系统调用次数
pub fn  inc_current_syscall_times(syscall_id: usize){
    TASK_MANAGER.inc_current_syscall_times(syscall_id);
}
/// 获取当前进程信息
pub fn get_current_task_info()->(TaskStatus,[u32; MAX_SYSCALL_NUM],usize){
    TASK_MANAGER.get_current_task_info()
}


```


os/src/syscall/mod.rs

```rust

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    // 这里应该判断syscall_id 范围
    inc_syscall_times(syscall_id);
    ...
}


```



os/src/syscall/process.rs

```rust


pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let (status,syscall_times,start_time)=get_current_task_info();
    // 获取当前毫秒
    let current_ms: usize=get_time_ms();
    unsafe {
        *ti = TaskInfo {
            status: status,
            syscall_times: syscall_times,
            time: current_ms-start_time,
        };
    }
    0
}
/// 增加系统调用统计
pub  fn inc_syscall_times(syscall_id: usize)->(){
    inc_current_syscall_times(syscall_id);
}

```




