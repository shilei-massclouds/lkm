system Controller {
    initial_state: State::Base;
    state State::Base {
        actions {
            on Action::Begin {
                drives {
                    EarlyRoot.Action::Show;
                    LateRoot.Action::Show;
                    EarlyRoot.Action::Grow;
                }
            }
        }
    }
}

system EarlyRoot {
    initial_state: State::Base;
    state State::Base {
        actions {
            on Action::Show { }
            on Action::Grow {
                drives {
                    Level2First.Action::Show;
                    Level2Second.Action::Show;
                    Level2First.Action::Grow;
                }
            }
        }
    }
}

system LateRoot {
    initial_state: State::Base;
    state State::Base { actions { on Action::Show { } } }
}

system Level2First {
    parent: EarlyRoot;
    initial_state: State::Base;
    state State::Base {
        actions {
            on Action::Show { }
            on Action::Grow {
                drives {
                    Level3First.Action::Show;
                    Level3Second.Action::Show;
                    Level3First.Action::Grow;
                }
            }
        }
    }
}

system Level2Second {
    parent: EarlyRoot;
    initial_state: State::Base;
    state State::Base { actions { on Action::Show { } } }
}

system Level3First {
    parent: Level2First;
    initial_state: State::Base;
    state State::Base {
        actions {
            on Action::Show { }
            on Action::Grow {
                drives {
                    Level4First.Action::Show;
                    Level4Second.Action::Show;
                }
            }
        }
    }
}

system Level3Second {
    parent: Level2First;
    initial_state: State::Base;
    state State::Base { actions { on Action::Show { } } }
}

system Level4First {
    parent: Level3First;
    initial_state: State::Base;
    state State::Base { actions { on Action::Show { } } }
}

system Level4Second {
    parent: Level3First;
    initial_state: State::Base;
    state State::Base { actions { on Action::Show { } } }
}
