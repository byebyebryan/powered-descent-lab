# Planner V2 session and CLI replacement results

The owned Planner V2 session and optional CLI adapter are accepted for the
current synchronous lab setup. The approved replacement validation completed
on 2026-10-05 UTC, with all three 44-case matrices passing, exact preserved
flight behavior, 42 matching saved-source CLI replays and the common report
published on the existing LAN server. This repairs an output-boundary defect;
it does not introduce a new trajectory policy or promote a game controller.

The [approved plan](waypoint_v2_session_integration_plan.md) defines this scope.
The [stopped first pass](waypoint_v2_session_integration_results.md) and all its
artifacts remain unchanged. Its ninth CLI flight was not retained and is not
retroactively claimed as a landing. The successful replacement is separate
evidence from a new source freeze.

## What changed

The first pass confused the origin of a whole executed piece with intervention
E, where its local correction begins. The repaired validator binds progress
origin to the cycle's current state, correction entry to selected E, and the
executed correction endpoint to actual H. Every reported handoff requires a
selected proposal and matching full entry/end states, identity and consumed
command prefix. Terminal progress follows the previous H without inventing a
new piece at a pre-piece deadline.

In the measured `v2_ridge_early` replacement flight, those boundaries are still
origin 0, E 1272 and H 2234. The flight lands, writes a complete CLI bundle and
replays successfully. Repeated-correction cases also preserve the next cycle's
actual H state. No trajectory selection, fixture, physics, policy or correction
limit changed.

On invalid output progress, the shared writer now retains the completed raw
flight, compact summary and diagnostic progress before refusing a bundle
receipt. Writer failures produce nonzero `bundle_failed` JSON with the actual
physical/planning fields and error reason; they do not claim complete artifact
retention after an I/O failure. The validation harness records exit metadata,
stdout and stderr before parsing, so malformed stdout no longer hides a
returned process exit. Spawn/timeout failures remain failed attempts, not
invented exits.

Regression coverage includes distinct origin/E/H, invalid endpoints and
proposal metadata, missing local search/selected proposal, raw retention
without a receipt, and a real tracked corrected scenario with complete bundle
hashes and portable read-only replay. A small input-only pack case loader
supports that test; the CLI itself consumes ordinary scenario JSON and does
not look up archived answers or fixture recipes during flight execution.

## Measured acceptance

| Matrix | Cases | Mandatory landings | Clear direct | Blocked corrected | Integrity | Supported source replay |
| --- | --- | --- | --- | --- | --- | --- |
| Native evaluator | 44 | 36/36 | 11/11 | 25/25 | 44/44 | 42/42 |
| Real CLI | 44 | 36/36 | 11/11 | 25/25 | 44/44 | 42/42 |
| Real CLI repeat | 44 | 36/36 | 11/11 | 25/25 | 44/44 | 42/42 |

Each matrix keeps eight diagnostics separate: two target landings, four
`NoClearing` stops before departure, and two unsupported inputs. Each finite
stop retains zero commands, zero physics steps and one initial sample; it is
not a flight or a landing. Unsupported inputs have no physical/mission outcome,
ordinary flight, manifest or successful physical replay claim. There are no
crashes, unverified simulations or implementation-error outcomes.

The 11 clear controls include flat, uphill and downhill uncut terrain and use
zero corrections. The 25 mandatory terrain flights have blocked initial
nominals and execute corrections before landing. There are 36 actual handoffs
across the supported pack, including up to three in an individual flight; this
is an observed count, not a newly fixed acceptance threshold.

All three exact comparisons passed: accepted historical baseline to new
native, native to CLI, and first CLI to repeat. Each comparison checks 132
scenario/flight/compact JSON artifacts and 42 complete rich report payloads.
Commands, selected programs, cycles, segments, full state/contact histories,
fuel, clock and outcomes are retained. Different evaluator/CLI executable
identities remain explicit; repeat requires the same CLI binary.

The numerical exclusions remain only the three flight wall-time fields and
five compact wall-time fields. Native row comparison separately excludes its
three timings and independently verified derived artifact hashes. CLI repeat
additionally excludes only proven output-root paths, progress wall times and
independently verified timing-dependent receipt hashes. No physics, state,
selection or provenance fields are normalized away.

The real CLI also ran 42 saved-source replays from the first CLI matrix. All
252 actual component digests match: commands/actions, events, samples, full
final state, incoming contact and manifest. These start from the original
scenario and consume recorded commands; they do not regenerate a proposal,
continue a finite prefix or step a saved snapshot. Replay success for a finite
stop means a matching prefix, not a target landing.

