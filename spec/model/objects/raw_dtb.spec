/* RawDtb model specification. */

/*
 * RawDtb 表示启动参数 dtb_pa 指向的原始设备树二进制。
 * 它分层确认原始 dtb 的固定头部可读边界，再验证头部和完整物理范围。
 * 这是规格前置证明边界：Linux/RISC-V setup_vm() 主要先建立 FDT
 * fixmap 映射，后续 parse_dtb()/early_init_dt_scan() 再验证 header
 * 并扫描内容；本规格在 EarlyVm.Preset 前置收口这些安全前提。
 */
object RawDtb: ResourceObject {
    initial_state: State::Base;
    parent: BootInitFlow;

    attrs {
        header: DtbHeader;
        header_range: PhysAddrRange<DtbHeader>;
        range: PhysAddrRange<Dtb>;
    }

    /*
     * Base 表示只知道启动参数中给出了 dtb 物理地址，尚未验证头部。
     */
    state State::Base {
        transitions {
            /*
             * Preset 只确认原始 dtb 固定头部的范围可安全读取。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootArgs.state == State::Online;
                    OpenSBI.state == State::Online;
                    firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa);
                }

                may_change {
                    RawDtb.header;
                    RawDtb.header_range;
                }

                ensures {
                    BootArgs.dtb_pa != 0;
                    dtb_header_range_addition_safe(BootArgs.dtb_pa, size_of::<DtbHeader>());
                    header_range.start == BootArgs.dtb_pa;
                    header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>();
                    firmware_dtb_header_accessible(header_range);
                    raw_dtb_nodes_unparsed(self);
                }
            }
        }
    }

    /*
     * Prepared 表示原始 dtb 固定头部可安全读取，但头部与完整范围尚未验证。
     */
    state State::Prepared {
        invariant {
            header_range.start == BootArgs.dtb_pa;
            header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>();
            firmware_dtb_header_accessible(header_range);
            raw_dtb_nodes_unparsed(self);
        }

        transitions {
            /*
             * Setup 验证头部标识、读取 total_size 并确定原始 dtb 的完整物理范围。
             * 该范围证明用于后续 FDT fixmap 容量检查和映射安全性。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    OpenSBI.state == State::Online;
                    firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa);
                    Config.state == State::Online;
                }

                may_change {
                    RawDtb.range;
                }

                ensures {
                    valid_dtb_magic(header);
                    valid_dtb_header(header);
                    header.total_size >= size_of::<DtbHeader>();
                    dtb_range_addition_safe(BootArgs.dtb_pa, header.total_size);
                    range.start == BootArgs.dtb_pa;
                    range.end == BootArgs.dtb_pa + header.total_size;
                    firmware_dtb_range_accessible_from_handoff_contract(range);
                    fits_in_fixmap_slot(
                        RawDtb.range,
                        Config.fixmap.fdt,
                        Config.page_size
                    );
                    raw_dtb_nodes_unparsed(self);
                }
            }
        }
    }

    /*
     * Ready 表示原始 dtb 的头部和完整物理范围均已验证。
     */
    state State::Ready {
        invariant {
            valid_dtb_magic(header);
            valid_dtb_header(header);
            header.total_size >= size_of::<DtbHeader>();
            dtb_range_addition_safe(BootArgs.dtb_pa, header.total_size);
            range.start == BootArgs.dtb_pa;
            range.end == BootArgs.dtb_pa + header.total_size;
            firmware_dtb_range_accessible_from_handoff_contract(range);
            fits_in_fixmap_slot(
                RawDtb.range,
                Config.fixmap.fdt,
                Config.page_size
            );
            raw_dtb_nodes_unparsed(self);
        }
    }
}
