## 1. 实现的功能

在process.rs 添加 BankerDeadlockDetector 用于记录信号量分配信息

在 sys_mutex_create和sys_mutex_create 做数据初始化操作

在 sys_mutex_lock和 sys_semaphore_down 做死锁检测

## 2. 简答作业

### 2.1 需要回收的资源有哪些？

需要回收ProcessControlBlockInner  中包含的内容 包含页表 打开文件，锁，信号量，条件变量等

其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？

不需要回收，因为在ProcessControlBlock drop 时会释放这些资源

## 3.荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
   
   > 在群里 于赵剑秋 对于  “某一类资源”  在源码中对应物是什么
   > 
   > 资源的种类可以类比Semaphore id，Available就是Semaphore创建时的count，但是后续要动态维护

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
   
   > [rCore-Tutorial-Book-v3 3.6.0-alpha.1 文档](https://rcore-os.cn/rCore-Tutorial-Book-v3/)
   > 
   > 导学视频
   > 
   > chatgt
   > 
   > 
   > 
   > - [死锁的处理策略_预防死锁_避免死锁（银行家算法）_检测和解除](https://blog.csdn.net/weixin_45990326/article/details/119952188)
   > 
   > - [rCore-Tutorial-Book-v3 3.6.0-alpha.1 文档 第8章 练习参考答案](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter8/7answer.html)
   > 
   > - [如何理解死锁检测不需要知道进程运行所需资源总量信息？](https://www.zhihu.com/question/429700459)
   > 
   > - 《计算机操作系统》第四版 汤小丹 3.7 避免死锁
   > 
   > - 《操作系统概念 第7版》7.5.3 银行家算法

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。