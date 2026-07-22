# tools2

Independent first-stage Signal derivation toolchain. It intentionally does not import or replace `tools/`.

```bash
make -C tools2 test
tools2/pyveri2 -t Root.Start
```

The default spec is `tools2/tests/fixtures/pipeline.spec`. Select another spec with `-f SPEC`, apply a scenario
with `-s SCENARIO`, and use `--work-dir tools2/build` to retain stage JSON. Long-lived explicit output belongs
below `tools2/out/`.

Snapshot continuation experiment:

```bash
tools2/pyveri2 -t Root.Start --snapshot-out /tmp/tools2.snapshot.json
tools2/pyveri2 -t Root.Inspect -s /tmp/tools2.snapshot.json
```
