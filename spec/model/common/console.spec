/*
 * Console registration and early/real console handoff model.
 *
 * Linux reference shape:
 * - earlycon registers a CON_BOOT console through register_console().
 * - 8250 console registration creates a real ttyS console.
 * - When the real console becomes CON_CONSDEV, printk unregisters boot
 *   consoles unless keep_bootcon was requested.
 *
 * Layering rule:
 * - EarlyCon is the early output backend object. It is not itself a printk
 *   registry entry.
 * - BootConsole is the printk console entry that wraps EarlyCon with CON_BOOT
 *   and printbuffer semantics.
 * - Serial8250Console is the real console entry registered by the probed 8250
 *   port.
 * - ConsoleRegistry owns register_console() policy, printk routing, handoff
 *   cursor transfer and keep_bootcon decisions. Drivers may request
 *   registration, but must not own these global policy facts.
 *
 * Next runtime layer:
 * - Uart8250Port remains the platform-probed resource instance: MMIO, irq,
 *   line, clock and register access shape.
 * - Serial8250RuntimePort is the 8250 runtime behavior layer over that port:
 *   IER/IIR/LSR handling, IRQ-time RX/TX dispatch shape and port locking.
 * - Serial8250Console is only the printk console entry; it may submit TX work
 *   to the runtime port, but it is not the TTY runtime and does not own RX.
 * - TtyPort is the minimal uart_state/tty_port container bound to the 8250
 *   line; TtyFlipBuffer and TtyXmitFifo model the RX staging buffer and
 *   future ordinary TTY TX FIFO separately from printk console TX.
 */

predicate boot_console_registered<T, E>(boot_console: T, earlycon: E) -> bool;
predicate boot_console_con_boot_flag_set<T>(boot_console: T) -> bool;
predicate boot_console_printbuffer_flag_set<T>(boot_console: T) -> bool;
predicate boot_console_write_routes_to_earlycon<T, E>(boot_console: T, earlycon: E) -> bool;
predicate boot_console_kept_by_policy<T>(boot_console: T) -> bool;
predicate boot_console_unregistered<T>(boot_console: T) -> bool;
predicate boot_console_removed_from_registry<T, R>(boot_console: T, registry: R) -> bool;
predicate boot_console_offline_trace_emitted<T>(boot_console: T) -> bool;
predicate earlycon_backend_disabled_after_handoff<T, R>(earlycon: T, registry: R) -> bool;
predicate earlycon_backend_access_panics_after_handoff<T>(earlycon: T) -> bool;
predicate earlycon_offline_trace_emitted<T>(earlycon: T) -> bool;

predicate console_candidate_non_stdout_path<T, D>(candidate: T, device_tree: D) -> bool;
predicate console_candidate_not_platform_topology_mutating<T>(candidate: T) -> bool;

predicate console_registry_ready<T>(registry: T) -> bool;
predicate console_registry_register_console_api_ready<T>(registry: T) -> bool;
predicate console_registry_has_boot_console<T, B>(registry: T, boot_console: B) -> bool;
predicate console_registry_has_real_console<T, C>(registry: T, console: C) -> bool;
predicate console_registry_preferred_console_from_stdout_path<T, D>(registry: T, device_tree: D) -> bool;
predicate console_registry_keep_bootcon_policy_ready<T>(registry: T) -> bool;
predicate console_registry_keep_bootcon_disabled<T>(registry: T) -> bool;
predicate console_registry_keep_bootcon_enabled<T>(registry: T) -> bool;
predicate console_registry_printk_route_boot_console<T, B>(registry: T, boot_console: B) -> bool;
predicate console_registry_printk_route_real_console<T, C>(registry: T, console: C) -> bool;
predicate console_registry_non_stdout_registration_rejected<T, C>(registry: T, candidate: C) -> bool;
predicate console_registry_non_stdout_preserves_boot_route<T, B, C>(
    registry: T,
    boot_console: B,
    candidate: C
) -> bool;
predicate console_registry_non_stdout_leaves_real_console_unchanged<T, C>(registry: T, candidate: C) -> bool;
predicate console_registry_non_stdout_no_handoff_committed<T, C>(registry: T, candidate: C) -> bool;
predicate console_registry_duplicate_preferred_registration_idempotent<T, C>(registry: T, console: C) -> bool;
predicate console_registry_flushes_boot_pending_before_serial_handoff<T>(registry: T) -> bool;
predicate console_registry_blocks_legacy_earlycon_drain_after_handoff<T>(registry: T) -> bool;

predicate uart8250_port_resources_ready<T, D, R>(port: T, device: D, device_tree: R) -> bool;
predicate uart8250_port_mmio_resource_bound<T>(port: T) -> bool;
predicate uart8250_port_mapbase_bound<T>(port: T) -> bool;
predicate uart8250_port_membase_ioremapped<T, I, M>(port: T, ioremap: I, mapping: M) -> bool;
predicate uart8250_port_uses_ioremap<T>(port: T) -> bool;
predicate uart8250_port_irq_resource_ready<T, R>(port: T, resource: R) -> bool;
predicate uart8250_port_logical_irq_bound<T, L>(port: T, logical_irq: L) -> bool;
predicate uart8250_port_interrupt_output_still_deferred<T>(port: T) -> bool;
predicate uart8250_port_reg_shift_ready<T>(port: T) -> bool;
predicate uart8250_port_reg_io_width_ready<T>(port: T) -> bool;
predicate uart8250_port_clock_ready<T>(port: T) -> bool;
predicate uart8250_port_line_assigned<T>(port: T) -> bool;
predicate uart8250_port_registered<T>(port: T) -> bool;
predicate uart8250_port_registration_returned_line<T>(port: T) -> bool;
predicate uart8250_port_bound_to_stdout_node<T, D>(port: T, device_tree: D) -> bool;

type Serial8250RxByteRef {
}

type Serial8250TxByteRef {
}

type TtyFlipBufferRecordRef {
}

type Serial8250RxBatchRef {
}

type Serial8250TxBatchRef {
}

predicate serial8250_runtime_port_over_uart8250_port<T, P>(runtime: T, port: P) -> bool;
predicate serial8250_runtime_port_owns_irqtime_register_state<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_handles_ier_iir_lsr<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_owns_port_lock_irqsave<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_not_device_tree_parser<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_not_irqchip_controller<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_not_console_registry<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_cooperates_with_irq_action<T, A>(runtime: T, action: A) -> bool;
predicate serial8250_runtime_port_cooperates_with_tty_port<T, P>(runtime: T, tty_port: P) -> bool;
predicate serial8250_runtime_port_accepts_console_tx_from<T, C>(runtime: T, console: C) -> bool;
predicate serial8250_runtime_port_ready<T, P, A, TP>(runtime: T, port: P, action: A, tty_port: TP) -> bool;
predicate serial8250_runtime_port_online<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_tty_flip_buffer_bound<T, F>(runtime: T, flip_buffer: F) -> bool;
predicate serial8250_runtime_port_tty_xmit_fifo_bound<T, X>(runtime: T, xmit_fifo: X) -> bool;
predicate serial8250_runtime_port_rx_interrupts_deferred_until_enable<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_rdi_enabled<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_rlsi_enabled<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_thri_demand_driven<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_handle_interrupt_requires_hardirq<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_handle_interrupt_reads_iir<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_handle_interrupt_reads_lsr<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_handle_interrupt_checks_modem_status<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_handle_interrupt_rx_before_tx<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_handle_interrupt_no_plic_claim_or_complete<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_handle_rx_interrupt<T, C>(runtime: T, cause: C) -> bool;
predicate serial8250_runtime_port_handle_tx_interrupt<T, C>(runtime: T, cause: C) -> bool;
predicate serial8250_runtime_port_receive_chars_consumes_lsr_dr<T, B>(runtime: T, byte: B) -> bool;
predicate serial8250_runtime_port_receive_chars_bounded<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_receive_chars_batch_within_limit<T, B>(runtime: T, batch: B) -> bool;
predicate serial8250_runtime_port_receive_chars_pushes_flip_buffer<T, F>(runtime: T, flip_buffer: F) -> bool;
predicate serial8250_runtime_port_transmit_chars_uses_tx_load_size<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_transmit_chars_keeps_thri_when_queue_nonempty<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_transmit_chars_stops_thri_when_empty<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_start_tx_sets_thri<T>(runtime: T) -> bool;
predicate serial8250_runtime_port_stop_tx_clears_thri<T>(runtime: T) -> bool;

predicate tty_port_bound_to_uart8250_port<T, P>(tty_port: T, port: P) -> bool;
predicate tty_port_minimal_uart_state_container<T>(tty_port: T) -> bool;
predicate tty_port_owns_flip_buffer<T, F>(tty_port: T, flip_buffer: F) -> bool;
predicate tty_port_owns_xmit_fifo<T, X>(tty_port: T, xmit_fifo: X) -> bool;
predicate tty_port_not_mmio_accessor<T>(tty_port: T) -> bool;
predicate tty_port_not_irq_dispatcher<T>(tty_port: T) -> bool;
predicate tty_port_not_console_registry<T>(tty_port: T) -> bool;
predicate tty_port_ready<T, P, F, X>(tty_port: T, port: P, flip_buffer: F, xmit_fifo: X) -> bool;
predicate tty_port_n_tty_line_discipline_available<T, L>(tty_port: T, registry: L) -> bool;
predicate tty_port_initialized<T>(tty_port: T) -> bool;
predicate tty_port_uart_startup_drives_runtime_enable<T, R>(tty_port: T, runtime: R) -> bool;

predicate tty_flip_buffer_bound_to_tty_port<T, P>(flip_buffer: T, tty_port: P) -> bool;
predicate tty_flip_buffer_rx_staging_only<T>(flip_buffer: T) -> bool;
predicate tty_flip_buffer_push_is_rx_observation_boundary<T>(flip_buffer: T) -> bool;
predicate tty_flip_buffer_does_not_model_full_n_tty_read<T>(flip_buffer: T) -> bool;
predicate tty_flip_buffer_ready<T, P>(flip_buffer: T, tty_port: P) -> bool;
predicate tty_flip_buffer_record_ready<T, R>(flip_buffer: T, record: R) -> bool;
predicate tty_flip_buffer_char_inserted<T, B, R>(flip_buffer: T, byte: B, record: R) -> bool;
predicate tty_flip_buffer_push_committed<T, R>(flip_buffer: T, record: R) -> bool;
predicate tty_flip_buffer_push_after_rx_insert<T, R>(flip_buffer: T, record: R) -> bool;
predicate tty_flip_buffer_batch_push_committed<T, R, B>(flip_buffer: T, record: R, batch: B) -> bool;
predicate tty_flip_buffer_batch_push_len_matches<T, B>(flip_buffer: T, batch: B) -> bool;
predicate tty_flip_buffer_no_overflow<T>(flip_buffer: T) -> bool;

predicate n_tty_bound_to_tty_port<T, P>(n_tty: T, tty_port: P) -> bool;
predicate n_tty_bound_to_flip_buffer<T, F>(n_tty: T, flip_buffer: F) -> bool;
predicate n_tty_observes_termios_lflag<T>(n_tty: T) -> bool;
predicate n_tty_canonical_line_readiness_first_slice<T>(n_tty: T) -> bool;
predicate n_tty_canonical_read_returns_through_newline<T>(n_tty: T) -> bool;
predicate n_tty_noncanonical_byte_readiness_first_slice<T>(n_tty: T) -> bool;
predicate n_tty_echo_and_erase_deferred<T>(n_tty: T) -> bool;
predicate n_tty_full_waitqueue_deferred<T>(n_tty: T) -> bool;

predicate tty_xmit_fifo_bound_to_tty_port<T, P>(xmit_fifo: T, tty_port: P) -> bool;
predicate tty_xmit_fifo_for_ordinary_tty_write<T>(xmit_fifo: T) -> bool;
predicate tty_xmit_fifo_distinct_from_printk_console_tx<T, C>(xmit_fifo: T, console: C) -> bool;
predicate tty_xmit_fifo_runtime_tx_integration_deferred<T>(xmit_fifo: T) -> bool;
predicate tty_xmit_fifo_runtime_tx_integrated<T, R>(xmit_fifo: T, runtime: R) -> bool;
predicate tty_xmit_fifo_ready<T, P>(xmit_fifo: T, tty_port: P) -> bool;
predicate tty_xmit_fifo_byte_queued<T, B>(xmit_fifo: T, byte: B) -> bool;
predicate tty_xmit_fifo_dequeue_returns<T, B>(xmit_fifo: T, byte: B) -> bool;

predicate tty_xmit_fifo_probe_ready<T>(probe: T) -> bool;
predicate tty_xmit_fifo_probe_production_side<T>(probe: T) -> bool;
predicate tty_xmit_fifo_probe_kunit_not_stimulus<T>(probe: T) -> bool;
predicate tty_xmit_fifo_probe_enqueue_committed<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_xmit_fifo_probe_dequeue_committed<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_xmit_fifo_probe_byte_round_trip<T, B>(probe: T, byte: B) -> bool;
predicate tty_xmit_fifo_probe_queue_empty_after_dequeue<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_xmit_fifo_probe_distinct_from_printk_console_tx<T, F, C>(
    probe: T,
    xmit_fifo: F,
    console: C
) -> bool;
predicate tty_xmit_fifo_probe_keeps_runtime_tx_deferred<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_xmit_fifo_probe_does_not_kick_uart_thri<T, P>(probe: T, port: P) -> bool;
predicate tty_xmit_fifo_probe_does_not_mutate_printk_tx_queue<T, C>(probe: T, console: C) -> bool;
predicate tty_xmit_fifo_probe_no_overflow<T, F>(probe: T, xmit_fifo: F) -> bool;

