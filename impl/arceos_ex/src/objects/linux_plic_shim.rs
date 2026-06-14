use core::ffi::c_void;

fn trap(symbol: &str) -> ! {
    crate::arch::riscv64::sbi::putstr("linux plic shim trap: ");
    crate::arch::riscv64::sbi::putstr(symbol);
    crate::arch::riscv64::sbi::putstr("\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

#[unsafe(no_mangle)]
pub static mut __cpu_online_mask: [usize; 1] = [1];

#[unsafe(no_mangle)]
pub static mut __cpu_present_mask: [usize; 1] = [1];

#[unsafe(no_mangle)]
pub static mut __mmiowb_state: usize = 0;

#[unsafe(no_mangle)]
pub static mut __per_cpu_offset: [usize; 8] = [0; 8];

#[unsafe(no_mangle)]
pub static cpu_bit_bitmap: [usize; 65] = [
    0,
    1usize << 0,
    1usize << 1,
    1usize << 2,
    1usize << 3,
    1usize << 4,
    1usize << 5,
    1usize << 6,
    1usize << 7,
    1usize << 8,
    1usize << 9,
    1usize << 10,
    1usize << 11,
    1usize << 12,
    1usize << 13,
    1usize << 14,
    1usize << 15,
    1usize << 16,
    1usize << 17,
    1usize << 18,
    1usize << 19,
    1usize << 20,
    1usize << 21,
    1usize << 22,
    1usize << 23,
    1usize << 24,
    1usize << 25,
    1usize << 26,
    1usize << 27,
    1usize << 28,
    1usize << 29,
    1usize << 30,
    1usize << 31,
    1usize << 32,
    1usize << 33,
    1usize << 34,
    1usize << 35,
    1usize << 36,
    1usize << 37,
    1usize << 38,
    1usize << 39,
    1usize << 40,
    1usize << 41,
    1usize << 42,
    1usize << 43,
    1usize << 44,
    1usize << 45,
    1usize << 46,
    1usize << 47,
    1usize << 48,
    1usize << 49,
    1usize << 50,
    1usize << 51,
    1usize << 52,
    1usize << 53,
    1usize << 54,
    1usize << 55,
    1usize << 56,
    1usize << 57,
    1usize << 58,
    1usize << 59,
    1usize << 60,
    1usize << 61,
    1usize << 62,
    1usize << 63,
];

#[unsafe(no_mangle)]
pub static mut kmalloc_caches: [usize; 16] = [0; 16];

#[unsafe(no_mangle)]
pub static nr_cpu_ids: u32 = 1;

#[unsafe(no_mangle)]
pub static of_fwnode_ops: [usize; 8] = [0; 8];

#[unsafe(no_mangle)]
pub extern "C" fn ___ratelimit() -> i32 {
    trap("___ratelimit")
}

#[unsafe(no_mangle)]
pub extern "C" fn __cpuhp_setup_state() -> i32 {
    trap("__cpuhp_setup_state")
}

#[unsafe(no_mangle)]
pub extern "C" fn __irq_set_handler() {
    trap("__irq_set_handler")
}

#[unsafe(no_mangle)]
pub extern "C" fn __kmalloc_cache_noprof() -> *mut c_void {
    trap("__kmalloc_cache_noprof")
}

#[unsafe(no_mangle)]
pub extern "C" fn __kmalloc_noprof() -> *mut c_void {
    trap("__kmalloc_noprof")
}

#[unsafe(no_mangle)]
pub extern "C" fn __platform_driver_register() -> i32 {
    trap("__platform_driver_register")
}

#[unsafe(no_mangle)]
pub extern "C" fn __raw_spin_lock_init() {
    trap("__raw_spin_lock_init")
}

#[unsafe(no_mangle)]
pub extern "C" fn __stack_chk_fail() -> ! {
    trap("__stack_chk_fail")
}

#[unsafe(no_mangle)]
pub extern "C" fn _printk() -> i32 {
    trap("_printk")
}

#[unsafe(no_mangle)]
pub extern "C" fn _raw_spin_lock_irqsave() -> usize {
    trap("_raw_spin_lock_irqsave")
}

#[unsafe(no_mangle)]
pub extern "C" fn _raw_spin_unlock_irqrestore() {
    trap("_raw_spin_unlock_irqrestore")
}

#[unsafe(no_mangle)]
pub extern "C" fn bitmap_free() {
    trap("bitmap_free")
}

#[unsafe(no_mangle)]
pub extern "C" fn bitmap_zalloc() -> *mut usize {
    trap("bitmap_zalloc")
}

#[unsafe(no_mangle)]
pub extern "C" fn devm_platform_ioremap_resource() -> *mut c_void {
    trap("devm_platform_ioremap_resource")
}

#[unsafe(no_mangle)]
pub extern "C" fn disable_percpu_irq() {
    trap("disable_percpu_irq")
}

#[unsafe(no_mangle)]
pub extern "C" fn enable_percpu_irq() {
    trap("enable_percpu_irq")
}

#[unsafe(no_mangle)]
pub extern "C" fn generic_handle_domain_irq() -> i32 {
    trap("generic_handle_domain_irq")
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_edge_irq() {
    trap("handle_edge_irq")
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_fasteoi_irq() {
    trap("handle_fasteoi_irq")
}

#[unsafe(no_mangle)]
pub extern "C" fn iounmap() {
    trap("iounmap")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_create_mapping_affinity() -> u32 {
    trap("irq_create_mapping_affinity")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_free_irqs_top() {
    trap("irq_domain_free_irqs_top")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_instantiate() -> i32 {
    trap("irq_domain_instantiate")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_set_info() {
    trap("irq_domain_set_info")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_translate_onecell() -> i32 {
    trap("irq_domain_translate_onecell")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_translate_twocell() -> i32 {
    trap("irq_domain_translate_twocell")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_find_matching_fwspec() -> *mut c_void {
    trap("irq_find_matching_fwspec")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_get_irq_data() -> *mut c_void {
    trap("irq_get_irq_data")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_modify_status() {
    trap("irq_modify_status")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_set_affinity() -> i32 {
    trap("irq_set_affinity")
}

#[unsafe(no_mangle)]
pub extern "C" fn kfree() {
    trap("kfree")
}

#[unsafe(no_mangle)]
pub extern "C" fn of_iomap() -> *mut c_void {
    trap("of_iomap")
}

#[unsafe(no_mangle)]
pub extern "C" fn of_irq_count() -> i32 {
    trap("of_irq_count")
}

#[unsafe(no_mangle)]
pub extern "C" fn of_irq_parse_one() -> i32 {
    trap("of_irq_parse_one")
}

#[unsafe(no_mangle)]
pub extern "C" fn of_match_node() -> *const c_void {
    trap("of_match_node")
}

#[unsafe(no_mangle)]
pub extern "C" fn of_property_read_variable_u32_array() -> i32 {
    trap("of_property_read_variable_u32_array")
}

#[unsafe(no_mangle)]
pub extern "C" fn register_syscore_ops() -> i32 {
    trap("register_syscore_ops")
}

#[unsafe(no_mangle)]
pub extern "C" fn riscv_get_intc_hwnode() -> *mut c_void {
    trap("riscv_get_intc_hwnode")
}

#[unsafe(no_mangle)]
pub extern "C" fn riscv_hartid_to_cpuid() -> i32 {
    trap("riscv_hartid_to_cpuid")
}

#[unsafe(no_mangle)]
pub extern "C" fn riscv_of_parent_hartid() -> i32 {
    trap("riscv_of_parent_hartid")
}
