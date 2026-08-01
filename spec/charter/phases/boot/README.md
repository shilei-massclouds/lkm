# Boot leaf namespace

本目录只为 `BootInitFlow.Setup` 直接驱动的 boot 叶阶段保留导航位置，不是核心语义规格文件，也不
声明 `Boot`、`BootPhase`、wrapper lifecycle 或 checkpoint。

`BootInitFlow` 的实例级 Charter 唯一位于
[`../boot-init-flow.md`](../boot-init-flow.md)。当前三个叶阶段尚无独立 Charter 文件；在后续
charter-first 校准各叶阶段时，才分别建立与实际 PhaseObject 同名的文件，不预先创建空规格：

- `CorePreparePhase`；
- `MmCoreInitPhase`；
- `SchedInitPhase`。

`start_kernel()` 到 `setup_arch()` 返回由 `BootInitFlow.Setup` 直接编排，不是 boot 叶阶段，也不在本
namespace 中建立 wrapper。

现有 formal 与 coding 文件分别位于 `spec/model/phases/boot/` 和 `spec/coding/phases/boot/`。这些
物理 namespace 不产生额外对象身份或语义权威。
