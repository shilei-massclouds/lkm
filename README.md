# LKM

LKM 是一个围绕目标内核、规格模型和验证工具持续演进的实验仓库。当前主要内容包括 `spec/` 下的章程与规格、`tools/pyveri/` 规格推导验证器，以及 `impl/arceos_ex/` 目标内核实现。

## 目录

- `docs/ROADMAP.md`：仍有后续责任的统一开发计划和任务优先级入口；完成项见 `docs/roadmap/completed.md`。
- `docs/DEFECTS.md`：尚未闭环的实现缺陷和间歇性问题记录。
- `spec/`：项目章程、规格文档与配图资源。
- `tools/pyveri/`：Python 版规格推导验证器工具工程目录。
- `impl/providers/`：外部 Linux object provider registry 与来源说明。
- `impl/arceos_ex/`：当前主要目标内核实现与测试入口。

`spec/model/objects/*.spec` 的逐文件 coding 归属由
[`spec/coding/objects/README.md`](spec/coding/objects/README.md) 统一索引；多个 model 文件可以共享
一个专题，但不得留下未声明归属。

## 初步验证

在仓库根目录执行规格推导与基础测试：

```sh
make verify
make verify VERBOSE=1
make verify REPORT=graph
make test
```

`make verify` 会对默认规格 `spec/model/main.spec` 执行完整、严格的 parse/model/derive/check
验证，默认只输出各阶段概要以及包括 deferred/trimmed 在内的计数。需要查看完整 trace 和逐项
deferred/trimmed 明细时使用 `make verify VERBOSE=1`；只有 `VERBOSE` 的值严格等于 `1` 才启用
详细文本输出，验证范围、严格门禁和退出码保持不变。`REPORT=graph` 保持 trace SVG 报告行为，
不受 `VERBOSE` 影响。

`make test` 的规格验证阶段只执行一次 `make verify VERBOSE=1`，并通过测试日志输出详细报告；
它不会再额外执行默认概要模式。除此之外，`make test` 会汇总 checkpoint inventory/Linux
mapping/coverage/instrumentation plan 产物漂移检查、默认运行入口、用户态启动入口、KUnit 风格检查和
smoke 启动测试；当前默认同时覆盖 native PLIC provider 与 Linux object provider，适合作为修改后的第一轮回归入口。

checkpoint inventory、Linux mapping、Linux mapping coverage 和 Linux instrumentation plan 是已提交的审阅产物。需要重新生成时运行：

```sh
make checkpoints
```

只做门禁验证时运行：

```sh
make test-checkpoints
```

`make checkpoints` 会按 inventory -> Linux mapping -> Linux mapping coverage -> Linux instrumentation plan 顺序更新产物。coverage report 是 mapping-only 的紧凑审阅视图，汇总映射分类、confidence、Linux 文件和 unmapped checkpoint family 覆盖情况，不包含逐 checkpoint 明细。instrumentation plan 只从 exact Linux mapping 派生，输出未来 Linux marker 同步工具需要的 marker 和 anchor fingerprint；range/unmapped 映射不会生成插桩计划项。

`test-checkpoints` 会先跑 checkpoint 工具单元测试，再以只读 `--check` 模式比较 `tools/out/checkpoints/` 下的 JSON/Markdown；它不会重写已提交产物。`make checkpoints-linux-check` 通过 instrumentation plan 的 `--check --check-markers` 同时校验已提交产物和 `../linux-6.12` markers，并报告 missing/stale/fingerprint mismatch。`make difftest` 会先串行运行 `test-checkpoints` 与 `checkpoints-linux-check`；任一失败都会在 paired runner 启动前停止，且不会自动更新产物或 Linux tree。`make test` 会在 QEMU/runtime 用例前运行本仓 artifact drift 门禁。

人工同步顺序固定为：修改 mapping/语义并审核 Linux instrumentation，显式运行 `make checkpoints`，再运行 `make checkpoints-linux-check` 和 `make difftest`。两棵仓库应分别审核和提交。

需要生成 Linux marker 补丁时，先更新本仓库审阅产物，再从已提交 instrumentation plan 输出可审阅 patch；该流程不会直接修改 `../linux-6.12`：

```sh
make checkpoints
python3 tools/checkpoints/plan_linux_instrumentation.py --emit-marker-patch /tmp/lkm-linux-markers.patch
```

随后人工审阅 `/tmp/lkm-linux-markers.patch`，确认无误后再显式应用到参考 Linux tree。

## 基本测试流水线

`make run TEST=<name>` 是单轮基本测试的完整入口；省略 `TEST` 时运行
`hello-native`。`make build TEST=<name>` 从同一 TOML 只构造 kernel image，不准备磁盘也不启动
QEMU。常用配置包括：

```sh
make run
make run TEST=kernel-smoke-native
make run TEST=user-smoke-native
make run TEST=shell
make run TEST=shell-lo
make run TEST=requested-init-native
make run TEST=distro-ls-native
make run TEST=distro-sh-native
make build TEST=hello-linux-object
```

`TEST` 是正式选择变量。旧 `APP` 只是它的变量名别名，接受相同的完整 TEST 名空间，例如
`make run APP=busybox-init-login-native` 等价于选择 `TEST=busybox-init-login-native`。命令行与真实环境变量写法
遵循同一规则；APP 不是 kernel app 行为覆盖，不能和显式 TEST 同时使用。公开 test name
`user-boot` 已退役，`TEST=user-boot` 与 `APP=user-boot` 都会在 build 前结构化失败并提示改用
`shell`；TOML 内部的 `kernel.app = "user-boot"` 仍只是 kernel 构建选择。

