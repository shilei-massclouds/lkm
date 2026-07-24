system Root {
    initial_state: State::Base;

    state State::Base {
        actions {
            on Action::Begin {
                emits {
                    Root.Action::Missing;
                }
            }
        }
    }
}