predicate tty_write_runtime_tx_probe_ready<T>(probe: T) -> bool;
predicate tty_write_runtime_tx_probe_production_side<T>(probe: T) -> bool;
predicate tty_write_runtime_tx_probe_kunit_not_stimulus<T>(probe: T) -> bool;
predicate tty_write_runtime_tx_probe_enqueues_xmit_fifo<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_write_runtime_tx_probe_starts_thri<T, R>(probe: T, runtime: R) -> bool;
predicate tty_write_runtime_tx_probe_observes_plic_claim<T, P>(probe: T, plic: P) -> bool;
predicate tty_write_runtime_tx_probe_observes_irq_dispatch<T, R>(probe: T, registry: R) -> bool;
predicate tty_write_runtime_tx_probe_observes_runtime_handler<T, R>(probe: T, runtime: R) -> bool;
predicate tty_write_runtime_tx_probe_drains_xmit_fifo<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_write_runtime_tx_probe_observes_plic_complete<T, P>(probe: T, plic: P) -> bool;
predicate tty_write_runtime_tx_probe_observes_plic_loop_exit<T, P>(probe: T, plic: P) -> bool;
predicate tty_write_runtime_tx_probe_queue_empty_after_irq<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_write_runtime_tx_probe_does_not_mutate_printk_tx_queue<T, C>(probe: T, console: C) -> bool;
predicate tty_write_runtime_tx_probe_local_irq_guard_observed<T>(probe: T) -> bool;
predicate tty_write_runtime_tx_probe_last_byte_matched<T, B>(probe: T, byte: B) -> bool;

predicate tty_write_batch_runtime_tx_probe_ready<T>(probe: T) -> bool;
predicate tty_write_batch_runtime_tx_probe_production_side<T>(probe: T) -> bool;
predicate tty_write_batch_runtime_tx_probe_kunit_not_stimulus<T>(probe: T) -> bool;
predicate tty_write_batch_runtime_tx_probe_uses_fixed_bounded_batch<T, B>(probe: T, batch: B) -> bool;
predicate tty_write_batch_runtime_tx_probe_enqueues_xmit_fifo_batch<T, F, B>(
    probe: T,
    xmit_fifo: F,
    batch: B
) -> bool;
predicate tty_write_batch_runtime_tx_probe_starts_thri<T, R>(probe: T, runtime: R) -> bool;
predicate tty_write_batch_runtime_tx_probe_observes_plic_claim<T, P>(probe: T, plic: P) -> bool;
predicate tty_write_batch_runtime_tx_probe_observes_irq_dispatch<T, R>(probe: T, registry: R) -> bool;
predicate tty_write_batch_runtime_tx_probe_observes_runtime_handler<T, R>(probe: T, runtime: R) -> bool;
predicate tty_write_batch_runtime_tx_probe_drains_xmit_fifo_batch<T, F, B>(
    probe: T,
    xmit_fifo: F,
    batch: B
) -> bool;
predicate tty_write_batch_runtime_tx_probe_observes_plic_complete<T, P>(probe: T, plic: P) -> bool;
predicate tty_write_batch_runtime_tx_probe_observes_plic_loop_exit<T, P>(probe: T, plic: P) -> bool;
predicate tty_write_batch_runtime_tx_probe_queue_empty_after_irq<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_write_batch_runtime_tx_probe_does_not_mutate_printk_tx_queue<T, C>(
    probe: T,
    console: C
) -> bool;
predicate tty_write_batch_runtime_tx_probe_local_irq_guard_observed<T>(probe: T) -> bool;
predicate tty_write_batch_runtime_tx_probe_bounded_drain_observed<T, R>(probe: T, runtime: R) -> bool;
predicate tty_write_batch_runtime_tx_probe_batch_count_matched<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_write_batch_runtime_tx_probe_last_byte_matched<T, B>(probe: T, batch: B) -> bool;
predicate tty_write_batch_runtime_tx_probe_no_overflow<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate tty_write_batch_runtime_tx_probe_no_underflow<T, F>(probe: T, xmit_fifo: F) -> bool;

predicate serial8250_rx_loopback_probe_ready<T>(probe: T) -> bool;
predicate serial8250_rx_loopback_probe_production_side<T>(probe: T) -> bool;
predicate serial8250_rx_loopback_probe_uses_smoke_stimulus<T>(probe: T) -> bool;
predicate serial8250_rx_loopback_probe_kunit_not_stimulus<T>(probe: T) -> bool;
predicate serial8250_rx_loopback_probe_saves_mcr<T, P>(probe: T, port: P) -> bool;
predicate serial8250_rx_loopback_probe_enables_loopback<T, P>(probe: T, port: P) -> bool;
predicate serial8250_rx_loopback_probe_writes_tx_byte<T, B>(probe: T, byte: B) -> bool;
predicate serial8250_rx_loopback_probe_triggers_real_rx_interrupt<T, P>(probe: T, plic: P) -> bool;
predicate serial8250_rx_loopback_probe_observes_irq_dispatch<T, R>(probe: T, registry: R) -> bool;
predicate serial8250_rx_loopback_probe_observes_runtime_handler<T, R>(probe: T, runtime: R) -> bool;
predicate serial8250_rx_loopback_probe_observes_flip_buffer_push<T, F>(probe: T, flip_buffer: F) -> bool;
predicate serial8250_rx_loopback_probe_restores_mcr<T, P>(probe: T, port: P) -> bool;
predicate serial8250_rx_loopback_probe_single_byte_first_round<T, B>(probe: T, byte: B) -> bool;
predicate serial8250_rx_loopback_probe_bounded_batch_followup_allowed<T>(probe: T) -> bool;
predicate serial8250_rx_loopback_probe_setup_failure_reports_structured_diagnostic<T>(probe: T) -> bool;

predicate serial8250_rx_batch_loopback_probe_ready<T>(probe: T) -> bool;
predicate serial8250_rx_batch_loopback_probe_production_side<T>(probe: T) -> bool;
predicate serial8250_rx_batch_loopback_probe_kunit_not_stimulus<T>(probe: T) -> bool;
predicate serial8250_rx_batch_loopback_probe_uses_fixed_bounded_batch<T, B>(probe: T, batch: B) -> bool;
predicate serial8250_rx_batch_loopback_probe_writes_tx_batch<T, B>(probe: T, batch: B) -> bool;
predicate serial8250_rx_batch_loopback_probe_triggers_real_rx_interrupt<T, P>(probe: T, plic: P) -> bool;
predicate serial8250_rx_batch_loopback_probe_observes_irq_dispatch<T, R>(probe: T, registry: R) -> bool;
predicate serial8250_rx_batch_loopback_probe_observes_runtime_handler<T, R>(probe: T, runtime: R) -> bool;
predicate serial8250_rx_batch_loopback_probe_observes_flip_buffer_batch_push<T, F>(probe: T, flip_buffer: F) -> bool;
predicate serial8250_rx_batch_loopback_probe_bounded_drain_observed<T, R>(probe: T, runtime: R) -> bool;
predicate serial8250_rx_batch_loopback_probe_batch_count_matched<T, F>(probe: T, flip_buffer: F) -> bool;
predicate serial8250_rx_batch_loopback_probe_no_overflow<T, F>(probe: T, flip_buffer: F) -> bool;
predicate serial8250_rx_batch_loopback_probe_setup_failure_reports_structured_diagnostic<T>(probe: T) -> bool;

predicate serial8250_rx_kunit_observer_ready<T>(observer: T) -> bool;
predicate serial8250_rx_kunit_observer_read_only<T>(observer: T) -> bool;
predicate serial8250_rx_kunit_observer_does_not_write_tx<T>(observer: T) -> bool;
predicate serial8250_rx_kunit_observer_does_not_set_loopback<T>(observer: T) -> bool;
predicate serial8250_rx_kunit_observer_does_not_call_handler<T>(observer: T) -> bool;
predicate serial8250_rx_kunit_observer_does_not_claim_or_complete<T>(observer: T) -> bool;
predicate serial8250_rx_kunit_observer_reads_loopback_probe<T, P>(observer: T, probe: P) -> bool;
predicate serial8250_rx_kunit_observer_reads_flip_buffer_result<T, F>(observer: T, flip_buffer: F) -> bool;

predicate tty_flip_buffer_push_publishes_ready_data_first_slice<T>(flip_buffer: T) -> bool;
predicate tty_flip_buffer_probe_bytes_not_user_stdin<T>(flip_buffer: T) -> bool;
predicate tty_flip_buffer_ready_data_cleared<T>(flip_buffer: T) -> bool;
predicate tty_flip_buffer_user_smoke_fixture_ready_data_bound<T>(flip_buffer: T) -> bool;
predicate tty_input_wait_bound_to_flip_buffer<T, F>(wait: T, flip_buffer: F) -> bool;
predicate tty_input_wait_irq_rx_wakeup_first_slice<T>(wait: T) -> bool;
predicate tty_input_wait_does_not_poll_uart_rx<T>(wait: T) -> bool;
predicate tty_input_wait_full_waitqueue_deferred<T>(wait: T) -> bool;

predicate serial8250_console_registered<T, P>(console: T, port: P) -> bool;
predicate serial8250_console_real_console<T>(console: T) -> bool;
predicate serial8250_console_consdev<T>(console: T) -> bool;
predicate serial8250_console_printbuffer_suppressed_for_boot_handoff<T>(console: T) -> bool;
predicate serial8250_console_matches_stdout_path<T, D>(console: T, device_tree: D) -> bool;
predicate serial8250_console_write_backend_ready<T, P>(console: T, port: P) -> bool;
predicate serial8250_console_write_uses_uart_membase<T, P>(console: T, port: P) -> bool;
predicate serial8250_console_write_uses_lsr_thr_polling<T, P>(console: T, port: P) -> bool;
predicate serial8250_console_write_does_not_use_sbi<T>(console: T) -> bool;
predicate serial8250_console_interrupt_output_deferred_until_irqchip<T>(console: T) -> bool;
predicate serial8250_console_interrupt_driven_ready<T>(console: T) -> bool;
predicate serial8250_console_tx_queue_guarded_by_local_irq_save<T>(console: T) -> bool;
predicate serial8250_console_tx_irq_kicks_thri<T>(console: T) -> bool;
predicate serial8250_console_tx_irq_handler_drains_queue<T>(console: T) -> bool;
predicate serial8250_console_tx_queue_empty_after_irq<T>(console: T) -> bool;
predicate serial8250_console_online_trace_emitted<T>(console: T) -> bool;
predicate serial8250_console_delivered_records_not_replayed_by_earlycon<T>(console: T) -> bool;