Exactly 132 new measured case attempts and 42 new saved replay invocations were
used. Together with the first pass's 53 attempts and zero saved CLI replays,
the cumulative planning-attempt count is 185. No extra measured mission,
replacement matrix, tuning or input change was used. Unit/integration tests
and input-only preflight are separate verification.

## Source freeze and retained evidence

All replacement matrices and saved CLI replays use clean source freeze
`561b6e5ea640fc0309b0a1232c47abc03d108843`. No source commit/edit, input or binary
changed between the matrices. Local checkpoints also include `511ea0c` for
the approved replacement allowance and `a1c2643` for invocation logging.

| Identity | SHA256 |
| --- | --- |
| Native Rust source tree | `09f66d8fd28d317ddd4ae2b3393199db2df7493e08e19f2ae6eb11c666a5906f` |
| Integration source tree | `df374c17aa0b2b918eee8191d9bd8b8b7c48fbf7cddb721003b61e0e36956a1e` |
| Evaluator executable | `e0dc22764106d86cb84be51310486a2032d42bfed6cb2ef4cbbc5f47ccec22fe` |
| Feature-enabled CLI executable | `7907a28a6012c6871f25899fbe7c52471f2fc26199feeffa87d918f7497992aa` |
| Frozen pack file | `37d68e3fc7b53cd316c1cca05b5ec53ba75b010e15477f755581871273328a8d` |

The native capture is
`outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native/`.
Replacement validation lives under
`outputs/validation/waypoint_v2_session_repair_20261005/`, including baseline
and pre-capture receipts, native parity, `cli-a/`, `cli-b/`, `replays/`,
CLI/repeat/replay check receipts, prepublication browser inspection, final
browser screenshots and preservation evidence. Generated artifacts are not
checked into Git. The retained baseline is `capture-reliability-20261004-b`.

The full feature-enabled workspace passed 1,016 tests with nine ignored.
After the final selected-proposal fail-closed adjustment, all nine affected
bundle tests and all nine feature-enabled CLI unit/integration tests passed
again; unaffected legacy suites were not repeated. All 62 JavaScript tests,
formatting, strict all-target workspace Clippy and both release builds passed.
Clippy uses only the established `single_element_loop` exception.

The default-off CLI build/tests, help and normal dependency tree were checked:
only `run`, `replay` and `report` are exposed, and `pd-eval` is absent from its
normal dependencies. Input-only preflight checked all 44 exact bound scenarios:
42 supported, two rejected, zero simulations and zero output roots.

All 22 protected scopes, comprising 1,976 historical/input/report files,
remain unchanged. Publication changed only the current V2 selection and its
common report site; home, report index and waypoint topic navigation remain
byte-identical. The report server stayed PID 483461 on port 8000 without a
restart. No push or default-controller promotion occurred.

## Latency on this machine

| Measured scope | Samples | Median ms | P95 ms | Maximum ms |
| --- | --- | --- | --- | --- |
| First CLI piece advance | 78 | 255.8 | 405.6 | 477.6 |
| Repeat CLI piece advance | 78 | 255.0 | 405.3 | 473.9 |
| First CLI finalization | 42 | 10.27 | 14.64 | 15.43 |
| Repeat CLI finalization | 42 | 10.31 | 14.63 | 15.43 |

Piece wall time includes planning, private queries, admission/prefix/certificate
proofs and execution. Finalization separately times the final source proof.
Existing planning/execution/replay buckets are preserved subtotals, not total
piece work. Output writing is outside this advance/finalization table. These
are measurements from the approved matrices, with no timing success threshold
or hard real-time claim; they are not per-tick game performance.

## Reports and usage

