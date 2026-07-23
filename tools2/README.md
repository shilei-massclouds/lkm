# tools2

Independent first-stage Signal derivation toolchain. It intentionally does not import or replace `tools/`.

```bash
make -C tools2 test
tools2/bin/pyveri
tools2/bin/pyveri -u Kernel.Startup --snapshot-out /tmp/kernel-presend.snapshot.json
VERBOSE=1 tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Start
```

The default spec is `spec/model/main.spec`. Select another spec with `-f SPEC`, apply a scenario with
`-s SCENARIO`, and use `--work-dir tools2/build` to retain stage JSON. The shortcut uses unbounded depth and
breadth unless either budget is supplied explicitly. Long-lived explicit output belongs below `tools2/out/`.

Snapshot continuation experiment:

```bash
tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Start --snapshot-out /tmp/tools2.snapshot.json
tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Inspect -s /tmp/tools2.snapshot.json
```

The shortcut defaults to `Human -> ComputerProject.Preset`; `Startup` is accepted as the external alias for
`Preset`. Use `-u/--until SIGNAL` to stop immediately before that canonical Signal is sent and export the stable
pre-send snapshot. All tools2 JSON and snapshots use protocol version 4. Every
sent Signal is strict: rejection or handler failure makes the root result fail.

For the main model, the default request constructs `HardwareProject`, `FirmwareProject`, and `KernelProject` in
declaration order, then hands off to `Computer -> Riscv64Platform -> OpenSBI -> Kernel`. Project construction drives
are synchronous; runtime startup sends retain FIFO creation order.

Text output defaults to a compact, hierarchy-indented Signal propagation view. In that view `Preset` is displayed
as `Startup`, while JSON and snapshots remain canonical. Set `VERBOSE=1` exactly to restore the detailed text view;
unset `VERBOSE`, `VERBOSE=0`, and every other value keep compact output. This setting affects rendering only.
