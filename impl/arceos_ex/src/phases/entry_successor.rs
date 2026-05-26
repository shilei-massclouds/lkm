use crate::objects::{
    entry_prelude::EntryPreludeObjects, entry_successor::EntrySuccessorObjects,
    state::EventResult,
};

pub fn setup(
    objects: &mut EntrySuccessorObjects,
    entry_prelude: &mut EntryPreludeObjects,
) -> EventResult {
    objects.setup(entry_prelude)
}
