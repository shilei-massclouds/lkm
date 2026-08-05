//! IRQ-safe acquire/release spin lock with guard-owned data access.

use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct IrqSpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for IrqSpinLock<T> {}
unsafe impl<T: Send> Sync for IrqSpinLock<T> {}

impl<T> IrqSpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> IrqSpinLockGuard<'_, T> {
        let saved_sstatus = crate::arch::riscv64::csr::save_and_disable_supervisor_interrupts();
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
        IrqSpinLockGuard {
            lock: self,
            saved_sstatus,
        }
    }

    #[allow(dead_code)]
    pub fn try_lock(&self) -> Option<IrqSpinLockGuard<'_, T>> {
        let saved_sstatus = crate::arch::riscv64::csr::save_and_disable_supervisor_interrupts();
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
            return None;
        }
        Some(IrqSpinLockGuard {
            lock: self,
            saved_sstatus,
        })
    }

    #[allow(dead_code)]
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }
}

pub struct IrqSpinLockGuard<'a, T> {
    lock: &'a IrqSpinLock<T>,
    saved_sstatus: usize,
}

impl<T> Deref for IrqSpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the acquire operation grants this guard shared access and
        // no mutable access exists without the same guard.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for IrqSpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: successful acquisition grants this guard the unique mutable
        // capability for the protected value.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for IrqSpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
        crate::arch::riscv64::csr::restore_supervisor_interrupts(self.saved_sstatus);
    }
}
