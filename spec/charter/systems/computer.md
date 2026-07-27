> **AI 保护锁：已经锁定；未经用户明确解锁，AI 只能提出建议，不得直接修改；授权修改完成后必须重新锁定。**

# Computer System

计算机系统Computer System，简称计算机Computer，是由硬件平台、固件、内核集成后形成的一体化系统，它的生命周期包括定义规格、构造组装和集成、启动与应用/测试三个阶段，分别对应标准生命周期的Preset、Setup和Enable。人类Human触发计算机系统的上述生命周期过程。

## 生命周期

* 定义规格 - Preset

  规格将会严格约束计算机系统的行为，包括对硬件平台、固件和内核的规格定义。计算机系统向这三个子系统依次发出Preset信号，要求完成各层次的规则定义。本阶段成功后进入Prepared状态，表明计算机各层的规格都已经充分建立。

* 构造组装和集成 - Setup

  首先，分别构造硬件平台、固件和内核，具体方式是向这三个子系统发出Setup信号；然后集成形成一体化系统。本阶段执行成功后进入Ready状态，表明计算机作为整体系统已经就绪，处于加电启动的前夕。

* 启动与应用/测试 - Enable

  人类是计算机系统启动的信号源，具体来说是按下开机键。过程是：人类向计算机系统发出启动信号，计算机系统进入Online状态，然后异步向硬件平台发出Enable信号以启动硬件平台；此后硬件平台、OpenSBI依次向下一级异步发出Enable信号完成接力启动，直至内核启动；内核完成启动并建立应用环境后，进入内核配置选定的应用或测试。

  注意：计算机系统启动后，不等待硬件平台、固件或内核的后续启动结果。

## Mapping

- Model: `spec/model/systems/computer.spec`
- Coding: `spec/coding/systems/computer.md`
- Implementation: `impl/arceos_ex/src/systems/computer.rs`
