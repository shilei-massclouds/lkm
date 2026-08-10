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

        Action::HandleRescheduleIpi {
            state_effect: StateEffect::None;
            depends_on { self.state == State::Online; }
            ensures {
                interrupt_ssip_pending_cleared(self);
                interrupt_need_resched_recorded_cpu_local(self);
                interrupt_reschedule_ipi_does_not_switch_in_handler(self);
                interrupt_duplicate_reschedule_ipi_coalesced(self);
                interrupt_reschedule_ipi_uses_formal_overlay(self);
                interrupt_reschedule_ipi_does_not_consume_inbox(self);
            }
        }

        Action::EnableRescheduleIpiSource {
            state_effect: StateEffect::None;
            depends_on { self.state == State::Online; }
            ensures {
                interrupt_reschedule_ipi_source_gate_open(self);
            }
        }

        Action::HandleTimerInterrupt(clockevent: SchedulerClockevent) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                clockevent.state == State::Ready;
            }
            drives { clockevent.Action::HandleTimerInterrupt; }
            ensures {
                interrupt_timer_event_acknowledged_cpu_local(self);
                interrupt_timer_clockevent_rearmed(self, clockevent);
                interrupt_timer_need_resched_coalesced_cpu_local(self);
                interrupt_timer_does_not_schedule_in_hardirq(self);
                interrupt_timer_preserves_kernel_pending_until_user_return(self);
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
predicate interrupt_reschedule_ipi_source_gate_open<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_ssip_pending_cleared<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_need_resched_recorded_cpu_local<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_reschedule_ipi_does_not_switch_in_handler<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_duplicate_reschedule_ipi_coalesced<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_reschedule_ipi_uses_formal_overlay<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_reschedule_ipi_does_not_consume_inbox<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_timer_event_acknowledged_cpu_local<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_timer_clockevent_rearmed<I: InterruptType, C: SchedulerClockevent>(interrupt: I, clockevent: C) -> bool;
predicate interrupt_timer_need_resched_coalesced_cpu_local<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_timer_does_not_schedule_in_hardirq<I: InterruptType>(interrupt: I) -> bool;
predicate interrupt_timer_preserves_kernel_pending_until_user_return<I: InterruptType>(interrupt: I) -> bool;
