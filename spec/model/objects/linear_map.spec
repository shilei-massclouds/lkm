/* LinearMap model specification. */

/*
 * LinearMap 表示 PAGE_OFFSET 起始的物理内存线性映射虚拟区域。
 * 入口前导期只预留该区域，完整 RAM banks 映射由后续完整页表阶段建立。
 */
object LinearMap: AddressSpaceObject {
    initial_state: State::Base;
    parent: KernelAddrSpace;

    attrs {
        range: VirtAddrRange<PhysicalMemory>;
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                }

                ensures {
                    linear_map_area_reserved(self);
                    linear_map_starts_at_page_offset(self);
                    fixmap_adjacent_to_linear_map(FixMap, self);
                }
            }
        }
    }

    /* Ready 表示区域布局已保留；映射发布是 controller 的独立事实。 */
    state State::Ready {
        invariant {
            linear_map_area_reserved(self);
            linear_map_starts_at_page_offset(self);
            fixmap_adjacent_to_linear_map(FixMap, LinearMap);
        }
    }
}
