#[macro_export]
macro_rules! irqchip_declare {
    ($name:ident, $compatible:literal, $entry:ident) => {
        const _: () = {
            #[used]
            #[unsafe(link_section = ".irqchip.init")]
            static IRQCHIP_ENTRY: $crate::objects::irq_time::IrqChipInitEntry =
                $crate::objects::irq_time::IrqChipInitEntry::new(
                    stringify!($name),
                    $compatible.as_bytes(),
                    $entry,
                );
        };
    };
}
