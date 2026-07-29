# tools2

权威校准与旧工具退役路线见
[`spec/charter/tools2/semantic-validation-and-retirement.md`](../spec/charter/tools2/semantic-validation-and-retirement.md)。

Independent first-stage Signal derivation toolchain. It intentionally does not import or replace `tools/`.

```bash
make -C tools2 test
make -C tools2 test-focused TEST=tools2.tests.test_signal_pipeline.SignalPipelineTests.test_drives_and_emits_order
tools2/bin/test-python tools2.tests.test_signal_animation.SignalAnimationTests.test_animation_projects_optional_model_handler_description -v
tools2/bin/pyveri
tools2/bin/pyveri -u Kernel.Enable --snapshot-out /tmp/kernel-enable-presend.snapshot.json
tools2/bin/pyveri -u Kernel.Enable --html-out tools2/out/main-animation.html
tools2/bin/pyveri -t Kernel.Enable -u BootInitFlow.Setup --html-out tools2/out/group2-animation.html
VERBOSE=1 tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Start
```

The Python test entry discovers every `tools2/<package>/src` next to a `pyproject.toml`; neither full nor focused
tests require a handwritten `PYTHONPATH`. The dotted unittest form also works directly from the repository root
because importing `tools2.tests` performs the same discovery.

The default spec is `spec/model/main.spec`. Select another spec with `-f SPEC`, apply a scenario with
`-s SCENARIO`, and use `--work-dir tools2/build` to retain stage JSON. The shortcut uses unbounded depth and
breadth unless either budget is supplied explicitly. Long-lived explicit output belongs below `tools2/out/`.

When `-t/--trigger` is explicit and `-s/--scenario` is omitted, the shortcut loads
`tools2/scenarios/<CanonicalSignal>.snapshot.json`. An explicit scenario always wins. `Startup` normalizes to
`Preset`, so both spellings share one file. A missing or unsafe default path is a usage error; omit `-t` to run the
unique `external Human` orchestration from the model initial state. The committed `Kernel.Enable` scenario is the
canonical output of the pre-send command shown above, not an implicit derive rollback or synthesized state.
When that canonical snapshot is selected automatically, its boundary provenance restores the real Signal source;
an explicit `--source` overrides it. Explicit `-s` scenarios retain the ordinary explicit-or-`Human` source rule.

Snapshot continuation experiment:

```bash
tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Start --snapshot-out /tmp/tools2.snapshot.json
tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Inspect -s /tmp/tools2.snapshot.json
```

The shortcut defaults to the external Human orchestration, whose first real Signal is `Human -> Computer.Preset`;
there is no `Human.Startup` envelope. Explicit `-t` sends only that one root Signal and does not add lifecycle
successors. `Startup` is accepted as the external alias for `Preset`. Use `-u/--until SIGNAL` to stop immediately
before that canonical Signal is sent and export the stable pre-send snapshot. All tools2 JSON and snapshots use
protocol version 8. Every
sent Signal is strict: rejection or handler failure makes the root result fail.
Repository source paths in tools2 JSON are checkout-relative, so model fingerprints and snapshots remain stable
across working directories and equivalent checkout locations.

For the main model, Human synchronously drives Computer.Preset and Computer.Setup, then asynchronously enqueues
Computer.Enable after both succeed. Computer's three handlers do not trigger each other. `Config` and `Lds` are
initially Ready; `Kernel.Setup` enables them in that order, then records ELF linking and boot Image construction.
Kernel.Ready contains only the resulting file-construction fact. OpenSBI.Enable selects `kernel_load_pa` and
independently establishes loaded-for-handoff and PMD-alignment facts.
Kernel.Preset adopts the external Linux RV64 boot contract without asserting artifact compliance. Runtime handoff uses
`Computer.Enable -> Riscv64Platform.Enable -> OpenSBI.Enable -> Kernel.Enable`. Synchronous drives retain source
order and asynchronous sends retain FIFO creation order. OpenSBI.Enable establishes a0/a1, live `satp == 0`, DTB,
ordered-boot and primary-hart facts; it does not establish the interrupt-closed fact, which first appears after the
Kernel-owned `InterruptType.Preset`. `Startup` remains only the external alias for `Preset`, never for `Enable`.

Text output defaults to a compact, hierarchy-indented Signal propagation view. In that view `Preset` is displayed
as `Startup`, while JSON and snapshots remain canonical. Set `VERBOSE=1` exactly to restore the detailed text view;
unset `VERBOSE`, `VERBOSE=0`, and every other value keep compact output. This setting affects rendering only.

## Offline Signal animation

The animation implementation is maintained independently below `tools2/animate/`: its Python package validates
tools2 v8 `model.json + view.json` and replays the view event sequence into deterministic animation v3 causal
moments and frames, while the nested Svelte 5 + TypeScript frontend only plays those frames. Receipt produces a
request moment; synchronous drives/root responses produce feedback, asynchronous emits responses produce settle,
and truncated/stopped Signals produce terminal moments. Synchronous parent feedback therefore follows all nested responses.
It neither imports the old `tools/` SVG renderer nor derives behavior in the browser. Previously generated
self-contained v1/v2 HTML remains independently openable; the current generator and bundle publish only v3.

`tools2/bin/pyveri --html-out PATH` writes one atomic, self-contained HTML file and can be combined with text `-o`,
stdout, `-s`, `--snapshot-out`, and `--work-dir`. Successful HTML generation preserves check exit status 0 or 1;
animation protocol or I/O failure returns 2. The independently installable stage package exposes
`lkm-animate MODEL VIEW -o HTML` for already-produced v8 files.

The committed JavaScript/CSS in `tools2/animate/frontend/dist/` is the bundle used by Python. Rebuild and verify it
with the pinned lockfile:

```bash
cd tools2/animate/frontend
npm ci
npm run check
npm test
npm run build
npm run bundle-check
npx playwright install chromium
npm run test:e2e
```

From the repository root, the corresponding convenience targets are `make -C tools2 test-frontend`,
`make -C tools2 bundle-check`, `make -C tools2 test-browser`, and `make -C tools2 test-all`. Browser tests generate
their model/view/HTML fixtures under a temporary directory, load them through `file://`, reject network requests,
and exercise every causal moment of the main model's `Kernel.Enable` before-send trace. Explicit long-lived demos belong in the ignored `tools2/out/`
directory; `tools2/out/pipeline-animation.html` is the fixed small visual checkpoint.
