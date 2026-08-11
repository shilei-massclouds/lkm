/* Per-CPU Scheduler / Linux struct rq semantics. */

enum PrevDisposition {
    Runnable,
    Blocked,
}

enum SchedulerQueueRuntimeState {
    None,
    Some,
}

enum SchedClassKind {
    Stop,
    Deadline,
    Realtime,
    Fair,
    Idle,
}

enum SchedulerInboxMessageKind { Activate, Wake }
type SchedulerInboxOrdinal { }

type SchedulerRef {
}

type SchedClassRef {
}

predicate cpu_owns_scheduler<C: CPU, S>(cpu: C, scheduler: S) -> bool;
predicate scheduler_ref_targets<R: SchedulerRef, S>(scheduler_ref: R, scheduler: S) -> bool;
predicate scheduler_ref_ready<R: SchedulerRef>(scheduler_ref: R) -> bool;
predicate scheduler_ref_cpu_is<R: SchedulerRef, C: CpuRef>(scheduler_ref: R, cpu_ref: C) -> bool;
predicate current_scheduler_ref_private_to_cpu<R: SchedulerRef, C>(scheduler_ref: R, current_cpu: C) -> bool;
predicate current_scheduler_ref_from_current_task<R: SchedulerRef, C, T: TaskRef, F: TaskFlow>(
    scheduler_ref: R,
    current_cpu: C,
    current_task_ref: T,
    current_flow: F
) -> bool;
predicate scheduler_possible_cpu_inventory_ready<S, G>(scheduler: S, cpu_group: G) -> bool;
predicate scheduler_ready_after_sched_init<S>(scheduler: S) -> bool;
predicate scheduler_online_with_owner_cpu<S, C>(scheduler: S, cpu: C) -> bool;
predicate scheduler_ap_online_only_at_cpu_online_handoff<S, C>(scheduler: S, cpu: C) -> bool;
predicate scheduler_cpu_local_lock_ready<S>(scheduler: S) -> bool;
predicate scheduler_curr_ref_is<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_idle_ref_is<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_stop_ref_is<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_class_queue_ready<S>(scheduler: S, class: SchedClassKind) -> bool;
predicate scheduler_class_priority_order_is_linux<S>(scheduler: S) -> bool;
predicate scheduler_class_queues_store_only_task_refs<S>(scheduler: S) -> bool;
predicate scheduler_runqueue_capacity_covers_all_deliverable_tasks<S>(scheduler: S) -> bool;
predicate scheduler_root_domains_are_shared_separate_objects<S>(scheduler: S) -> bool;
predicate scheduler_fairness_bandwidth_migration_deferred<S>(scheduler: S) -> bool;
predicate scheduler_scx_trimmed_for_reference_config<S>(scheduler: S) -> bool;
predicate scheduler_inbound_inbox_cpu_local<S>(scheduler: S) -> bool;
predicate scheduler_inbox_capacity_covers_all_deliverable_tasks<S>(scheduler: S) -> bool;
predicate scheduler_inbox_one_pending_notice_per_task<S>(scheduler: S) -> bool;
predicate scheduler_inbox_duplicate_notice_coalesced<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_inbox_reservation_ready<S, T: TaskRef, C: CpuRef>(scheduler: S, task_ref: T, target_cpu: C) -> bool;
predicate scheduler_inbox_reservation_cancelled_without_residue<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_inbox_message_published_release<S, T: TaskRef, C: CpuRef>(
    scheduler: S,
    task_ref: T,
    target_cpu: C,
    ordinal: SchedulerInboxOrdinal
) -> bool;
predicate scheduler_reschedule_ipi_requested_after_release<S, C: CpuRef>(
    scheduler: S,
    target_cpu: C
) -> bool;
predicate scheduler_inbox_message_target_and_generation_valid<S, T: TaskRef, C: CpuRef>(
    scheduler: S,
    task_ref: T,
    target_cpu: C
) -> bool;
predicate scheduler_inbox_ordinal_fresh<S>(scheduler: S, ordinal: SchedulerInboxOrdinal) -> bool;
predicate scheduler_inbox_message_consumed_once<S>(scheduler: S, ordinal: SchedulerInboxOrdinal) -> bool;
predicate scheduler_inbox_rejects_stale_wrong_target_or_duplicate<S>(scheduler: S) -> bool;
predicate scheduler_need_resched_set<S>(scheduler: S) -> bool;
predicate scheduler_need_resched_coalesces_duplicate_ipi<S>(scheduler: S) -> bool;
predicate scheduler_idle_sleep_recheck_complete<S>(scheduler: S) -> bool;
predicate scheduler_idle_wfi_only_without_visible_work<S>(scheduler: S) -> bool;
predicate scheduler_idle_uses_common_switch_protocol<S>(scheduler: S) -> bool;
predicate scheduler_user_round_robin_cpu_local<S>(scheduler: S) -> bool;
predicate scheduler_user_task_cpu_ownership_immutable<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_user_dispatch_commits_satp_and_local_sfence<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_user_slice_is_10ms<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_need_resched_consumed_only_at_safe_boundary<S>(scheduler: S) -> bool;
predicate scheduler_user_return_work_pending<S>(scheduler: S) -> bool;
predicate scheduler_user_safe_boundary_drains_visible_inbox<S>(scheduler: S) -> bool;
predicate scheduler_user_safe_boundary_retains_unreleased_leaf<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_user_safe_boundary_saves_overlay<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_kernel_tick_remains_pending_until_user_return<S>(scheduler: S) -> bool;
predicate scheduler_no_competitor_consumes_pending_and_renews_slice<S>(scheduler: S) -> bool;
predicate scheduler_user_overlay_aba_restored<S>(scheduler: S) -> bool;
predicate scheduler_blocking_user_wait_drains_cpu_local_work<S>(scheduler: S) -> bool;
predicate scheduler_blocking_user_wait_preserves_leaf_aba<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_shared_mm_cross_cpu_deferred<S>(scheduler: S) -> bool;

