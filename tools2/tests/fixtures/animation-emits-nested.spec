system Root {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Start -> State::Ready {
                emits {
                    Worker.Transition::Run;
                }
            }
        }
    }

    state State::Ready {}
}

system Worker {
    parent: Root;
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Run -> State::Ready {
                drives {
                    Child.Action::Configure;
                }
            }
        }
    }

    state State::Ready {}
}

system Child {
    parent: Worker;
    initial_state: State::Base;

    state State::Base {
        actions {
            on Action::Configure {
                ensures {
                    configured(self);
                }
            }
        }
    }
}
