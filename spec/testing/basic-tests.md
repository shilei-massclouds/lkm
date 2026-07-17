# Basic test pipeline

本文档规定单轮、单内核、单 QEMU 生命周期的“基本测试”接口。stress、paired difftest、
rc.local 和 OpenRC login 等复合或专用编排不属于基本测试；它们可以调用基本测试，也可以在迁移前
继续使用自己的 runner，但不得改变这里的配置和结果语义。

## Stable interface and stages

仓库根目录的稳定入口是：

- `make run TEST=<name>` 执行一个完整基本测试；未指定 `TEST` 时使用 `hello-native`。
- `make build TEST=<name>` 只解析同一份配置并构造该测试的 kernel image；它不准备磁盘，也不启动 QEMU。
- `make disk` 独立构造或检查 canonical rootfs；输入未变化时复用，输入变化或 `FORCE=1` 时重建。

基本测试固定经过七个阶段：配置解析、内核构建、磁盘准备、PreScript、QEMU 生命周期、
PostScript、结果汇总与清理。runner 必须在开始其它阶段前一次性解析、严格校验并冻结配置，写出
`manifest.json`；后续阶段只能消费 manifest，不能重新解释 TOML 或从脚本输出取得行为参数。

根 Make 命令行只允许选择 `TEST` 和覆盖文档列出的宿主工具/路径参数。`APP`、provider、probe、
profile、磁盘模式、QEMU 内存/SMP/cmdline/device、stdin、退出策略和期望结果等行为参数只能来自
该测试的 TOML。基本测试不得提供公开的 `justrun` 或其它绕过解析、构建和生命周期检查的 QEMU
入口。

## Versioned TOML

每个基本测试由统一 cases 目录中的单个 `<name>.toml` 描述。schema version 1 的允许字段为：

- 顶层：`schema_version`、`name`、`timeout_seconds`，以及可选的 `pre_script`、`post_script`。
- `kernel`：`app`、`provider`、`probe`、`profile`，以及可选的 `probe_file`、
  `stress_mem_bytes`、`extra_rustflags`。
- `disk`：`mode`，以及该模式规定的 `path`、`readonly`、`generator` 或 `size`。
- `qemu`：`memory_mb`、`smp`、`kernel_cmdline`、结构化 device 开关、`exit_policy`、
  可选的 marker 退出值，以及顺序化 `stdin_steps`。
- `expect`：可选的 host `process_exit`、guest `guest_exit_status`、必要/禁止 marker，及必要 marker
  的最小/精确计数。

未知字段、错误类型、未知 disk mode/device/exit policy、name 与文件名不一致、非法脚本权限、
缺少 mode 所需来源，均在构建前失败。配置只保存数据值；不得保存 `make ...`、完整 QEMU shell
命令或由 runner 执行的任意命令字符串。

disk mode 的含义固定如下：

- `none`：不准备且不附加 block image。
- `canonical-readonly`：检查 canonical rootfs，并以只读 QEMU drive 附加。
- `private-copy`：检查 canonical rootfs，复制到本轮私有路径并可写附加；最终必须删除副本。
- `generated`：由 `generator` 选择 runner 内建的生成器，在本轮私有路径生成并最终删除。
- `external`：附加配置中给出的既有绝对或仓库相对 raw image；runner 不构造、不删除来源。

当前内核在 InitcallPhase 要求 live virtio-blk，因而真实 hello/smoke acceptance 配置仍应挂载磁盘。
`none` 是框架能力，只有在目标 kernel 配置确实不要求 block device，或在 runner 隔离测试中，才能
作为成功用例；不得仅凭 QEMU 返回 0 把 guest 的启动失败算作成功。

## Scripts and isolation

PreScript/PostScript 必须是配置相对路径解析后的普通可执行文件。runner 只提供固定环境：测试名、
冻结配置路径、kernel、disk、QEMU log、result 和 artifact 目录，以及受控的 `PATH`、`TMPDIR`、
`LC_ALL`。脚本不会收到 `APP`、provider、probe、QEMU 参数等可变行为环境，也没有返回或覆盖这些
参数的接口。

PreScript 失败时 QEMU 不得启动。只要 QEMU 阶段已经开始，PostScript 就必须在正常退出、expect
失败、异常退出或超时后执行；PostScript 自身失败也使整轮失败，但不能覆盖更早的诊断。

## QEMU lifecycle and result

runner 以独立进程组启动 QEMU，按声明顺序等待 stdin step 的 ready marker 后发送 payload，执行
整体 timeout，并实现三种退出策略：等待进程退出、等待 guest 正常关机对应的进程退出、或在指定
marker 后主动终止。timeout、异常和 marker 后终止都必须回收整个进程组；不能把 QEMU 留给调用者。

host process exit 与 guest exit 是两个独立观测。guest exit status 从稳定的
`user exit status=N` 日志事实提取；kernel/smoke/KUnit 等没有该事实的测试应使用 marker/count
断言。仅有 host QEMU exit 0 不足以证明测试通过。

每次 `run` 始终创建 QEMU log 和 `result.json`。result schema version 1 至少包含测试/config/
manifest 标识、开始结束时间、统一 success/failure 与退出码、七阶段状态和耗时、QEMU exit/timeout/
termination facts、每项 expectation 的实际值，以及 cleanup 的进程和私有磁盘结果。配置错误或前置
阶段失败时也要写 result 和空的 QEMU log。任一必需阶段、脚本、expect 或 cleanup 失败，runner
统一返回非零。
