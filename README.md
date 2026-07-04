# LKM

LKM 是一个围绕目标内核、规格模型和验证工具持续演进的实验仓库。当前主要内容包括 `spec/` 下的章程与规格、`tools/pyveri/` 规格推导验证器，以及 `impl/arceos_ex/` 目标内核实现。

## 目录

- `docs/ROADMAP.md`：统一开发计划和任务优先级入口。
- `docs/DEFECTS.md`：尚未闭环的实现缺陷和间歇性问题记录。
- `spec/`：项目章程、规格文档与配图资源。
- `tools/pyveri/`：Python 版规格推导验证器工具工程目录。
- `impl/providers/`：外部 Linux object provider registry 与来源说明。
- `impl/arceos_ex/`：当前主要目标内核实现与测试入口。

## 初步验证

在仓库根目录执行规格推导与基础测试：

```sh
make verify
make test
```

`make verify` 会对默认规格 `spec/model/main.spec` 执行推导检查。`make test` 会汇总规格验证、checkpoint inventory/Linux mapping/coverage/instrumentation plan 产物漂移检查、默认运行入口、用户态启动入口、KUnit 风格检查和 smoke 启动测试；当前默认同时覆盖 native PLIC provider 与 Linux object provider，适合作为修改后的第一轮回归入口。

checkpoint inventory、Linux mapping、Linux mapping coverage 和 Linux instrumentation plan 是已提交的审阅产物。需要重新生成时运行：

```sh
make checkpoints
```

只做门禁验证时运行：

```sh
make test-checkpoints
```

`make checkpoints` 会按 inventory -> Linux mapping -> Linux mapping coverage -> Linux instrumentation plan 顺序更新产物。coverage report 是 mapping-only 的紧凑审阅视图，汇总映射分类、confidence、Linux 文件和 unmapped checkpoint family 覆盖情况，不包含逐 checkpoint 明细。instrumentation plan 只从 exact Linux mapping 派生，输出未来 Linux marker 同步工具需要的 marker 和 anchor fingerprint；range/unmapped 映射不会生成插桩计划项。

`test-checkpoints` 会先跑 checkpoint 工具单元测试，再以只读 `--check` 模式比较 `tools/out/checkpoints/` 下的 JSON/Markdown；它不会重写已提交产物。Linux marker 扫描通过 `tools/checkpoints/plan_linux_instrumentation.py --check-markers` 显式运行，当前不属于默认门禁，因为 `../linux-6.12` 尚未插入 marker。`make test` 会在 QEMU/runtime 用例前运行 artifact drift 门禁。

## 运行目标内核

默认运行 hello 应用：

```sh
make run
```

运行用户态启动路径：

```sh
make run APP=user-boot
```

`APP=user-boot` 会走普通用户态 payload 读取与启动路径。默认命令行会指定 `init=/bin/sh`，进入 Alpine rootfs 内的 BusyBox shell；在 shell 中输入 `exit` 可退出并触发 `user exit status=0`。该入口也是 `docs/DEFECTS.md` 中 DF-0001 的固定复现入口之一。定位这类间歇性问题时，不要只用 probe 路径替代普通路径，因为额外观测可能改变时序。

### Rootfs Overlay

`APP=user-boot` 默认从 Alpine minirootfs 构造 ext2 rootfs。仓库的 rootfs overlay 是镜像构造期覆盖，不是运行期 overlayfs：构造 disk 时先把 Alpine rootfs 解到 staging 目录，再按 `impl/arceos_ex/tests/user/rootfs-overlay.map` 编译用户态测试程序并复制到 rootfs 内的目标路径。

当前默认 map 把 dynamic musl `user_smoke` 覆盖为 `/sbin/init`：

```text
/sbin/init    user_smoke  musl  dynamic
```

手工 `make run APP=user-boot` 默认通过 `init=/bin/sh` 进入发行版 shell，不会选择 overlay 后的 `/sbin/init`。单独运行当前用户态 smoke 时，需要显式让内核走默认 fallback 列表：

```sh
make run APP=user-boot QEMU_APPEND='earlycon=sbi' FORCE=1
```

