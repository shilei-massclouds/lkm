# EntrySuccessorPhase Coding

入口后继子阶段对应 model `EntrySuccessorPhase`，实现落点为
`impl/arceos_ex/src/phases/boot/entry_successor.rs`。它只由 BootPhase.Setup 驱动；完成后返回父
continuation，不拥有 CorePrepare sibling 的启动权。

## Transition 映射

### Preset: Base -> Prepared

`preset()` 先检查自身精确 Base 和全部 model `depends_on`：EntryPrelude Online，Vm/BootTask/
KernelImage 状态，EarlyVm、BootInitStack、InterruptStream、RawDtb 和 FixMap 状态。检查通过后才
发出 `EntrySuccessorPhase.Started`。

随后严格按 model 顺序驱动 BootInitStack.Enable、EarlyDtb.Preset、InterruptStream.Setup、
BootCPU.Setup/Enable、PrintkBuffer.Preset、EarlyDtb.Setup、InitMM.Setup、EarlyIoremap.Setup、
SBI.Setup、Params.Preset、MemBlock.Setup、Vm.Enable、MemBlock.Enable 和 EarlyDtb.Cleanup。
驱动完成后检查 start_kernel deferred facts，提交 Prepared，读回并发出
`EntrySuccessorPhase.Prepared`，再按 emits 调用 Setup。

### Setup: Prepared -> Ready

Setup 检查精确 Prepared，并验证 EntrySuccessor Ready invariant：入口先导 Online、完整内核虚拟
地址空间与 boot CPU 状态、early IRQ 关闭、EarlyDtb 销毁、MemBlock/参数/console 等对象状态
以及 deferred facts。成功后提交 Ready，发出 `EntrySuccessorPhase.Ready`，再调用 Enable。

### Enable: Ready -> Online

Enable 检查精确 Ready并重新确认 invariant，提交 Online 并发出
`EntrySuccessorPhase.Online`。随后返回 `boot::setup_after_entry_successor()`；不得直接调用
CorePreparePhase。

## 状态与 checkpoint

`ENTRY_SUCCESSOR_PHASE_STATE` 持久记录四状态；`is_online()` 只匹配 Online。

| Checkpoint | owner state | Position |
| --- | --- | --- |
| `EntrySuccessorPhase.Started` | Base | Preset source/dependency 检查后 |
| `EntrySuccessorPhase.Prepared` | Prepared | Preset drives/ensures 完成后 |
| `EntrySuccessorPhase.Ready` | Ready | Setup invariant 检查和提交后 |
| `EntrySuccessorPhase.Online` | Online | Enable 提交后、父 continuation 前 |

## 保留的实现规则

原 `entry-successor.spec` 的有效规则在此继续作为 MUST：

- `init_vmlinux_build_id()`、`page_address_init()` 和 start_kernel 位置必须保留为结构化 deferred
  facts，不能因未实现对应对象而消失。
- MemBlock 必须记录 RISC-V `setup_bootmem()` 的 `phys_ram_base`、kernel-map offset、DMA32 zone
  input 和 hugetlb CMA deferred 边界。
- SwapperVm 必须保留 `CONFIG_STRICT_KERNEL_RWX` 最终权限拆分 deferred 边界。
- `printk::write_str("arceos_ex object kernel\n")` 是 PrintkBuffer.Preset 后的 Write action，不是
  phase transition。
- 所有 drives 保持 model 源顺序，阶段内不得提前打开中断、任务并发或 SMP 并发。