predicate serial8250_console_burst_irq_tx_probe_ready<T>(probe: T) -> bool;
predicate serial8250_console_burst_irq_tx_probe_production_side<T>(probe: T) -> bool;
predicate serial8250_console_burst_irq_tx_probe_kunit_not_stimulus<T>(probe: T) -> bool;
predicate serial8250_console_burst_irq_tx_probe_uses_printk_frontend<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_burst_irq_tx_probe_submits_multiple_records<T>(probe: T) -> bool;
predicate serial8250_console_burst_irq_tx_probe_kicks_thri<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_burst_irq_tx_probe_observes_plic_claim<T, P>(probe: T, plic: P) -> bool;
predicate serial8250_console_burst_irq_tx_probe_observes_irq_dispatch<T, R>(probe: T, registry: R) -> bool;
predicate serial8250_console_burst_irq_tx_probe_observes_runtime_handler<T, R>(probe: T, runtime: R) -> bool;
predicate serial8250_console_burst_irq_tx_probe_drains_console_tx_queue<T, C>(
    probe: T,
    console: C
) -> bool;
predicate serial8250_console_burst_irq_tx_probe_observes_plic_complete<T, P>(probe: T, plic: P) -> bool;
predicate serial8250_console_burst_irq_tx_probe_observes_plic_loop_exit<T, P>(probe: T, plic: P) -> bool;
predicate serial8250_console_burst_irq_tx_probe_queue_empty_after_irq<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_burst_irq_tx_probe_write_count_matched<T>(probe: T) -> bool;
predicate serial8250_console_burst_irq_tx_probe_drain_count_matched<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_burst_irq_tx_probe_last_byte_matched<T>(probe: T) -> bool;
predicate serial8250_console_burst_irq_tx_probe_local_irq_guard_observed<T>(probe: T) -> bool;
predicate serial8250_console_burst_irq_tx_probe_no_overflow<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_burst_irq_tx_probe_does_not_mutate_tty_xmit_fifo<T, F>(
    probe: T,
    xmit_fifo: F
) -> bool;
predicate serial8250_console_long_irq_tx_probe_ready<T>(probe: T) -> bool;
predicate serial8250_console_long_irq_tx_probe_production_side<T>(probe: T) -> bool;
predicate serial8250_console_long_irq_tx_probe_kunit_not_stimulus<T>(probe: T) -> bool;
predicate serial8250_console_long_irq_tx_probe_uses_printk_frontend<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_long_irq_tx_probe_observes_tx_load_size<T, R>(probe: T, runtime: R) -> bool;
predicate serial8250_console_long_irq_tx_probe_message_exceeds_single_load<T>(probe: T) -> bool;
predicate serial8250_console_long_irq_tx_probe_kicks_thri<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_long_irq_tx_probe_observes_plic_claim<T, P>(probe: T, plic: P) -> bool;
predicate serial8250_console_long_irq_tx_probe_observes_irq_dispatch<T, R>(probe: T, registry: R) -> bool;
predicate serial8250_console_long_irq_tx_probe_observes_runtime_handler<T, R>(probe: T, runtime: R) -> bool;
predicate serial8250_console_long_irq_tx_probe_observes_multiple_irq_rounds<T, P>(
    probe: T,
    plic: P
) -> bool;
predicate serial8250_console_long_irq_tx_probe_observes_tx_load_budget<T, R>(probe: T, runtime: R) -> bool;
predicate serial8250_console_long_irq_tx_probe_drains_console_tx_queue<T, C>(
    probe: T,
    console: C
) -> bool;
predicate serial8250_console_long_irq_tx_probe_observes_plic_complete<T, P>(probe: T, plic: P) -> bool;
predicate serial8250_console_long_irq_tx_probe_observes_plic_loop_exit<T, P>(probe: T, plic: P) -> bool;
predicate serial8250_console_long_irq_tx_probe_queue_empty_after_irq<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_long_irq_tx_probe_write_count_matched<T>(probe: T) -> bool;
predicate serial8250_console_long_irq_tx_probe_drain_count_matched<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_long_irq_tx_probe_last_byte_matched<T>(probe: T) -> bool;
predicate serial8250_console_long_irq_tx_probe_local_irq_guard_observed<T>(probe: T) -> bool;
predicate serial8250_console_long_irq_tx_probe_no_overflow<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_long_irq_tx_probe_does_not_mutate_tty_xmit_fifo<T, F>(
    probe: T,
    xmit_fifo: F
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_ready<T>(probe: T) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_production_side<T>(probe: T) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_kunit_not_stimulus<T>(probe: T) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_uses_printk_frontend<T, C>(
    probe: T,
    console: C
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_submits_multiple_records<T>(probe: T) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_observes_tx_load_size<T, R>(
    probe: T,
    runtime: R
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_each_record_exceeds_single_load<T>(probe: T) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_kicks_thri<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_observes_plic_claim<T, P>(
    probe: T,
    plic: P
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_observes_irq_dispatch<T, R>(
    probe: T,
    registry: R
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_observes_runtime_handler<T, R>(
    probe: T,
    runtime: R
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_observes_multiple_irq_rounds<T, P>(
    probe: T,
    plic: P
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_observes_tx_load_budget<T, R>(
    probe: T,
    runtime: R
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_drains_console_tx_queue<T, C>(
    probe: T,
    console: C
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_observes_plic_complete<T, P>(
    probe: T,
    plic: P
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_observes_plic_loop_exit<T, P>(
    probe: T,
    plic: P
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_queue_empty_after_irq<T, C>(
    probe: T,
    console: C
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_write_count_matched<T>(probe: T) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_drain_count_matched<T, C>(
    probe: T,
    console: C
) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_last_byte_matched<T>(probe: T) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_local_irq_guard_observed<T>(probe: T) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_no_overflow<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_long_burst_irq_tx_probe_does_not_mutate_tty_xmit_fifo<T, F>(
    probe: T,
    xmit_fifo: F
) -> bool;
predicate serial8250_console_tx_quiesce_probe_ready<T>(probe: T) -> bool;
predicate serial8250_console_tx_quiesce_probe_production_side<T>(probe: T) -> bool;
predicate serial8250_console_tx_quiesce_probe_kunit_not_stimulus<T>(probe: T) -> bool;
predicate serial8250_console_tx_quiesce_probe_does_not_write_printk<T>(probe: T) -> bool;
predicate serial8250_console_tx_quiesce_probe_observes_queue_empty<T, C>(probe: T, console: C) -> bool;
predicate serial8250_console_tx_quiesce_probe_observes_thri_stopped<T, R>(probe: T, runtime: R) -> bool;
predicate serial8250_console_tx_quiesce_probe_no_spurious_plic_claim<T, P>(probe: T, plic: P) -> bool;
predicate serial8250_console_tx_quiesce_probe_no_spurious_irq_dispatch<T, R>(probe: T, registry: R) -> bool;
predicate serial8250_console_tx_quiesce_probe_no_uart_tx_drain<T, R>(probe: T, runtime: R) -> bool;
predicate serial8250_console_tx_quiesce_probe_no_tty_xmit_mutation<T, F>(probe: T, xmit_fifo: F) -> bool;
predicate serial8250_console_tx_quiesce_probe_no_overflow<T, C>(probe: T, console: C) -> bool;

predicate console_handoff_ready<T, B, S>(handoff: T, boot_console: B, serial_console: S) -> bool;
predicate console_handoff_triggered_by_register_console<T, R>(handoff: T, registry: R) -> bool;
predicate console_handoff_boot_console_unregistered<T, B>(handoff: T, boot_console: B) -> bool;
predicate console_handoff_boot_console_retained_by_keep_bootcon<T, B>(handoff: T, boot_console: B) -> bool;
predicate console_handoff_printk_route_switched<T, R, S>(handoff: T, registry: R, serial_console: S) -> bool;
predicate printk_frontend_only_for_payload_smoke<T, R>(payload: T, registry: R) -> bool;

object NonStdoutConsoleCandidate: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    DeviceTree.state == State::Ready;
                }

                ensures {
                    console_candidate_non_stdout_path(NonStdoutConsoleCandidate, DeviceTree);
                    console_candidate_not_platform_topology_mutating(NonStdoutConsoleCandidate);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            console_candidate_non_stdout_path(NonStdoutConsoleCandidate, DeviceTree);
            console_candidate_not_platform_topology_mutating(NonStdoutConsoleCandidate);
        }
    }
}

/*
 * BootConsole is a registry identity, not a second earlycon backend. Its write
 * operation routes to EarlyCon while it is online, but its lifecycle tracks the
 * CON_BOOT entry inside ConsoleRegistry.
 */
object BootConsole: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PrintkBuffer.state == State::Prepared;
                }

                ensures {
                    boot_console_con_boot_flag_set(BootConsole);
                    boot_console_printbuffer_flag_set(BootConsole);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            boot_console_con_boot_flag_set(BootConsole);
            boot_console_printbuffer_flag_set(BootConsole);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    PrintkBuffer.state == State::Prepared;
                }

                ensures {
                    boot_console_registered(BootConsole, EarlyCon);
                    boot_console_con_boot_flag_set(BootConsole);
                    boot_console_printbuffer_flag_set(BootConsole);
                    boot_console_write_routes_to_earlycon(BootConsole, EarlyCon);
                }
            }
        }
    }

    state State::Online {
        invariant {
            boot_console_registered(BootConsole, EarlyCon);
            boot_console_con_boot_flag_set(BootConsole);
            boot_console_printbuffer_flag_set(BootConsole);
            boot_console_write_routes_to_earlycon(BootConsole, EarlyCon);
        }

        transitions {
            on Transition::Disable -> State::Offline {
                depends_on {
                    ConsoleRegistry.state == State::Ready;
                    Serial8250Console.state == State::Ready;
                    console_registry_keep_bootcon_disabled(ConsoleRegistry);
                }

                ensures {
                    boot_console_unregistered(BootConsole);
                    boot_console_removed_from_registry(BootConsole, ConsoleRegistry);
                    boot_console_offline_trace_emitted(BootConsole);
                }
            }
        }
    }

    state State::Offline {
        invariant {
            boot_console_unregistered(BootConsole);
            boot_console_removed_from_registry(BootConsole, ConsoleRegistry);
            boot_console_offline_trace_emitted(BootConsole);
        }
    }
}

object Uart8250Port: DeviceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    DeviceTree.state == State::Ready;
                    Ioremap.state == State::Ready;
                    platform_bus_ns16550a_driver_probe_return_zero(PlatformBus);
                    device_tree_stdout_path_resolves_to_node(DeviceTree, DeviceNodeRef::Ns16550aSerial);
                }

                ensures {
                    uart8250_port_resources_ready(Uart8250Port, DeviceRef::Ns16550aSerial, DeviceTree);
                    uart8250_port_mmio_resource_bound(Uart8250Port);
                    uart8250_port_mapbase_bound(Uart8250Port);
                    uart8250_port_membase_ioremapped(Uart8250Port, Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    uart8250_port_uses_ioremap(Uart8250Port);
                    uart8250_port_reg_shift_ready(Uart8250Port);
                    uart8250_port_reg_io_width_ready(Uart8250Port);
                    uart8250_port_clock_ready(Uart8250Port);
                    uart8250_port_line_assigned(Uart8250Port);
                    uart8250_port_registered(Uart8250Port);
                    uart8250_port_registration_returned_line(Uart8250Port);
                    uart8250_port_bound_to_stdout_node(Uart8250Port, DeviceTree);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            uart8250_port_resources_ready(Uart8250Port, DeviceRef::Ns16550aSerial, DeviceTree);
            uart8250_port_mmio_resource_bound(Uart8250Port);
            uart8250_port_mapbase_bound(Uart8250Port);
            uart8250_port_membase_ioremapped(Uart8250Port, Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            uart8250_port_uses_ioremap(Uart8250Port);
            uart8250_port_reg_shift_ready(Uart8250Port);
            uart8250_port_reg_io_width_ready(Uart8250Port);
            uart8250_port_clock_ready(Uart8250Port);
            uart8250_port_line_assigned(Uart8250Port);
            uart8250_port_registered(Uart8250Port);
            uart8250_port_registration_returned_line(Uart8250Port);
            uart8250_port_bound_to_stdout_node(Uart8250Port, DeviceTree);
        }
    }
}

/*
 * Serial8250RuntimePort is the runtime behavior layer over Uart8250Port. It
 * owns the 8250 IRQ-time register/lock/dispatch shape, while Uart8250Port
 * keeps resource identity and Serial8250Console keeps printk console policy.
 * It models the Linux-like serial8250_handle_irq() boundary: IRQ core enters
 * through IrqAction, this object reads IIR/LSR under the port lock, handles RX
 * before TX, and never owns PLIC claim/complete.
 */
object Serial8250RuntimePort: DeviceObject {
    initial_state: State::Base;

    processes {
        Action::HandleInterrupt(cause: InterruptCauseRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                IrqAction.state == State::Ready;
                serial8250_runtime_port_handle_interrupt_requires_hardirq(self);
                serial8250_runtime_port_handles_ier_iir_lsr(self);
                serial8250_runtime_port_owns_port_lock_irqsave(self);
            }
            drives {
                self.Action::ReceiveChars(Serial8250RxByteRef::Uart0RxProbe);
                self.Action::TransmitChars;
            }
            ensures {
                serial8250_runtime_port_handle_interrupt_reads_iir(self);
                serial8250_runtime_port_handle_interrupt_reads_lsr(self);
                serial8250_runtime_port_handle_interrupt_checks_modem_status(self);
                serial8250_runtime_port_handle_interrupt_rx_before_tx(self);
                serial8250_runtime_port_handle_interrupt_no_plic_claim_or_complete(self);
                serial8250_runtime_port_handle_rx_interrupt(self, cause);
                serial8250_runtime_port_handle_tx_interrupt(self, cause);
            }
        }

        Action::ReceiveChars(byte: Serial8250RxByteRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                TtyFlipBuffer.state == State::Ready;
                serial8250_runtime_port_rdi_enabled(self);
                serial8250_runtime_port_rlsi_enabled(self);
            }
            drives {
                TtyFlipBuffer.Action::InsertChar(byte);
                TtyFlipBuffer.Action::Push(TtyFlipBufferRecordRef::Uart0RxProbe);
            }
            ensures {
                serial8250_runtime_port_receive_chars_consumes_lsr_dr(self, byte);
                serial8250_runtime_port_receive_chars_bounded(self);
                serial8250_runtime_port_receive_chars_batch_within_limit(
                    self,
                    Serial8250RxBatchRef::Uart0RxBatchProbe
                );
                serial8250_runtime_port_receive_chars_pushes_flip_buffer(self, TtyFlipBuffer);
            }
        }

        Action::TransmitChars {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                serial8250_runtime_port_thri_demand_driven(self);
            }
            ensures {
                serial8250_runtime_port_transmit_chars_uses_tx_load_size(self);
                serial8250_runtime_port_transmit_chars_keeps_thri_when_queue_nonempty(self);
                serial8250_runtime_port_transmit_chars_stops_thri_when_empty(self);
            }
        }

        Action::StartTx {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
            }
            ensures {
                serial8250_runtime_port_start_tx_sets_thri(self);
            }
        }

        Action::StopTx {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
            }
            ensures {
                serial8250_runtime_port_stop_tx_clears_thri(self);
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Uart8250Port.state == State::Ready;
                    IrqAction.state == State::Ready;
                    TtyPort.state == State::Ready;
                }

                ensures {
                    serial8250_runtime_port_over_uart8250_port(Serial8250RuntimePort, Uart8250Port);
                    serial8250_runtime_port_owns_irqtime_register_state(Serial8250RuntimePort);
                    serial8250_runtime_port_handles_ier_iir_lsr(Serial8250RuntimePort);
                    serial8250_runtime_port_owns_port_lock_irqsave(Serial8250RuntimePort);
                    serial8250_runtime_port_not_device_tree_parser(Serial8250RuntimePort);
                    serial8250_runtime_port_not_irqchip_controller(Serial8250RuntimePort);
                    serial8250_runtime_port_not_console_registry(Serial8250RuntimePort);
                    serial8250_runtime_port_cooperates_with_irq_action(Serial8250RuntimePort, IrqAction);
                    serial8250_runtime_port_cooperates_with_tty_port(Serial8250RuntimePort, TtyPort);
                    serial8250_runtime_port_accepts_console_tx_from(Serial8250RuntimePort, Serial8250Console);
                    serial8250_runtime_port_tty_flip_buffer_bound(Serial8250RuntimePort, TtyFlipBuffer);
                    serial8250_runtime_port_tty_xmit_fifo_bound(Serial8250RuntimePort, TtyXmitFifo);
                    serial8250_runtime_port_rx_interrupts_deferred_until_enable(Serial8250RuntimePort);
                    serial8250_runtime_port_handle_interrupt_requires_hardirq(Serial8250RuntimePort);
                    serial8250_runtime_port_ready(Serial8250RuntimePort, Uart8250Port, IrqAction, TtyPort);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            serial8250_runtime_port_over_uart8250_port(Serial8250RuntimePort, Uart8250Port);
            serial8250_runtime_port_ready(Serial8250RuntimePort, Uart8250Port, IrqAction, TtyPort);
            serial8250_runtime_port_handles_ier_iir_lsr(Serial8250RuntimePort);
            serial8250_runtime_port_owns_port_lock_irqsave(Serial8250RuntimePort);
            serial8250_runtime_port_handle_interrupt_requires_hardirq(Serial8250RuntimePort);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    Serial8250RuntimePort.state == State::Ready;
                    UartExternalIrqEnable.state == State::Ready;
                    TtyPort.state == State::Online;
                }

                ensures {
                    serial8250_runtime_port_online(Serial8250RuntimePort);
                    serial8250_runtime_port_rdi_enabled(Serial8250RuntimePort);
                    serial8250_runtime_port_rlsi_enabled(Serial8250RuntimePort);
                    serial8250_runtime_port_thri_demand_driven(Serial8250RuntimePort);
                    serial8250_runtime_port_handle_interrupt_requires_hardirq(Serial8250RuntimePort);
                }
            }
        }
    }

    state State::Online {
        invariant {
            serial8250_runtime_port_over_uart8250_port(Serial8250RuntimePort, Uart8250Port);
            serial8250_runtime_port_ready(Serial8250RuntimePort, Uart8250Port, IrqAction, TtyPort);
            serial8250_runtime_port_online(Serial8250RuntimePort);
            serial8250_runtime_port_rdi_enabled(Serial8250RuntimePort);
            serial8250_runtime_port_rlsi_enabled(Serial8250RuntimePort);
            serial8250_runtime_port_thri_demand_driven(Serial8250RuntimePort);
            serial8250_runtime_port_handle_interrupt_requires_hardirq(Serial8250RuntimePort);
        }
    }
}

