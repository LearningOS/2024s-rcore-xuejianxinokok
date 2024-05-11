## 1. 实现的功能

- spawn
  
  - fork + exec

- stride 调度算法
  
  TaskControlBlockInner 添加2个字段  stride，priority
  
  在 processor.rs 文件按中 run_tasks() 中计算 
  
  ```
  /对于获得调度的进程P，将对应的stride加上其对应的步长pass（只与进程的优先权有关系）
              let pass = BIG_STRIDE / task_inner.priority as usize;
              task_inner.stride += pass;
              // 只在第一次运行时记录开始时间
              if task_inner.start_time <= 0 {
                  // 记录开始时间
                  task_inner.start_time = get_time_ms();
              }
  ```
  
  在 TaskManager.fetch 找到 stride 最小的进程

## 2. 简答作业

### 2.1

在实际情况中，轮到 p1 执行是不准确的。尽管 p1 的 stride 值比 p2 的大，但是当 p1.stride = 255，p2.stride = 250 时，p2 执行一个时间片后，p1 的 stride 会溢出变为 0，而不是 255 + 255 = 510，所以 p1 的 stride 不会大于 p2，因此下一次执行的仍然是 p2。

在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。这是因为当所有进程的优先级都大于等于 2 时，即所有进程的 stride 都小于等于 BigStride / 2，那么最小的 stride 和最大的 stride 之差不会超过 BigStride 的一半。

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // 计算差值
        let diff = self.0.wrapping_sub(other.0);
        // 如果 diff <= BigStride / 2，则 self < other
        // 否则 self > other
        if diff <= BigStride / 2 {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```

## 3.荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
   
   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
   
   > [rCore-Tutorial-Book-v3 3.6.0-alpha.1 文档](https://rcore-os.cn/rCore-Tutorial-Book-v3/)
   > 
   > 导学视频
   > 
   > chatgpt

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。