predicate scheduler_clockevent_cpu_local<C>(clockevent: C) -> bool;
predicate scheduler_clockevent_muxes_cpu0_oneshot_and_scheduler<C>(clockevent: C) -> bool;
predicate scheduler_clockevent_slice_deadline_10ms<C, T: TaskRef>(clockevent: C, task_ref: T) -> bool;
predicate scheduler_clockevent_deadline_advances_from_prior<C>(clockevent: C) -> bool;
predicate scheduler_clockevent_skips_missed_periods<C>(clockevent: C) -> bool;
predicate scheduler_clockevent_hardirq_acknowledges_and_rearms<C>(clockevent: C) -> bool;
predicate scheduler_clockevent_hardirq_only_merges_need_resched<C>(clockevent: C) -> bool;
predicate scheduler_clockevent_pending_coalesced<C>(clockevent: C) -> bool;

predicate scheduler_queue_runtime_state_is<S>(scheduler: S, state: SchedulerQueueRuntimeState) -> bool;
predicate scheduler_queue_task_refs_empty<S>(scheduler: S) -> bool;
predicate scheduler_queue_task_refs_some<S>(scheduler: S) -> bool;
predicate scheduler_contains_task<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_task_on_rq_is<S, T: TaskRef>(scheduler: S, task_ref: T, on_rq: bool) -> bool;
predicate scheduler_task_class_is<S, T: TaskRef>(scheduler: S, task_ref: T, class: SchedClassKind) -> bool;
predicate scheduler_task_declares_running<T: TaskRef>(task_ref: T) -> bool;
predicate scheduler_task_declares_sleeping<T: TaskRef>(task_ref: T) -> bool;
predicate scheduler_task_pending_wake_signal_matches<T: TaskRef>(task_ref: T) -> bool;
predicate scheduler_task_has_no_pending_wake_signal<T: TaskRef>(task_ref: T) -> bool;
predicate scheduler_pending_wake_signal_consumed_once<T: TaskRef>(task_ref: T) -> bool;
predicate scheduler_terminal_sender_is_zombie<T: TaskRef>(task_ref: T) -> bool;
predicate scheduler_terminal_pending_wake_discarded<T: TaskRef>(task_ref: T) -> bool;
predicate scheduler_terminal_prev_deactivated_before_pick<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_terminal_switch_is_nonidentity<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_inbox_wake_current_sleep_becomes_pending<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_inbox_wake_blocked_task_enqueued<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_inbox_obsolete_wake_consumed_without_enqueue<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_sleep_wake_handoff_has_no_lost_window<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;