/*
 * TtyPort is the minimal uart_state/tty_port container bound to the 8250 line.
 * It connects the runtime UART port to RX flip-buffer and future ordinary TTY
 * TX FIFO state, but it does not access MMIO, dispatch IRQs or own console
 * registry policy.
 */
object TtyPort: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Uart8250Port.state == State::Ready;
                    TtyLineDisciplineRegistry.state == State::Prepared;
                }

                drives {
                    TtyFlipBuffer.Transition::Setup;
                    TtyXmitFifo.Transition::Setup;
                }

                ensures {
                    tty_port_bound_to_uart8250_port(TtyPort, Uart8250Port);
                    tty_port_minimal_uart_state_container(TtyPort);
                    tty_port_owns_flip_buffer(TtyPort, TtyFlipBuffer);
                    tty_port_owns_xmit_fifo(TtyPort, TtyXmitFifo);
                    tty_port_not_mmio_accessor(TtyPort);
                    tty_port_not_irq_dispatcher(TtyPort);
                    tty_port_not_console_registry(TtyPort);
                    tty_port_n_tty_line_discipline_available(TtyPort, TtyLineDisciplineRegistry);
                    tty_port_ready(TtyPort, Uart8250Port, TtyFlipBuffer, TtyXmitFifo);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tty_port_bound_to_uart8250_port(TtyPort, Uart8250Port);
            tty_port_minimal_uart_state_container(TtyPort);
            tty_port_owns_flip_buffer(TtyPort, TtyFlipBuffer);
            tty_port_owns_xmit_fifo(TtyPort, TtyXmitFifo);
            tty_port_not_mmio_accessor(TtyPort);
            tty_port_not_irq_dispatcher(TtyPort);
            tty_port_not_console_registry(TtyPort);
            tty_port_n_tty_line_discipline_available(TtyPort, TtyLineDisciplineRegistry);
            tty_port_ready(TtyPort, Uart8250Port, TtyFlipBuffer, TtyXmitFifo);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    TtyPort.state == State::Ready;
                    TtyFlipBuffer.state == State::Ready;
                    TtyXmitFifo.state == State::Ready;
                }

                ensures {
                    tty_port_initialized(TtyPort);
                    tty_port_uart_startup_drives_runtime_enable(TtyPort, Serial8250RuntimePort);
                }
            }
        }
    }

    state State::Online {
        invariant {
            tty_port_bound_to_uart8250_port(TtyPort, Uart8250Port);
            tty_port_ready(TtyPort, Uart8250Port, TtyFlipBuffer, TtyXmitFifo);
            tty_port_initialized(TtyPort);
            tty_port_uart_startup_drives_runtime_enable(TtyPort, Serial8250RuntimePort);
        }
    }
}

/*
 * TtyFlipBuffer is the RX staging boundary reached by serial8250 RX interrupt
 * handling. The first RX round validates insert/push and publishes a bounded
 * raw ready-data slice for the current stdin/read first slice. This is still
 * not full N_TTY; canonical processing, wait queues, hangup, signal restart
 * and job-control semantics remain outside this object.
 */
object TtyFlipBuffer: ConsoleObject {
    initial_state: State::Base;

    processes {
        Action::InsertChar(byte: Serial8250RxByteRef) -> TtyFlipBufferRecordRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                tty_flip_buffer_ready(self, TtyPort);
            }
            ensures {
                tty_flip_buffer_record_ready(self, TtyFlipBufferRecordRef::Uart0RxProbe);
                tty_flip_buffer_char_inserted(self, byte, TtyFlipBufferRecordRef::Uart0RxProbe);
            }
        }

        Action::Push(record: TtyFlipBufferRecordRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                tty_flip_buffer_record_ready(self, record);
            }
            ensures {
                tty_flip_buffer_push_committed(self, record);
                tty_flip_buffer_push_after_rx_insert(self, record);
                tty_flip_buffer_push_publishes_ready_data_first_slice(self);
                tty_flip_buffer_probe_bytes_not_user_stdin(self);
                tty_flip_buffer_no_overflow(self);
            }
        }

        Action::ClearReadyData {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
            }
            ensures {
                tty_flip_buffer_ready_data_cleared(self);
                tty_flip_buffer_probe_bytes_not_user_stdin(self);
                tty_flip_buffer_no_overflow(self);
            }
        }

        Action::SeedUserSmokeReadyDataFixture {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                tty_flip_buffer_ready_data_cleared(self);
            }
            ensures {
                tty_flip_buffer_user_smoke_fixture_ready_data_bound(self);
                tty_flip_buffer_push_publishes_ready_data_first_slice(self);
                tty_flip_buffer_no_overflow(self);
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Uart8250Port.state == State::Ready;
                    TtyLineDisciplineRegistry.state == State::Prepared;
                }

                ensures {
                    tty_flip_buffer_bound_to_tty_port(TtyFlipBuffer, TtyPort);
                    tty_flip_buffer_rx_staging_only(TtyFlipBuffer);
                    tty_flip_buffer_push_is_rx_observation_boundary(TtyFlipBuffer);
                    tty_flip_buffer_does_not_model_full_n_tty_read(TtyFlipBuffer);
                    tty_flip_buffer_ready(TtyFlipBuffer, TtyPort);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tty_flip_buffer_bound_to_tty_port(TtyFlipBuffer, TtyPort);
            tty_flip_buffer_rx_staging_only(TtyFlipBuffer);
            tty_flip_buffer_push_is_rx_observation_boundary(TtyFlipBuffer);
            tty_flip_buffer_does_not_model_full_n_tty_read(TtyFlipBuffer);
            tty_flip_buffer_ready(TtyFlipBuffer, TtyPort);
        }
    }
}

/*
 * NTtyLineDiscipline is the first N_TTY boundary above TtyFlipBuffer. The
 * current slice observes termios ICANON and turns bounded RX ready-data into
 * Linux-like read/readiness decisions: canonical mode is readable only through
 * a newline-terminated slice; noncanonical mode keeps byte readiness. Echo,
 * erase, special characters, signal generation, hangup and real wait queues
 * remain outside this first slice.
 */
object NTtyLineDiscipline: ConsoleObject {
    initial_state: State::Base;

    processes {
        Action::EvaluateReadiness {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                TtyFlipBuffer.state == State::Ready;
                n_tty_bound_to_flip_buffer(self, TtyFlipBuffer);
            }
            ensures {
                n_tty_observes_termios_lflag(self);
                n_tty_canonical_line_readiness_first_slice(self);
                n_tty_noncanonical_byte_readiness_first_slice(self);
                n_tty_echo_and_erase_deferred(self);
                n_tty_full_waitqueue_deferred(self);
            }
        }

        Action::ReadLineOrBytes {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                TtyFlipBuffer.state == State::Ready;
                n_tty_bound_to_flip_buffer(self, TtyFlipBuffer);
            }
            ensures {
                n_tty_observes_termios_lflag(self);
                n_tty_canonical_read_returns_through_newline(self);
                n_tty_noncanonical_byte_readiness_first_slice(self);
                n_tty_echo_and_erase_deferred(self);
                n_tty_full_waitqueue_deferred(self);
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    TtyPort.state == State::Ready;
                    TtyFlipBuffer.state == State::Ready;
                    TtyLineDisciplineRegistry.state == State::Prepared;
                }

                ensures {
                    n_tty_bound_to_tty_port(NTtyLineDiscipline, TtyPort);
                    n_tty_bound_to_flip_buffer(NTtyLineDiscipline, TtyFlipBuffer);
                    n_tty_observes_termios_lflag(NTtyLineDiscipline);
                    n_tty_canonical_line_readiness_first_slice(NTtyLineDiscipline);
                    n_tty_noncanonical_byte_readiness_first_slice(NTtyLineDiscipline);
                    n_tty_echo_and_erase_deferred(NTtyLineDiscipline);
                    n_tty_full_waitqueue_deferred(NTtyLineDiscipline);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            n_tty_bound_to_tty_port(NTtyLineDiscipline, TtyPort);
            n_tty_bound_to_flip_buffer(NTtyLineDiscipline, TtyFlipBuffer);
            n_tty_observes_termios_lflag(NTtyLineDiscipline);
            n_tty_canonical_line_readiness_first_slice(NTtyLineDiscipline);
            n_tty_noncanonical_byte_readiness_first_slice(NTtyLineDiscipline);
            n_tty_echo_and_erase_deferred(NTtyLineDiscipline);
            n_tty_full_waitqueue_deferred(NTtyLineDiscipline);
        }
    }
}

/*
 * TtyInputWait is the minimal stdin wait boundary used by read/ppoll before a
 * full Linux wait_queue_head_t / poll_table implementation exists. It waits
 * for the existing serial8250 IRQ RX path to publish data that
 * NTtyLineDiscipline can later report as readable; it does not read UART RX
 * directly.
 */
object TtyInputWait: ConsoleObject {
    initial_state: State::Base;

    processes {
        Action::WaitReadable {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                NTtyLineDiscipline.state == State::Ready;
                n_tty_bound_to_flip_buffer(NTtyLineDiscipline, TtyFlipBuffer);
            }
            drives {
                NTtyLineDiscipline.Action::EvaluateReadiness;
            }
            ensures {
                tty_input_wait_irq_rx_wakeup_first_slice(self);
                tty_input_wait_does_not_poll_uart_rx(self);
                tty_input_wait_full_waitqueue_deferred(self);
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    NTtyLineDiscipline.state == State::Ready;
                }

                ensures {
                    tty_input_wait_bound_to_flip_buffer(self, TtyFlipBuffer);
                    tty_input_wait_irq_rx_wakeup_first_slice(self);
                    tty_input_wait_does_not_poll_uart_rx(self);
                    tty_input_wait_full_waitqueue_deferred(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tty_input_wait_bound_to_flip_buffer(TtyInputWait, TtyFlipBuffer);
            tty_input_wait_irq_rx_wakeup_first_slice(TtyInputWait);
            tty_input_wait_does_not_poll_uart_rx(TtyInputWait);
            tty_input_wait_full_waitqueue_deferred(TtyInputWait);
        }
    }
}

/*
 * TtyXmitFifo is the ordinary TTY write FIFO. It is intentionally distinct
 * from the printk console TX queue already used by Serial8250Console.
 */
object TtyXmitFifo: ConsoleObject {
    initial_state: State::Base;

