# Agent Instructions

- Do not guess fixes. When investigating a problem, first use reproducible observations to narrow the scope before drawing conclusions or changing code.
- Specify before implementing. Changes that affect behavior, interfaces, object boundaries, or Linux differential semantics must update the applicable spec/coding/guidance files before implementation.
- After every code change, run the repository root `make test` as the final regression gate. Focused tests may be used while locating issues, but they do not replace `make test`.
