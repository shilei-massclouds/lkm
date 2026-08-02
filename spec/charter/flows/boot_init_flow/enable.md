# BootInitFlow.Enable 与 Online

Enable 驱动 `BootInitScheduleHandoffPhase`，只完成 CPU0 Scheduler idle/curr metadata、runqueue 与首次
调度的可逆预检；不创建 Flow、active binding 或 dispatch kind。随后提交 BootInitFlow.Online。

BootInitFlow.Online 后按固定顺序执行同一 Flow 的 Actions：

1. 完成 boot idle setup 和退出 inherited preempt-disabled guard；
2. `yields CpuGroup.cpus[0].scheduler.Action::Schedule()`；
3. identity 时由目标完成后的通用 resume attempt 立即从 yields 后继续；
4. non-identity 时 Scheduler 显式保存 BootTask context、Task.Suspend、恢复 next context、提交 bindings 和
   next Task.Continue；BootInitFlow lane token 保持 pending；
5. 未来 Scheduler 恢复 BootTask 后，contextual BootInitFlow.Continue 校验 context epoch/token，先回到
   `schedule()` 返回 continuation，再驱动 `BootIdleEntryPhase` 和 idle loop。

本轮 canonical before-send 边界固定在第 2 步 token/Signal 尚未创建的位置。发送 Schedule 本身不改变
BootTask 或 Flow；只有 non-identity SwitchTo 的显式步骤改变 Task/CPU binding。`BootIdleEntryPhase`
现在是 BootInitFlow 的 Online child，而不是另一个 Flow 的子对象。

陷入期间 BootTask 保持 OnCpu、BootInitFlow 保持 Online；effective-flow 栈切到 Trap/Interrupt/
Exception leaf。若陷入中调度切出，恢复先落到该 leaf，再回到 BootInitFlow continuation。

## 引用

- [BootInitFlow](README.md)
- [BootInitFlow.Enable model](../../../model/flows/boot_init_flow/main.spec)
- [BootInitFlow.Enable coding](../../../coding/flows/boot_init_flow/enable.md)
