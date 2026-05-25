#[derive(Clone, Copy, Eq, PartialEq)]
pub enum State {
    Base,
    Prepared,
    Ready,
    Online,
    #[allow(dead_code)]
    Destroyed,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum EventResult {
    Success,
    Failed,
    Blocked,
}

impl EventResult {
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }
}
