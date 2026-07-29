system Controller {
    initial_state: State::Base;
    state State::Base {
        actions {
            on Action::Begin {
                drives { Root.Action::Grow; }
            }
        }
    }
}

system Root {
    initial_state: State::Base;
    state State::Base {
        actions {
            on Action::Grow {
                drives {
                    Up01.Action::Show;
                    Up02.Action::Show;
                    Up03.Action::Show;
                    Up04.Action::Show;
                    Up05.Action::Show;
                    Up06.Action::Show;
                    Up07.Action::Show;
                    Up08.Action::Show;
                    Up09.Action::Show;
                    Up10.Action::Show;
                    Up11.Action::Show;
                    Up12.Action::Show;
                }
            }
        }
    }
}

system Up01 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up02 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up03 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up04 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up05 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up06 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up07 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up08 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up09 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up10 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up11 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
system Up12 { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Show { } } } }
