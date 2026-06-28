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

`make verify` 会对默认规格 `spec/model/main.spec` 执行推导检查。`make test` 会汇总规格验证、默认运行入口、用户态启动入口、KUnit 风格检查和 smoke 启动测试；当前默认同时覆盖 native PLIC provider 与 Linux object provider，适合作为修改后的第一轮回归入口。

## 运行目标内核

默认运行 hello 应用：

```sh
make run
```

运行用户态启动路径：

```sh
make run APP=user-boot
```

`APP=user-boot` 会走普通用户态 payload 读取与启动路径，是 `docs/DEFECTS.md` 中 DF-0001 的固定复现入口之一。定位这类间歇性问题时，不要只用 probe 路径替代普通路径，因为额外观测可能改变时序。

需要让 checkpoint 自报基本进度时，可以启用 announce probe：

```sh
make run APP=user-boot PROBE=announce
```

`LOG=trace` 目前仍作为兼容入口保留，等价于启用 `PROBE=announce`；新用法应优先使用 `PROBE=announce`。

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

压力测试根目录位于 `impl/arceos_ex/tests/stress/`。DF-0001 的默认 case 已绑定普通 `make run APP=user-boot`，可以直接从仓库根目录执行：

```sh
impl/arceos_ex/tests/stress/runner.py --runs 30
```

只检查配置和输出目录生成、不执行 QEMU：

```sh
impl/arceos_ex/tests/stress/runner.py --runs 0
```

也可以显式指定 case：

```sh
impl/arceos_ex/tests/stress/runner.py \
  impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml \
  --runs 30
```

每次压力测试会在 `impl/arceos_ex/tests/stress/out/<timestamp>-<case>/` 下保存原始日志、结构化事件、去重后的事件序列、分类统计和 `report.md`。重复序列只保存第一次代表样本，后续 run 通过计数和 run id 归档。

## 维护方式

文档、规格、工具和实现采用持续迭代方式维护，变更通过 Git 提交记录演进过程。提交前请保持 diff 聚焦，不要混入生成输出或无关格式化变更。