    processes {
        Action::Enqueue(byte: Serial8250TxByteRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                tty_xmit_fifo_ready(self, TtyPort);
            }
            ensures {
                tty_xmit_fifo_byte_queued(self, byte);
            }
        }

        Action::DequeueForTx -> Serial8250TxByteRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                tty_xmit_fifo_ready(self, TtyPort);
            }
            ensures {
                tty_xmit_fifo_dequeue_returns(self, Serial8250TxByteRef::Uart0TxProbe);
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Uart8250Port.state == State::Ready;
                }

                ensures {
                    tty_xmit_fifo_bound_to_tty_port(TtyXmitFifo, TtyPort);
                    tty_xmit_fifo_for_ordinary_tty_write(TtyXmitFifo);
                    tty_xmit_fifo_distinct_from_printk_console_tx(TtyXmitFifo, Serial8250Console);
                    tty_xmit_fifo_runtime_tx_integration_deferred(TtyXmitFifo);
                    tty_xmit_fifo_ready(TtyXmitFifo, TtyPort);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tty_xmit_fifo_bound_to_tty_port(TtyXmitFifo, TtyPort);
            tty_xmit_fifo_for_ordinary_tty_write(TtyXmitFifo);
            tty_xmit_fifo_distinct_from_printk_console_tx(TtyXmitFifo, Serial8250Console);
            tty_xmit_fifo_ready(TtyXmitFifo, TtyPort);
        }
    }
}

/*
 * TtyXmitFifoProbe is the first controlled ordinary-TTY TX FIFO stimulus.
 * It exercises only TtyXmitFifo enqueue/dequeue and proves that this FIFO is
 * still separate from printk console TX and from UART THRI interrupt kicking.
 */
object TtyXmitFifoProbe: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    TtyPort.state == State::Online;
                    TtyXmitFifo.state == State::Ready;
                    Serial8250Console.state == State::Online;
                    Serial8250RuntimePort.state == State::Online;
                    Serial8250RxBatchLoopbackProbe.state == State::Ready;
                }

                drives {
                    TtyXmitFifo.Action::Enqueue(Serial8250TxByteRef::Uart0TxProbe);
                    TtyXmitFifo.Action::DequeueForTx;
                }

                ensures {
                    tty_xmit_fifo_probe_ready(TtyXmitFifoProbe);
                    tty_xmit_fifo_probe_production_side(TtyXmitFifoProbe);
                    tty_xmit_fifo_probe_kunit_not_stimulus(TtyXmitFifoProbe);
                    tty_xmit_fifo_probe_enqueue_committed(TtyXmitFifoProbe, TtyXmitFifo);
                    tty_xmit_fifo_probe_dequeue_committed(TtyXmitFifoProbe, TtyXmitFifo);
                    tty_xmit_fifo_probe_byte_round_trip(
                        TtyXmitFifoProbe,
                        Serial8250TxByteRef::Uart0TxProbe
                    );
                    tty_xmit_fifo_probe_queue_empty_after_dequeue(TtyXmitFifoProbe, TtyXmitFifo);
                    tty_xmit_fifo_probe_distinct_from_printk_console_tx(
                        TtyXmitFifoProbe,
                        TtyXmitFifo,
                        Serial8250Console
                    );
                    tty_xmit_fifo_probe_keeps_runtime_tx_deferred(TtyXmitFifoProbe, TtyXmitFifo);
                    tty_xmit_fifo_probe_does_not_kick_uart_thri(TtyXmitFifoProbe, Uart8250Port);
                    tty_xmit_fifo_probe_does_not_mutate_printk_tx_queue(
                        TtyXmitFifoProbe,
                        Serial8250Console
                    );
                    tty_xmit_fifo_probe_no_overflow(TtyXmitFifoProbe, TtyXmitFifo);
                    tty_xmit_fifo_byte_queued(TtyXmitFifo, Serial8250TxByteRef::Uart0TxProbe);
                    tty_xmit_fifo_dequeue_returns(TtyXmitFifo, Serial8250TxByteRef::Uart0TxProbe);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tty_xmit_fifo_probe_ready(TtyXmitFifoProbe);
            tty_xmit_fifo_probe_production_side(TtyXmitFifoProbe);
            tty_xmit_fifo_probe_kunit_not_stimulus(TtyXmitFifoProbe);
        }
    }
}

/*
 * TtyWriteRuntimeTxProbe is the first controlled ordinary-TTY write TX
 * integration. It submits one byte through TtyXmitFifo, starts THRI through
 * Serial8250RuntimePort, and observes the normal PLIC/IRQ/runtime handler
 * cycle draining the FIFO without using printk's console TX queue.
 */
object TtyWriteRuntimeTxProbe: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    TtyXmitFifoProbe.state == State::Ready;
                    TtyPort.state == State::Online;
                    TtyXmitFifo.state == State::Ready;
                    Serial8250RuntimePort.state == State::Online;
                    UartExternalIrqEnable.state == State::Ready;
                    Plic.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                }

                drives {
                    TtyXmitFifo.Action::Enqueue(Serial8250TxByteRef::Uart0TxProbe);
                    Serial8250RuntimePort.Action::StartTx;
                    RiscvIntc.Action::HandleExternalInput(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    Serial8250RuntimePort.Action::HandleInterrupt(InterruptCauseRef::SupervisorExternalIrq);
                    Serial8250RuntimePort.Action::TransmitChars;
                    TtyXmitFifo.Action::DequeueForTx;
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                }

                ensures {
                    tty_write_runtime_tx_probe_ready(TtyWriteRuntimeTxProbe);
                    tty_write_runtime_tx_probe_production_side(TtyWriteRuntimeTxProbe);
                    tty_write_runtime_tx_probe_kunit_not_stimulus(TtyWriteRuntimeTxProbe);
                    tty_write_runtime_tx_probe_enqueues_xmit_fifo(
                        TtyWriteRuntimeTxProbe,
                        TtyXmitFifo
                    );
                    tty_write_runtime_tx_probe_starts_thri(
                        TtyWriteRuntimeTxProbe,
                        Serial8250RuntimePort
                    );
                    tty_write_runtime_tx_probe_observes_plic_claim(TtyWriteRuntimeTxProbe, Plic);
                    tty_write_runtime_tx_probe_observes_irq_dispatch(
                        TtyWriteRuntimeTxProbe,
                        IrqHandlerRegistry
                    );
                    tty_write_runtime_tx_probe_observes_runtime_handler(
                        TtyWriteRuntimeTxProbe,
                        Serial8250RuntimePort
                    );
                    tty_write_runtime_tx_probe_drains_xmit_fifo(
                        TtyWriteRuntimeTxProbe,
                        TtyXmitFifo
                    );
                    tty_write_runtime_tx_probe_observes_plic_complete(TtyWriteRuntimeTxProbe, Plic);
                    tty_write_runtime_tx_probe_observes_plic_loop_exit(TtyWriteRuntimeTxProbe, Plic);
                    tty_write_runtime_tx_probe_queue_empty_after_irq(
                        TtyWriteRuntimeTxProbe,
                        TtyXmitFifo
                    );
                    tty_write_runtime_tx_probe_does_not_mutate_printk_tx_queue(
                        TtyWriteRuntimeTxProbe,
                        Serial8250Console
                    );
                    tty_write_runtime_tx_probe_local_irq_guard_observed(TtyWriteRuntimeTxProbe);
                    tty_write_runtime_tx_probe_last_byte_matched(
                        TtyWriteRuntimeTxProbe,
                        Serial8250TxByteRef::Uart0TxProbe
                    );
                    tty_xmit_fifo_runtime_tx_integrated(TtyXmitFifo, Serial8250RuntimePort);
                    tty_xmit_fifo_dequeue_returns(TtyXmitFifo, Serial8250TxByteRef::Uart0TxProbe);
                    serial8250_runtime_port_start_tx_sets_thri(Serial8250RuntimePort);
                    serial8250_runtime_port_transmit_chars_stops_thri_when_empty(Serial8250RuntimePort);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tty_write_runtime_tx_probe_ready(TtyWriteRuntimeTxProbe);
            tty_write_runtime_tx_probe_production_side(TtyWriteRuntimeTxProbe);
            tty_write_runtime_tx_probe_kunit_not_stimulus(TtyWriteRuntimeTxProbe);
        }
    }
}

/*
 * TtyWriteBatchRuntimeTxProbe is the bounded follow-up for ordinary-TTY
 * write TX. It submits a fixed small batch through TtyXmitFifo and observes
 * the same real IRQ-time TransmitChars path drain the batch under tx_loadsz
 * constraints without using printk's console TX queue.
 */
object TtyWriteBatchRuntimeTxProbe: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    TtyWriteRuntimeTxProbe.state == State::Ready;
                    TtyPort.state == State::Online;
                    TtyXmitFifo.state == State::Ready;
                    Serial8250RuntimePort.state == State::Online;
                    UartExternalIrqEnable.state == State::Ready;
                    Plic.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                }

                drives {
                    TtyXmitFifo.Action::Enqueue(Serial8250TxByteRef::Uart0TxProbe);
                    TtyXmitFifo.Action::Enqueue(Serial8250TxByteRef::Uart0TxProbe);
                    TtyXmitFifo.Action::Enqueue(Serial8250TxByteRef::Uart0TxProbe);
                    TtyXmitFifo.Action::Enqueue(Serial8250TxByteRef::Uart0TxProbe);
                    Serial8250RuntimePort.Action::StartTx;
                    RiscvIntc.Action::HandleExternalInput(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    Serial8250RuntimePort.Action::HandleInterrupt(InterruptCauseRef::SupervisorExternalIrq);
                    Serial8250RuntimePort.Action::TransmitChars;
                    TtyXmitFifo.Action::DequeueForTx;
                    TtyXmitFifo.Action::DequeueForTx;
                    TtyXmitFifo.Action::DequeueForTx;
                    TtyXmitFifo.Action::DequeueForTx;
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                }

                ensures {
                    tty_write_batch_runtime_tx_probe_ready(TtyWriteBatchRuntimeTxProbe);
                    tty_write_batch_runtime_tx_probe_production_side(TtyWriteBatchRuntimeTxProbe);
                    tty_write_batch_runtime_tx_probe_kunit_not_stimulus(TtyWriteBatchRuntimeTxProbe);
                    tty_write_batch_runtime_tx_probe_uses_fixed_bounded_batch(
                        TtyWriteBatchRuntimeTxProbe,
                        Serial8250TxBatchRef::Uart0TxBatchProbe
                    );
                    tty_write_batch_runtime_tx_probe_enqueues_xmit_fifo_batch(
                        TtyWriteBatchRuntimeTxProbe,
                        TtyXmitFifo,
                        Serial8250TxBatchRef::Uart0TxBatchProbe
                    );
                    tty_write_batch_runtime_tx_probe_starts_thri(
                        TtyWriteBatchRuntimeTxProbe,
                        Serial8250RuntimePort
                    );
                    tty_write_batch_runtime_tx_probe_observes_plic_claim(
                        TtyWriteBatchRuntimeTxProbe,
                        Plic
                    );
                    tty_write_batch_runtime_tx_probe_observes_irq_dispatch(
                        TtyWriteBatchRuntimeTxProbe,
                        IrqHandlerRegistry
                    );
                    tty_write_batch_runtime_tx_probe_observes_runtime_handler(
                        TtyWriteBatchRuntimeTxProbe,
                        Serial8250RuntimePort
                    );
                    tty_write_batch_runtime_tx_probe_drains_xmit_fifo_batch(
                        TtyWriteBatchRuntimeTxProbe,
                        TtyXmitFifo,
                        Serial8250TxBatchRef::Uart0TxBatchProbe
                    );
                    tty_write_batch_runtime_tx_probe_observes_plic_complete(
                        TtyWriteBatchRuntimeTxProbe,
                        Plic
                    );
                    tty_write_batch_runtime_tx_probe_observes_plic_loop_exit(
                        TtyWriteBatchRuntimeTxProbe,
                        Plic
                    );
                    tty_write_batch_runtime_tx_probe_queue_empty_after_irq(
                        TtyWriteBatchRuntimeTxProbe,
                        TtyXmitFifo
                    );
                    tty_write_batch_runtime_tx_probe_does_not_mutate_printk_tx_queue(
                        TtyWriteBatchRuntimeTxProbe,
                        Serial8250Console
                    );
                    tty_write_batch_runtime_tx_probe_local_irq_guard_observed(TtyWriteBatchRuntimeTxProbe);
                    tty_write_batch_runtime_tx_probe_bounded_drain_observed(
                        TtyWriteBatchRuntimeTxProbe,
                        Serial8250RuntimePort
                    );
                    tty_write_batch_runtime_tx_probe_batch_count_matched(
                        TtyWriteBatchRuntimeTxProbe,
                        TtyXmitFifo
                    );
                    tty_write_batch_runtime_tx_probe_last_byte_matched(
                        TtyWriteBatchRuntimeTxProbe,
                        Serial8250TxBatchRef::Uart0TxBatchProbe
                    );
                    tty_write_batch_runtime_tx_probe_no_overflow(TtyWriteBatchRuntimeTxProbe, TtyXmitFifo);
                    tty_write_batch_runtime_tx_probe_no_underflow(TtyWriteBatchRuntimeTxProbe, TtyXmitFifo);
                    tty_xmit_fifo_runtime_tx_integrated(TtyXmitFifo, Serial8250RuntimePort);
                    serial8250_runtime_port_start_tx_sets_thri(Serial8250RuntimePort);
                    serial8250_runtime_port_transmit_chars_stops_thri_when_empty(Serial8250RuntimePort);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tty_write_batch_runtime_tx_probe_ready(TtyWriteBatchRuntimeTxProbe);
            tty_write_batch_runtime_tx_probe_production_side(TtyWriteBatchRuntimeTxProbe);
            tty_write_batch_runtime_tx_probe_kunit_not_stimulus(TtyWriteBatchRuntimeTxProbe);
        }
    }
}

