# LKM

LKM 是一个围绕目标内核、规格模型和验证工具持续演进的实验仓库。当前主要内容包括 `spec/` 下的章程与规格、`tools/pyveri/` 规格推导验证器，以及 `impl/arceos_ex/` 目标内核实现。

## 目录

- `docs/ROADMAP.md`：统一开发计划和任务优先级入口。
- `docs/DEFECTS.md`：尚未闭环的实现缺陷和间歇性问题记录。
- `spec/`：项目章程、规格文档与配图资源。
- `tools/pyveri/`：Python 版规格推导验证器工具工程目录。
- `impl/arceos_ex/`：当前主要目标内核实现与测试入口。

## 初步验证

在仓库根目录执行规格推导与基础测试：

```sh
make verify
make test
```

`make verify` 会对默认规格 `spec/model/main.spec` 执行推导检查。`make test` 会汇总规格验证、KUnit 风格检查和 smoke 启动测试，适合作为修改后的第一轮回归入口。

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

需要查看 checkpoint/trace 输出时，可以增加 `LOG=trace`：

```sh
make run APP=user-boot LOG=trace
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
