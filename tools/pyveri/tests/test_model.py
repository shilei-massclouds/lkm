from __future__ import annotations

import unittest
from pathlib import Path

from pyveri.model import Severity, build_model
from pyveri.parser import parse_file, parse_text
from pyveri.view import (
    build_drives_view,
    build_object_view,
    build_timeline_view,
    render_dot,
    render_svg,
    render_text,
)


class ModelBuilderTests(unittest.TestCase):
    def test_build_current_entry_prelude_model(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"

        result = build_model(parse_file(spec))

        self.assertTrue(result.ok, [diag.format() for diag in result.errors])
        self.assertEqual(len(result.errors), 0)
        self.assertIn("StartupTimeline", result.model.objects)
        self.assertEqual(
            result.model.children["StartupTimeline"],
            ["PreparePhase", "BootPhase"],
        )
        self.assertEqual(result.model.objects["BootPhase"].children, ["EntryPreludePhase"])

    def test_builds_object_view(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"

        result = build_model(parse_file(spec))
        view = build_object_view(result.model)
        text = render_text(view)
        dot = render_dot(view)

        self.assertIn("StartupTimeline: TimelineObject", text)
        self.assertIn("StartupTimeline -> PreparePhase [parent]", text)
        self.assertNotIn("drives", text)
        self.assertIn('"StartupTimeline" -> "PreparePhase"', dot)
        self.assertNotIn("drives", dot)

    def test_builds_drives_view(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"

        result = build_model(parse_file(spec))
        view = build_drives_view(result.model)
        text = render_text(view)
        dot = render_dot(view)

        self.assertIn("StartupTimeline.Setup", text)
        self.assertIn("  -> PreparePhase.Setup", text)
        self.assertIn("BootPhase.Setup", text)
        self.assertIn("EntryPreludePhase.Setup", text)
        self.assertIn("rankdir=LR", dot)
        self.assertIn('"StartupTimeline.Setup" -> "PreparePhase.Setup"', dot)

    def test_builds_timeline_view(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"

        result = build_model(parse_file(spec))
        view = build_timeline_view(result.model)
        text = render_text(view)
        svg = render_svg(view)

        self.assertIn("timeline view:", text)
        self.assertIn("PreparePhase: ready (State::Ready)", text)
        self.assertIn("PreparePhase: online (State::Online)", text)
        self.assertIn("  - Riscv64.State::Online", text)
        self.assertIn("  - PhysicalMemory.State::Online", text)
        self.assertIn("EntryPreludePhase: ready (State::Ready)", text)
        self.assertIn("BootPhase: ready (State::Ready)", text)
        self.assertIn("  - RootStream.State::Prepared", text)
        self.assertIn("  - Soc.State::Prepared", text)
        self.assertIn("  - Vm.State::Ready", text)
        self.assertNotIn("StartupTimeline", text)
        self.assertIn("<svg", svg)
        self.assertIn("PreparePhase", svg)
        self.assertIn("BootPhase", svg)
        self.assertIn("EntryPreludePhase", svg)
        self.assertNotIn("StartupTimeline", svg)

    def test_reports_unknown_drive_event(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Setup -> State::Ready {
                            drives {
                                B.Event::Missing;
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }

            object B: T {
                initial_state: State::Base;

                state State::Base {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any("unknown event reference: B.Event::Missing" in diag.message for diag in result.errors)
        )

    def test_reports_unknown_state_reference(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    invariant {
                        B.state == State::Ready;
                    }
                }
            }

            object B: T {
                initial_state: State::Base;

                state State::Base {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "unknown state reference: B.state == State::Ready" in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )


if __name__ == "__main__":
    unittest.main()
