# Agent Instructions

- Do not guess fixes. When investigating a problem, first use reproducible observations to narrow the scope before drawing conclusions or changing code.
- For kernel/runtime/user-mode behavior issues, locate the failing boundary through existing checkpoints, diagnostics, or other reproducible observations before changing behavior. Do not make blind trial fixes from symptoms alone. If existing checkpoints are insufficient, first add or extend a long-term-useful checkpoint/diagnostic, then reproduce and use that evidence to update specs and implementation.
- Specify before implementing. Changes that affect behavior, interfaces, object boundaries, or Linux differential semantics must update the applicable spec/coding/guidance files before implementation.
- After every code change, run the repository root `make test` as the final regression gate. Focused tests may be used while locating issues, but they do not replace `make test`.
- Run the repository root regression gate as a direct `make test` command. Do not wrap it in `/bin/sh -c`, `/bin/bash -lc`, external pipelines, or shell redirection unless explicitly diagnosing host command-environment issues.
