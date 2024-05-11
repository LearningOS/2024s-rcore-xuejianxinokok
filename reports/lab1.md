## 1. 实现的功能 sys_task_info

1. 需要在 TaskControlBlock  添加 启动时间  start_time, 和 系统调用次数 syscall_times: [u32; MAX_SYSCALL_NUM]
2. 在系统调用入口处统计系统调用次数
3. 在  TaskManager.run_first_task 和 run_next_task 方法中记录  启动时间  start_time, 用当前时间-start_time就是系统调用时刻距离任务第一次被调度时刻的时长

## 2. 简答作业

### 2.1

- 版本: RustSBI-QEMU Version 0.2.0-alpha.2

- ch2b_bad_address  发生了页错误

[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003ac, kernel killed it.

 (0x0 as *mut u8) 表示空指针

- ch2b_bad_instructions 发生了非法指令

[kernel] IllegalInstruction in application, kernel killed it.

sret 是特权指令，在用户模式下无法访问

- ch2b_bad_register

[kernel] IllegalInstruction in application, kernel killed it.

csrr  读写控制状态寄存器 是特权指令，在用户模式下无法访问

### 2.2 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:

1. L40：刚进入 `__restore` 时，`a0` 代表了什么值。请指出 `__restore` 的两种使用情景。
   
   a0(x10)  指向TrapContext
   
   __restore场景1: 运行第一个程序 run_first_task
   
   
   
   __restore场景2：从内核态返回到用户态  run_next_task
   
   

2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。
   
   ```nasm
   ld t0, 32*8(sp)   加载sstatus 到t0
   ld t1, 33*8(sp)   加载sepc到t0   
   ld t2, 2*8(sp)    user stack  建立用户态栈环境
   csrw sstatus, t0   恢复控制状态寄存器  SPP 保存了trap之前的特权级
   csrw sepc, t1      保存了trap返回地址
   csrw sscratch, t2  恢复用户栈
   ```

3. L50-L56：为何跳过了 x2 和 x4？
   
   sp(x2) 指向内核栈顶
   
   x4  application does not use it

4. L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？
   
   sp->user stack   ,sscratch->kernel stack

5. `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
   
   发生状态切换在sret指令，因为L43-L48  已经恢复了 sstatus 和sepc 并建立了用户栈

6. L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？
   
    sp->kernel stack,  sscratch->user stack

7. 从 U 态进入 S 态是哪一条指令发生的？ 在 ecall 指令

## 3.荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
   
   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
   
   > [rCore-Tutorial-Book-v3 3.6.0-alpha.1 文档](https://rcore-os.cn/rCore-Tutorial-Book-v3/)
   > 
   > 导学视频

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。