predicate scheduler_schedule_occurrence_fresh<S>(scheduler: S) -> bool;
predicate scheduler_schedule_has_no_payload<S>(scheduler: S) -> bool;
predicate scheduler_schedule_sender_is_current_fixed_flow<S>(scheduler: S) -> bool;
predicate scheduler_schedule_sender_cpu_ref_matches_owner<S>(scheduler: S) -> bool;
predicate scheduler_schedule_prev_derived_from_sender_and_current_binding<S, T: TaskRef>(
    scheduler: S,
    prev_ref: T
) -> bool;
predicate scheduler_schedule_rejects_cross_cpu_stale_or_wrong_binding<S>(scheduler: S) -> bool;
predicate scheduler_schedule_request_preserves_task_lifecycle<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_prepare_prev_precedes_pick_next<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_pick_next_precedes_class_handoff<S>(scheduler: S) -> bool;
predicate scheduler_put_prev_never_precedes_pick<S>(scheduler: S) -> bool;
predicate scheduler_schedule_next_is<S, T: TaskRef>(scheduler: S, next_ref: T) -> bool;
predicate scheduler_schedule_identity_resumes_yield_source<S, T: TaskRef>(
    scheduler: S,
    task_ref: T
) -> bool;
predicate scheduler_schedule_nonidentity_dispatches_next_task<S, T: TaskRef>(
    scheduler: S,
    task_ref: T
) -> bool;

predicate scheduler_prepare_prev_result_is<S, T: TaskRef>(
    scheduler: S,
    task_ref: T,
    disposition: PrevDisposition
) -> bool;
predicate scheduler_prepare_prev_preserves_task_lifecycle<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_prepare_prev_running_keeps_runnable<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_prepare_prev_signal_recovers_running<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_prepare_prev_block_deactivates_before_pick<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_deactivate_removes_class_membership<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_blocked_prev_not_reenqueued_by_put_prev<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_runnable_prev_class_policy_preserved<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;

predicate scheduler_pick_combined_callback_available<S>(scheduler: S) -> bool;
predicate scheduler_pick_fallback_available<S>(scheduler: S) -> bool;
predicate scheduler_pick_callback_and_fallback_equivalent<S>(scheduler: S) -> bool;
predicate scheduler_pick_task_done<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_put_prev_task_done<S, T: TaskRef, U: TaskRef>(scheduler: S, prev_ref: T, next_ref: U) -> bool;
predicate scheduler_set_next_task_done<S, T: TaskRef>(scheduler: S, next_ref: T) -> bool;
predicate scheduler_identity_class_bookkeeping_callback_defined<S>(scheduler: S) -> bool;

predicate scheduler_switch_preflight_complete<S, T: TaskRef, U: TaskRef>(
    scheduler: S,
    prev_ref: T,
    next_ref: U
) -> bool;
predicate scheduler_next_dispatch_preflight_complete<S, T: TaskRef, F: TaskFlow>(
    scheduler: S,
    task_ref: T,
    flow: F
) -> bool;
predicate scheduler_preflight_dispatch_flow_is<T: TaskRef, F: TaskFlow>(task_ref: T, flow: F) -> bool;
predicate scheduler_preflight_dispatch_refs_stable<S, T: TaskRef, F: TaskFlow>(scheduler: S, task_ref: T, flow: F) -> bool;
predicate scheduler_fixed_flow_preflight_valid<S, T: TaskRef, F: TaskFlow>(scheduler: S, task_ref: T, flow: F) -> bool;
predicate scheduler_dispatch_flow_generation_valid<S, F: TaskFlow>(scheduler: S, flow: F) -> bool;
predicate scheduler_dispatch_signal_capacity_ready<S, F: TaskFlow>(scheduler: S, flow: F) -> bool;
predicate scheduler_optional_root_trap_preflight_valid<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_active_trap_leaf_generation_valid<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_trap_leaf_context_epoch_matches<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;
predicate scheduler_task_does_not_forward_flow_dispatch<T: TaskRef>(task_ref: T) -> bool;
predicate scheduler_switch_signal_capacity_preflight_complete<S, T: TaskRef>(scheduler: S, next_ref: T) -> bool;
predicate scheduler_switch_order_save_suspend_restore_finish<S, T: TaskRef, U: TaskRef>(
    scheduler: S,
    prev_ref: T,
    next_ref: U
) -> bool;
predicate scheduler_switch_preserves_prev_disposition<S, T: TaskRef>(scheduler: S, prev_ref: T) -> bool;
predicate scheduler_switch_current_bindings_committed_before_dispatch<S, T: TaskRef>(scheduler: S, next_ref: T) -> bool;
predicate scheduler_switch_finish_runs_on_next_stack<S, T: TaskRef>(scheduler: S, next_ref: T) -> bool;
predicate scheduler_switch_local_interrupts_mask_intermediate_current<S, T: TaskRef, U: TaskRef>(
    scheduler: S,
    prev_ref: T,
    next_ref: U
) -> bool;
predicate scheduler_switch_local_interrupts_restore_after_next_trap_owner<S, T: TaskRef>(
    scheduler: S,
    next_ref: T
) -> bool;
predicate scheduler_identity_has_no_switch_or_task_dispatch<S, T: TaskRef>(scheduler: S, task_ref: T) -> bool;

