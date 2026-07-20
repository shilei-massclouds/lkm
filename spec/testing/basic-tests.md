# Basic test pipeline

本文档规定“基本测试”的执行拓扑和稳定接口。一个基本测试必须只解析一份冻结配置、构造一个
kernel image，并拥有一次完整 QEMU 生命周期。测试内容可以是 kernel checkpoint callback、用户程序、
发行版命令、rc.local、BusyBox init/login 或人工诊断；是否需要输入不改变基本测试身份。stress 只通过
重复调用一个基本测试形成复合测试，paired difftest 只通过严格顺序执行两个基本测试形成复合测试。

## Stable interface and stages

仓库根目录的稳定入口是：

- `make run TEST=<name>` 执行一个完整基本测试；未指定 `TEST` 时使用 `hello-native`。
- `make build TEST=<name>` 只解析同一份配置并构造 kernel image；不检查 TTY、不准备磁盘、不启动 QEMU。
- `TEST` 是唯一正式选择变量。旧 `APP` 只作为 `TEST` 的变量名别名，接受完整且相同的 test name
  namespace；`make run APP=<name>` / `make build APP=<name>` 必须把 name 原样交给 runner。Make 命令行
  和真实进程环境中的显式值采用同一规则。它不能覆盖 TOML 中的任何行为字段。
- `user-smoke-native` 和 `user-smoke-linux-object` 是自动 user-mode acceptance；`shell` 和
  `shell-lo` 是 opt-in terminal diagnostic。测试身份只由显式 test name 决定，runner 不得
  根据 TTY、调用者或命令上下文在 smoke 与 shell 之间隐式切换。
- `scripted-shell` 和 `scripted-shell-lo` 是上述两个 terminal identity 的自动 scripted counterpart，
  分别固定使用 native 与 linux-object provider，并作为 acceptance 进入默认门禁。旧
  `script-shell` / `script-shell-lo` 与更早的 `distro-sh-native` / `distro-sh-linux-object` 名称直接
  移除，不得增加 compatibility alias、retired-name 映射或别名 TOML。counterpart 是显式 identity
  对应关系，不得据此复制 QEMU case，
  或把 scripted pipe 与人工 PTY transport 声明为等价。
- `ltp` 和 `ltp-lo` 是自动 LTP syscall acceptance，分别固定使用 native 与 linux-object
  provider。旧 `ltp-shell-manual-*` 名称退役且不得保留兼容映射或别名 TOML。
- `make disk ROOTFS=<profile>` 独立构造只读 rootfs template；`ROOTFS` 默认且当前只允许 `canonical`，
  因而 `make disk` 等价于 `make disk ROOTFS=canonical`。未知 profile 必须立即失败。

基本测试固定经过配置解析、内核构建、磁盘准备、PreScript、QEMU 生命周期、PostScript、结果汇总与
清理七个阶段。runner 在其它阶段前一次性严格校验并冻结配置到 `manifest.json`；后续阶段只消费
manifest。`make run` 不构造 rootfs，也不执行 rootfs overlay；template 缺失或 input manifest 过期时，
必须在 QEMU 前失败并提示先运行 `make disk ROOTFS=canonical`。

根 Make 调用只允许用正式 `TEST` 或其 `APP` 变量名别名选择测试，以及覆盖文档列出的宿主工具/路径
参数。无论来自 Make 命令行还是进程环境，显式同时提供 `TEST` 与 `APP` 都必须因歧义失败；未知
APP 值与相同的未知 TEST 值走同一结构化配置失败路径。
provider、probe、profile、磁盘模式、QEMU 内存/SMP/cmdline/device、interaction、stdin、退出策略和
期望结果只能来自 TOML。基本测试不得提供公开的 `justrun` 或其它绕过解析、构建和生命周期检查的
QEMU 入口。

## Versioned TOML

每个基本测试由统一 cases 目录中的单个 `<name>.toml` 描述。runner 必须兼容读取 schema v1；所有
正式配置使用 schema v2。v2 允许字段为：

- 顶层：`schema_version`、`name`、`purpose = "acceptance" | "diagnostic"`、
  `timeout_seconds`，以及可选且必须显式声明的 `pre_script`、`post_script`。
- `kernel` 是严格 union。未声明 `target` 或 `target = "arceos_ex"` 时允许 `app`、`provider`、
  `probe`、`profile`，以及可选的 `probe_file`、`stress_mem_bytes`、`extra_rustflags`；
  `stress_mem_bytes` 只允许 recorder 实现已声明的 `32768`、`65536`、`131072`、`262144`
  或 `524288`，未知容量必须在构建前失败；
  `target = "linux"` 时禁止这些 arceos_ex 字段，并通过 `LINUX_PROVIDER_DIR` 构建启用
  `CONFIG_LKM_CHECKPOINTS` 的 `arch/riscv/boot/Image`。
