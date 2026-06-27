#[cfg(plic_provider_linux_object)]
use super::state::{failed_condition, LifecycleEvent};
use super::{
    device::DeviceRef,
    device_tree::DeviceTree,
    irq_time::{IrqHandlerKind, IrqHandlerRegistry, LogicalIrq, Plic, PlicIrqDomain},
    state::{EventResult, State},
};

pub fn setup_registered_provider(device_tree: &DeviceTree, plic: &Plic) -> EventResult {
    provider_setup_registered_provider(device_tree, plic)
}

pub fn exercise_uart_leaf_chip_callbacks(source: u32, logical_irq: LogicalIrq) -> EventResult {
    provider_exercise_uart_leaf_chip_callbacks(source, logical_irq)
}

pub fn exercise_unmapped_irq_boundary() -> EventResult {
    provider_exercise_unmapped_irq_boundary()
}

pub fn translate_one_cell_specifier(domain: &PlicIrqDomain, specifier: &[u32]) -> Option<u32> {
    provider_translate_one_cell_specifier(domain, specifier)
}

pub fn map_source(domain: &mut PlicIrqDomain, source: u32) -> Option<LogicalIrq> {
    provider_map_source(domain, source)
}

pub fn resolve_hwirq(domain: &PlicIrqDomain, source: u32) -> Option<LogicalIrq> {
    provider_resolve_hwirq(domain, source)
}

pub fn record_irq_action_request(
    logical_irq: LogicalIrq,
    device: DeviceRef,
    handler_kind: IrqHandlerKind,
) -> bool {
    if !logical_irq.is_valid() || handler_kind == IrqHandlerKind::None {
        return false;
    }

    provider_record_irq_action_request(logical_irq, device, handler_kind)
}

pub fn enable_mapped_source(plic: &Plic, source: u32, logical_irq: LogicalIrq) -> bool {
    if plic.state() != State::Ready
        || !plic.source_enable_ready()
        || source == 0
        || source > plic.source_count()
        || !logical_irq.is_valid()
    {
        return false;
    }

    provider_enable_mapped_source(plic, source, logical_irq)
}

pub fn handle_external_interrupt(
    plic: &Plic,
    domain: &PlicIrqDomain,
    registry: &IrqHandlerRegistry,
) -> bool {
    if !runtime_ready(plic, domain, registry) {
        return false;
    }

    provider_handle_external_interrupt(plic, domain, registry)
}

pub fn claim_count(plic: &Plic) -> usize {
    provider_claim_count(plic)
}

pub fn zero_claim_count(plic: &Plic) -> usize {
    provider_zero_claim_count(plic)
}

pub fn dispatch_count(plic: &Plic) -> usize {
    provider_dispatch_count(plic)
}

pub fn complete_count(plic: &Plic) -> usize {
    provider_complete_count(plic)
}

pub fn loop_exit_count(plic: &Plic) -> usize {
    provider_loop_exit_count(plic)
}

pub fn last_claimed_source(plic: &Plic) -> u32 {
    provider_last_claimed_source(plic)
}

pub fn last_completed_source(plic: &Plic) -> u32 {
    provider_last_completed_source(plic)
}

pub fn claim_count_for_source(plic: &Plic, source: u32) -> usize {
    if source == 0 || source > plic.source_count() {
        return 0;
    }

    provider_claim_count_for_source(plic, source)
}

pub fn dispatch_count_for_source(plic: &Plic, source: u32) -> usize {
    if source == 0 || source > plic.source_count() {
        return 0;
    }

    provider_dispatch_count_for_source(plic, source)
}

pub fn complete_count_for_source(plic: &Plic, source: u32) -> usize {
    if source == 0 || source > plic.source_count() {
        return 0;
    }

    provider_complete_count_for_source(plic, source)
}

