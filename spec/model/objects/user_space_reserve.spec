/* UserSpaceReserve model specification. */

object UserSpaceReserve: AddressSpaceObject {
    initial_state: State::Base;
    parent: KernelAddrSpace;

    attrs {
        range: VirtAddrRange<User>;
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                }

                ensures {
                    canonical_user_address_range_reserved(self);
                    kernel_mappings_exclude_user_reserve(KernelAddrSpace, self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            canonical_user_address_range_reserved(self);
            kernel_mappings_exclude_user_reserve(KernelAddrSpace, self);
        }
    }
}