/*
 * Serial8250RxLoopbackProbe is the production-side RX stimulus boundary for
 * the first runtime RX round. It uses 8250 loopback: save MCR, enable
 * loopback, write one TX byte supplied by the smoke/probe path, let hardware
 * produce a real RX interrupt, observe the normal PLIC/IRQ/runtime path, then
 * restore MCR. It is not a KUnit observer.
 */
object Serial8250RxLoopbackProbe: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Serial8250RuntimePort.state == State::Online;
                    UartExternalIrqEnable.state == State::Ready;
                    Plic.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                    TtyFlipBuffer.state == State::Ready;
                }

                drives {
                    Serial8250RuntimePort.Action::StartTx;
                    RiscvIntc.Action::HandleExternalInput(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    Serial8250RuntimePort.Action::HandleInterrupt(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                }

                ensures {
                    serial8250_rx_loopback_probe_ready(Serial8250RxLoopbackProbe);
                    serial8250_rx_loopback_probe_production_side(Serial8250RxLoopbackProbe);
                    serial8250_rx_loopback_probe_uses_smoke_stimulus(Serial8250RxLoopbackProbe);
                    serial8250_rx_loopback_probe_kunit_not_stimulus(Serial8250RxLoopbackProbe);
                    serial8250_rx_loopback_probe_saves_mcr(Serial8250RxLoopbackProbe, Uart8250Port);
                    serial8250_rx_loopback_probe_enables_loopback(Serial8250RxLoopbackProbe, Uart8250Port);
                    serial8250_rx_loopback_probe_writes_tx_byte(
                        Serial8250RxLoopbackProbe,
                        Serial8250TxByteRef::Uart0TxProbe
                    );
                    serial8250_rx_loopback_probe_triggers_real_rx_interrupt(Serial8250RxLoopbackProbe, Plic);
                    serial8250_rx_loopback_probe_observes_irq_dispatch(Serial8250RxLoopbackProbe, IrqHandlerRegistry);
                    serial8250_rx_loopback_probe_observes_runtime_handler(
                        Serial8250RxLoopbackProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_rx_loopback_probe_observes_flip_buffer_push(
                        Serial8250RxLoopbackProbe,
                        TtyFlipBuffer
                    );
                    serial8250_rx_loopback_probe_restores_mcr(Serial8250RxLoopbackProbe, Uart8250Port);
                    serial8250_rx_loopback_probe_single_byte_first_round(
                        Serial8250RxLoopbackProbe,
                        Serial8250RxByteRef::Uart0RxProbe
                    );
                    serial8250_rx_loopback_probe_bounded_batch_followup_allowed(Serial8250RxLoopbackProbe);
                    serial8250_rx_loopback_probe_setup_failure_reports_structured_diagnostic(
                        Serial8250RxLoopbackProbe
                    );
                    tty_flip_buffer_push_committed(TtyFlipBuffer, TtyFlipBufferRecordRef::Uart0RxProbe);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            serial8250_rx_loopback_probe_ready(Serial8250RxLoopbackProbe);
            serial8250_rx_loopback_probe_production_side(Serial8250RxLoopbackProbe);
            serial8250_rx_loopback_probe_uses_smoke_stimulus(Serial8250RxLoopbackProbe);
            serial8250_rx_loopback_probe_kunit_not_stimulus(Serial8250RxLoopbackProbe);
            serial8250_rx_loopback_probe_restores_mcr(Serial8250RxLoopbackProbe, Uart8250Port);
            serial8250_rx_loopback_probe_setup_failure_reports_structured_diagnostic(
                Serial8250RxLoopbackProbe
            );
            tty_flip_buffer_push_committed(TtyFlipBuffer, TtyFlipBufferRecordRef::Uart0RxProbe);
        }
    }
}

/*
 * Serial8250RxBatchLoopbackProbe is the bounded follow-up RX stimulus. It
 * runs after the single-byte loopback probe has proven the first path, writes
 * a fixed batch smaller than the runtime drain limit through 8250 loopback,
 * and observes one normal RX handler pass pushing the batch into flip buffer.
 */
object Serial8250RxBatchLoopbackProbe: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Serial8250RxLoopbackProbe.state == State::Ready;
                    Serial8250RuntimePort.state == State::Online;
                    Plic.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                    TtyFlipBuffer.state == State::Ready;
                }

                drives {
                    Serial8250RuntimePort.Action::StartTx;
                    RiscvIntc.Action::HandleExternalInput(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    Serial8250RuntimePort.Action::HandleInterrupt(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                }

                ensures {
                    serial8250_rx_batch_loopback_probe_ready(Serial8250RxBatchLoopbackProbe);
                    serial8250_rx_batch_loopback_probe_production_side(Serial8250RxBatchLoopbackProbe);
                    serial8250_rx_batch_loopback_probe_kunit_not_stimulus(Serial8250RxBatchLoopbackProbe);
                    serial8250_rx_batch_loopback_probe_uses_fixed_bounded_batch(
                        Serial8250RxBatchLoopbackProbe,
                        Serial8250RxBatchRef::Uart0RxBatchProbe
                    );
                    serial8250_rx_batch_loopback_probe_writes_tx_batch(
                        Serial8250RxBatchLoopbackProbe,
                        Serial8250RxBatchRef::Uart0RxBatchProbe
                    );
                    serial8250_rx_batch_loopback_probe_triggers_real_rx_interrupt(
                        Serial8250RxBatchLoopbackProbe,
                        Plic
                    );
                    serial8250_rx_batch_loopback_probe_observes_irq_dispatch(
                        Serial8250RxBatchLoopbackProbe,
                        IrqHandlerRegistry
                    );
                    serial8250_rx_batch_loopback_probe_observes_runtime_handler(
                        Serial8250RxBatchLoopbackProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_rx_batch_loopback_probe_observes_flip_buffer_batch_push(
                        Serial8250RxBatchLoopbackProbe,
                        TtyFlipBuffer
                    );
                    serial8250_rx_batch_loopback_probe_bounded_drain_observed(
                        Serial8250RxBatchLoopbackProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_rx_batch_loopback_probe_batch_count_matched(
                        Serial8250RxBatchLoopbackProbe,
                        TtyFlipBuffer
                    );
                    serial8250_rx_batch_loopback_probe_no_overflow(
                        Serial8250RxBatchLoopbackProbe,
                        TtyFlipBuffer
                    );
                    serial8250_rx_batch_loopback_probe_setup_failure_reports_structured_diagnostic(
                        Serial8250RxBatchLoopbackProbe
                    );
                    tty_flip_buffer_batch_push_committed(
                        TtyFlipBuffer,
                        TtyFlipBufferRecordRef::Uart0RxProbe,
                        Serial8250RxBatchRef::Uart0RxBatchProbe
                    );
                    tty_flip_buffer_batch_push_len_matches(
                        TtyFlipBuffer,
                        Serial8250RxBatchRef::Uart0RxBatchProbe
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            serial8250_rx_batch_loopback_probe_ready(Serial8250RxBatchLoopbackProbe);
            serial8250_rx_batch_loopback_probe_production_side(Serial8250RxBatchLoopbackProbe);
            serial8250_rx_batch_loopback_probe_kunit_not_stimulus(Serial8250RxBatchLoopbackProbe);
            serial8250_rx_batch_loopback_probe_setup_failure_reports_structured_diagnostic(
                Serial8250RxBatchLoopbackProbe
            );
        }
    }
}

/*
 * Serial8250RxKunitObserver is only the checker for the RX loopback result.
 * It may read counters/facts and emit diagnostics through the allowed sink,
 * but it must not write TX, set loopback, call handlers, or touch PLIC
 * claim/complete state.
 */
object Serial8250RxKunitObserver: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Serial8250RxLoopbackProbe.state == State::Ready;
                    TtyFlipBuffer.state == State::Ready;
                }

                ensures {
                    serial8250_rx_kunit_observer_ready(Serial8250RxKunitObserver);
                    serial8250_rx_kunit_observer_read_only(Serial8250RxKunitObserver);
                    serial8250_rx_kunit_observer_does_not_write_tx(Serial8250RxKunitObserver);
                    serial8250_rx_kunit_observer_does_not_set_loopback(Serial8250RxKunitObserver);
                    serial8250_rx_kunit_observer_does_not_call_handler(Serial8250RxKunitObserver);
                    serial8250_rx_kunit_observer_does_not_claim_or_complete(Serial8250RxKunitObserver);
                    serial8250_rx_kunit_observer_reads_loopback_probe(
                        Serial8250RxKunitObserver,
                        Serial8250RxLoopbackProbe
                    );
                    serial8250_rx_kunit_observer_reads_flip_buffer_result(
                        Serial8250RxKunitObserver,
                        TtyFlipBuffer
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            serial8250_rx_kunit_observer_ready(Serial8250RxKunitObserver);
            serial8250_rx_kunit_observer_read_only(Serial8250RxKunitObserver);
            serial8250_rx_kunit_observer_does_not_write_tx(Serial8250RxKunitObserver);
            serial8250_rx_kunit_observer_does_not_set_loopback(Serial8250RxKunitObserver);
            serial8250_rx_kunit_observer_does_not_call_handler(Serial8250RxKunitObserver);
            serial8250_rx_kunit_observer_does_not_claim_or_complete(Serial8250RxKunitObserver);
            serial8250_rx_kunit_observer_reads_loopback_probe(
                Serial8250RxKunitObserver,
                Serial8250RxLoopbackProbe
            );
            serial8250_rx_kunit_observer_reads_flip_buffer_result(Serial8250RxKunitObserver, TtyFlipBuffer);
        }
    }
}

/*
 * Serial8250Console is the real console entry created from Uart8250Port. It
 * becomes the active printk route only through ConsoleRegistry policy.
 */
object Serial8250Console: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Uart8250Port.state == State::Ready;
                    Console.state == State::Prepared;
                    ConsoleDriverSet.state == State::Prepared;
                    DeviceTree.state == State::Ready;
                }

                ensures {
                    serial8250_console_registered(Serial8250Console, Uart8250Port);
                    serial8250_console_real_console(Serial8250Console);
                    serial8250_console_consdev(Serial8250Console);
                    serial8250_console_printbuffer_suppressed_for_boot_handoff(Serial8250Console);
                    serial8250_console_matches_stdout_path(Serial8250Console, DeviceTree);
                    serial8250_console_write_backend_ready(Serial8250Console, Uart8250Port);
                    serial8250_console_write_uses_uart_membase(Serial8250Console, Uart8250Port);
                    serial8250_console_write_uses_lsr_thr_polling(Serial8250Console, Uart8250Port);
                    serial8250_console_write_does_not_use_sbi(Serial8250Console);
                    serial8250_console_interrupt_output_deferred_until_irqchip(Serial8250Console);
                    serial8250_console_online_trace_emitted(Serial8250Console);
                    serial8250_console_delivered_records_not_replayed_by_earlycon(Serial8250Console);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            serial8250_console_registered(Serial8250Console, Uart8250Port);
            serial8250_console_real_console(Serial8250Console);
            serial8250_console_consdev(Serial8250Console);
            serial8250_console_printbuffer_suppressed_for_boot_handoff(Serial8250Console);
            serial8250_console_matches_stdout_path(Serial8250Console, DeviceTree);
            serial8250_console_write_backend_ready(Serial8250Console, Uart8250Port);
            serial8250_console_write_uses_uart_membase(Serial8250Console, Uart8250Port);
            serial8250_console_write_uses_lsr_thr_polling(Serial8250Console, Uart8250Port);
            serial8250_console_write_does_not_use_sbi(Serial8250Console);
            serial8250_console_interrupt_output_deferred_until_irqchip(Serial8250Console);
            serial8250_console_online_trace_emitted(Serial8250Console);
            serial8250_console_delivered_records_not_replayed_by_earlycon(Serial8250Console);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    UartInterruptChainProbe.state == State::Ready;
                }

                ensures {
                    serial8250_console_interrupt_driven_ready(Serial8250Console);
                }
            }
        }
    }

    state State::Online {
        invariant {
            serial8250_console_registered(Serial8250Console, Uart8250Port);
            serial8250_console_real_console(Serial8250Console);
            serial8250_console_consdev(Serial8250Console);
            serial8250_console_matches_stdout_path(Serial8250Console, DeviceTree);
            serial8250_console_write_backend_ready(Serial8250Console, Uart8250Port);
            serial8250_console_write_uses_uart_membase(Serial8250Console, Uart8250Port);
            serial8250_console_write_does_not_use_sbi(Serial8250Console);
            serial8250_console_interrupt_driven_ready(Serial8250Console);
            serial8250_console_online_trace_emitted(Serial8250Console);
            serial8250_console_delivered_records_not_replayed_by_earlycon(Serial8250Console);
        }
    }
}

