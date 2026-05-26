#![no_std]
#![no_main]

mod arch;
mod objects;
mod phases;
mod trace;

use core::arch::global_asm;
use core::panic::PanicInfo;

use objects::{
    boot_args::BootArgs,
    state::{EventErrorCode, EventResult, Lifecycle, LifecycleEvent, State},
};
use trace::Checkpoint;

#[unsafe(link_section = ".data.phase")]
static mut STARTUP_TIMELINE: Lifecycle = Lifecycle::new(State::Base);

global_asm!(
    r#"
    .section .head.text.entry, "ax"
    .globl _start
_start:
    mv s0, a0
    mv s1, a1

    /*
     * EntryPreludePhase early prefix.
     *
     * These operations must happen before Rust can run because Rust needs a
     * valid gp, zeroed static state, a task pointer, and a stack. They are
     * kept in specification order and use the same checkpoint characters as
     * the Rust object events that follow.
     */
    li a0, '['
    call arceos_ex_head_checkpoint
    li a0, 123
    call arceos_ex_head_checkpoint
    li a0, 125
    call arceos_ex_head_checkpoint
    li a0, '('
    call arceos_ex_head_checkpoint
    li a0, 'A'
    call arceos_ex_head_checkpoint

    csrw sie, zero
    csrw sip, zero
    li a0, 'I'
    call arceos_ex_head_checkpoint

    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop
    li a0, 'K'
    call arceos_ex_head_checkpoint

    li t0, (0b11 << 9) | (0b11 << 13)
    csrrc zero, sstatus, t0
    li a0, 'O'
    call arceos_ex_head_checkpoint

    la t0, _sbss
    la t1, _ebss
1:
    bgeu t0, t1, 2f
    sd zero, 0(t0)
    addi t0, t0, 8
    j 1b
2:
    li a0, 'Z'
    call arceos_ex_head_checkpoint

    la t0, head_boot_hartid
    sd s0, 0(t0)
    li a0, 'H'
    call arceos_ex_head_checkpoint
    li a0, 'G'
    call arceos_ex_head_checkpoint

    la tp, init_task_storage
    li a0, 'T'
    call arceos_ex_head_checkpoint

    la sp, init_stack_end
    addi sp, sp, -256
    la t0, head_init_stack_sp
    sd sp, 0(t0)
    li a0, 'S'
    call arceos_ex_head_checkpoint

    mv a0, s0
    mv a1, s1
    tail rust_entry

    .section .boot.stack, "aw", @nobits
    .align 12
    .space 4096 * 4

    .section .head.handoff, "aw", @nobits
    .align 3
    .globl head_boot_hartid
head_boot_hartid:
    .space 8
    .globl head_init_stack_sp
head_init_stack_sp:
    .space 8
"#
);

#[cfg(checkpoint_sbi_char)]
global_asm!(
    r#"
    .section .head.text.checkpoint, "ax"
    .align 2
    .globl arceos_ex_head_checkpoint
arceos_ex_head_checkpoint:
    li a7, 1
    ecall
    ret
"#
);

#[cfg(not(checkpoint_sbi_char))]
global_asm!(
    r#"
    .section .head.text.checkpoint, "ax"
    .align 2
    .globl arceos_ex_head_checkpoint
arceos_ex_head_checkpoint:
    ret
"#
);

#[unsafe(no_mangle)]
extern "C" fn rust_entry(hartid: usize, dtb_pa: usize) -> ! {
    let boot_args = BootArgs::new(hartid, dtb_pa);
    startup_timeline_continue_after_head_prefix(&boot_args)
}

fn startup_timeline_continue_after_head_prefix(boot_args: &BootArgs) -> ! {
    require_startup_event(phases::prepare::adopt_head_prefix(boot_args));
    phases::boot::setup_after_head_prefix(boot_args)
}

pub fn startup_timeline_ready() -> ! {
    if !phases::prepare::is_online() || !phases::boot::is_ready() {
        arch::riscv64::sbi::putstr("arceos_ex startup invariant failed\n");
        arch::riscv64::sbi::system_shutdown()
    }

    require_startup_event(startup_timeline().transition(
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::StartupTimelineReady,
    ));
    app_main();
    arch::riscv64::sbi::system_shutdown()
}

fn require_startup_event(result: EventResult) {
    match result {
        EventResult::Success => {}
        EventResult::Failed(error) | EventResult::Blocked(error) => {
            arch::riscv64::sbi::putstr("arceos_ex startup event failed:");
            arch::riscv64::sbi::putchar(match error.code {
                EventErrorCode::DuplicateLifecycleEvent => b'D',
                EventErrorCode::InvalidTransition => b'I',
                EventErrorCode::ConditionFailed => b'C',
                EventErrorCode::UnexpectedState => b'U',
            });
            arch::riscv64::sbi::putchar(b'\n');
            arch::riscv64::sbi::system_shutdown()
        }
    }
}

fn startup_timeline() -> &'static mut Lifecycle {
    // SAFETY: early boot is single-hart and the startup timeline state is only
    // mutated by the top-level startup process in specification order.
    unsafe { &mut *core::ptr::addr_of_mut!(STARTUP_TIMELINE) }
}

pub fn app_main() {
    objects::printk::write_str("Hello, world!\n");
    objects::earlycon::drain_printk();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    arch::riscv64::sbi::putstr("arceos_ex panic\n");
    arch::riscv64::sbi::system_shutdown()
}
