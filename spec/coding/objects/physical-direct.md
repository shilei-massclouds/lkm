# PhysicalDirect Coding

`PhysicalDirect` 必须由独立 controller 实现边界表示 `satp == 0` 的直接物理执行环境；不得为它分配
页表或把它实现成 `KernelAddrSpace` 的别名。

`ActivateOnCpu(cpu_ref)` 是 controller 链唯一的 InitialActivation。它必须先解析 CpuRef，确认该 CPU
尚无 controller association 且入口 live `satp` 为零，然后在该 CPU translation journal 的首槽记录
InitialActivation、无旧 controller、目标 PhysicalDirect、SATP 0、同步完成和提交序号。最后以 release
顺序发布 association 与 committed count；失败不得修改 journal 或 association。离开 PhysicalDirect
只改变目标 CPU 的 association，不销毁本 controller。

稳定规则 ID：`riscv64_must_physical_direct_initial_activation_commit_per_cpu_fact`（MUST）。停止状态的
AP 在体系结构入口实际执行本 Action 前仍没有 association 或 live SATP；boot data 只能携带期望输入，
不能预先提交 receipt。
