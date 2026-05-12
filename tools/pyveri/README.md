# pyveri

`pyveri` 是组件化内核规格的 Python 版推导验证器工具目录。

当前阶段的输入规格文件是：

```text
../../spec/entry-prelude-object-model.spec
```

## 当前目标

第一版验证器只面向入口前导期对象模型，做基于规格本身的纸上推导验证：

- 解析 `.spec` 文件，忽略 `/* ... */` 和 `// ...` 注释。
- 建立对象、状态、事件、依赖、不变量和延期义务的模型。
- 从 `StartupTimeline.Event::Setup` 开始，推导当前启动时间轴是否能达到目标状态。
- 报告已证明、无法证明、矛盾和 `deferred` 条目。

## 非目标

当前验证器不依赖运行时数据，也不处理以下输入：

- 内核插桩轨迹
- 模拟器采集结果
- 硬件采集结果
- 参考内核差分记录

这些属于后续测试阶段或实证验证工具的范围。

## 目录

- `src/pyveri/`：后续 Python 源码实现目录。
- `tests/`：后续验证器测试用例目录。
- `DEVELOPMENT.md`：验证器第一版实现计划。

## 当前运行方式

### 源码方式

在仓库根目录执行：

```bash
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --derive
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --derive --strict
PYTHONPATH=tools/pyveri/src python -m pyveri parse spec/entry-prelude-object-model.spec
PYTHONPATH=tools/pyveri/src python -m pyveri model spec/entry-prelude-object-model.spec
PYTHONPATH=tools/pyveri/src python -m pyveri derive spec/entry-prelude-object-model.spec --strict
PYTHONPATH=tools/pyveri/src python -m pyveri check spec/entry-prelude-object-model.spec
PYTHONPATH=tools/pyveri/src python -m pyveri view spec/entry-prelude-object-model.spec object
PYTHONPATH=tools/pyveri/src python -m pyveri render spec/entry-prelude-object-model.spec object --format dot -o object.gv
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --derive --strict --work-dir tools/build
PYTHONPATH=tools/pyveri/src python -m pyveri render spec/entry-prelude-object-model.spec object --format dot --work-dir tools/build
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --text object
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --graph object -o object.gv
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --text drives
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --graph drives -o drives.gv
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --text timeline
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --graph timeline -o timeline.svg
PYTHONPATH=tools/pyveri/src python -m unittest discover -s tools/pyveri/tests
```

当前 `pyveri` 已作为 driver 调度独立阶段工具，CLI 保留 `parse`、`model`、`derive`、`check`、`view` 和 `render` 子命令以及旧参数形式兼容。driver 默认使用临时目录保存中间文件；传入 `--work-dir tools/build` 时会保留本次流水线生成的 AST、model、derive、check、view 和 render 中间文件。

独立工具链已经提供 `common` 公共库骨架、`parse`、`model`、`derive`、`check`、`view` 和 `render` 阶段工具。源码方式运行：

```bash
PYTHONPATH=tools/common/src:tools/parse/src:tools/pyveri/src python -m parse_tool spec/entry-prelude-object-model.spec -o tools/build/entry-prelude-object-model.ast.json
PYTHONPATH=tools/common/src:tools/model/src:tools/pyveri/src python -m model_tool tools/build/entry-prelude-object-model.ast.json -o tools/build/entry-prelude-object-model.model.json
PYTHONPATH=tools/common/src:tools/derive/src:tools/pyveri/src python -m derive_tool tools/build/entry-prelude-object-model.model.json -o tools/build/entry-prelude-object-model.derive.json
PYTHONPATH=tools/common/src:tools/check/src python -m check_tool tools/build/entry-prelude-object-model.derive.json -o tools/build/entry-prelude-object-model.check.json
PYTHONPATH=tools/common/src:tools/view/src:tools/pyveri/src python -m view_tool tools/build/entry-prelude-object-model.model.json object -o tools/build/entry-prelude-object-model.object.view.json
PYTHONPATH=tools/common/src:tools/render/src:tools/pyveri/src python -m render_tool tools/build/entry-prelude-object-model.object.view.json --format dot -o tools/build/entry-prelude-object-model.object.gv
PYTHONPATH=tools/common/src:tools/parse/src:tools/pyveri/src python -m unittest discover -s tools/parse/tests
PYTHONPATH=tools/common/src:tools/parse/src:tools/model/src:tools/pyveri/src python -m unittest discover -s tools/model/tests
PYTHONPATH=tools/common/src:tools/parse/src:tools/model/src:tools/derive/src:tools/pyveri/src python -m unittest discover -s tools/derive/tests
PYTHONPATH=tools/common/src:tools/parse/src:tools/model/src:tools/derive/src:tools/check/src:tools/pyveri/src python -m unittest discover -s tools/check/tests
PYTHONPATH=tools/common/src:tools/parse/src:tools/model/src:tools/view/src:tools/pyveri/src python -m unittest discover -s tools/view/tests
PYTHONPATH=tools/common/src:tools/parse/src:tools/model/src:tools/view/src:tools/render/src:tools/pyveri/src python -m unittest discover -s tools/render/tests
```

