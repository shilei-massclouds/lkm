predicate source_completed<S>(source: S) -> bool;

system Source {
    initial_state: State::Base;

    state State::Base {
        actions {
            on Action::Run {
                state_effect: StateEffect::None;
                yields {
                    Scheduler.Action::Schedule;
                }
                ensures {
                    source_completed(self);
                }
            }
        }
    }
}

system Scheduler {
    initial_state: State::Online;

    state State::Online {
        actions {
            on Action::Schedule {
                state_effect: StateEffect::None;
            }
        }
    }
}
