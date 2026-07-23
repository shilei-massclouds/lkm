enum Mode {
    Fast,
    Safe,
}

system Root {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Start -> State::Ready {
                drives {
                    Child.Action::Configure(mode: Mode::Fast);
                }
                emits {
                    Async.Transition::Run;
                    Sink.Action::Observe;
                }
                ensures {
                    started(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            started(self);
        }
        actions {
            on Action::Inspect {
                depends_on {
                    async_done(Async);
                }
                ensures {
                    inspected(self);
                }
            }
        }
    }
}

system Child {
    parent: Root;
    initial_state: State::Base;

    state State::Base {
        actions {
            on Action::Configure(mode: Mode) {
                ensures {
                    configured(self, mode);
                }
            }
        }
    }
}

system Async {
    parent: Root;
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Run -> State::Ready {
                depends_on {
                    Root.state == State::Ready;
                }
                ensures {
                    async_done(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            async_done(self);
        }
    }
}

system Sink {
    parent: Root;
    initial_state: State::Base;
    state State::Base {
        actions { on Action::Observe { ensures { observed(self); } } }
    }
}
