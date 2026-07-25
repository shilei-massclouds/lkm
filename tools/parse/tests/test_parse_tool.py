from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from common import AST_SCHEMA, AST_VERSION, read_json
from parse_tool.__main__ import main
from parse_tool.parser import _read_source_with_includes, _read_with_includes


class ParseToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "model"
            / "main.spec"
        )

    def test_parse_writes_ast_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "model-main.ast.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = main([str(self.spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            self.assertEqual(data["schema"], AST_SCHEMA)
            self.assertEqual(data["version"], AST_VERSION)
            document = data["document"]
            self.assertGreaterEqual(len(document["objects"]), 19)
            computer = next(
                item for item in document["objects"] if item["name"] == "Computer"
            )
            self.assertEqual(computer["kind"], "ComputerObject")

    def test_entry_spans_are_serialized(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "model-main.ast.json"
            exit_code = main([str(self.spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            kernel_image = next(
                item for item in data["document"]["objects"] if item["name"] == "KernelImage"
            )
            ready = next(state for state in kernel_image["states"] if state["name"] == "Ready")
            enable = next(transition for transition in ready["transitions"] if transition["name"] == "Enable")
            entry = enable["depends_on"][0]["entries"][0]

            self.assertEqual(entry["text"], "EarlyVm.state == State::Online")
            expanded = _read_with_includes(self.spec, seen=set(), stack=[])[0].splitlines()
            line = expanded[entry["span"]["start_line"] - 1]
            self.assertIn("EarlyVm.state == State::Online", line)

    def test_type_lifecycle_and_parameterized_transition_are_serialized(self) -> None:
        source = """
            type Item {
            }

            type Carrier {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Preset(item: Item) -> State::Prepared {
                            depends_on { item.state == State::Ready; }
                            ensures { carrier_bound(self, item); }
                        }
                    }
                }

                state State::Prepared {
                    invariant { carrier_prepared(self); }
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "type-lifecycle.spec"
            output = Path(tmp) / "type-lifecycle.ast.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(main([str(spec), "-o", str(output)]), 0)
            carrier = read_json(output)["document"]["types"][1]
            transition = carrier["states"][0]["transitions"][0]

            self.assertEqual(carrier["initial_state"], "Base")
            self.assertEqual([state["name"] for state in carrier["states"]], ["Base", "Prepared"])
            self.assertEqual(transition["parameters"], [{"name": "item", "type": "Item"}])
            self.assertEqual(transition["target_state"], "Prepared")

    def test_within_only_once_is_serialized(self) -> None:
        source = """
            context GuardedContext: Context {
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within GuardedContext only-once {
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "only-once.spec"
            output = Path(tmp) / "only-once.ast.json"
            spec.write_text(source, encoding="utf-8")

            exit_code = main([str(spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            transition = data["document"]["objects"][0]["states"][0]["transitions"][0]
            self.assertTrue(transition["within"][0]["only_once"])

    def test_structured_boundaries_round_trip_with_include_source(self) -> None:
        child_source = """
            object IncludedBoundaryOwner: SystemObject {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            ensures {
                                deferred_fact(IncludedBoundaryOwner);
                                trimmed_fact(IncludedBoundaryOwner);
                            }

                            deferred included.001 {
                                category: DeferredCategory::Feature;
                                summary: "Complete the included feature.";
                                evidence { deferred_fact(IncludedBoundaryOwner); }
                                close_when: "The feature and its tests are complete.";
                            }
                            trimmed included.002 {
                                category: TrimmedCategory::BuildConfig;
                                summary: "The path is disabled in this build.";
                                evidence { trimmed_fact(IncludedBoundaryOwner); }
                                revisit_when: "The build enables the path.";
                            }
                        }
                    }
                }

                state State::Prepared {
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            child = root / "included.spec"
            spec = root / "root.spec"
            output = root / "root.ast.json"
            child.write_text(child_source, encoding="utf-8")
            spec.write_text('include "included.spec";\n', encoding="utf-8")

            self.assertEqual(main([str(spec), "-o", str(output)]), 0)
            data = read_json(output)
            transition = data["document"]["objects"][0]["states"][0]["transitions"][0]
            boundaries = transition["boundaries"]

            self.assertEqual([item["id"] for item in boundaries], ["included.001", "included.002"])
            self.assertEqual([item["status"] for item in boundaries], ["deferred", "trimmed"])
            self.assertEqual(boundaries[0]["category"], "DeferredCategory::Feature")
            self.assertEqual(boundaries[0]["resolution"], "The feature and its tests are complete.")
            self.assertEqual(boundaries[1]["category"], "TrimmedCategory::BuildConfig")
            self.assertEqual(boundaries[1]["resolution"], "The build enables the path.")
            self.assertEqual(boundaries[0]["evidence"][0]["entries"][0]["text"], "deferred_fact(IncludedBoundaryOwner)")
            self.assertTrue(
                boundaries[0]["span"]["source_file"].endswith("included.spec")
            )
            self.assertGreater(boundaries[0]["span"]["source_line"], 0)

    def test_event_body_members_preserve_source_order(self) -> None:
        source = """
            context GuardedContext: Context {
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            drives {
                                B.Transition::Setup;
                            }

                            within GuardedContext {
                                drives {
                                    C.Transition::Setup;
                                }
                            }

                            emits {
                                Transition::Enable;
                            }

                            drives {
                                D.Transition::Setup;
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "ordered-body.spec"
            output = Path(tmp) / "ordered-body.ast.json"
            spec.write_text(source, encoding="utf-8")

            exit_code = main([str(spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            transition = data["document"]["objects"][0]["states"][0]["transitions"][0]
            self.assertEqual(
                [member["kind"] for member in transition["body_members"]],
                ["drives", "within", "emits", "drives"],
            )
            self.assertEqual(
                transition["body_members"][1]["within"]["drives"][0]["entries"][0]["text"],
                "C.Transition::Setup",
            )
            self.assertEqual(
                transition["emits"][0]["entries"][0]["text"],
                "Transition::Enable",
            )

    def test_declare_statements_preserve_source_order_and_site_ordinals(self) -> None:
        source = """
            type Item {
                processes {
                    Transition::Preset {
                    }
                }
            }

            object RootController: SystemObject {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            drives {
                                declare first of Item;
                                first.Transition::Preset;
                                declare second of Item;
                            }
                        }
                    }
                }

                state State::Prepared {
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "declare-order.spec"
            output = Path(tmp) / "declare-order.ast.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(main([str(spec), "-o", str(output)]), 0)
            data = read_json(output)
            transition = data["document"]["objects"][0]["states"][0]["transitions"][0]
            statements = transition["drives"][0]["statements"]

            self.assertEqual(
                [(item["kind"], item["text"]) for item in statements],
                [
                    ("declare", "declare first of Item"),
                    ("call", "first.Transition::Preset"),
                    ("declare", "declare second of Item"),
                ],
            )
            self.assertEqual([item["ordinal"] for item in statements], [1, 2, 3])
            self.assertEqual(statements[0]["owner_process"], "RootController.Transition::Preset")
            self.assertEqual(statements[0]["alias"], "first")
            self.assertEqual(statements[0]["declared_type"], "Item")

    def test_declare_syntax_rejects_invalid_locations_and_forms(self) -> None:
        cases = {
            "outside-drives": (
                "ensures { declare child of Item; }",
                "declare is only allowed in drives",
            ),
            "missing-semicolon": (
                "drives { declare child of Item }",
                "drives statement is missing semicolon",
            ),
            "malformed": (
                "drives { declare child Item; }",
                "malformed declare statement",
            ),
        }
        for name, (body, expected) in cases.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                source = f"""
                    type Item {{
                    }}

                    object RootController: SystemObject {{
                        initial_state: State::Base;
                        state State::Base {{
                            transitions {{
                                on Transition::Preset -> State::Prepared {{
                                    {body}
                                }}
                            }}
                        }}
                        state State::Prepared {{
                        }}
                    }}
                """
                spec = Path(tmp) / f"{name}.spec"
                output = Path(tmp) / f"{name}.ast.json"
                spec.write_text(source, encoding="utf-8")
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    exit_code = main([str(spec), "-o", str(output)])

                self.assertEqual(exit_code, 1)
                self.assertIn(expected, stderr.getvalue())

    def test_output_parent_directory_is_created(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "nested" / "model-main.ast.json"
            exit_code = main([str(self.spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            self.assertTrue(output.exists())

    def test_include_expands_relative_to_current_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            nested = root / "nested"
            nested.mkdir()
            (nested / "child.spec").write_text(
                """
                object IncludedObject: SystemObject {
                    initial_state: State::Base;

                    state State::Base {
                    }
                }
                """,
                encoding="utf-8",
            )
            spec = root / "root.spec"
            spec.write_text(
                """
                include "nested/child.spec";

                object RootObject: SystemObject {
                    initial_state: State::Base;

                    state State::Base {
                    }
                }
                """,
                encoding="utf-8",
            )
            output = root / "root.ast.json"

            exit_code = main([str(spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            object_names = {item["name"] for item in data["document"]["objects"]}
            self.assertIn("IncludedObject", object_names)
            self.assertIn("RootObject", object_names)

    def test_duplicate_include_is_ignored(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            child = root / "child.spec"
            child.write_text(
                """
                object IncludedOnce: SystemObject {
                    initial_state: State::Base;

                    state State::Base {
                    }
                }
                """,
                encoding="utf-8",
            )
            spec = root / "root.spec"
            spec.write_text(
                """
                include "child.spec";
                include "child.spec";
                """,
                encoding="utf-8",
            )
            output = root / "root.ast.json"

            exit_code = main([str(spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            object_names = [item["name"] for item in data["document"]["objects"]]
            self.assertEqual(object_names.count("IncludedOnce"), 1)

    def test_source_include_expansion_preserves_comments(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            child = root / "child.spec"
            child.write_text(
                """
                /*
                 * Included note.
                 */
                object IncludedWithComment: SystemObject {
                    initial_state: State::Base;
                }
                """,
                encoding="utf-8",
            )
            spec = root / "root.spec"
            spec.write_text('include "child.spec";\n', encoding="utf-8")

            expanded = _read_source_with_includes(spec, seen=set(), stack=[])[0]

            self.assertIn("Included note.", expanded)
            self.assertIn("IncludedWithComment", expanded)

    def test_missing_input_returns_usage_error_code(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main([str(self.spec.with_name("missing.spec")), "-o", "/tmp/out.json"])

        self.assertEqual(exit_code, 2)
        self.assertIn("error: cannot read", stderr.getvalue())

    def test_parse_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "parse_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