未使用 `-o` 时，CLI 会先输出解析摘要，再执行静态模型装配和引用检查，并输出默认推导摘要；使用 `--derive` 时输出完整推导报告。默认情况下，推导结果为 `blocked` 仍返回 0，便于查看报告；需要把未达目标作为命令失败时使用 `--strict`。
使用 `--graph object -o <file>` 时，图内容直接写入文件。
`object` 视图只输出对象之间的静态 `parent` 父子关系。
`drives` 视图输出事件之间的驱动关系。
`timeline` 文本视图按时间先后输出，图形视图直接输出自底向上的 SVG 时间轴。
当前主开发环境为 WSL2/Linux，命令示例默认使用 `/` 路径分隔符和 `PYTHONPATH=... command` 形式。

### 安装命令

如果当前 Python 环境具备 Python 打包构建工具，可以在仓库根目录执行：

```bash
python -m pip install -e tools/pyveri
pyveri spec/entry-prelude-object-model.spec
pyveri spec/entry-prelude-object-model.spec --derive
pyveri spec/entry-prelude-object-model.spec --text object
pyveri spec/entry-prelude-object-model.spec --graph object -o object.gv
pyveri spec/entry-prelude-object-model.spec --graph drives -o drives.gv
pyveri spec/entry-prelude-object-model.spec --graph timeline -o timeline.svg
```

### 本地命令脚本

如果不安装 Python 包，可以直接使用仓库内的本地脚本。

Windows PowerShell：

```powershell
.\tools\pyveri\bin\pyveri.cmd spec\entry-prelude-object-model.spec --graph object -o object.gv
```

也可以临时加入 PATH，使命令形式变成 `pyveri ...`：

```powershell
$env:PATH = "$(Resolve-Path tools\pyveri\bin);$env:PATH"
pyveri spec\entry-prelude-object-model.spec --graph object -o object.gv
```

Linux：

```bash
sh tools/pyveri/bin/pyveri spec/entry-prelude-object-model.spec --graph object -o object.gv
```

也可以临时加入 PATH：

```bash
PATH="$PWD/tools/pyveri/bin:$PATH"
pyveri spec/entry-prelude-object-model.spec --graph object -o object.gv
```

Windows PowerShell 命令仍可用于旧开发环境：

```powershell
$env:PYTHONPATH='tools\pyveri\src'
python -m pyveri spec\entry-prelude-object-model.spec --derive
python -m unittest discover -s tools\pyveri\tests
```

### 查看 DOT 图

安装 Graphviz 后，可以把 `--graph object` 输出渲染为 SVG。

Windows PowerShell：

```powershell
pyveri spec\entry-prelude-object-model.spec --graph object -o object.gv
dot -Tsvg object.gv -o object.svg
start object.svg
```

Linux：

```bash
pyveri spec/entry-prelude-object-model.spec --graph object -o object.gv
dot -Tsvg object.gv -o object.svg
xdg-open object.svg
```

`-o` 会由 `pyveri` 直接写出 ASCII 编码的 Graphviz 文件，避免 Windows PowerShell 重定向生成 UTF-16 文件。
如果没有安装 Graphviz，也可以把 `object.gv` 的内容粘贴到在线 Graphviz/DOT viewer 中查看。
