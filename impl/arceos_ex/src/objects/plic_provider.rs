use super::{
    irq_time::{IrqHandlerRegistry, LogicalIrq, Plic, PlicIrqDomain},
    state::State,
};

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
