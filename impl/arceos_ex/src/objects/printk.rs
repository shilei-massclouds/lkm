use super::state::{EventResult, State};

const BUFFER_SIZE: usize = 4096;

static mut PRINTK_BUFFER: PrintkBuffer = PrintkBuffer::new();

pub struct PrintkBuffer {
    state: State,
    buffer: [u8; BUFFER_SIZE],
    read: usize,
    write: usize,
}

impl PrintkBuffer {
    pub const fn new() -> Self {
        Self {
            state: State::Base,
            buffer: [0; BUFFER_SIZE],
            read: 0,
            write: 0,
        }
    }

    pub fn preset(&mut self) -> EventResult {
        if self.state != State::Base {
            return EventResult::Failed;
        }

        self.state = State::Prepared;
        EventResult::Success
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        if self.state != State::Prepared {
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
}

pub fn preset() -> EventResult {
    unsafe { (&raw mut PRINTK_BUFFER).as_mut().unwrap().preset() }
}

pub fn write_str(message: &str) {
    unsafe {
        (&raw mut PRINTK_BUFFER)
            .as_mut()
            .unwrap()
            .write_bytes(message.as_bytes());
    }
}

pub fn drain_to(sink: impl FnMut(u8)) {
    unsafe {
        (&raw mut PRINTK_BUFFER).as_mut().unwrap().drain_to(sink);
    }
}
