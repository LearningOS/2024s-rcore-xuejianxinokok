## 参考资料

[页面置换](https://nankai.gitbook.io/ucore-os-on-risc-v64/lab3/ye-mian-zhi-huan)


[南开大学实验指导书](https://nankai.gitbook.io/ucore-os-on-risc-v64/lab6/tiao-du-suan-fa-kuang-jia)

代码 

https://github.com/nkgongxl/ucoreonrv/blob/code_practice/lab6/kern/schedule/default_sched_stride.c

考察round-robin调度器，在假设所有进程都充分使用了其拥有的 CPU 时间资源的情况下，所有进程得到的 CPU 时间应该是相等的。但是有时候我们希望调度器能够更智能地为每个进程分配合理的 CPU 资源。假设我们为不同的进程分配不同的优先级，则我们有可能希望每个进程得到的时间资源与他们的优先级成正比关系。Stride调度是基于这种想法的一个较为典型和简单的算法。除了简单易于实现以外，它还有如下的特点：

可控性：如我们之前所希望的，可以证明 Stride Scheduling对进程的调度次数正比于其优先级。

确定性：在不考虑计时器事件的情况下，整个调度机制都是可预知和重现的。该算法的基本思想可以考虑如下： 1. 为每个runnable的进程设置一个当前状态stride，表示该进程当前的调度权。另外定义其对应的pass值，表示对应进程在调度后，stride 需要进行的累加值。 2. 每次需要调度时，从当前 runnable 态的进程中选择 stride最小的进程调度。 3. 对于获得调度的进程P，将对应的stride加上其对应的步长pass（只与进程的优先权有关系）。 4. 在一段固定的时间之后，回到 2.步骤，重新调度当前stride最小的进程。

可以证明，如果令 P.pass =BigStride / P.priority 其中P.priority表示进程的优先权（大于 1），而 BigStride表示一个预先定义的大常数，则该调度方案为每个进程分配的时间将与其优先级成正比。


我们在进程控制块中也记录了一些和调度有关的信息：

```c
struct proc_struct {
    // ...
    // 表示这个进程是否需要调度
    volatile bool need_resched;
    // run queue的指针
    struct run_queue *rq;
    // 与这个进程相关的run queue表项
    list_entry_t run_link;
    // 这个进程剩下的时间片
    int time_slice;
    // 以下几个都和Stride调度算法实现有关
    // 这个进程在优先队列中对应的项
    skew_heap_entry_t lab6_run_pool;


    // 该进程的Stride值
    uint32_t lab6_stride;
    // 该进程的优先级
    uint32_t lab6_priority;
};
```
