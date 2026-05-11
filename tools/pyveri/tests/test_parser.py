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

    def test_parse_current_entry_prelude_spec(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"

        document = parse_file(spec)

        object_names = {obj.name for obj in document.objects}
        self.assertIn("StartupTimeline", object_names)
        self.assertIn("PreparePhase", object_names)
        self.assertIn("BootPhase", object_names)
        self.assertIn("EntryPreludePhase", object_names)
        self.assertGreaterEqual(len(document.objects), 19)

    def test_parse_error_for_unknown_top_level_declaration(self) -> None:
        with self.assertRaises(ParseError):
            parse_text("unknown Thing;")


if __name__ == "__main__":
    unittest.main()
