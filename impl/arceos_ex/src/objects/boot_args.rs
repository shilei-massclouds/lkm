use super::state::State;

pub struct BootArgs {
    state: State,
    boot_hartid: usize,
    dtb_pa: usize,
}

impl BootArgs {
    pub const fn new(boot_hartid: usize, dtb_pa: usize) -> Self {
        Self {
            state: State::Ready,
            boot_hartid,
            dtb_pa,
        }
    }

    pub const fn state(&self) -> State {
        self.state
    }

    pub const fn boot_hartid(&self) -> usize {
        self.boot_hartid
    }

    pub const fn dtb_pa(&self) -> usize {
        self.dtb_pa
    }
}
