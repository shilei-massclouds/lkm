use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    if ctx.params.state() != State::Ready {
        printk::write_str("params are not ready\n");
        return SmokeResult::Failed;
    }
    if ctx.early_param.state() != State::Ready {
        printk::write_str("early params are not ready\n");
        return SmokeResult::Failed;
    }
    if ctx.boot_param.state() != State::Ready {
        printk::write_str("boot params are not ready\n");
        return SmokeResult::Failed;
    }
    if ctx.payload_param.state() != State::Ready {
        printk::write_str("payload params are not ready\n");
        return SmokeResult::Failed;
    }
    if ctx.boot_param.boot_arg_count() == 0 {
        printk::write_str("boot params did not parse command line\n");
        return SmokeResult::Failed;
    }
    if ctx.boot_param.unknown_count() != 0 {
        printk::write_str("boot params reported unexpected unknown options\n");
        return SmokeResult::Failed;
    }
    if ctx.boot_param.payload_boundary().is_some() {
        printk::write_str("payload boundary should be absent on default command line\n");
        return SmokeResult::Failed;
    }
    if ctx.payload_param.arg_count() != 0 {
        printk::write_str("payload params should be empty on default command line\n");
        return SmokeResult::Failed;
    }

    let static_cmdline = ctx.static_command_line.as_bytes();
    printk::write_str("Params:\n  Static cmdline : \"");
    write_bytes(static_cmdline);
    printk::write_str("\"\n");
    printk::write_fmt(format_args!(
        "  Boot args      : {}\n",
        ctx.boot_param.boot_arg_count()
    ));
    let mut index = 0usize;
    while index < ctx.boot_param.boot_arg_count() {
        printk::write_fmt(format_args!("    [{}] ", index));
        if let Some(arg) = ctx.boot_param.boot_arg(static_cmdline, index) {
            write_bytes(arg);
        } else {
            printk::write_str("<missing>");
        }
        printk::write_str("\n");
        index += 1;
    }
    printk::write_fmt(format_args!(
        "  Unknown        : {}\n  Payload args   : {}\n",
        ctx.boot_param.unknown_count(),
        ctx.payload_param.arg_count()
    ));
    let mut payload_index = 0usize;
    while payload_index < ctx.payload_param.arg_count() {
        printk::write_fmt(format_args!("    [{}] ", payload_index));
        if let Some(arg) = ctx
            .payload_param
            .arg(static_cmdline, &ctx.boot_param, payload_index)
        {
            write_bytes(arg);
        } else {
            printk::write_str("<missing>");
        }
        printk::write_str("\n");
        payload_index += 1;
    }
    SmokeResult::Passed
}

fn write_bytes(bytes: &[u8]) {
    for byte in bytes {
        if byte.is_ascii_graphic() || *byte == b' ' {
            printk::write_byte(*byte);
        } else {
            printk::write_byte(b'?');
        }
    }
}
