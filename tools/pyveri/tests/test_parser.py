from __future__ import annotations

import unittest
from pathlib import Path

from pyveri.parser import ParseError, parse_file, parse_text, strip_comments


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
            object StartupTimeline: TimelineObject {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Setup -> State::Ready {
                            drives {
                                PreparePhase.Event::Setup;
                                BootPhase.Event::Setup;
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
        self.assertEqual(obj.name, "StartupTimeline")
        self.assertEqual(obj.kind, "TimelineObject")
        self.assertEqual(obj.initial_state, "Base")
        self.assertEqual([state.name for state in obj.states], ["Base", "Ready"])
        event = obj.states[0].events[0]
        self.assertEqual(event.name, "Setup")
        self.assertEqual(event.target_state, "Ready")
        self.assertEqual(
            event.drives[0].entries,
            [
                "PreparePhase.Event::Setup",
                "BootPhase.Event::Setup",
            ],
        )

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

    def test_parse_current_entry_prelude_spec(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"

        document = parse_file(spec)

        object_names = {obj.name for obj in document.objects}
        self.assertIn("StartupTimeline", object_names)
        self.assertIn("PreparePhase", object_names)
        self.assertIn("BootPhase", object_names)
        self.assertIn("EntryPreludePhase", object_names)
        self.assertGreaterEqual(len(document.objects), 19)

    def test_current_entry_prelude_entry_spans_use_source_lines(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"

        document = parse_file(spec)
        kernel_image = next(obj for obj in document.objects if obj.name == "KernelImage")
        ready = next(state for state in kernel_image.states if state.name == "Ready")
        enable = next(event for event in ready.events if event.name == "Enable")
        entry, span = enable.depends_on[0].entry_spans[0]

        self.assertEqual(entry, "EarlyVm.state == State::Online")
        line = spec.read_text(encoding="utf-8").splitlines()[span.start_line - 1]
        self.assertIn("EarlyVm.state == State::Online", line)

    def test_parse_error_for_unknown_top_level_declaration(self) -> None:
        with self.assertRaises(ParseError):
            parse_text("unknown Thing;")


if __name__ == "__main__":
    unittest.main()
