/*
 * Generic initcall table model.
 *
 * The model layer describes initcall registration as an abstract relation:
 * an object registers one of its no-argument entry actions into a table level.
 * It does not require Linux's linker-section implementation. The coding layer
 * may satisfy the same model by LDS sections, generated static arrays, or a
 * dynamic registry.
 */

enum InitcallLevel {
    Early,
    Pure,
    Core,
    CoreSync,
    Postcore,
    PostcoreSync,
    Arch,
    ArchSync,
    Subsys,
    SubsysSync,
    Fs,
    FsSync,
    Rootfs,
    Device,
    DeviceSync,
    Late,
    LateSync,
}

enum InitcallRunLevel {
    Pure,
    Core,
    Postcore,
    Arch,
    Subsys,
    Fs,
    Device,
    Late,
}

type InitcallEntryPrototype {
}

type InitcallEntry {
}

predicate initcall_entry_prototype_no_args<T>(prototype: T) -> bool;
predicate initcall_entry_prototype_returns_result<T>(prototype: T) -> bool;
predicate initcall_entry_ref_ready<T>(entry: T) -> bool;
predicate initcall_entry_has_prototype<T, P>(entry: T, prototype: P) -> bool;
predicate initcall_entry_owner_bound<T, O>(entry: T, owner: O) -> bool;
predicate initcall_entry_operation_bound<T, O>(entry: T, owner: O) -> bool;
predicate initcall_entry_invoked<T>(entry: T) -> bool;
predicate initcall_entry_return_recorded<T>(entry: T) -> bool;
predicate initcall_entry_skipped_recorded<T>(entry: T) -> bool;
predicate initcall_entry_run_context_checked<T>(entry: T) -> bool;

predicate initcall_table_registration_committed<T, E>(
    table: T,
    level: InitcallLevel,
    entry: E
) -> bool;
predicate initcall_table_registration_owner_bound<T, E, O>(
    table: T,
    entry: E,
    owner: O
) -> bool;
predicate initcall_table_entry_registered<T, E>(
    table: T,
    level: InitcallLevel,
    entry: E
) -> bool;
predicate initcall_table_registered_entries_collected<T>(table: T) -> bool;
predicate initcall_table_level_mapping_ready<T>(table: T) -> bool;
predicate initcall_table_run_levels_ready<T>(table: T) -> bool;
predicate initcall_table_entry_operation_bindings_ready<T>(table: T) -> bool;
predicate initcall_table_static_ranges_ready<T, S>(table: T, static_objects: S) -> bool;
predicate initcall_table_level_count_ready<T>(table: T) -> bool;
predicate initcall_table_all_levels_ran<T>(table: T) -> bool;
predicate initcall_table_entries_recorded_as_properties<T>(table: T) -> bool;
predicate initcall_command_line_scratch_reused_per_level<T, C>(
    table: T,
    command_line: C
) -> bool;
predicate initcall_param_parser_applied<T>(table: T) -> bool;
predicate initcall_filter_applied<T>(table: T) -> bool;
predicate initcall_run_context_checked<T>(table: T) -> bool;

/*
 * InitcallTableType is reusable table behavior. Object instances still use
 * lifecycle wrappers where the current checker requires explicit instance
 * transitions.
 */
type InitcallTableType: KernelObject {
    lifecycle {
        /*
         * Preset collects entries registered by owner objects into a table.
         * The source may be LDS sections, generated static data, or a dynamic
         * registry; model semantics only require the registration relation.
         */
        Event::Preset {
            state_effect: StateEffect::Always;
            ensures {
                initcall_table_registered_entries_collected(self);
                initcall_table_level_mapping_ready(self);
                initcall_table_run_levels_ready(self);
                initcall_table_entry_operation_bindings_ready(self);
                initcall_table_level_count_ready(self);
                initcall_table_entries_recorded_as_properties(self);
            }
        }

        /*
         * Setup corresponds to executing the collected table. For Linux-like
         * boot this is do_initcalls(); early initcalls are driven separately by
         * do_pre_smp_initcalls().
         */
        Event::Setup {
            state_effect: StateEffect::Always;
            ensures {
                initcall_table_all_levels_ran(self);
                initcall_command_line_scratch_reused_per_level(self, SavedCommandLine);
                initcall_param_parser_applied(self);
                initcall_filter_applied(self);
                initcall_run_context_checked(self);
            }
        }
    }

    processes {
        /*
         * Register is an abstract model action. It is intentionally not tied
         * to a runtime call site; object Preset events use it to declare the
         * entry they contribute to this table.
         */
        Action::Register(level: InitcallLevel, entry: InitcallEntry) {
            state_effect: StateEffect::None;
            depends_on {
                initcall_entry_ref_ready(entry);
            }
            ensures {
                initcall_table_registration_committed(self, level, entry);
                initcall_table_entry_registered(self, level, entry);
            }
        }
    }
}