The current [common batch report](http://192.168.1.110:8000/reports/eval/planner_v2_lab_suite/)
is reachable from report home through Waypoint planning. Useful examples are
the [direct clear flight](http://192.168.1.110:8000/reports/eval/planner_v2_lab_suite/runs/v2_clear_685/),
[single handoff](http://192.168.1.110:8000/reports/eval/planner_v2_lab_suite/runs/v2_ridge_early/)
and [three handoffs](http://192.168.1.110:8000/reports/eval/planner_v2_lab_suite/runs/v2_plateau_wide/).
The native PASSED badge certifies its native pack, not an invented CLI
`BatchReport`. CLI matrices remain separate validation evidence, not competing
user-facing batch editions.

The maintained published-site check verifies 44 cases, 42 preserved rich
payloads, 36 actual handoffs and 46 receipt-bound pages. Browser checks pass
home/topic/batch navigation and direct, one/repeated-handoff, finite-stop and
unsupported details at 1440 px and 390 px. H markers match actual raw segment
states on both plots; toggling and selection work. Four screenshots were
manually inspected. Existing small-screen table scrolling is retained; there
is no document-wide overflow or stripped-down replacement. Prepublication
inspection reported only the optional favicon 404; final page/CDP checks had
no JavaScript or report-resource errors. The headless Chromium process logged
background registration and GPU-adapter diagnostics while the plots rendered
and interaction checks passed. This is automated/local visual verification,
not human product acceptance.

Build the opt-in lab adapter and supply an ordinary scenario plus explicit pads:

```sh
rtk proxy cargo build --release -p pd-cli --features planner-v2
rtk proxy target/release/pd-cli waypoint-v2-flight SCENARIO_JSON --source-pad SOURCE_PAD --target-pad TARGET_PAD --preflight-only
rtk proxy target/release/pd-cli waypoint-v2-flight SCENARIO_JSON --source-pad SOURCE_PAD --target-pad TARGET_PAD --output-dir NEW_OUTPUT_DIRECTORY
rtk proxy target/release/pd-cli waypoint-v2-replay --bundle-dir SAVED_BUNDLE_DIRECTORY
```

Flight output directories are create-only. Exit zero requires verified target
landing and a complete bundle; finite stops and unsupported inputs are typed
nonzero. Matching replay exit zero does not imply landing. Rich supported
details are in the printed output root's `report.html`, alongside raw files
and additive `bundle.json` / `progress.json`.

The exact measured orchestration commands were:

```sh
rtk proxy target/release/pd-eval run-pack fixtures/packs/planner_v2_lab_suite.json \
  --output-dir outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native \
  --workers 4 --no-reuse --enforce-regression-policy --no-publish
rtk proxy node scripts/check-planner-v2-integration.mjs --mode native-parity \
  --baseline outputs/eval/planner_v2_lab_suite/capture-reliability-20261004-b \
  --native outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native
rtk proxy node scripts/check-planner-v2-integration.mjs --mode capture-cli \
  --baseline outputs/eval/planner_v2_lab_suite/capture-reliability-20261004-b \
  --native outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native \
  --directory outputs/validation/waypoint_v2_session_repair_20261005/cli-a
rtk proxy node scripts/check-planner-v2-integration.mjs --mode replay-cli \
  --native outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native \
  --directory outputs/validation/waypoint_v2_session_repair_20261005/cli-a \
  --output outputs/validation/waypoint_v2_session_repair_20261005/replays
rtk proxy node scripts/check-planner-v2-integration.mjs --mode capture-cli \
  --baseline outputs/eval/planner_v2_lab_suite/capture-reliability-20261004-b \
  --native outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native \
  --directory outputs/validation/waypoint_v2_session_repair_20261005/cli-b \
  --first outputs/validation/waypoint_v2_session_repair_20261005/cli-a
rtk proxy target/release/pd-eval publish-planner-v2 \
  --dir outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native
rtk proxy node scripts/check-planner-v2-workflow.mjs
rtk proxy node scripts/check-planner-v2-browser.mjs \
  --root-url http://192.168.1.110:8000/ --cdp-url http://127.0.0.1:9331/ \
  --output-dir outputs/validation/waypoint_v2_session_repair_20261005/browser
```

Only after all native/CLI/repeat/replay and prepublication inspection gates
passed did saved publication select the native capture. Read-only integration
checks use `check-cli` and `check-replays` with the corresponding native,
directory and replay-output arguments. The disposable browser has been closed;
port 9331 is not a permanent service. Existing output roots must not be reused
for another measured pass.

## Remaining boundary and next decision

This closes the approved session/CLI integration pass. `advance_piece` operates
on one owned live plant and retains actual H for the next call; `finish` is
required before accepting terminal success. It is a synchronous evaluator-backed
lab adapter, not an evaluator-free library, `ControllerSpec`, per-tick engine
integration or arbitrary saved-snapshot restart.

The surviving limits remain four hard diagnostic stops, unsupported
vehicles/gravities and unproved arbitrary-terrain or live-perturbation coverage.
The measured quarter-second-scale piece work also needs a deliberate host
scheduling boundary before an interactive game integration. Choose that host
boundary next; extract a standalone core only if the host requires it. No new
trajectory research or policy tuning is needed to close this adapter defect.