- `disk`：`mode`，template mode 的 `profile`，以及其它 mode 规定的 `path`、`readonly`、
  `generator` 或 `size`。
- `qemu`：`memory_mb`、`smp`、`kernel_cmdline`、结构化 device/network/host-forward、`exit_policy`、
  `interaction = "none" | "scripted" | "terminal"`、可选 marker 退出值和 `stdin_steps`。
- `expect`：可选的 host `process_exit`、guest `guest_exit_status`、必要/禁止 marker，及必要 marker
  的最小/精确计数。acceptance 必须至少声明一个 observable fact；diagnostic 可以只记录会话结果。

未知字段、错误类型、未知 disk/profile/device/exit policy/interaction、name 与文件名不一致、非法脚本
权限或缺少 mode 所需来源，都必须在 build 或 QEMU 前失败。配置只保存数据值；不得保存 `make ...`、
完整 QEMU shell 命令或由 runner 执行的任意命令字符串。

正式 test name 中 `-linux` 固定表示参考 Linux kernel target；`-linux-object` 继续只表示
arceos_ex 的 Linux-object provider，二者不得互相别名或推断。新增 composite 专用 identity 包括
`user-smoke-stress-mem`、`kernel-smoke-stress-mem`、`distro-sh-checkpoints{,-linux}`、
`busybox-init-checkpoints{,-linux}` 和 `rc-local-linux`。BusyBox init/login acceptance identity 为
`busybox-init-login-native` 与 `busybox-init-login-linux-object`；不得保留误称 OpenRC 的 test name。
`distro-sh-checkpoints` 与 `distro-sh-checkpoints-linux` 必须使用同一 scripted payload：连续两次
`/bin/ls`，再执行 `/sbin/poweroff -f`。不得用一侧 shell builtin `exit`、另一侧额外 exec
的方式构造可观测 checkpoint 数量差异。

自动与人工 shell identity 按 provider 固定对应：`scripted-shell` 对应 `shell`，`scripted-shell-lo`
对应 `shell-lo`。每一对都必须使用 `kernel.app = "user-boot"`、相同
provider、release profile、canonical rootfs template 的相同初始内容、128 MiB、8 个 vCPU、
`earlycon=sbi init=/bin/sh`、virtio RNG 和 `guest-shutdown` exit policy。人工 identity 使用
`private-copy`、`interaction = "terminal"`、空 `stdin_steps`、3600 秒和 diagnostic purpose，允许
操作者在私有可写副本上任意输入，且保持 opt-in。自动 identity 使用 `template-readonly`、
`interaction = "scripted"`、120 秒和 acceptance purpose；在 `~ #` 后必须恰好发送
`echo "OK"\nls\nls /lib\nls /\nexit\n`，要求 guest status 0，独立的 `OK` 输出行恰好出现一次、
`ld-musl-riscv64.so.1` 恰好出现一次、`lost+found` 恰好出现两次，并继续禁止 visible unsupported
与 panic marker。`OK` 判据必须带行边界，不得把回显的 `echo "OK"` 命令当作成功证据。自动 identity
按上述命令顺序分别验证 shell builtin、PATH 中的发行版 `ls`、`/lib` 内容和显式 root listing，
并进入默认门禁。
自动证据只覆盖该固定命令闭环与上述共享启动环境，不扩张为 pipe/PTY 或任意人工会话的行为等价性。

未来新增同时提供 native 与 linux-object provider 的 basic test 时，正式 test name 必须使用无 provider
后缀的基础名表示 `kernel.provider = "native"`，并使用同一基础名加 `-lo` 表示
`kernel.provider = "linux-object"`；`lo` 在该命名位置固定表示 linux-object。现有
`*-native` / `*-linux-object` 配置为兼容既有公开接口而保留，本规则不要求批量迁移它们。

v2 disk mode 的含义固定如下：

- `none`：不准备且不附加 block image。
- `template-readonly`：必须声明 `profile = "canonical"`；只核对 template 和 input manifest，以只读
  QEMU drive 附加。
- `private-copy`：必须声明 `profile = "canonical"`；核对 template 后复制到本轮私有路径，可写附加，
  最终必须删除副本。
- `generated`：由 `generator` 选择 runner 内建生成器，在本轮私有路径生成并最终删除。
- `external`：附加配置给出的既有绝对或仓库相对 raw image；runner 不构造、不删除来源。

v1 `canonical-readonly` 兼容映射为 v2 `template-readonly`/canonical；v1 无 profile 的 `private-copy`
兼容映射为 canonical。兼容映射必须记录在 manifest/result 中，不能复制一份长期别名 TOML。

