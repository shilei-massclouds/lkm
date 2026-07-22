# tools2

Independent first-stage Signal derivation toolchain. It intentionally does not import or replace `tools/`.

```bash
make -C tools2 test
tools2/bin/pyveri -t ComputerProject.Preset
```

The default spec is `spec/model/main.spec`. Select another spec with `-f SPEC`, apply a scenario with
`-s SCENARIO`, and use `--work-dir tools2/build` to retain stage JSON. The shortcut uses unbounded depth and
breadth unless either budget is supplied explicitly. Long-lived explicit output belongs below `tools2/out/`.

Snapshot continuation experiment:

```bash
tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Start --snapshot-out /tmp/tools2.snapshot.json
tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Inspect -s /tmp/tools2.snapshot.json
```
