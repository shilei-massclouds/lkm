/* CPU-local interrupt gate, pending state and handler bindings. */

type InterruptType: ResourceObject {
    parent: TrapType;
    initial_state: State::Base;
    ext_state: LocalInterruptExtState;

    processes {
        Action::DisableLocal {
            state_effect: StateEffect::Conditional;
            transitions {
                LocalInterruptExtState::Enabled -> LocalInterruptExtState::Disabled;
                LocalInterruptExtState::Disabled -> LocalInterruptExtState::Disabled;
            }
            ensures { cpu_local_interrupts_disabled(self); }
            result {
                Enabled: Success(disabled);
                Disabled: Success(no_change);
            }
        }

        Action::EnableLocal {
            state_effect: StateEffect::Conditional;
            transitions {
                LocalInterruptExtState::Disabled -> LocalInterruptExtState::Enabled;
                LocalInterruptExtState::Enabled -> LocalInterruptExtState::Enabled;
            }
            ensures { cpu_local_interrupts_enabled(self); }
            result {
                Disabled: Success(enabled);
                Enabled: Success(no_change);
            }
        }

        Action::SaveAndDisable {
            state_effect: StateEffect::Conditional;
            transitions {
                LocalInterruptExtState::Enabled -> LocalInterruptExtState::Disabled;
                LocalInterruptExtState::Disabled -> LocalInterruptExtState::Disabled;
            }
            ensures {
                cpu_local_interrupts_saved_and_disabled(self);
                cpu_local_interrupts_disabled(self);
            }
            result {
                Enabled: Success(saved_enabled_then_disabled);
                Disabled: Success(saved_disabled);
            }
        }

        Action::Restore {
            state_effect: StateEffect::Conditional;
            transitions {
                SavedInterruptState::Enabled -> LocalInterruptExtState::Enabled;
                SavedInterruptState::Disabled -> LocalInterruptExtState::Disabled;
            }
            ensures { cpu_local_interrupts_restored(self); }
            result {
                SavedEnabled: Success(restored_enabled);
                SavedDisabled: Success(restored_disabled);
            }
        }
    }

    state State::Base {
        transitions {
            /*
             * 对 BootCPU，先关闭它的全部中断分路门控，再完成一次待决
             * 中断清除写。本边界只建立该写完成及其顺序事实，不保证硬件
             * 驱动的待决位随后保持为零；也不改变或判定中断总门控，或
             * 建立 handler、fallback、正式分派框架。
             */
            on Transition::Preset -> State::Prepared {
                ensures {
                    interrupt_class_gates_closed(self);
                    interrupt_pending_clear_write_completed(self);
                    interrupt_class_gates_closed_before_pending_clear_write_completed(self);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            interrupt_class_gates_closed(self);
        }
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    interrupt_handler_bindings_ready(self);
                    interrupt_dispatch_ready(self);
                    cpu_local_interrupts_disabled(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            interrupt_handler_bindings_ready(self);
            interrupt_dispatch_ready(self);
            cpu_local_interrupts_disabled(self);
        }
        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    interrupt_total_gate_open(self);
                    cpu_local_interrupts_enabled(self);
                }
            }
        }
    }

    state State::Online {
        invariant {
            interrupt_total_gate_open(self);
            interrupt_dispatch_ready(self);
        }
    }
}

predicate interrupt_total_gate_closed<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_total_gate_open<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_class_gates_closed<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_pending_clear_write_completed<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_class_gates_closed_before_pending_clear_write_completed<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_handler_bindings_ready<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_dispatch_ready<I: InterruptType>(interrupt: I) -> bool;
