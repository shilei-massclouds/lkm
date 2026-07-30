/* FixMap model specification. */

/*
 * FixMap 表示入口前导期可用的固定虚拟地址槽位集合。
 * 当前只建模 FDT 槽位，并记录 RawDtb 是否已被安排到该槽位。
 * FDT 槽位容量检查是规格侧的显式前置条件；Linux 实现侧对应
 * FIX_FDT/FIX_FDT_SIZE/MAX_FDT_SIZE 布局和 create_fdt_early_page_table()
 * 的固定映射窗口。
 */
object FixMap: AddressSpaceObject {
    initial_state: State::Base;
    parent: KernelAddrSpace;

    attrs {
        fdt_slot: FixMapSlotRange<Fdt>;
    }

    /*
     * Base 表示 fixmap 槽位布局来自配置，但尚未把 RawDtb 安排到 FDT 槽位。
     */
    state State::Base {
        transitions {
            /*
             * Preset 检查 FDT 槽位存在且能容纳 RawDtb，并把 RawDtb 安排到该槽位。
             * 该检查把 Linux 隐含在 fixmap 布局常量中的容量前提显式化。
             */
            on Transition::Preset -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    RawDtb.state == State::Ready;
                    has_slot(Config.fixmap, FixMapSlot::Fdt);
                    fits_in_fixmap_slot(
                        RawDtb.range,
                        Config.fixmap.fdt,
                        Config.page_size
                    );
                }

                may_change {
                    FixMap.fdt_slot;
                }

                ensures {
                    attrs_accessible(self);
                    fdt_slot == Config.fixmap.fdt;
                    slot_contains(fdt_slot, RawDtb);
                }
            }
        }
    }

    /*
     * Ready 表示 FDT 槽位已经承载 RawDtb，后续页表映射可直接引用该槽位。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            fdt_slot == Config.fixmap.fdt;
            slot_contains(fdt_slot, RawDtb);
        }
    }
}
