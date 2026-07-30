/* KernelAddrSpace model specification. */

object KernelAddrSpace: AddressSpaceObject {
    initial_state: State::Base;
    parent: Kernel;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    KernelImage.state == State::Ready;
                    LinearMap.state == State::Base;
                    UserSpaceReserve.state == State::Base;
                }

                drives {
                    LinearMap.Transition::Preset;
                    UserSpaceReserve.Transition::Preset;
                }

                ensures {
                    kernel_addr_space_region_parentage_ready(self);
                    kernel_addr_space_layout_declared(self);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            KernelImage.state == State::Ready;
            LinearMap.state == State::Ready;
            UserSpaceReserve.state == State::Ready;
            kernel_addr_space_region_parentage_ready(self);
            kernel_addr_space_layout_declared(self);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    FixMap.state == State::Ready;
                }

                ensures {
                    kernel_addr_space_regions_disjoint(
                        KernelImage.virt_range,
                        FixMap,
                        LinearMap,
                        UserSpaceReserve
                    );
                    kernel_mappings_exclude_user_reserve(self, UserSpaceReserve);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            KernelImage.state == State::Ready
                || KernelImage.state == State::Online;
            FixMap.state == State::Ready;
            LinearMap.state == State::Ready;
            UserSpaceReserve.state == State::Ready;
            kernel_addr_space_regions_disjoint(
                KernelImage.virt_range,
                FixMap,
                LinearMap,
                UserSpaceReserve
            );
            kernel_mappings_exclude_user_reserve(self, UserSpaceReserve);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    SwapperVm.state == State::Ready;
                }

                ensures {
                    kernel_addr_space_final_swapper_mappings_published(self, SwapperVm);
                    kernel_addr_space_online_is_not_all_cpus_switched(self);
                    translation_controller_readiness_does_not_imply_cpu_activation(SwapperVm);
                }
            }
        }
    }

    state State::Online {
        invariant {
            SwapperVm.state == State::Ready;
            kernel_addr_space_final_swapper_mappings_published(self, SwapperVm);
            kernel_addr_space_online_is_not_all_cpus_switched(self);
            translation_controller_readiness_does_not_imply_cpu_activation(SwapperVm);
            kernel_mappings_exclude_user_reserve(self, UserSpaceReserve);
        }
    }
}