`FORCE=1` 会重新生成 disk，确保当前 overlay map 和 `impl/arceos_ex/tests/user/smoke/` 下的源码被重新编译进 rootfs。若不加 `FORCE=1`，`make run APP=user-boot` 会复用已有的 `impl/arceos_ex/build/virtio-blk.raw`。

需要不污染默认 disk 时，可以指定临时 disk：

```sh
make run APP=user-boot QEMU_APPEND='earlycon=sbi' VIRTIO_BLK_IMAGE=/tmp/lkm-user-smoke.raw FORCE=1
```

禁用构造期 overlay、直接使用 Alpine rootfs 内容时：

```sh
make run APP=user-boot ROOTFS_OVERLAY=none FORCE=1
```

需要让 checkpoint 自报基本进度时，可以启用 announce probe：

```sh
make run APP=user-boot PROBE=announce
```

`LOG=trace` 目前仍作为兼容入口保留，等价于启用 `PROBE=announce`；新用法应优先使用 `PROBE=announce`。

### Kernel Command Line

`impl/arceos_ex/Makefile` 对普通 app 默认设置：

```text
QEMU_APPEND ?= earlycon=sbi
```

对 `APP=user-boot`，默认值是：

```text
QEMU_APPEND ?= earlycon=sbi init=/bin/sh
```

`make run` 会把该值原样传给 QEMU 的 `-append`。需要指定 Linux-like requested init 时，可以覆盖：

```sh
make run APP=user-boot QEMU_APPEND='earlycon=sbi init=/bin/ls' FORCE=1
```

`init=` 只改变内核命令行下的 init 选择语义；rootfs overlay 仍用于构造测试 disk 时注入稳定 fixture。`make test` 的 user-boot smoke case 会显式传入 `QEMU_APPEND=earlycon=sbi`、临时 disk 和默认 overlay map，因此仍验证 `user_smoke`，不会进入交互式 shell。

## Provider 机制

Provider 是构建时选择机制，用于决定内核镜像链接哪一组底层实现对象。当前已经接入的是 PLIC provider：默认 `PLIC_PROVIDER=native` 使用仓库内实现，`PLIC_PROVIDER=linux-object` 会把 Linux 构建出的 `drivers/irqchip/irq-sifive-plic.o` 直接作为 linker input 组成新的 kernel image。这个变量影响 `make build` 产物，不是运行期动态切换开关。

常用入口保持不变，只需要通过 make 变量选择 provider：

```sh
make run
make run APP=user-boot
make run PLIC_PROVIDER=linux-object
make run APP=user-boot PLIC_PROVIDER=linux-object
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
make run PLIC_PROVIDER=linux-object LINUX_PROVIDER_DIR=/path/to/linux-6.12
```

## 压力测试

压力测试根目录位于 `impl/arceos_ex/tests/stress/`。日常入口是：

```sh
make stress-test
```

默认 `STRESS_RUNS=10`，应用到套件里的每个 case。不指定 `STRESS_CASES` 时会运行默认压力测试套件，当前覆盖 DF-0001 的非 probe `APP=user-boot` overlay `/sbin/init` 路径、DF-0002 的非 probe `APP=smoke` 路径，以及 DF-0003 的非 PTY `/bin/sh` delayed-input `/bin/ls` 外部命令路径。`STRESS_TIMEOUT` 默认不传给 runner，由各 case 的 `timeout_seconds` 生效；当前 case 默认是 120 秒。

常用覆盖方式：

```sh
make stress-test STRESS_RUNS=30
make stress-test STRESS_TIMEOUT=60
```

只检查配置和输出目录生成、不执行 QEMU：

```sh
make stress-test STRESS_RUNS=0
```

也可以显式指定 case；此时只运行指定 case：

```sh
make stress-test \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml \
  STRESS_RUNS=30
```

每次压力测试会在 `impl/arceos_ex/tests/stress/out/<timestamp>-<case>/` 下保存原始日志、结构化事件、去重后的事件序列、分类统计和 `report.md`。重复序列只保存第一次代表样本，后续 run 通过计数和 run id 归档。

## 维护方式

文档、规格、工具和实现采用持续迭代方式维护，变更通过 Git 提交记录演进过程。提交前请保持 diff 聚焦，不要混入生成输出或无关格式化变更。