type SchedulerClockevent: ResourceObject {
    parent: Scheduler;
    initial_state: State::Ready;

    processes {
        Action::BeginSlice(task_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(task_ref);
            }
            ensures {
                scheduler_clockevent_slice_deadline_10ms(self, task_ref);
                scheduler_clockevent_deadline_advances_from_prior(self);
                scheduler_clockevent_skips_missed_periods(self);
            }
        }

        Action::HandleTimerInterrupt {
            state_effect: StateEffect::None;
            depends_on { self.state == State::Ready; }
            ensures {
                scheduler_clockevent_hardirq_acknowledges_and_rearms(self);
                scheduler_clockevent_hardirq_only_merges_need_resched(self);
                scheduler_clockevent_pending_coalesced(self);
            }
        }
    }

    state State::Ready {
        invariant {
            scheduler_clockevent_cpu_local(self);
            scheduler_clockevent_muxes_cpu0_oneshot_and_scheduler(self);
        }
    }
}

type Scheduler: ResourceObject {
    parent: CPU;
    initial_state: State::Base;
    ext_state: SchedulerQueueRuntimeState;
    task_refs: TaskRefSet;

    associations {
        mutable curr: TaskRef;
        mutable idle: TaskRef;
        mutable stop: TaskRef;
        mutable selected_next: TaskRef;
        mutable canonical_ref: SchedulerRef;
    }

    owned {
        lock: RawSpinLock;
        clockevent: SchedulerClockevent;
    }

    processes {
        Action::PublishInbound(
            task_ref: TaskRef,
            target_cpu: CpuRef,
            ordinal: SchedulerInboxOrdinal
        ) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                task_ref_ready(task_ref);
                scheduler_inbox_ordinal_fresh(self, ordinal);
                scheduler_inbox_reservation_ready(self, task_ref, target_cpu);
            }
            ensures {
                scheduler_inbox_message_published_release(self, task_ref, target_cpu, ordinal);
                scheduler_reschedule_ipi_requested_after_release(self, target_cpu);
                scheduler_inbox_duplicate_notice_coalesced(self, task_ref);
                scheduler_inbox_rejects_stale_wrong_target_or_duplicate(self);
            }
        }

        Action::ReserveInbound(task_ref: TaskRef, target_cpu: CpuRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                task_ref_ready(task_ref);
            }
            ensures {
                scheduler_inbox_reservation_ready(self, task_ref, target_cpu);
                scheduler_inbox_one_pending_notice_per_task(self);
            }
        }

        Action::ConsumeInbound(
            task_ref: TaskRef,
            target_cpu: CpuRef,
            ordinal: SchedulerInboxOrdinal
        ) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                scheduler_inbox_message_published_release(self, task_ref, target_cpu, ordinal);
                scheduler_inbox_message_target_and_generation_valid(self, task_ref, target_cpu);
                scheduler_inbox_ordinal_fresh(self, ordinal);
            }
            drives {
                self.Action::ConsumeWakeForCurrent(task_ref) ||
                    self.Transition::EnqueueTask(task_ref) ||
                    self.Action::ConsumeObsoleteWake(task_ref);
            }
            ensures {
                scheduler_inbox_message_consumed_once(self, ordinal);
                scheduler_inbox_rejects_stale_wrong_target_or_duplicate(self);
                scheduler_sleep_wake_handoff_has_no_lost_window(self, task_ref);
            }
        }

        Action::ConsumeWakeForCurrent(task_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                task_ref == self.curr;
                scheduler_task_declares_sleeping(task_ref);
            }
            ensures {
                scheduler_task_pending_wake_signal_matches(task_ref);
                scheduler_inbox_wake_current_sleep_becomes_pending(self, task_ref);
            }
        }

        Action::ConsumeObsoleteWake(task_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                task_ref_ready(task_ref);
            }
            ensures {
                scheduler_inbox_obsolete_wake_consumed_without_enqueue(self, task_ref);
            }
        }

        Action::MarkNeedResched {
            state_effect: StateEffect::None;
            ensures {
                scheduler_need_resched_set(self);
                scheduler_need_resched_coalesces_duplicate_ipi(self);
            }
        }

        Action::UserReturnSafePoint {
            state_effect: StateEffect::None;
            sender_flow_context: true;
            depends_on {
                self.state == State::Online;
                task_ref_ready(self.curr);
                scheduler_user_return_work_pending(self);
            }
            drives {
                self.Action::Schedule || self.Action::RenewIdentitySlice;
            }
            ensures {
                scheduler_need_resched_consumed_only_at_safe_boundary(self);
                scheduler_user_safe_boundary_drains_visible_inbox(self);
                scheduler_user_safe_boundary_retains_unreleased_leaf(self, self.curr);
                scheduler_user_safe_boundary_saves_overlay(self, self.curr);
                scheduler_user_overlay_aba_restored(self);
            }
        }

        Action::BlockingUserWaitSafePoint {
            state_effect: StateEffect::None;
            sender_flow_context: true;
            depends_on {
                self.state == State::Online;
                task_ref_ready(self.curr);
                scheduler_user_return_work_pending(self);
            }
            drives {
                self.Action::UserReturnSafePoint;
            }
            ensures {
                scheduler_blocking_user_wait_drains_cpu_local_work(self);
                scheduler_blocking_user_wait_preserves_leaf_aba(self, self.curr);
            }
        }

        Action::RenewIdentitySlice {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                task_ref_ready(self.curr);
            }
            drives { self.clockevent.Action::BeginSlice(self.curr); }
            ensures {
                scheduler_no_competitor_consumes_pending_and_renews_slice(self);
            }
        }

        Action::RunIdle {
            state_effect: StateEffect::None;
            depends_on { self.state == State::Online; }
            ensures {
                scheduler_idle_sleep_recheck_complete(self);
                scheduler_idle_wfi_only_without_visible_work(self);
                scheduler_idle_uses_common_switch_protocol(self);
            }
        }

        Transition::EnqueueTask(task_ref: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready || self.state == State::Online;
                task_ref_ready(task_ref);
                task_not_enqueued(task_ref);
            }
            transitions {
                SchedulerQueueRuntimeState::None -> SchedulerQueueRuntimeState::Some;
                SchedulerQueueRuntimeState::Some -> SchedulerQueueRuntimeState::Some;
            }
            ensures {
                scheduler_queue_runtime_state_is(self, SchedulerQueueRuntimeState::Some);
                scheduler_queue_task_refs_some(self);
                scheduler_contains_task(self, task_ref);
                scheduler_task_on_rq_is(self, task_ref, true);
            }
            result {
                None: Success(first_task_enqueued);
                Some: Success(additional_task_enqueued);
                AlreadyQueued: Failed(duplicate_enqueue);
            }
        }

        Action::DeactivateTask(task_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(task_ref);
                scheduler_contains_task(self, task_ref);
            }
            ensures {
                scheduler_task_on_rq_is(self, task_ref, false);
                scheduler_deactivate_removes_class_membership(self, task_ref);
            }
        }

        Action::PreparePrev(prev_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                task_ref_ready(prev_ref);
                scheduler_schedule_prev_derived_from_sender_and_current_binding(self, prev_ref);
            }
            drives {
                self.Action::PreparePrevRunnable(prev_ref) ||
                    self.Action::PreparePrevBlocked(prev_ref);
            }
            ensures {
                scheduler_prepare_prev_preserves_task_lifecycle(self, prev_ref);
                scheduler_blocked_prev_not_reenqueued_by_put_prev(self, prev_ref);
            }
        }

        Action::PreparePrevRunnable(prev_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                scheduler_task_declares_running(prev_ref) ||
                    (scheduler_task_declares_sleeping(prev_ref) &&
                        scheduler_task_pending_wake_signal_matches(prev_ref));
            }
            drives {
                self.Action::PreparePrevStillRunning(prev_ref) ||
                    self.Action::PreparePrevSignalRecovery(prev_ref);
            }
            ensures {
                scheduler_prepare_prev_result_is(self, prev_ref, PrevDisposition::Runnable);
                scheduler_prepare_prev_preserves_task_lifecycle(self, prev_ref);
            }
        }

        Action::PreparePrevStillRunning(prev_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                scheduler_task_declares_running(prev_ref);
            }
            ensures {
                scheduler_prepare_prev_running_keeps_runnable(self, prev_ref);
            }
        }

        Action::PreparePrevSignalRecovery(prev_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                scheduler_task_declares_sleeping(prev_ref);
                scheduler_task_pending_wake_signal_matches(prev_ref);
            }
            ensures {
                scheduler_prepare_prev_signal_recovers_running(self, prev_ref);
                scheduler_pending_wake_signal_consumed_once(prev_ref);
            }
        }

        Action::PreparePrevBlocked(prev_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                scheduler_task_declares_sleeping(prev_ref);
                scheduler_task_has_no_pending_wake_signal(prev_ref);
            }
            drives {
                self.Action::DeactivateTask(prev_ref);
            }
            ensures {
                scheduler_prepare_prev_result_is(self, prev_ref, PrevDisposition::Blocked);
                scheduler_prepare_prev_block_deactivates_before_pick(self, prev_ref);
                scheduler_blocked_prev_not_reenqueued_by_put_prev(self, prev_ref);
            }
        }

        Action::PrepareTerminalPrev(prev_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                scheduler_terminal_sender_is_zombie(prev_ref);
                scheduler_task_declares_sleeping(prev_ref);
            }
            drives {
                self.Action::DeactivateTask(prev_ref);
            }
            ensures {
                scheduler_terminal_pending_wake_discarded(prev_ref);
                scheduler_prepare_prev_result_is(self, prev_ref, PrevDisposition::Blocked);
                scheduler_terminal_prev_deactivated_before_pick(self, prev_ref);
                scheduler_blocked_prev_not_reenqueued_by_put_prev(self, prev_ref);
            }
        }

        Action::PickNextTask(prev_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                task_ref_ready(prev_ref);
                scheduler_prepare_prev_result_is(self, prev_ref, PrevDisposition::Runnable) ||
                    scheduler_prepare_prev_result_is(self, prev_ref, PrevDisposition::Blocked);
            }
            drives {
                self.Action::PickTask(prev_ref);
                self.Action::PutPrevTask(prev_ref, self.selected_next);
                self.Action::SetNextTask(self.selected_next);
            }
            ensures {
                scheduler_pick_combined_callback_available(self);
                scheduler_pick_fallback_available(self);
                scheduler_pick_callback_and_fallback_equivalent(self);
                scheduler_pick_next_precedes_class_handoff(self);
                scheduler_put_prev_never_precedes_pick(self);
                scheduler_schedule_next_is(self, self.selected_next);
            }
        }

        Action::PickTask(prev_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(prev_ref);
                scheduler_prepare_prev_result_is(self, prev_ref, PrevDisposition::Runnable) ||
                    scheduler_prepare_prev_result_is(self, prev_ref, PrevDisposition::Blocked);
            }
            ensures {
                scheduler_pick_task_done(self, self.selected_next);
                scheduler_class_priority_order_is_linux(self);
            }
        }

        Action::PutPrevTask(prev_ref: TaskRef, next_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(prev_ref);
                task_ref_ready(next_ref);
                scheduler_pick_task_done(self, next_ref);
                scheduler_prepare_prev_result_is(self, prev_ref, PrevDisposition::Runnable) ||
                    scheduler_prepare_prev_result_is(self, prev_ref, PrevDisposition::Blocked);
            }
            ensures {
                scheduler_put_prev_task_done(self, prev_ref, next_ref);
                scheduler_blocked_prev_not_reenqueued_by_put_prev(self, prev_ref);
                scheduler_runnable_prev_class_policy_preserved(self, prev_ref);
            }
        }

        Action::SetNextTask(next_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(next_ref);
                scheduler_pick_task_done(self, next_ref);
            }
            ensures {
                scheduler_set_next_task_done(self, next_ref);
            }
        }

        Action::Schedule {
            state_effect: StateEffect::None;
            sender_flow_context: true;
            depends_on {
                self.state == State::Online;
                task_ref_ready(self.curr);
            }
            drives {
                self.Action::PreparePrev(self.curr);
                self.Action::PickNextTask(self.curr);
                self.Action::SwitchTo(self.curr, self.selected_next) ||
                    self.Action::ScheduleIdentity;
            }
            ensures {
                scheduler_schedule_occurrence_fresh(self);
                scheduler_schedule_has_no_payload(self);
                scheduler_schedule_sender_is_current_fixed_flow(self);
                scheduler_schedule_sender_cpu_ref_matches_owner(self);
                scheduler_schedule_prev_derived_from_sender_and_current_binding(self, self.curr);
                scheduler_schedule_rejects_cross_cpu_stale_or_wrong_binding(self);
                scheduler_schedule_request_preserves_task_lifecycle(self, self.curr);
                scheduler_prepare_prev_precedes_pick_next(self, self.curr);
            }
        }

        Action::ScheduleTerminal {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                scheduler_terminal_sender_is_zombie(self.curr);
            }
            drives {
                self.Action::PrepareTerminalPrev(self.curr);
                self.Action::PickNextTask(self.curr);
                self.Action::SwitchTo(self.curr, self.selected_next);
            }
            ensures {
                scheduler_terminal_pending_wake_discarded(self.curr);
                scheduler_terminal_prev_deactivated_before_pick(self, self.curr);
                scheduler_terminal_switch_is_nonidentity(self, self.curr);
            }
        }

        Action::ScheduleIdentity {
            state_effect: StateEffect::None;
            sender_flow_context: true;
            depends_on {
                self.state == State::Online;
                scheduler_schedule_next_is(self, self.curr);
            }
            ensures {
                scheduler_schedule_sender_is_current_fixed_flow(self);
                scheduler_schedule_identity_resumes_yield_source(self, self.curr);
                scheduler_identity_has_no_switch_or_task_dispatch(self, self.curr);
                scheduler_identity_class_bookkeeping_callback_defined(self);
            }
        }

        Action::PreflightNextDispatch(next_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(next_ref);
                task_ref_targets_online_task(next_ref);
                scheduler_fixed_flow_preflight_valid(self, next_ref, next_ref.flow);
                scheduler_dispatch_signal_capacity_ready(self, next_ref.flow);
            }
            ensures {
                scheduler_next_dispatch_preflight_complete(self, next_ref, next_ref.flow);
                scheduler_preflight_dispatch_flow_is(next_ref, next_ref.flow);
                scheduler_preflight_dispatch_refs_stable(self, next_ref, next_ref.flow);
                scheduler_dispatch_flow_generation_valid(self, next_ref.flow);
                scheduler_optional_root_trap_preflight_valid(self, next_ref);
                scheduler_active_trap_leaf_generation_valid(self, next_ref);
                scheduler_trap_leaf_context_epoch_matches(self, next_ref);
                task_flow_optional_root_preflight_valid(next_ref, next_ref.flow);
            }
        }

        Action::DispatchNext(next_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                scheduler_next_dispatch_preflight_complete(self, next_ref, next_ref.flow);
            }
            drives {
                self.clockevent.Action::BeginSlice(next_ref);
                next_ref.Transition::Dispatch;
                next_ref.flow.Action::Enter;
            }
            ensures {
                scheduler_task_does_not_forward_flow_dispatch(next_ref);
                scheduler_user_task_cpu_ownership_immutable(self, next_ref);
                scheduler_user_dispatch_commits_satp_and_local_sfence(self, next_ref);
                scheduler_user_slice_is_10ms(self, next_ref);
            }
        }

        Action::SwitchTo(prev_ref: TaskRef, next_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                task_ref_ready(prev_ref);
                task_ref_ready(next_ref);
                prev_ref != next_ref;
                scheduler_switch_preflight_complete(self, prev_ref, next_ref);
                scheduler_switch_signal_capacity_preflight_complete(self, next_ref);
            }
            drives {
                self.Action::PreflightNextDispatch(next_ref);
                prev_ref.Action::SaveCoreContext;
                prev_ref.Transition::Suspend;
                next_ref.Action::RestoreCoreContext;
                CurrentTask.Action::BindTask(next_ref, next_ref.flow);
                self.Action::FinishTaskSwitch(prev_ref, next_ref);
                self.Action::DispatchNext(next_ref);
            }
            ensures {
                scheduler_switch_order_save_suspend_restore_finish(self, prev_ref, next_ref);
                scheduler_switch_preserves_prev_disposition(self, prev_ref);
                scheduler_switch_current_bindings_committed_before_dispatch(self, next_ref);
                scheduler_schedule_nonidentity_dispatches_next_task(self, next_ref);
                scheduler_switch_local_interrupts_mask_intermediate_current(self, prev_ref, next_ref);
                scheduler_switch_local_interrupts_restore_after_next_trap_owner(self, next_ref);
            }
        }

        Action::FinishTaskSwitch(prev_ref: TaskRef, next_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                prev_ref != next_ref;
                task_ref_ready(prev_ref);
                task_ref_ready(next_ref);
            }
            ensures {
                scheduler_switch_finish_runs_on_next_stack(self, next_ref);
                scheduler_first_schedule_committed(self);
            }
        }

        Action::SelectScheduler(task_ref: TaskRef) -> SchedulerRef {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(task_ref);
            }
            ensures {
                scheduler_ref_ready(self.canonical_ref);
                scheduler_ref_targets(self.canonical_ref, self);
                scheduler_select_scheduler_returns(self, task_ref, self.canonical_ref);
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    scheduler_cpu_local_lock_ready(self);
                    scheduler_class_queue_ready(self, SchedClassKind::Stop);
                    scheduler_class_queue_ready(self, SchedClassKind::Deadline);
                    scheduler_class_queue_ready(self, SchedClassKind::Realtime);
                    scheduler_class_queue_ready(self, SchedClassKind::Fair);
                    scheduler_class_queue_ready(self, SchedClassKind::Idle);
                    scheduler_class_priority_order_is_linux(self);
                    scheduler_class_queues_store_only_task_refs(self);
                    scheduler_runqueue_capacity_covers_all_deliverable_tasks(self);
                    scheduler_root_domains_are_shared_separate_objects(self);
                    scheduler_fairness_bandwidth_migration_deferred(self);
                    scheduler_scx_trimmed_for_reference_config(self);
                    scheduler_inbound_inbox_cpu_local(self);
                    scheduler_inbox_capacity_covers_all_deliverable_tasks(self);
                    scheduler_inbox_one_pending_notice_per_task(self);
                    scheduler_user_round_robin_cpu_local(self);
                    scheduler_shared_mm_cross_cpu_deferred(self);
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    scheduler_ready_after_sched_init(self);
                    scheduler_queue_runtime_state_is(self, SchedulerQueueRuntimeState::None);
                    scheduler_queue_task_refs_empty(self);
                }
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    scheduler_online_with_owner_cpu(self, CurrentCPU);
                    scheduler_schedule_event_available(self);
                }
            }
        }
    }

    state State::Online {
        invariant {
            scheduler_ready_after_sched_init(self);
            scheduler_online_with_owner_cpu(self, CurrentCPU);
            scheduler_schedule_event_available(self);
            scheduler_class_priority_order_is_linux(self);
            scheduler_class_queues_store_only_task_refs(self);
            scheduler_runqueue_capacity_covers_all_deliverable_tasks(self);
            scheduler_inbound_inbox_cpu_local(self);
            scheduler_inbox_capacity_covers_all_deliverable_tasks(self);
            scheduler_inbox_one_pending_notice_per_task(self);
            scheduler_inbox_rejects_stale_wrong_target_or_duplicate(self);
            scheduler_user_round_robin_cpu_local(self);
            scheduler_need_resched_consumed_only_at_safe_boundary(self);
            scheduler_kernel_tick_remains_pending_until_user_return(self);
            scheduler_shared_mm_cross_cpu_deferred(self);
        }
    }
}
