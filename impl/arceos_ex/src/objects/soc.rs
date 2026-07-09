use super::state::EventResult;
use crate::checkpoint::Checkpoint;

pub struct Soc;

impl Soc {
    pub fn preset() -> EventResult {
        crate::checkpoint::checkpoint(Checkpoint::SocPrepared);
        Ok(())
    }
}
