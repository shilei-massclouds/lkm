# ElfObject

`ElfObject` 表示一个已经读取到 kernel-owned buffer 的 ELF artifact 及其 load plan。它支持 main
executable 与 interpreter 两种 role：main 的 `PT_INTERP` 绑定第二个 interpreter `ElfObject`，动态入口
最终指向 interpreter entry。

对象负责 ELF64/little-endian/RISC-V/type/header/program-header 校验、`PT_LOAD` 段与权限计划、load bias、
entry、auxv 输入和 `.bss` zero plan。实际 staging mapping 由 `UserAddressSpace` 消费该计划；
`ExecTransaction` 负责格式分派和 commit。

本对象不拥有 transaction、registry、handler 搜索或候选 init 策略。load 继续并入对象 setup/address-space
materialization，不引入独立 `ElfLoader` 生命周期对象；`ElfBinaryFormat` 只是 registry handler 类型。