/*
 * Serial8250ConsoleBurstIrqTxProbe is the printk-console stability follow-up.
 * It submits multiple printk records through the public printk frontend and
 * observes the same interrupt-driven serial8250 console queue drain without
 * touching ordinary TTY TX state.
 */
object Serial8250ConsoleBurstIrqTxProbe: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Serial8250Console.state == State::Online;
                    UartExternalIrqEnable.state == State::Ready;
                    Plic.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                }

                drives {
                    Serial8250RuntimePort.Action::StartTx;
                    RiscvIntc.Action::HandleExternalInput(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    Serial8250RuntimePort.Action::HandleInterrupt(InterruptCauseRef::SupervisorExternalIrq);
                    Serial8250RuntimePort.Action::TransmitChars;
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                }

                ensures {
                    serial8250_console_burst_irq_tx_probe_ready(Serial8250ConsoleBurstIrqTxProbe);
                    serial8250_console_burst_irq_tx_probe_production_side(Serial8250ConsoleBurstIrqTxProbe);
                    serial8250_console_burst_irq_tx_probe_kunit_not_stimulus(Serial8250ConsoleBurstIrqTxProbe);
                    serial8250_console_burst_irq_tx_probe_uses_printk_frontend(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_burst_irq_tx_probe_submits_multiple_records(
                        Serial8250ConsoleBurstIrqTxProbe
                    );
                    serial8250_console_burst_irq_tx_probe_kicks_thri(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_burst_irq_tx_probe_observes_plic_claim(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Plic
                    );
                    serial8250_console_burst_irq_tx_probe_observes_irq_dispatch(
                        Serial8250ConsoleBurstIrqTxProbe,
                        IrqHandlerRegistry
                    );
                    serial8250_console_burst_irq_tx_probe_observes_runtime_handler(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_console_burst_irq_tx_probe_drains_console_tx_queue(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_burst_irq_tx_probe_observes_plic_complete(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Plic
                    );
                    serial8250_console_burst_irq_tx_probe_observes_plic_loop_exit(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Plic
                    );
                    serial8250_console_burst_irq_tx_probe_queue_empty_after_irq(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_burst_irq_tx_probe_write_count_matched(
                        Serial8250ConsoleBurstIrqTxProbe
                    );
                    serial8250_console_burst_irq_tx_probe_drain_count_matched(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_burst_irq_tx_probe_last_byte_matched(
                        Serial8250ConsoleBurstIrqTxProbe
                    );
                    serial8250_console_burst_irq_tx_probe_local_irq_guard_observed(
                        Serial8250ConsoleBurstIrqTxProbe
                    );
                    serial8250_console_burst_irq_tx_probe_no_overflow(
                        Serial8250ConsoleBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_burst_irq_tx_probe_does_not_mutate_tty_xmit_fifo(
                        Serial8250ConsoleBurstIrqTxProbe,
                        TtyXmitFifo
                    );
                    serial8250_console_tx_queue_empty_after_irq(Serial8250Console);
                    serial8250_runtime_port_transmit_chars_stops_thri_when_empty(Serial8250RuntimePort);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            serial8250_console_burst_irq_tx_probe_ready(Serial8250ConsoleBurstIrqTxProbe);
            serial8250_console_burst_irq_tx_probe_production_side(Serial8250ConsoleBurstIrqTxProbe);
            serial8250_console_burst_irq_tx_probe_kunit_not_stimulus(Serial8250ConsoleBurstIrqTxProbe);
        }
    }
}

/*
 * Serial8250ConsoleLongIrqTxProbe submits one printk record whose CRLF-expanded
 * byte count exceeds one tx_loadsz. It observes that the real THRI/PLIC/IRQ
 * path drains it across multiple handler rounds; it does not touch TTY TX.
 */
object Serial8250ConsoleLongIrqTxProbe: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Serial8250ConsoleBurstIrqTxProbe.state == State::Ready;
                    Serial8250Console.state == State::Online;
                    UartExternalIrqEnable.state == State::Ready;
                    Plic.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                }

                drives {
                    Serial8250RuntimePort.Action::StartTx;
                    RiscvIntc.Action::HandleExternalInput(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    Serial8250RuntimePort.Action::HandleInterrupt(InterruptCauseRef::SupervisorExternalIrq);
                    Serial8250RuntimePort.Action::TransmitChars;
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    Serial8250RuntimePort.Action::HandleInterrupt(InterruptCauseRef::SupervisorExternalIrq);
                    Serial8250RuntimePort.Action::TransmitChars;
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                }

                ensures {
                    serial8250_console_long_irq_tx_probe_ready(Serial8250ConsoleLongIrqTxProbe);
                    serial8250_console_long_irq_tx_probe_production_side(Serial8250ConsoleLongIrqTxProbe);
                    serial8250_console_long_irq_tx_probe_kunit_not_stimulus(Serial8250ConsoleLongIrqTxProbe);
                    serial8250_console_long_irq_tx_probe_uses_printk_frontend(
                        Serial8250ConsoleLongIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_irq_tx_probe_observes_tx_load_size(
                        Serial8250ConsoleLongIrqTxProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_console_long_irq_tx_probe_message_exceeds_single_load(
                        Serial8250ConsoleLongIrqTxProbe
                    );
                    serial8250_console_long_irq_tx_probe_kicks_thri(
                        Serial8250ConsoleLongIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_irq_tx_probe_observes_plic_claim(
                        Serial8250ConsoleLongIrqTxProbe,
                        Plic
                    );
                    serial8250_console_long_irq_tx_probe_observes_irq_dispatch(
                        Serial8250ConsoleLongIrqTxProbe,
                        IrqHandlerRegistry
                    );
                    serial8250_console_long_irq_tx_probe_observes_runtime_handler(
                        Serial8250ConsoleLongIrqTxProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_console_long_irq_tx_probe_observes_multiple_irq_rounds(
                        Serial8250ConsoleLongIrqTxProbe,
                        Plic
                    );
                    serial8250_console_long_irq_tx_probe_observes_tx_load_budget(
                        Serial8250ConsoleLongIrqTxProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_console_long_irq_tx_probe_drains_console_tx_queue(
                        Serial8250ConsoleLongIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_irq_tx_probe_observes_plic_complete(
                        Serial8250ConsoleLongIrqTxProbe,
                        Plic
                    );
                    serial8250_console_long_irq_tx_probe_observes_plic_loop_exit(
                        Serial8250ConsoleLongIrqTxProbe,
                        Plic
                    );
                    serial8250_console_long_irq_tx_probe_queue_empty_after_irq(
                        Serial8250ConsoleLongIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_irq_tx_probe_write_count_matched(
                        Serial8250ConsoleLongIrqTxProbe
                    );
                    serial8250_console_long_irq_tx_probe_drain_count_matched(
                        Serial8250ConsoleLongIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_irq_tx_probe_last_byte_matched(
                        Serial8250ConsoleLongIrqTxProbe
                    );
                    serial8250_console_long_irq_tx_probe_local_irq_guard_observed(
                        Serial8250ConsoleLongIrqTxProbe
                    );
                    serial8250_console_long_irq_tx_probe_no_overflow(
                        Serial8250ConsoleLongIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_irq_tx_probe_does_not_mutate_tty_xmit_fifo(
                        Serial8250ConsoleLongIrqTxProbe,
                        TtyXmitFifo
                    );
                    serial8250_runtime_port_transmit_chars_uses_tx_load_size(Serial8250RuntimePort);
                    serial8250_runtime_port_transmit_chars_keeps_thri_when_queue_nonempty(
                        Serial8250RuntimePort
                    );
                    serial8250_runtime_port_transmit_chars_stops_thri_when_empty(
                        Serial8250RuntimePort
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            serial8250_console_long_irq_tx_probe_ready(Serial8250ConsoleLongIrqTxProbe);
            serial8250_console_long_irq_tx_probe_production_side(Serial8250ConsoleLongIrqTxProbe);
            serial8250_console_long_irq_tx_probe_kunit_not_stimulus(Serial8250ConsoleLongIrqTxProbe);
        }
    }
}

/*
 * Serial8250ConsoleLongBurstIrqTxProbe extends the printk-console TX load
 * check from one long record to several long records. Each record still exceeds
 * one tx_loadsz, so the only acceptable drain path is repeated real
 * THRI/PLIC/IRQ/runtime handler rounds. It remains a production-side printk
 * stimulus; KUnit may only observe its facts.
 */
object Serial8250ConsoleLongBurstIrqTxProbe: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Serial8250ConsoleLongIrqTxProbe.state == State::Ready;
                    Serial8250Console.state == State::Online;
                    UartExternalIrqEnable.state == State::Ready;
                    Plic.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                }

                drives {
                    Serial8250RuntimePort.Action::StartTx;
                    RiscvIntc.Action::HandleExternalInput(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    Serial8250RuntimePort.Action::HandleInterrupt(InterruptCauseRef::SupervisorExternalIrq);
                    Serial8250RuntimePort.Action::TransmitChars;
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    Serial8250RuntimePort.Action::HandleInterrupt(InterruptCauseRef::SupervisorExternalIrq);
                    Serial8250RuntimePort.Action::TransmitChars;
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                }

                ensures {
                    serial8250_console_long_burst_irq_tx_probe_ready(
                        Serial8250ConsoleLongBurstIrqTxProbe
                    );
                    serial8250_console_long_burst_irq_tx_probe_production_side(
                        Serial8250ConsoleLongBurstIrqTxProbe
                    );
                    serial8250_console_long_burst_irq_tx_probe_kunit_not_stimulus(
                        Serial8250ConsoleLongBurstIrqTxProbe
                    );
                    serial8250_console_long_burst_irq_tx_probe_uses_printk_frontend(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_burst_irq_tx_probe_submits_multiple_records(
                        Serial8250ConsoleLongBurstIrqTxProbe
                    );
                    serial8250_console_long_burst_irq_tx_probe_observes_tx_load_size(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_console_long_burst_irq_tx_probe_each_record_exceeds_single_load(
                        Serial8250ConsoleLongBurstIrqTxProbe
                    );
                    serial8250_console_long_burst_irq_tx_probe_kicks_thri(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_burst_irq_tx_probe_observes_plic_claim(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Plic
                    );
                    serial8250_console_long_burst_irq_tx_probe_observes_irq_dispatch(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        IrqHandlerRegistry
                    );
                    serial8250_console_long_burst_irq_tx_probe_observes_runtime_handler(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_console_long_burst_irq_tx_probe_observes_multiple_irq_rounds(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Plic
                    );
                    serial8250_console_long_burst_irq_tx_probe_observes_tx_load_budget(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_console_long_burst_irq_tx_probe_drains_console_tx_queue(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_burst_irq_tx_probe_observes_plic_complete(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Plic
                    );
                    serial8250_console_long_burst_irq_tx_probe_observes_plic_loop_exit(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Plic
                    );
                    serial8250_console_long_burst_irq_tx_probe_queue_empty_after_irq(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_burst_irq_tx_probe_write_count_matched(
                        Serial8250ConsoleLongBurstIrqTxProbe
                    );
                    serial8250_console_long_burst_irq_tx_probe_drain_count_matched(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_burst_irq_tx_probe_last_byte_matched(
                        Serial8250ConsoleLongBurstIrqTxProbe
                    );
                    serial8250_console_long_burst_irq_tx_probe_local_irq_guard_observed(
                        Serial8250ConsoleLongBurstIrqTxProbe
                    );
                    serial8250_console_long_burst_irq_tx_probe_no_overflow(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        Serial8250Console
                    );
                    serial8250_console_long_burst_irq_tx_probe_does_not_mutate_tty_xmit_fifo(
                        Serial8250ConsoleLongBurstIrqTxProbe,
                        TtyXmitFifo
                    );
                    serial8250_runtime_port_transmit_chars_uses_tx_load_size(Serial8250RuntimePort);
                    serial8250_runtime_port_transmit_chars_keeps_thri_when_queue_nonempty(
                        Serial8250RuntimePort
                    );
                    serial8250_runtime_port_transmit_chars_stops_thri_when_empty(
                        Serial8250RuntimePort
                    );
                }
            }
        }
    }

    state State::Ready {
    }
}

/*
 * Serial8250ConsoleTxQuiesceProbe is the post-load stability boundary for
 * printk console TX. It does not submit another printk record and does not
 * call any backend handler. It only observes that, after the long-burst probe
 * drained the queue and stopped THRI, a bounded idle window produces no extra
 * PLIC claim, IRQ dispatch, UART handler drain, printk write or ordinary TTY
 * xmit FIFO mutation.
 */
object Serial8250ConsoleTxQuiesceProbe: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Serial8250ConsoleLongBurstIrqTxProbe.state == State::Ready;
                    Serial8250Console.state == State::Online;
                    Plic.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                }

                ensures {
                    serial8250_console_tx_quiesce_probe_ready(Serial8250ConsoleTxQuiesceProbe);
                    serial8250_console_tx_quiesce_probe_production_side(
                        Serial8250ConsoleTxQuiesceProbe
                    );
                    serial8250_console_tx_quiesce_probe_kunit_not_stimulus(
                        Serial8250ConsoleTxQuiesceProbe
                    );
                    serial8250_console_tx_quiesce_probe_does_not_write_printk(
                        Serial8250ConsoleTxQuiesceProbe
                    );
                    serial8250_console_tx_quiesce_probe_observes_queue_empty(
                        Serial8250ConsoleTxQuiesceProbe,
                        Serial8250Console
                    );
                    serial8250_console_tx_quiesce_probe_observes_thri_stopped(
                        Serial8250ConsoleTxQuiesceProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_console_tx_quiesce_probe_no_spurious_plic_claim(
                        Serial8250ConsoleTxQuiesceProbe,
                        Plic
                    );
                    serial8250_console_tx_quiesce_probe_no_spurious_irq_dispatch(
                        Serial8250ConsoleTxQuiesceProbe,
                        IrqHandlerRegistry
                    );
                    serial8250_console_tx_quiesce_probe_no_uart_tx_drain(
                        Serial8250ConsoleTxQuiesceProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_console_tx_quiesce_probe_no_tty_xmit_mutation(
                        Serial8250ConsoleTxQuiesceProbe,
                        TtyXmitFifo
                    );
                    serial8250_console_tx_quiesce_probe_no_overflow(
                        Serial8250ConsoleTxQuiesceProbe,
                        Serial8250Console
                    );
                    serial8250_console_tx_queue_empty_after_irq(Serial8250Console);
                    serial8250_runtime_port_transmit_chars_stops_thri_when_empty(
                        Serial8250RuntimePort
                    );
                }
            }
        }
    }

    state State::Ready {
    }
}

/*
 * ConsoleRegistry models printk's register_console() policy surface. It owns
 * boot/real console membership, preferred-console selection, route switching,
 * boot pending record flush, legacy earlycon drain blocking and keep_bootcon.
 */
object ConsoleRegistry: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootConsole.state == State::Online;
                    PrintkBuffer.state == State::Prepared;
                }

                ensures {
                    console_registry_register_console_api_ready(ConsoleRegistry);
                    console_registry_has_boot_console(ConsoleRegistry, BootConsole);
                    console_registry_keep_bootcon_policy_ready(ConsoleRegistry);
                    console_registry_printk_route_boot_console(ConsoleRegistry, BootConsole);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            console_registry_register_console_api_ready(ConsoleRegistry);
            console_registry_has_boot_console(ConsoleRegistry, BootConsole);
            console_registry_keep_bootcon_policy_ready(ConsoleRegistry);
            console_registry_printk_route_boot_console(ConsoleRegistry, BootConsole);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    BootConsole.state == State::Online;
                    Serial8250Console.state == State::Ready;
                    PrintkBuffer.state == State::Ready;
                    DeviceTree.state == State::Ready;
                }

                ensures {
                    console_registry_ready(ConsoleRegistry);
                    console_registry_register_console_api_ready(ConsoleRegistry);
                    console_registry_has_boot_console(ConsoleRegistry, BootConsole);
                    console_registry_has_real_console(ConsoleRegistry, Serial8250Console);
                    console_registry_preferred_console_from_stdout_path(ConsoleRegistry, DeviceTree);
                    console_registry_keep_bootcon_policy_ready(ConsoleRegistry);
                    console_registry_keep_bootcon_disabled(ConsoleRegistry);
                    console_registry_printk_route_real_console(ConsoleRegistry, Serial8250Console);
                    console_registry_flushes_boot_pending_before_serial_handoff(ConsoleRegistry);
                    console_registry_blocks_legacy_earlycon_drain_after_handoff(ConsoleRegistry);
                }
            }

        }

        actions {
            Action::RegisterNonStdoutConsole<C: ConsoleObject>(candidate: C) {
                state_effect: StateEffect::None;
                depends_on {
                    ConsoleRegistry.state == State::Prepared;
                    BootConsole.state == State::Online;
                    DeviceTree.state == State::Ready;
                    console_registry_register_console_api_ready(ConsoleRegistry);
                    console_candidate_non_stdout_path(candidate, DeviceTree);
                    console_candidate_not_platform_topology_mutating(candidate);
                }

                ensures {
                    console_registry_non_stdout_registration_rejected(ConsoleRegistry, candidate);
                    console_registry_non_stdout_preserves_boot_route(ConsoleRegistry, BootConsole, candidate);
                    console_registry_non_stdout_leaves_real_console_unchanged(ConsoleRegistry, candidate);
                    console_registry_non_stdout_no_handoff_committed(ConsoleRegistry, candidate);
                    console_registry_printk_route_boot_console(ConsoleRegistry, BootConsole);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            console_registry_ready(ConsoleRegistry);
            console_registry_register_console_api_ready(ConsoleRegistry);
            console_registry_has_real_console(ConsoleRegistry, Serial8250Console);
            console_registry_preferred_console_from_stdout_path(ConsoleRegistry, DeviceTree);
            console_registry_keep_bootcon_policy_ready(ConsoleRegistry);
            console_registry_keep_bootcon_disabled(ConsoleRegistry);
            console_registry_printk_route_real_console(ConsoleRegistry, Serial8250Console);
            console_registry_flushes_boot_pending_before_serial_handoff(ConsoleRegistry);
            console_registry_blocks_legacy_earlycon_drain_after_handoff(ConsoleRegistry);
        }

        actions {
            Action::RegisterPreferredConsoleAgain<C: ConsoleObject>(console: C) {
                state_effect: StateEffect::None;
                depends_on {
                    ConsoleRegistry.state == State::Ready;
                    console_registry_has_real_console(ConsoleRegistry, console);
                    console_registry_preferred_console_from_stdout_path(ConsoleRegistry, DeviceTree);
                    console_registry_printk_route_real_console(ConsoleRegistry, console);
                }

                ensures {
                    console_registry_duplicate_preferred_registration_idempotent(ConsoleRegistry, console);
                    console_registry_has_real_console(ConsoleRegistry, console);
                    console_registry_preferred_console_from_stdout_path(ConsoleRegistry, DeviceTree);
                    console_registry_printk_route_real_console(ConsoleRegistry, console);
                }
            }
        }
    }

}

/*
 * KeepBootconConsoleRegistry is the same registry policy with keep_bootcon
 * enabled. BootConsole remains registered and online, while the active printk
 * route still moves to Serial8250Console.
 */
object KeepBootconConsoleRegistry: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootConsole.state == State::Online;
                    PrintkBuffer.state == State::Prepared;
                }

                ensures {
                    console_registry_register_console_api_ready(KeepBootconConsoleRegistry);
                    console_registry_has_boot_console(KeepBootconConsoleRegistry, BootConsole);
                    console_registry_keep_bootcon_policy_ready(KeepBootconConsoleRegistry);
                    console_registry_printk_route_boot_console(KeepBootconConsoleRegistry, BootConsole);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            console_registry_register_console_api_ready(KeepBootconConsoleRegistry);
            console_registry_has_boot_console(KeepBootconConsoleRegistry, BootConsole);
            console_registry_keep_bootcon_policy_ready(KeepBootconConsoleRegistry);
            console_registry_printk_route_boot_console(KeepBootconConsoleRegistry, BootConsole);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    BootConsole.state == State::Online;
                    Serial8250Console.state == State::Ready;
                    PrintkBuffer.state == State::Ready;
                    DeviceTree.state == State::Ready;
                }

                ensures {
                    console_registry_ready(KeepBootconConsoleRegistry);
                    console_registry_register_console_api_ready(KeepBootconConsoleRegistry);
                    console_registry_has_boot_console(KeepBootconConsoleRegistry, BootConsole);
                    console_registry_has_real_console(KeepBootconConsoleRegistry, Serial8250Console);
                    console_registry_preferred_console_from_stdout_path(KeepBootconConsoleRegistry, DeviceTree);
                    console_registry_keep_bootcon_policy_ready(KeepBootconConsoleRegistry);
                    console_registry_keep_bootcon_enabled(KeepBootconConsoleRegistry);
                    console_registry_printk_route_real_console(KeepBootconConsoleRegistry, Serial8250Console);
                    console_registry_flushes_boot_pending_before_serial_handoff(KeepBootconConsoleRegistry);
                    console_registry_blocks_legacy_earlycon_drain_after_handoff(KeepBootconConsoleRegistry);
                    boot_console_kept_by_policy(BootConsole);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            console_registry_ready(KeepBootconConsoleRegistry);
            console_registry_register_console_api_ready(KeepBootconConsoleRegistry);
            console_registry_has_boot_console(KeepBootconConsoleRegistry, BootConsole);
            console_registry_has_real_console(KeepBootconConsoleRegistry, Serial8250Console);
            console_registry_preferred_console_from_stdout_path(KeepBootconConsoleRegistry, DeviceTree);
            console_registry_keep_bootcon_policy_ready(KeepBootconConsoleRegistry);
            console_registry_keep_bootcon_enabled(KeepBootconConsoleRegistry);
            console_registry_printk_route_real_console(KeepBootconConsoleRegistry, Serial8250Console);
            console_registry_flushes_boot_pending_before_serial_handoff(KeepBootconConsoleRegistry);
            console_registry_blocks_legacy_earlycon_drain_after_handoff(KeepBootconConsoleRegistry);
            boot_console_kept_by_policy(BootConsole);
        }
    }
}

/*
 * ConsoleHandoff is the transaction produced by registering the preferred real
 * console. It commits route switch and backend shutdown facts; it must not be
 * reimplemented as driver-private state.
 */
object ConsoleHandoff: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ConsoleRegistry.state == State::Ready;
                    BootConsole.state == State::Online;
                    Serial8250Console.state == State::Ready;
                    console_registry_keep_bootcon_disabled(ConsoleRegistry);
                }

                drives {
                    BootConsole.Transition::Disable;
                    EarlyCon.Transition::Disable;
                }

                ensures {
                    console_handoff_ready(ConsoleHandoff, BootConsole, Serial8250Console);
                    console_handoff_triggered_by_register_console(ConsoleHandoff, ConsoleRegistry);
                    console_handoff_boot_console_unregistered(ConsoleHandoff, BootConsole);
                    console_handoff_printk_route_switched(ConsoleHandoff, ConsoleRegistry, Serial8250Console);
                    serial8250_console_online_trace_emitted(Serial8250Console);
                    boot_console_offline_trace_emitted(BootConsole);
                    earlycon_backend_disabled_after_handoff(EarlyCon, ConsoleRegistry);
                    earlycon_backend_access_panics_after_handoff(EarlyCon);
                    earlycon_offline_trace_emitted(EarlyCon);
                    printk_frontend_only_for_payload_smoke(PayloadPhase, ConsoleRegistry);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootConsole.state == State::Offline;
            Serial8250Console.state == State::Ready;
            ConsoleRegistry.state == State::Ready;
            console_registry_keep_bootcon_disabled(ConsoleRegistry);
            console_handoff_ready(ConsoleHandoff, BootConsole, Serial8250Console);
            console_handoff_triggered_by_register_console(ConsoleHandoff, ConsoleRegistry);
            console_handoff_boot_console_unregistered(ConsoleHandoff, BootConsole);
            console_handoff_printk_route_switched(ConsoleHandoff, ConsoleRegistry, Serial8250Console);
            serial8250_console_online_trace_emitted(Serial8250Console);
            boot_console_offline_trace_emitted(BootConsole);
            earlycon_backend_disabled_after_handoff(EarlyCon, ConsoleRegistry);
            earlycon_backend_access_panics_after_handoff(EarlyCon);
            earlycon_offline_trace_emitted(EarlyCon);
            printk_frontend_only_for_payload_smoke(PayloadPhase, ConsoleRegistry);
        }
    }
}

object KeepBootconConsoleHandoff: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KeepBootconConsoleRegistry.state == State::Ready;
                    BootConsole.state == State::Online;
                    Serial8250Console.state == State::Ready;
                    console_registry_keep_bootcon_enabled(KeepBootconConsoleRegistry);
                }

                ensures {
                    console_handoff_ready(KeepBootconConsoleHandoff, BootConsole, Serial8250Console);
                    console_handoff_triggered_by_register_console(
                        KeepBootconConsoleHandoff,
                        KeepBootconConsoleRegistry
                    );
                    console_handoff_boot_console_retained_by_keep_bootcon(KeepBootconConsoleHandoff, BootConsole);
                    console_handoff_printk_route_switched(
                        KeepBootconConsoleHandoff,
                        KeepBootconConsoleRegistry,
                        Serial8250Console
                    );
                    serial8250_console_online_trace_emitted(Serial8250Console);
                    boot_console_kept_by_policy(BootConsole);
                    console_registry_has_boot_console(KeepBootconConsoleRegistry, BootConsole);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootConsole.state == State::Online;
            Serial8250Console.state == State::Ready;
            KeepBootconConsoleRegistry.state == State::Ready;
            console_registry_keep_bootcon_enabled(KeepBootconConsoleRegistry);
            console_handoff_ready(KeepBootconConsoleHandoff, BootConsole, Serial8250Console);
            console_handoff_triggered_by_register_console(
                KeepBootconConsoleHandoff,
                KeepBootconConsoleRegistry
            );
            console_handoff_boot_console_retained_by_keep_bootcon(KeepBootconConsoleHandoff, BootConsole);
            console_handoff_printk_route_switched(
                KeepBootconConsoleHandoff,
                KeepBootconConsoleRegistry,
                Serial8250Console
            );
            serial8250_console_online_trace_emitted(Serial8250Console);
            boot_console_kept_by_policy(BootConsole);
            console_registry_has_boot_console(KeepBootconConsoleRegistry, BootConsole);
        }
    }
}
