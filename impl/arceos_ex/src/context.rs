use crate::objects::{entry_prelude::EntryPreludeObjects, entry_successor::EntrySuccessorObjects};

pub struct Context {
    pub entry_prelude: EntryPreludeObjects,
    pub entry_successor: EntrySuccessorObjects,
}

impl Context {
    pub const fn new() -> Self {
        Self {
            entry_prelude: EntryPreludeObjects::new(),
            entry_successor: EntrySuccessorObjects::new(),
        }
    }
}

static mut CONTEXT: Context = Context::new();

pub fn context() -> &'static mut Context {
    // SAFETY: the current boot path is single-hart and system-exclusive. The
    // context owns long-lived resource objects that must survive address-space
    // switches during startup.
    unsafe { &mut *core::ptr::addr_of_mut!(CONTEXT) }
}

pub fn context_ref() -> &'static Context {
    // SAFETY: read-only access is used for boundary checks before the mutable
    // phase path starts mutating the context.
    unsafe { &*core::ptr::addr_of!(CONTEXT) }
}
