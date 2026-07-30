from __future__ import annotations

import unittest
from pathlib import Path

from pyveri.parser import ParseError, _read_with_includes, parse_file, parse_text, strip_comments


class ParserTests(unittest.TestCase):
    def test_strip_comments_preserves_strings(self) -> None:
        source = 'object A: T { value: "not // comment"; /* gone */ state State::Base {} }'

        stripped = strip_comments(source)

        self.assertIn('"not // comment"', stripped)
        self.assertNotIn("gone", stripped)
        self.assertIn("state State::Base", stripped)

    def test_parse_minimal_object(self) -> None:
        document = parse_text(
            """
            object Kernel: KernelObject {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            drives {
                                PreparePhase.Transition::Setup;
                                BootPhase.Transition::Setup;
                            }

                            emits {
                                Transition::Enable;
                            }

                            ensures {
                                BootPhase.state == State::Ready;
                            }
                        }
                    }
                }

                state State::Ready {
                    invariant {
                        BootPhase.state == State::Ready;
                    }
                }
            }
            """
        )

        obj = document.objects[0]
        self.assertEqual(obj.name, "Kernel")
        self.assertEqual(obj.kind, "KernelObject")
        self.assertEqual(obj.initial_state, "Base")
        self.assertEqual([state.name for state in obj.states], ["Base", "Ready"])
        transition = obj.states[0].transitions[0]
        self.assertEqual(transition.name, "Setup")
        self.assertEqual(transition.target_state, "Ready")
        self.assertEqual(
            transition.drives[0].entries,
            [
                "PreparePhase.Transition::Setup",
                "BootPhase.Transition::Setup",
            ],
        )
        self.assertEqual(transition.emits[0].entries, ["Transition::Enable"])
        self.assertEqual(
            transition.ensures[0].entries,
            ["BootPhase.state == State::Ready"],
        )

    def test_parse_type_preserves_properties_and_blocks(self) -> None:
        document = parse_text(
            """
            type Completion {
                ext_state: CompletionExtState;
                done: CompletionTokenCount;

                processes {
                    Transition::Complete {
                        state_effect: StateEffect::Conditional;
                    }
                }
            }
            """
        )

        typ = document.types[0]

        self.assertEqual(typ.name, "Completion")
        self.assertEqual(
            typ.properties,
            {
                "ext_state": "CompletionExtState",
                "done": "CompletionTokenCount",
            },
        )
        self.assertEqual([block.kind for block in typ.blocks], ["processes"])

    def test_statement_entries_keeps_less_than_comparisons_separate(self) -> None:
        document = parse_text(
            """
            object Config: PrepareObject {
                initial_state: State::Online;

                state State::Online {
                    invariant {
                        pt_size_on_stack < page_size;
                        kernel_link_addr != 0;
                    }
                }
            }
            """
        )

        entries = document.objects[0].states[0].invariants[0].entries

        self.assertEqual(
            entries,
            [
                "pt_size_on_stack < page_size",
                "kernel_link_addr != 0",
            ],
        )

    def test_statement_entry_spans_report_entry_lines(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    invariant {
                        first();
                        second();
                    }
                }
            }
            """
        )

        block = document.objects[0].states[0].invariants[0]

        self.assertEqual(
            [(entry, span.start_line) for entry, span in block.entry_spans],
            [
                ("first()", 7),
                ("second()", 8),
            ],
        )


    def test_parse_exclusive_context_and_within_block(self) -> None:
        document = parse_text(
            """
            lock TaskPiLock: RawSpinLock;

            exclusive_context WakeContext {
                lock_ref: TaskPiLock;

                obj_refs {
                    A;
                    B;
                }
            }

            object A: T {
                initial_state: State::Ready;

                state State::Ready {
                    transitions {
                        on Transition::Enable -> State::Online {
                            within WakeContext {
                                entered_by {
                                    TaskPiLock.Transition::LockIrqSave;
                                }

                                depends_on {
                                    task_state_new(A);
                                }

                                drives {
                                    A.Action::SetTaskState(TaskRuntimeState::Running);
                                    B.Action::Touch(task: A);
                                }

                                exited_by {
                                    TaskPiLock.Transition::UnlockIrqRestore;
                                }

                                ensures {
                                    task_state_running(A);
                                }
                            }
                        }
                    }
                }

                state State::Online {
                }
            }

            object B: T {
                initial_state: State::Ready;

                state State::Ready {
                }
            }
            """
        )

        self.assertEqual([lock.name for lock in document.locks], ["TaskPiLock"])
        self.assertEqual(document.locks[0].kind, "RawSpinLock")
        context = document.exclusive_contexts[0]
        self.assertEqual(context.name, "WakeContext")
        self.assertEqual(context.lock_ref, "TaskPiLock")
        self.assertEqual(context.obj_refs, ["A", "B"])
        transition = document.objects[0].states[0].transitions[0]
        self.assertEqual(transition.within[0].context, "WakeContext")
        self.assertFalse(transition.within[0].only_once)
        self.assertEqual(
            transition.within[0].entered_by[0].entries,
            ["TaskPiLock.Transition::LockIrqSave"],
        )
        self.assertEqual(
            transition.within[0].drives[0].entries,
            [
                "A.Action::SetTaskState(TaskRuntimeState::Running)",
                "B.Action::Touch(task: A)",
            ],
        )
        self.assertEqual(
            transition.within[0].exited_by[0].entries,
            ["TaskPiLock.Transition::UnlockIrqRestore"],
        )

    def test_parse_within_only_once(self) -> None:
        document = parse_text(
            """
            context BootContext: Context {
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within BootContext only-once {
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
            """
        )

        transition = document.objects[0].states[0].transitions[0]
        self.assertTrue(transition.within[0].only_once)

    def test_parse_resource_context_guard_and_effects(self) -> None:
        document = parse_text(
            """
            lock TaskPiLock: RawSpinLock;

            context WakeContext: ResourceExclusiveContext {
                guard {
                    lock_ref: TaskPiLock;

                    entered_by {
                        TaskPiLock.Transition::LockIrqSave;
                    }

                    exited_by {
                        TaskPiLock.Transition::UnlockIrqRestore;
                    }
                }

                obj_refs {
                    A;
                    B;
                }

                effects {
                    interruptible: false;
                    preemptible: false;
                    sleepable: false;
                    exclusive_refs: obj_refs;
                }
            }

            object A: T {
                initial_state: State::Ready;

                state State::Ready {
                    transitions {
                        on Transition::Enable -> State::Online {
                            within WakeContext {
                                drives {
                                    A.Action::SetTaskState(TaskRuntimeState::Running);
                                    B.Action::Touch(task: A);
                                }
                            }
                        }
                    }
                }

                state State::Online {
                }
            }

            object B: T {
                initial_state: State::Ready;

                state State::Ready {
                }
            }
            """
        )

        context = document.exclusive_contexts[0]
        self.assertEqual(context.name, "WakeContext")
        self.assertEqual(context.kind, "ResourceExclusiveContext")
        self.assertIsNotNone(context.guard)
        assert context.guard is not None
        self.assertEqual(context.guard.lock_ref, "TaskPiLock")
        self.assertEqual(context.lock_ref, "TaskPiLock")
        self.assertEqual(context.guard.entered_by[0].entries, ["TaskPiLock.Transition::LockIrqSave"])
        self.assertEqual(context.guard.exited_by[0].entries, ["TaskPiLock.Transition::UnlockIrqRestore"])
        self.assertEqual(context.obj_refs, ["A", "B"])
        self.assertEqual(context.effects[0].entries, [
            "interruptible: false",
            "preemptible: false",
            "sleepable: false",
            "exclusive_refs: obj_refs",
        ])
        transition = document.objects[0].states[0].transitions[0]
        self.assertEqual(transition.within[0].context, "WakeContext")
        self.assertEqual(transition.within[0].entered_by, [])
        self.assertEqual(transition.within[0].exited_by, [])

    def test_parse_context_guard_holds(self) -> None:
        document = parse_text(
            """
            context SingleTaskContext: Context {
                guard {
                    holds {
                        cpu_concurrency: single_cpu;
                        task_concurrency: single_task;
                        local_interrupts: disabled;
                        preemption: disabled;
                    }
                }
            }
            """
        )

        context = document.exclusive_contexts[0]
        self.assertEqual(context.name, "SingleTaskContext")
        self.assertIsNotNone(context.guard)
        assert context.guard is not None
        self.assertEqual(context.guard.entered_by, [])
        self.assertEqual(context.guard.exited_by, [])
        self.assertEqual(context.guard.holds[0].entries, [
            "cpu_concurrency: single_cpu",
            "task_concurrency: single_task",
            "local_interrupts: disabled",
            "preemption: disabled",
        ])

    def test_parse_current_model_spec(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"

        document = parse_file(spec)

        object_names = {obj.name for obj in document.objects}
        self.assertIn("Computer", object_names)
        self.assertIn("Riscv64Platform", object_names)
        self.assertIn("OpenSBI", object_names)
        self.assertIn("Kernel", object_names)
        self.assertIn("BootCpuRegisters", object_names)
        boot_args = next(obj for obj in document.objects if obj.name == "BootArgs")
        self.assertEqual(boot_args.initial_state, "Online")
        self.assertEqual(boot_args.parent, "OpenSBI")
        self.assertEqual(boot_args.properties["source"], "firmware::boot_abi")
        self.assertEqual(
            [entry for block in boot_args.attrs for entry in block.entries],
            ["boot_hartid: HartId", "dtb_pa: PhysAddr<Dtb>"],
        )
        self.assertEqual([state.name for state in boot_args.states], ["Online"])
        self.assertFalse(boot_args.states[0].transitions)
        self.assertNotIn("Boot" + "HartContext", object_names)
        self.assertNotIn("EntryPreludePhase", object_names)
        self.assertGreaterEqual(len(document.objects), 19)

    def test_current_model_entry_spans_use_expanded_lines(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"

        document = parse_file(spec)
        kernel_image = next(obj for obj in document.objects if obj.name == "KernelImage")
        ready = next(state for state in kernel_image.states if state.name == "Ready")
        enable = next(transition for transition in ready.transitions if transition.name == "Enable")
        entry, span = enable.depends_on[0].entry_spans[0]

        self.assertEqual(entry, "gp_relative_addressing_ready(KernelImage)")
        line = _read_with_includes(spec, seen=set(), stack=[])[0].splitlines()[
            span.start_line - 1
        ]
        self.assertIn("gp_relative_addressing_ready(KernelImage)", line)

    def test_parse_error_for_unknown_top_level_declaration(self) -> None:
        with self.assertRaises(ParseError):
            parse_text("unknown Thing;")


if __name__ == "__main__":
    unittest.main()
