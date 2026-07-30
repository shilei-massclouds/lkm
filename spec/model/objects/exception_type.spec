/* CPU-local exception classifier and fallback coverage. */

type ExceptionType: ResourceObject {
    parent: TrapType;
    initial_state: State::Base;

    owned {
        page_fault: PageFaultExceptionType;
        syscall: SyscallExceptionType;
        breakpoint: BreakpointExceptionType;
        unexpected: UnexpectedExceptionType;
    }

    state State::Base {
        transitions {
            /* 为 TrapType 的正式响应入口准备四类异常的初始兜底。 */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    self.parent.state == State::Prepared;
                }
                drives {
                    self.page_fault.Transition::Preset;
                    self.syscall.Transition::Preset;
                    self.breakpoint.Transition::Preset;
                    self.unexpected.Transition::Preset;
                }
                ensures {
                    exception_fallback_covers_all_causes(self);
                    exception_default_bindings_fatal(self);
                    exception_initial_fallbacks_prepared(self);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            exception_fallback_covers_all_causes(self);
            exception_initial_fallbacks_prepared(self);
            self.page_fault.state == State::Prepared;
            self.syscall.state == State::Prepared;
            self.breakpoint.state == State::Prepared;
            self.unexpected.state == State::Prepared;
        }
        transitions {
            on Transition::Setup -> State::Ready {
                drives {
                    self.page_fault.Transition::Setup;
                    self.breakpoint.Transition::Setup;
                    self.unexpected.Transition::Setup;
                }
                ensures {
                    exception_classifier_ready(self);
                    exception_handler_bindings_ready(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            exception_classifier_ready(self);
            exception_handler_bindings_ready(self);
        }
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    self.page_fault.state == State::Online;
                    self.syscall.state == State::Online;
                    self.breakpoint.state == State::Online;
                    self.unexpected.state == State::Online;
                }
                ensures { exception_service_online(self); }
            }
        }
    }

    state State::Online {
        invariant { exception_service_online(self); }
    }
}

predicate exception_fallback_covers_all_causes<E: ExceptionType>(exception: E) -> bool;
predicate exception_default_bindings_fatal<E: ExceptionType>(exception: E) -> bool;
predicate exception_initial_fallbacks_prepared<E: ExceptionType>(exception: E) -> bool;
predicate exception_classifier_ready<E: ExceptionType>(exception: E) -> bool;
predicate exception_handler_bindings_ready<E: ExceptionType>(exception: E) -> bool;
predicate exception_service_online<E: ExceptionType>(exception: E) -> bool;
