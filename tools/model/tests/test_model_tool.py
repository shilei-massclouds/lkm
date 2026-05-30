from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from common import MODEL_SCHEMA, MODEL_VERSION, read_json
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main
from parse_tool.parser import _read_with_includes


class ModelToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "entry-prelude-object-model.spec"
        )

    def test_model_writes_model_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "entry-prelude-object-model.ast.json"
            model = Path(tmp) / "entry-prelude-object-model.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0)
            data = read_json(model)
            self.assertEqual(data["schema"], MODEL_SCHEMA)
            self.assertEqual(data["version"], MODEL_VERSION)
            self.assertTrue(data["summary"]["ok"])
            self.assertGreaterEqual(data["summary"]["objects"], 25)
            self.assertEqual(data["summary"]["errors"], 0)
            self.assertIn("StartupTimeline", data["model"]["objects"])

    def test_model_json_contains_indexed_children_and_events(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "entry-prelude-object-model.ast.json"
            model = Path(tmp) / "entry-prelude-object-model.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            data = read_json(model)
            objects = data["model"]["objects"]
            startup = objects["StartupTimeline"]
            setup = startup["states"]["Base"]["events"]["Setup"]
            event_stream = objects["EventStream"]
            event_preset = event_stream["states"]["Base"]["events"]["Preset"]

            self.assertEqual(startup["children"], ["PreparePhase", "BootPhase", "PayloadPhase"])
            self.assertEqual(setup["source_state"], "Base")
            self.assertEqual(setup["target_state"], "Ready")
            self.assertEqual(
                event_preset["ensures"][0]["entries"][0]["text"],
                "Riscv64.stvec == phys_addr(StaticObjects.early_event_entry)",
            )
            self.assertEqual(
                objects["PhysicalMemory"]["properties"]["access"],
                "Access::ReadOnly",
            )
            self.assertEqual(
                [entry["text"] for entry in setup["drives"][0]["entries"]],
                [
                    "PreparePhase.Event::Setup",
                    "PreparePhase.Event::Enable",
                    "BootPhase.Event::Setup",
                    "PayloadPhase.Event::Setup",
                    "PayloadPhase.Event::Enable",
                ],
            )

    def test_model_json_preserves_entry_spans(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "entry-prelude-object-model.ast.json"
            model = Path(tmp) / "entry-prelude-object-model.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            data = read_json(model)
            kernel_image = data["model"]["objects"]["KernelImage"]
            enable = kernel_image["states"]["Ready"]["events"]["Enable"]
            entry = enable["depends_on"][0]["entries"][0]

            self.assertEqual(entry["text"], "EarlyVm.state == State::Online")
            expanded = _read_with_includes(self.spec, seen=set(), stack=[]).splitlines()
            line = expanded[entry["span"]["start_line"] - 1]
            self.assertIn("EarlyVm.state == State::Online", line)

    def test_invalid_ast_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "bad.ast.json"
            model = Path(tmp) / "bad.model.json"
            ast.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: invalid AST JSON", stderr.getvalue())

    def test_duplicate_object_event_names_fail_model_stage(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Enable -> State::Ready {
                        }
                    }
                }

                state State::Ready {
                    events {
                        on Event::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "duplicate-event.spec"
            ast = Path(tmp) / "duplicate-event.ast.json"
            model = Path(tmp) / "duplicate-event.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "duplicate object event declaration: A.Event::Enable",
                stderr.getvalue(),
            )

    def test_lifecycle_names_outside_controlled_sets_fail_model_stage(self) -> None:
        source = """
            object A: T {
                initial_state: State::Reserved;

                state State::Reserved {
                    events {
                        on Event::Activate -> State::Done {
                        }
                    }
                }

                state State::Done {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "bad-lifecycle-name.spec"
            ast = Path(tmp) / "bad-lifecycle-name.ast.json"
            model = Path(tmp) / "bad-lifecycle-name.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            errors = stderr.getvalue()
            self.assertIn("A.initial_state State::Reserved", errors)
            self.assertIn("A.State::Reserved", errors)
            self.assertIn("A.Event::Activate", errors)
            self.assertIn("A.Event::Activate -> State::Done", errors)

    def test_disable_and_offline_lifecycle_names_and_transitions_are_allowed(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Setup -> State::Ready {
                        }
                    }
                }

                state State::Ready {
                    events {
                        on Event::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                    events {
                        on Event::Disable -> State::Offline {
                        }
                    }
                }

                state State::Offline {
                    events {
                        on Event::Cleanup -> State::Destroyed {
                        }
                    }
                }

                state State::Destroyed {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "offline-lifecycle.spec"
            ast = Path(tmp) / "offline-lifecycle.ast.json"
            model = Path(tmp) / "offline-lifecycle.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_lifecycle_transitions_outside_controlled_table_fail_model_stage(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "bad-lifecycle-transition.spec"
            ast = Path(tmp) / "bad-lifecycle-transition.ast.json"
            model = Path(tmp) / "bad-lifecycle-transition.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "invalid lifecycle transition: A.State::Base.Event::Enable -> State::Online",
                stderr.getvalue(),
            )

    def test_disable_from_base_to_offline_fails_model_stage(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Disable -> State::Offline {
                        }
                    }
                }

                state State::Offline {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "bad-disable-transition.spec"
            ast = Path(tmp) / "bad-disable-transition.ast.json"
            model = Path(tmp) / "bad-disable-transition.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "invalid lifecycle transition: A.State::Base.Event::Disable -> State::Offline",
                stderr.getvalue(),
            )

    def test_model_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "model_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
