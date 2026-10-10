# Maintained tooling

[Development workflow](../docs/development.md) · [Evaluation](../docs/evaluation.md) · [Report operations](../docs/reports.md)

Run tools from the repository root through `rtk proxy`. These are the current
tools, not the executable list from dated research protocols. Node tools use
built-in modules; no npm install is needed. Use each command's `--help` where
available for exact options.

| Tool | Responsibility and side effects |
| --- | --- |
| [check-planner-development.mjs](check-planner-development.mjs) | Maintained Rust/CLI/Node/docs gate. Builds and tests locally; no capture publication. The 44-input external-baseline check is explicit opt-in. |
| [check-docs.mjs](check-docs.mjs) | Read-only inline local Markdown links, ATX headings and explicit HTML IDs. No external fetch or generated-evidence requirement. |
| [check-planner-v2-workflow.mjs](check-planner-v2-workflow.mjs) | Read-only acceptance and shared batch/detail/receipt checks for existing native evidence. Needs a compiled evaluator and saved captures. |
| [check-planner-v2-integration.mjs](check-planner-v2-integration.mjs) | Explicit session/CLI modes: `native-parity`, `check-cli` and `check-replays` inspect saved evidence; `capture-cli` plans up to 44 flights; `replay-cli` executes up to 42 saved-command replays and writes a new receipt directory. Not part of the ordinary gate. |
| [check-planner-v2-browser.mjs](check-planner-v2-browser.mjs) | Uses an existing server and explicitly supplied disposable local browser. Opens/closes its own tab; optional create-only screenshots/receipt, no report regeneration. |
| [serve-reports](serve-reports) | Explicit tmux HTTP server start/status/attach/stop. Defaults to unauthenticated LAN serving of all `outputs/`. |
| [render_terminal_suite.py](render_terminal_suite.py) | Optional uv/Python 3.11 illustration generator. Rewrites tracked SVG illustrations and an ignored local HTML preview; no flight execution. |
| `test*.mjs` | Automatically discovered maintained Node tests, with no accepted external capture required by the ordinary gate. |

[planner-v2-report-checks.mjs](planner-v2-report-checks.mjs) and
[planner-v2-browser-helpers.mjs](planner-v2-browser-helpers.mjs) are shared support
modules, not separate campaign entrypoints. Tests with batch/tree/common names
retain coverage of the shared report contract; those names do not designate
separate report templates.

The dated [terrain-correction collection](../docs/ballistic_terrain_correction_results.md)
also retains `resume-ballistic-feedback-sweep.py` and
`finish-ballistic-feedback-sweep.py` as exact-capture, append-only recovery tools.
They are not ordinary workflow steps or new flight allowances: their run modes
create diagnostic attempts/artifacts, never overwrite retained evidence or
publish reports. The resume tool's `--verify` mode is read-only. Its
`test-ballistic-collector-recovery.py` checks same-clock goal/H ordering without
external captures or flights.

The dated [frozen terminal-centering sweep](../docs/ballistic_terminal_centering_sweep_results.md)
retains `run-terminal-centering-sweep.py` and
`continue-terminal-centering-sweep.py`. Their `verify` modes are read-only;
create-only run modes belong to the closed 1002-invocation allowance, not new
flight authorization. The first capture remains interrupted, while the second
copies its unchanged prefix and contains only the exact observed domain error.
`test-terminal-centering-sweep.py` and
`test-terminal-centering-continuation.py` check admission and exception boundaries
without flights. These tools do not publish reports or change server state.

The subsequent [terminal coordination pass](../docs/ballistic_terminal_coordination_results.md)
retains `run-terminal-coordination.py`: `verify-panel`/`verify-sweep` are read-only.
Its create-only measured allowance is closed after failed admission; the conditional
full sweep did not run. `test-terminal-coordination.py` exercises cohort denominators,
prefix preservation, admission and typed-domain evidence without flights or captures.
The separately authorized [frozen-coordination 1k](../docs/ballistic_terminal_coordination_sweep_results.md)
adds `diagnostic-sweep` and read-only `verify-diagnostic`. Its 1,005-record
allowance is now closed: 639/1000 landings, unchanged candidate, failed panel
verdict retained. Diagnostic authority never passes the older conditional gate;
collector tests also reject executable/Rust/renderer/planning-script drift.

Retired nominal/V1/conservative-ballistic research runners are intentionally
absent. Use the [research archive](../docs/history.md) to understand their saved
results, not to restore their obsolete frontdoors. Scripts inspect or create
local evidence; they do not implicitly authorize commits, pushes, publication
or server lifecycle changes.
