/* RISC-V platform runtime system and its mutable boot-hart register context. */

object BootHartContext: PrepareObject {
    initial_state: State::Base;
    parent: Riscv64Platform;

    attrs {
        a0: Gpr<HartId>;
        a1: Gpr<PhysAddr<Dtb>>;
        sp: Gpr<Addr>;
        tp: Gpr<Addr>;
        gp: Gpr<Addr>;
        sstatus: Csr<Sstatus>;
        sie: Csr<Sie>;
        sip: Csr<Sip>;
        stvec: Csr<TrapVector>;
        sscratch: Csr<usize>;
        satp: Csr<Satp>;
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    boot_hart_context_constructed();
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
            }
        }
    }

    state State::Online {
        invariant {
            attrs_accessible(self);
            boot_hart_context_constructed();
        }
    }
}

object Riscv64Platform: PlatformObject {
    initial_state: State::Base;
    parent: Computer;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    HardwareProject.state == State::Ready;
                    Riscv64.state == State::Online;
                    riscv64_platform_system_spec_established();
                    riscv64_platform_constructed();
                    boot_hart_context_constructed();
                }

                drives {
                    BootHartContext.Transition::Preset;
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            BootHartContext.state == State::Prepared;
        }

        transitions {
            on Transition::Setup -> State::Ready {
                drives {
                    BootHartContext.Transition::Setup;
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootHartContext.state == State::Ready;
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    BootHartContext.Transition::Enable;
                }

                emits {
                    OpenSBI.Transition::Preset;
                }
            }
        }
    }

    state State::Online {
        invariant {
            BootHartContext.state == State::Online;
        }
    }
}
