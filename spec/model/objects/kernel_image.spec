/* Kernel image loaded in memory before the entry prelude begins. */

predicate gp_relative_addressing_ready<I: ImageObject>(image: I) -> bool;
predicate kernel_image_bss_zeroing_completed<I: ImageObject>(image: I) -> bool;
predicate kernel_image_bss_ordinary_writable<I: ImageObject>(image: I) -> bool;

object KernelImage: ImageObject {
    initial_state: State::Base;
    parent: KernelAddrSpace;

    attrs {
        start: Derived<SymbolAddr, Lds.kernel_start>;
        phys_start: PhysAddr<KernelImage>;
        end: Derived<SymbolAddr, Lds.kernel_end>;
        virt_range: Derived<VirtAddrRange<KernelImage>, range(Config.kernel_link_addr, Config.kernel_link_addr + Config.kernel_image_va_window_size)>;
        segments: SegmentSet<KernelImageSegment>;
    }

    /*
     * Base 表示内核映像已进入模型空间，但相对 gp 寻址基准和 BSS 状态
     * 尚未由本阶段处理。
     */
    state State::Base {
        transitions {
            /*
             * 建立相对 gp 寻址基准，支持对内核映像中全局数据的快速访问机制。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Lds.state == State::Online;
                    Riscv64.state == State::Online;
                }

                ensures {
                    kernel_image_phys_start_observed_at_entry(self, phys_start);
                    phys_start == OpenSBI.kernel_load_pa;
                    gp_relative_addressing_ready(KernelImage);
                }
            }
        }
    }

    /*
     * Prepared 表示相对 gp 寻址机制已经可用，且 BSS 边界已可识别。
     */
    state State::Prepared {
        invariant {
            valid_segment_set(segments);
            segments.bss.range == range(Lds.bss_start, Lds.bss_end);
            inside(segments.bss.range.start, segments.bss.range.end, start, end);
            kernel_image_phys_start_observed_at_entry(self, phys_start);
            phys_start == OpenSBI.kernel_load_pa;
            gp_relative_addressing_ready(KernelImage);
        }

        transitions {
            /*
             * 为内核映像的BSS段清零，让落到该段的全局变量初值为零。清零完成后，BSS 作为普通可写内存使用。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Lds.state == State::Online;
                }

                may_change {
                    memory(segments.bss.range);
                }

                ensures {
                    phys_start == OpenSBI.kernel_load_pa;
                    kernel_image_bss_zeroing_completed(self);
                    kernel_image_bss_ordinary_writable(self);
                    fits_in_kernel_image_range(self, virt_range);
                    gp_relative_addressing_ready(KernelImage);
                }
            }
        }
    }

    /*
     * Ready 表示 BSS 清零已经完成、BSS 可作为普通可写内存使用，
     * 且相对 gp 寻址机制在当前执行环境中保持可用。
     */
    state State::Ready {
        invariant {
            valid_segment_set(segments);
            segments.bss.range == range(Lds.bss_start, Lds.bss_end);
            inside(segments.bss.range.start, segments.bss.range.end, start, end);
            phys_start == OpenSBI.kernel_load_pa;
            kernel_image_bss_zeroing_completed(self);
            kernel_image_bss_ordinary_writable(self);
            fits_in_kernel_image_range(self, virt_range);
            gp_relative_addressing_ready(KernelImage);
        }

        transitions {
            /*
             * Enable 确认相对 gp 寻址机制在当前执行环境中仍然可用。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    gp_relative_addressing_ready(KernelImage);
                }

                ensures {
                    phys_start == OpenSBI.kernel_load_pa;
                    gp_relative_addressing_ready(KernelImage);
                }
            }
        }
    }

    /*
     * Online 表示内核映像可通过当前执行环境访问，且相对 gp 寻址机制可用。
     */
    state State::Online {
        invariant {
            valid_segment_set(segments);
            segments.bss.range == range(Lds.bss_start, Lds.bss_end);
            inside(segments.bss.range.start, segments.bss.range.end, start, end);
            phys_start == OpenSBI.kernel_load_pa;
            kernel_image_bss_zeroing_completed(self);
            kernel_image_bss_ordinary_writable(self);
            gp_relative_addressing_ready(KernelImage);
        }
    }
}
