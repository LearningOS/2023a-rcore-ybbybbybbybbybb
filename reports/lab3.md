# 编程作业

- spawn :过程类似 fork(exec())，只是新建 TCB 时直接 TaskControlBlock::new(elf_data)，然后修改其中的 parent。
- stride 调度算法 :在 TaskControlBlockInner 中添加 stride 与 pass 字段，set_prio 时 pass改为 BIG_STRIDE / prio

# 简答作业

# 实际情况是轮到 p1 执行吗？为什么？

- 不是，p2.stride 类型为 u8 值域为 0-255 所以 p2.stridev + 10 溢出为 4，还是轮到 p2 执行

# 为什么STRIDE_MAX – STRIDE_MIN <= BigStride / 2？

- 因为优先级大于等于 2，所以每次增加的步长都小于等于 BigStride/2，而 stride 小的优先执行，设stride最小为 stride1，第二小为 stride2，stride1 + BigStride/2 - stride2 <= BigStride/2

# 补全下列代码中的 partial_cmp 函数

``` rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if (self.0 < other.0 && other.0 - self.0 < 128) ||
           (self.0 > other.0 && self.0 - other.0 > 127) {
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

# 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
    - 叶可禾 
    - 范天奇

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
    - _RISC-V-Reader-Chinese-v2p1_
    - _rCore-Tutorial-Book 第三版_

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。