`qemu.interaction` 的互斥规则是：`none` 禁止 `stdin_steps`；`scripted` 必须至少有一个 step；
`terminal` 禁止 steps。v1 按是否存在 `stdin_steps` 映射到 `scripted` 或 `none`。

当前内核在 InitcallPhase 要求 live virtio-blk，因而真实 hello/smoke acceptance 仍应挂载磁盘。
`none` 是框架能力，只有目标 kernel 确实不要求 block device 或 runner 隔离测试才能作为成功用例。

## Scripts and isolation

PreScript/PostScript 只在 TOML 明确声明时运行，不自动发现文件。路径相对 TOML 所在目录解析，推荐
同目录 `<test>.pre.sh` 和 `<test>.post.sh`。runner 只提供测试名、冻结配置路径、kernel、disk、QEMU
log、result、artifact 目录及受控 `PATH`、`TMPDIR`、`LC_ALL`；脚本不能返回或覆盖行为参数。

PreScript 失败时 QEMU 不得启动。只要 QEMU 阶段已经开始，PostScript 就必须在正常退出、expect
失败、异常、terminal 中断或超时后执行；PostScript 失败也使 execution 失败，但不能覆盖更早诊断。

## QEMU interaction and lifecycle

`none` 和 `scripted` 使用封闭 stdin 或声明的 marker/payload steps。它们的 `qemu.log` 和 marker
匹配必须保留完整 guest 字节；但写给宿主展示流时必须跨任意输出分块抑制 guest 的光标位置查询
`ESC[6n`，防止该查询穿过 `tee` 到达调用终端并把终端响应遗留给调用 shell。runner 不得用读取或
清空调用者 stdin 的方式补救该响应。`terminal` 必须使用 PTY：操作者输入字节原样转发，guest 输出
同时写到当前终端和 `qemu.log`，包括由真实交互会话消费的终端查询。`make run` 在 kernel build 和
PreScript 之前检查真实 stdin/stdout TTY；无 TTY 时结构化失败。`build` 和 `manifest` 不要求 TTY。

runner 以独立进程组启动 QEMU，执行整体 timeout，并实现等待 process exit/guest shutdown、marker
或完整合法 `stress_mem` record 后终止的退出策略。`stress_mem` 只有在 header、声明长度和完整 hex
payload 均可解析时才算到达；partial record 不得提前终止。expectation 的观察面是完整
`qemu.log` 与已解码 `stress_mem` payload 的联合：不得因为存在 record 就丢弃串口上的登录、shell
或其他 guest 证据，checkpoint 也不得仅作为未解码 hex 存在。timeout、异常、正常退出和 terminal 中断都必须恢复原终端属性、执行适用的
PostScript、回收完整进程组并删除私有磁盘。不能把 QEMU 或 raw terminal 留给调用者。

host process exit 与 guest exit 是独立事实。guest exit status 从稳定的 `user exit status=N` 日志提取；
kernel smoke/checkpoint callback 等没有该事实的测试使用 marker/count。仅有 QEMU exit 0 不证明通过。

## Result schema v2

每次 `build`/`run` 始终创建 `qemu.log` 和 result schema v2。result 至少记录 request/canonical test、
compatibility alias、config/manifest、command、purpose、开始结束时间、七阶段、QEMU/interaction、每项
expectation、cleanup、`execution_status = "completed" | "failed"` 和
`verdict = "passed" | "failed" | "inconclusive"`。

- acceptance 在 execution completed 且全部 assertion 成功时为 passed；assertion 失败为 failed。
- diagnostic 正常完成为 inconclusive；它不把人工会话声明成自动验收通过。
- build-only 正常完成为 inconclusive；配置/build/disk/script/QEMU lifecycle/cleanup 失败为 execution failed。
- completed 且 verdict 为 passed/inconclusive 返回 0；execution failed 或 verdict failed 返回非零。

配置失败、build-only 和兼容别名请求也必须留下明确的结构化记录。正式 checkpoint callback profile
名为 `checkpoint-kunit-native`、`checkpoint-kunit-linux-object`；runner 将旧 `kunit-*` 名称映射到正式
配置。变量名别名 APP 不改变 test name；旧 test name `hello`、`smoke` 分别映射到 native 正式配置。
所有 test-name 别名都只存在于 runner 映射中，不保留别名 TOML。

公开 test name `user-boot` 已退役，不能再映射到 user smoke 或 shell。`make run/build TEST=user-boot`
与对应的 `APP=user-boot` 请求必须在 build 前生成 schema v2 结构化配置失败结果，错误明确提示改用
`shell`；`kernel.app = "user-boot"` 仍是 TOML 内部选择 user-mode kernel payload 的合法值，不能
把该内部值重新暴露成测试身份。