fn runtime_ready(plic: &Plic, domain: &PlicIrqDomain, registry: &IrqHandlerRegistry) -> bool {
    plic.state() == State::Ready
        && plic.chained_handler_ready()
        && plic.claim_action_ready()
        && plic.complete_action_ready()
        && plic.claim_loop_until_zero()
        && plic.zero_claim_stops_dispatch()
        && plic.completes_each_claimed_source()
        && plic.claim_before_dispatch()
        && plic.complete_after_handler()
        && domain.state() == State::Ready
        && domain.dispatch_ops_ready()
        && registry.state() == State::Ready
        && registry.dispatch_ready()
}

fn domain_ready(domain: &PlicIrqDomain) -> bool {
    domain.state() == State::Ready
        && domain.hwirq_valid_range_ready()
        && domain.logical_irq_allocator_ready()
        && domain.mapping_table_ready()
        && domain.translate_specifier_ready()
        && domain.dispatch_ops_ready()
        && domain.source_zero_reserved()
        && domain.one_cell_specifier()
}

#[cfg(not(plic_provider_linux_object))]
fn provider_setup_registered_provider(_device_tree: &DeviceTree, _plic: &Plic) -> EventResult {
    Ok(())
}

#[cfg(plic_provider_linux_object)]
fn provider_setup_registered_provider(device_tree: &DeviceTree, plic: &Plic) -> EventResult {
    super::linux_plic_shim::run_linux_initcall6()?;
    if !super::linux_plic_shim::platform_driver_registered()
        || super::linux_plic_shim::platform_driver_probe_ptr() == 0
    {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    super::linux_plic_shim::platform_driver_match_and_probe(device_tree, plic)
}

#[cfg(not(plic_provider_linux_object))]
fn provider_exercise_uart_leaf_chip_callbacks(
    _source: u32,
    _logical_irq: LogicalIrq,
) -> EventResult {
    Ok(())
}

#[cfg(plic_provider_linux_object)]
fn provider_exercise_uart_leaf_chip_callbacks(source: u32, logical_irq: LogicalIrq) -> EventResult {
    if !super::linux_plic_shim::exercise_uart_leaf_chip_callbacks(source, logical_irq) {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    Ok(())
}

#[cfg(not(plic_provider_linux_object))]
fn provider_exercise_unmapped_irq_boundary() -> EventResult {
    Ok(())
}

#[cfg(plic_provider_linux_object)]
fn provider_exercise_unmapped_irq_boundary() -> EventResult {
    if !super::linux_plic_shim::exercise_unmapped_irq_boundary() {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    Ok(())
}

fn provider_translate_one_cell_specifier(domain: &PlicIrqDomain, specifier: &[u32]) -> Option<u32> {
    if !domain_ready(domain) {
        return None;
    }

    domain.native_translate_one_cell_specifier(specifier)
}

fn provider_map_source(domain: &mut PlicIrqDomain, source: u32) -> Option<LogicalIrq> {
    if !domain_ready(domain) {
        return None;
    }

    domain.native_map_source(source)
}

fn provider_resolve_hwirq(domain: &PlicIrqDomain, source: u32) -> Option<LogicalIrq> {
    if !domain_ready(domain) {
        return None;
    }

    domain.native_resolve_hwirq(source)
}

#[cfg(not(plic_provider_linux_object))]
fn provider_record_irq_action_request(
    _logical_irq: LogicalIrq,
    _device: DeviceRef,
    _handler_kind: IrqHandlerKind,
) -> bool {
    true
}

#[cfg(plic_provider_linux_object)]
fn provider_record_irq_action_request(
    logical_irq: LogicalIrq,
    device: DeviceRef,
    handler_kind: IrqHandlerKind,
) -> bool {
    super::linux_plic_shim::record_irq_action_request(logical_irq, device, handler_kind)
}

#[cfg(not(plic_provider_linux_object))]
fn provider_enable_mapped_source(plic: &Plic, source: u32, _logical_irq: LogicalIrq) -> bool {
    plic.native_enable_source(source)
}

#[cfg(plic_provider_linux_object)]
fn provider_enable_mapped_source(_plic: &Plic, source: u32, logical_irq: LogicalIrq) -> bool {
    super::linux_plic_shim::enable_mapped_source(source, logical_irq)
}

#[cfg(not(plic_provider_linux_object))]
fn provider_handle_external_interrupt(
    plic: &Plic,
    domain: &PlicIrqDomain,
    registry: &IrqHandlerRegistry,
) -> bool {
    plic.native_handle_external_interrupt(domain, registry)
}

#[cfg(plic_provider_linux_object)]
fn provider_handle_external_interrupt(
    _plic: &Plic,
    _domain: &PlicIrqDomain,
    _registry: &IrqHandlerRegistry,
) -> bool {
    super::linux_plic_shim::handle_external_interrupt()
}

#[cfg(not(plic_provider_linux_object))]
fn provider_claim_count(plic: &Plic) -> usize {
    plic.native_claim_count()
}

#[cfg(plic_provider_linux_object)]
fn provider_claim_count(_plic: &Plic) -> usize {
    super::linux_plic_shim::runtime_claim_count()
}

#[cfg(not(plic_provider_linux_object))]
fn provider_zero_claim_count(plic: &Plic) -> usize {
    plic.native_zero_claim_count()
}

#[cfg(plic_provider_linux_object)]
fn provider_zero_claim_count(_plic: &Plic) -> usize {
    super::linux_plic_shim::runtime_zero_claim_count()
}

#[cfg(not(plic_provider_linux_object))]
fn provider_dispatch_count(plic: &Plic) -> usize {
    plic.native_dispatch_count()
}

#[cfg(plic_provider_linux_object)]
fn provider_dispatch_count(_plic: &Plic) -> usize {
    super::linux_plic_shim::runtime_dispatch_count()
}

#[cfg(not(plic_provider_linux_object))]
fn provider_complete_count(plic: &Plic) -> usize {
    plic.native_complete_count()
}

#[cfg(plic_provider_linux_object)]
fn provider_complete_count(_plic: &Plic) -> usize {
    super::linux_plic_shim::runtime_complete_count()
}

#[cfg(not(plic_provider_linux_object))]
fn provider_loop_exit_count(plic: &Plic) -> usize {
    plic.native_loop_exit_count()
}

#[cfg(plic_provider_linux_object)]
fn provider_loop_exit_count(_plic: &Plic) -> usize {
    super::linux_plic_shim::runtime_loop_exit_count()
}

#[cfg(not(plic_provider_linux_object))]
fn provider_last_claimed_source(plic: &Plic) -> u32 {
    plic.native_last_claimed_source()
}

#[cfg(plic_provider_linux_object)]
fn provider_last_claimed_source(_plic: &Plic) -> u32 {
    super::linux_plic_shim::runtime_last_claimed_source()
}

#[cfg(not(plic_provider_linux_object))]
fn provider_last_completed_source(plic: &Plic) -> u32 {
    plic.native_last_completed_source()
}

#[cfg(plic_provider_linux_object)]
fn provider_last_completed_source(_plic: &Plic) -> u32 {
    super::linux_plic_shim::runtime_last_completed_source()
}

#[cfg(not(plic_provider_linux_object))]
fn provider_claim_count_for_source(plic: &Plic, source: u32) -> usize {
    plic.native_claim_count_for_source(source)
}

#[cfg(plic_provider_linux_object)]
fn provider_claim_count_for_source(_plic: &Plic, source: u32) -> usize {
    super::linux_plic_shim::runtime_claim_count_for_source(source)
}

#[cfg(not(plic_provider_linux_object))]
fn provider_dispatch_count_for_source(plic: &Plic, source: u32) -> usize {
    plic.native_dispatch_count_for_source(source)
}

#[cfg(plic_provider_linux_object)]
fn provider_dispatch_count_for_source(_plic: &Plic, source: u32) -> usize {
    super::linux_plic_shim::runtime_dispatch_count_for_source(source)
}

#[cfg(not(plic_provider_linux_object))]
fn provider_complete_count_for_source(plic: &Plic, source: u32) -> usize {
    plic.native_complete_count_for_source(source)
}

#[cfg(plic_provider_linux_object)]
fn provider_complete_count_for_source(_plic: &Plic, source: u32) -> usize {
    super::linux_plic_shim::runtime_complete_count_for_source(source)
}
