use super::state::EventResult;
use crate::trace::Checkpoint;

pub struct Soc;

impl Soc {
    pub fn preset() -> EventResult {
        crate::trace::checkpoint(Checkpoint::SocPrepared);
        Ok(())
    }
}
