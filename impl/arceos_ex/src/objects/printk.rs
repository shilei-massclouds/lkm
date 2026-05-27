use super::state::{EventResult, Lifecycle, LifecycleEvent, State};
use crate::trace::Checkpoint;
use core::fmt::{self, Write};

#[allow(dead_code)]
const BUFFER_SIZE: usize = 4096;

#[allow(dead_code)]
static mut PRINTK_BUFFER: PrintkBuffer = PrintkBuffer::new();

#[allow(dead_code)]
pub struct PrintkBuffer {
    lifecycle: Lifecycle,
    buffer: [u8; BUFFER_SIZE],
    read: usize,
    write: usize,
}

#[allow(dead_code)]
impl PrintkBuffer {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            buffer: [0; BUFFER_SIZE],
            read: 0,
            write: 0,
        }
    }

    pub fn preset(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::PrintkBufferPrepared,
        )
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        if self.lifecycle.state() != State::Prepared {
            return;
        }

        for byte in bytes {
            let next = (self.write + 1) % BUFFER_SIZE;
            if next == self.read {
                self.read = (self.read + 1) % BUFFER_SIZE;
            }
            self.buffer[self.write] = *byte;
            self.write = next;
        }
    }

    pub fn drain_to(&mut self, mut sink: impl FnMut(u8)) {
        while self.read != self.write {
            let byte = self.buffer[self.read];
            self.read = (self.read + 1) % BUFFER_SIZE;
            sink(byte);
        }
    }

    pub fn is_prepared(&self) -> bool {
        self.lifecycle.state() == State::Prepared
    }
}

#[allow(dead_code)]
pub fn preset() -> EventResult {
    unsafe { (&raw mut PRINTK_BUFFER).as_mut().unwrap().preset() }
}

#[allow(dead_code)]
pub fn write_str(message: &str) {
    unsafe {
        (&raw mut PRINTK_BUFFER)
            .as_mut()
            .unwrap()
            .write_bytes(message.as_bytes());
    }
}

#[allow(dead_code)]
pub fn write_byte(byte: u8) {
    unsafe {
        (&raw mut PRINTK_BUFFER)
            .as_mut()
            .unwrap()
            .write_bytes(&[byte]);
    }
}

#[allow(dead_code)]
pub fn write_fmt(args: fmt::Arguments<'_>) {
    let _ = PrintkWriter.write_fmt(args);
}

pub fn is_prepared() -> bool {
    unsafe { (&raw const PRINTK_BUFFER).as_ref().unwrap().is_prepared() }
}

#[allow(dead_code)]
pub fn drain_to(sink: impl FnMut(u8)) {
    unsafe {
        (&raw mut PRINTK_BUFFER).as_mut().unwrap().drain_to(sink);
    }
}

#[allow(dead_code)]
struct PrintkWriter;

impl Write for PrintkWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}
