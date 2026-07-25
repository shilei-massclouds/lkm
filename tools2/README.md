# tools2

权威校准与旧工具退役路线见
[`spec/charter/tools2/semantic-validation-and-retirement.md`](../spec/charter/tools2/semantic-validation-and-retirement.md)。

Independent first-stage Signal derivation toolchain. It intentionally does not import or replace `tools/`.

```bash
make -C tools2 test
tools2/bin/pyveri
tools2/bin/pyveri -u Kernel.Enable --snapshot-out /tmp/kernel-enable-presend.snapshot.json
tools2/bin/pyveri -u Kernel.Enable --html-out tools2/out/main-animation.html
VERBOSE=1 tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Start
```

The default spec is `spec/model/main.spec`. Select another spec with `-f SPEC`, apply a scenario with
`-s SCENARIO`, and use `--work-dir tools2/build` to retain stage JSON. The shortcut uses unbounded depth and
breadth unless either budget is supplied explicitly. Long-lived explicit output belongs below `tools2/out/`.

When `-t/--trigger` is explicit and `-s/--scenario` is omitted, the shortcut loads
`tools2/scenarios/<CanonicalSignal>.snapshot.json`. An explicit scenario always wins. `Startup` normalizes to
`Preset`, so both spellings share one file. A missing or unsafe default path is a usage error; omit `-t` to run the
default `Computer.Preset` request from the model initial state. The committed `Kernel.Enable` scenario is the
canonical output of the pre-send command shown above, not an implicit derive rollback or synthesized state.

Snapshot continuation experiment:

```bash
tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Start --snapshot-out /tmp/tools2.snapshot.json
tools2/bin/pyveri -f tools2/tests/fixtures/pipeline.spec -t Root.Inspect -s /tmp/tools2.snapshot.json
```

The shortcut defaults to `Human -> Computer.Preset`; `Startup` is accepted as the external alias for
`Preset`. Use `-u/--until SIGNAL` to stop immediately before that canonical Signal is sent and export the stable
pre-send snapshot. All tools2 JSON and snapshots use protocol version 4. Every
sent Signal is strict: rejection or handler failure makes the root result fail.
Repository source paths in tools2 JSON are checkout-relative, so model fingerprints and snapshots remain stable
across working directories and equivalent checkout locations.

For the main model, the default request prepares `Riscv64Platform`, `OpenSBI`, and `Kernel` in declaration order,
then sets up the same three systems. `Config` and `Lds` are initially Ready; `Kernel.Setup` enables them in that
order before constructing the kernel image. Runtime handoff uses
`Computer.Enable -> Riscv64Platform.Enable -> OpenSBI.Enable -> Kernel.Enable`. Synchronous drives retain source
order and asynchronous sends retain FIFO creation order. `Startup` remains only the external alias for `Preset`,
never for `Enable`.

Text output defaults to a compact, hierarchy-indented Signal propagation view. In that view `Preset` is displayed
as `Startup`, while JSON and snapshots remain canonical. Set `VERBOSE=1` exactly to restore the detailed text view;
unset `VERBOSE`, `VERBOSE=0`, and every other value keep compact output. This setting affects rendering only.

## Offline Signal animation

The animation implementation is maintained independently below `tools2/animate/`: its Python package validates
tools2 v4 `model.json + view.json` and precomputes deterministic animation v1 frames, while the nested Svelte 5 +
TypeScript frontend only plays those frames. It neither imports the old `tools/` SVG renderer nor derives behavior
in the browser.

`tools2/bin/pyveri --html-out PATH` writes one atomic, self-contained HTML file and can be combined with text `-o`,
stdout, `-s`, `--snapshot-out`, and `--work-dir`. Successful HTML generation preserves check exit status 0 or 1;
animation protocol or I/O failure returns 2. The independently installable stage package exposes
`lkm-animate MODEL VIEW -o HTML` for already-produced v4 files.

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
and exercise every step of the successfully completed full main-model trace. Explicit long-lived demos belong in the ignored `tools2/out/`
directory; `tools2/out/pipeline-animation.html` is the fixed small visual checkpoint.