配置位于 `impl/arceos_ex/tests/basic/cases/`。APP、provider、probe、profile、磁盘策略、QEMU
cmdline/device/stdin 和结果断言都以 TOML 为唯一来源；APP 只能选择整份 TEST 配置，不能覆盖
其中字段。`QEMU`、编译器、
`LINUX_PROVIDER_DIR`、rootfs cache/source 和输出根等宿主参数仍可按文档覆盖。

每次执行在 `impl/arceos_ex/tests/basic/out/` 下保留冻结的 `manifest.json`、完整 `qemu.log` 和
result schema v2。流水线区分 execution status 与 passed/failed/inconclusive verdict，并负责 timeout、
none/scripted/terminal interaction、guest exit、PTY/进程组回收、PreScript/PostScript 和私有磁盘清理；
QEMU 返回 0 本身不代表 guest 测试通过。

### Canonical rootfs

`make disk` 等价于 `make disk ROOTFS=canonical`，独立构造
`impl/arceos_ex/build/rootfs/canonical.raw`；未知 profile 会失败。该镜像使用确定的 BusyBox
`/sbin/init` 配置，以仓库内 `/etc/inittab` 启动 tty1/ttyS0 getty，不引用 minirootfs 中未安装的 OpenRC；
镜像锁定 root，并内置稳定的 `test/test` 非 root 账户。仓库 fixture、
rc.local ELF launcher/script 位于 `/opt/lkm/tests/`，经过校验的 sibling LTP staging 位于 `/opt/ltp`。

builder 记录 Alpine tarball、配置/构造脚本、fixture、工具和 LTP 输入指纹；输入变化会自动重建，
输入不变则复用，`make disk FORCE=1` 强制重建。`make run` 只核对 template manifest，缺失/过期时
提示先构造；它绝不构造 rootfs 或覆盖 `/etc`。rc.local/BusyBox init login 双 provider acceptance 进入默认回归；
`user-smoke-{native,linux-object}` 是非交互自动验收；`shell`（native）与 `shell-lo`（linux-object）是使用 canonical
私有可写副本、`init=/bin/sh` 和真实 PTY 的人工诊断，正常退出 verdict 为 inconclusive。两者身份不因
调用上下文切换，shell 与独立的 LTP terminal diagnostic 都仅显式运行。

## Provider 机制

Provider 是构建时选择机制，用于决定内核镜像链接哪一组底层实现对象。当前 `native` 使用仓库内
实现，`linux-object` 会把 Linux 构建出的 `drivers/irqchip/irq-sifive-plic.o` 作为 linker input。
基本测试通过名称成对选择 provider，而不是通过 Make 变量覆盖：

未来新增双 provider basic test 时，无后缀基础名固定表示 native，`-lo` 后缀固定表示
linux-object；现有 `*-native` / `*-linux-object` 名称继续保留，不随本次 shell 改名批量迁移。

常用入口保持不变，只需要通过 make 变量选择 provider：

```sh
make run TEST=hello-native
make run TEST=hello-linux-object
make run TEST=user-smoke-native
make run TEST=user-smoke-linux-object
```

完整回归入口是：

```sh
make test
```

`make test` 默认会跑 native 与 `linux-object` 两套 provider 矩阵。临时只跑 native 时，可以清空 provider 测试列表：

```sh
make test TEST_PLIC_PROVIDERS=
```

Linux object provider 声明位于 `impl/providers/linux-6.12.mk`。默认依赖仓库同级的兄弟 Linux tree：

```text
<parent>/
  linux-6.12/
  lkm/
```

当前期望对象路径是 `../linux-6.12/drivers/irqchip/irq-sifive-plic.o`。如果 Linux tree 不在默认位置，可以显式指定：

```sh
make test LINUX_PROVIDER_DIR=/path/to/linux-6.12
make run TEST=hello-linux-object LINUX_PROVIDER_DIR=/path/to/linux-6.12
```

## 压力测试

压力测试根目录位于 `impl/arceos_ex/tests/stress/`。日常入口是：

```sh
make test-stress
```

`make stress-test` 保留为兼容别名。默认 `STRESS_RUNS=10`，应用到套件里的每个 case。不指定
`STRESS_CASES` 时分别循环 `user-smoke-native`、`kernel-smoke-native` 和 `distro-sh-native`。
Composite runner 不执行 command/setup/QEMU；每轮只在固定子目录调用一次 basic test，再读取其
`result.json` 与 `qemu.log`。非零 runs 的整个入口只先运行一次 `make disk`；超时只由 basic TOML 决定。

常用覆盖方式：

```sh
make test-stress STRESS_RUNS=30
```

只检查配置和输出目录生成、不执行 QEMU：

```sh
make test-stress STRESS_RUNS=0
```

也可以显式指定 case；此时只运行指定 case：

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml \
  STRESS_RUNS=30
```

每次压力测试会在 `impl/arceos_ex/tests/stress/out/<timestamp>-<case>/` 下保存原始日志、结构化事件、去重后的事件序列、分类统计和 `report.md`。重复序列只保存第一次代表样本，后续 run 通过计数和 run id 归档。

单 case 可传入 `STRESS_BASELINE=<report-dir>` 对比 schema-v2 历史报告；历史失败率、分类/序列增删与
最近序列首个分歧只作分析信息，不额外改变当前调用的退出状态。差分测试也只顺序执行左右两个
basic test，两侧独立完成且 expectations 通过后才比较 checkpoint 序列。

## 维护方式

文档、规格、工具和实现采用持续迭代方式维护，变更通过 Git 提交记录演进过程。提交前请保持 diff 聚焦，不要混入生成输出或无关格式化变更。
