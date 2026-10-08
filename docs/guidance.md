# Guidance Architecture

[Documentation home](README.md) · [Architecture](architecture.md) · [Evaluation](evaluation.md)

This document is the stable boundary between terminal guidance, direct
transfer, waypoint guidance, and waypoint planning. It describes ownership and
compatibility rather than controller tuning. The current V2 evaluator boundary
and retained V1 history are summarized in
[Waypoint Planning](waypoint_planning.md).

## Ownership

Terminal guidance owns braking, lateral cleanup, descent-rate control,
attitude, and touchdown after a terminal entry has been accepted. It may apply
local terrain-clearance constraints while selecting a terminal candidate, but
it does not plan a route or interpret waypoint profiles.

Direct transfer owns source-pad clearance, boost, coast, route-local corridor
constraints, and the decision to hand the craft to terminal guidance. It may
reacquire source clearance after a premature direct terminal entry. It does not
evaluate waypoint contracts.

Waypoint guidance owns the currently active preplanned leg, capture-window
lifecycle, handoff contract, continuation viability, and final-waypoint entry
into terminal guidance. It is terrain-blind: waypoint placement and arrival
envelopes must already encode a terrain-valid route.

The maintained planner is an evaluator-owned outer loop: `pd-eval` constructs a
terrain-blind nominal flight, audits the fixed program against actual terrain,
applies local correction, and replans from the actual handoff state. This
evaluator-owned outer loop is not a `pd-control` controller or a per-tick
runtime planner. It does not convert its corrections into the authored-route
controller's preplanned waypoint list. Flight execution, handoff proof and final
source replay—not a geometric proposal alone—establish its flown result.
The chord-based V1 setup planner and `pd_plan::plan` are retired. Their saved
route contracts remain readable. The policy-3 batch and report contract are recorded in the
[activation results](waypoint_v2_eval_activation_results.md).

## Lifecycle Contract

The following lifecycle describes `pd-control` guidance over the resolved or
authored route; it is not the V2 evaluator's multi-segment flight loop.

- terminal guidance starts only after terminal spatial ownership and entry
  policy pass
- direct transfer progresses through source clearance, boost, coast, and
  terminal ownership
- waypoint guidance keeps the active leg until its window contract passes or
  the waypoint-plane deadline resolves it
- a recoverable final waypoint enters terminal guidance directly
- a final waypoint without recoverability evidence retains the transfer
  fallback instead of assuming terminal safety
- route-contract reports stop at contract completion; landing reports continue
  to physical touchdown

## Compatibility Surface

The following controller and guidance contracts are persisted or consumed
across crates and must remain stable during behavior-preserving refactors. The
native V2 evaluator keeps a separate batch schema rather than converting its
records to the controller `BatchReport`; see the
[V2 activation results](waypoint_v2_eval_activation_results.md).

- `ControllerSpec` JSON shape and maintained built-in controller aliases
- canonical controller IDs
- terminal and transfer configuration field names and defaults
- controller phase strings
- telemetry metric keys and marker IDs
- batch schema `34` waypoint and terminal-recovery fields plus schema `36`'s
  optional planner provenance, diagnostics, and identity-neutral per-solve
  monotonic wall-time evidence
- deterministic mission outcomes, handoff evidence, and landing summaries

Internal Rust types and module paths are not compatibility surfaces. They may
be reorganized to make ownership explicit as long as the persisted contracts
above remain unchanged.

Retired pathwise/recoverability aliases and enabled experimental boost flags
fail explicitly before execution. The historical fields still decode, default
to false and preserve saved descriptor identity; decoding is not execution admission.

## Implementation Layout

The current `pd-control` layout follows those ownership boundaries:

- `controllers.rs` owns the controller registry plus the legacy baseline and
  staged controllers
- `guidance.rs` owns shared state-target acceleration and command allocation
- `terminal/mod.rs` owns the terminal update loop and guidance-plan lifecycle;
  `terminal/config.rs` preserves the public serialized configuration,
  `terminal/state.rs` owns internal command and entry DTOs,
  `terminal/planning.rs` owns deterministic candidate ordering and ballistic
  helpers, and `terminal/terrain.rs` isolates local candidate clearance
- `transfer/mod.rs` owns the direct-transfer and waypoint update loop;
  `transfer/config.rs` preserves the public serialized configuration,
  `transfer/state.rs` owns lifecycle state and internal DTOs, and
  `transfer/math.rs` owns shared ballistic and command-conversion helpers
- `transfer/telemetry.rs` owns transfer and waypoint metric emission plus
  waypoint handoff-marker assembly; it receives already-computed guidance
  products and does not recompute control decisions
- `transfer/waypoint.rs` owns pure waypoint geometry, capture prediction, and
  handoff kinematics
- `transfer/mod.rs` also owns the maintained endpoint scoring; frozen
  pathwise/recoverability scorers and their comparison frontdoors are retired
- `transfer/tests.rs` owns the transfer and waypoint controller tests without
  changing their access to module-private fixtures

The controller evaluator follows the same separation. Persisted batch/report DTOs live in
`pd-eval/src/model.rs`; pack validation and expansion live in `resolution.rs`;
execution, artifact/cache support, comparison, and review derivation live in
their named modules. The batch report shell delegates overview, diagnostics,
review-tree, and comparison rendering to `pd-eval/src/report/` modules. Current
entrypoints and persisted schema paths remain stable. Obsolete research
Rust APIs are deliberately removed, rather than retained as compatibility shims.

`pd-core` retains neutral authored-route contracts, read-only historical planner
DTOs and the exact point-centred envelope query used by current flight geometry.
The obsolete V1 validation/shaping engine and complete-program playback are
retired; decoding saved data is separate from execution admission. `pd-plan`
owns current ballistic math and sealed finite policies, not simulations,
controller selection or report I/O.

The native V2 evaluator uses its policy-3 flight loop and native batch
schema; it does not change the ordinary `pd-cli run` controller path or add a
V2 controller. The common batch shell and rich detail
renderer are shared through `pd-report`; its raw rich-detail HTML/CSS/JS lives
in `pd-report/src/rich_template.rs`, with data construction and substitution in
the existing facade. V2-specific aggregation and handoff annotations remain in
its evaluator/report adapter. See the
[common-template results](planner_v2_common_report_templates_results.md).

This split is internal. Public controller exports still resolve through
`pd-control`, and persisted controller, phase, telemetry, and artifact contracts
remain unchanged.

## Current V2 design and support

The accepted flight policy is **policy 3**: construct a target-appropriate,
terrain-blind nominal transfer; audit that fixed command program on real terrain;
if genuinely terrain-blocked, select and execute a bounded local clearing
maneuver; regenerate from its actual handoff H. Terrain rejection must not select
a higher nominal arc. A missing feasible nominal is not itself an obstacle.
Local clearing needs useful progress and a finite safe continuation, not a direct
landing suffix or a literal terrain-feature far edge. Another correction may
follow. Initial rest and airborne acquisition use different realization helpers
but share this outer-loop contract.

The current source adds the bounded
[failure-only timing fallback](intervention_fallback_results.md): complete the
existing local search first, then try at most four new conflict-relative entries
only on finite exhaustion, skipping tested clocks. Successful primary searches
do no fallback work. That timing extension preserved nominal construction,
ranking and safety guards, improving its development challenge from 57/100 to
65/100 without lost successes. The subsequent
[handoff braking-room preference](handoff_room_results.md) preserves the stage
admission and original rank unless its winner has negative estimated room and
an already accepted nonnegative-room row exists; then original rank selects
within that subset. This cheap horizontal heuristic cannot reject candidates,
open fallback, or certify landing feasibility. Development challenge coverage is
68/100 with all 65 prior successful flights preserved. Both fresh benchmarks
passed without replacing the accepted site or relabeling its historical capture.

The subsequent [fresh 1k validation](terrain_validation_1k_results.md) measures
733/1000 landings under four unchanged procedural recipes: 387/387 initially
clear routes and 346/613 blocked routes. All 1008 attempts, including separate
controls/repeats, pass integrity and replay. Planner behavior is unchanged in
that pass; the different sample does not establish paired improvement or
arbitrary-terrain reliability. All 95 nominal-exhaustion stops occur after a
correction, and local clearing remains the largest finite-stop category.

The subsequent [diagnostic pack](terrain_diagnostics_results.md) freezes ten
exact inspected worlds for departure clearance, late airborne acquisition and
useful correction progress, with four successful comparisons. All 13 baseline
attempts/repeats match their original complete non-timing records. The six finite
failures remain unsolved; this is a reproducible capability checkpoint, not a
coverage gain. Neither positive cheap braking room nor correction count alone
certifies recovery. Keep these selected development cases separate from a future
untouched generalization test and from the default 44-case acceptance pack.

The follow-up [cap-only diagnostic](terrain_cap_probe_results.md) isolates the
two six-correction truncations without changing their earlier flights. Under
an explicitly separate cap-12 probe, `030` lands with seven corrections and
`280` makes eight before a later `NoClearing`. Controls and exact repeats verify;
the ordinary default stays six. Short hops alone are not a proven progress
defect, and larger caps do not solve earlier nominal/local-clearing exhaustion.

The subsequent [paired 1k relaxed-cap sweep](terrain_cap_sweep_results.md)
reruns all original worlds under isolated cap 24: 748/1000 verified landings,
with all 733 old successes preserved. None reaches cap 24; the maximum used is
13 in a landing. All 1010 attempts verify, including three controls/seven repeats.
The remaining 154 `NoClearing` and 98 `NoNominal` stops are no longer count-cap
truncations. A generous finite budget is a supported default candidate, pending
maintained-pack acceptance and saved-policy compatibility review; ordinary
production remains cap 6. This is paired development, not held-out reliability.

The [bounded handoff diagnostic](terrain_handoff_probe_results.md) then queries
five saved nominal stops at original H, earlier settled coast on the same selected
maneuver and a preselected accepted alternative. All 19 probes verify: earlier
queries yield three clear nominals and two terrain-blocked nominals; max-room
alternatives yield two clear and three missing nominals. Original/early prefixes
and controls/repeats agree. Delayed replanning is a demonstrated limitation in
those selected cases, not proof that a global progress relaxation is safe or that
the sweep rate has improved. The [bounded early-exit pass](terrain_early_exit_results.md)
implements one optional clear-nominal exit while retaining the old H fallback:
three selected subjects now land, two keep exact old-flight fallbacks, and all
36 core cap-6 benchmark landings pass. Actual early ends and queued checked
nominals are explicitly bound; original H witnesses are not overwritten.
The subsequent [paired early-exit 1k rerun](terrain_early_exit_sweep_results.md)
records 817/1000 landings under the same isolated cap 24: 69 gained, all 748 old
successes preserved, and all 387 clear direct flights unchanged. All four terrain
recipes improve and all 1010 attempts verify. Remaining stops are 148 clearing
and 35 nominal exhaustions, not crashes; 65 still stop before any correction.
The next validation candidate is a separately frozen fresh 100-world sample,
not additional acquisition/controller tuning or automatic default-cap promotion.

The supported initial request is a route-free `LandingOnPad` mission with the
tested vehicle, Earth gravity, 120 Hz physics / 60 Hz commands, upright rest on
the source pad, a target to its right, and valid flat pad shelves. Airborne
planning is supported from actual session handoffs; arbitrary externally supplied
airborne starts, other vehicles/gravities, random-terrain reliability and live
disturbance recovery are not established. These are coverage boundaries, not
proof that a stopped case is physically impossible.

The [session/CLI replacement](waypoint_v2_session_repair_results.md) is the latest
accepted measured checkpoint. Its 44-case native capture is the current report:
36 mandatory landings (11 direct, 25 corrected), two diagnostic landings, four
zero-step diagnostic `NoClearing` stops and two unsupported inputs. The default
evaluation entrypoints and Rust policy default select policy 3. Policies 1/2
have explicit saved-identity recognition but no executable selector. V1 search
is retired, and ordinary controller defaults remain unchanged.

V2 implementation ownership is:

- `pd-plan/src/ballistic.rs` and its canonical-initial child: pure bridge,
  kinematic and canonical-basis math without a research feature gate.
- `pd-plan/src/waypoint_v2.rs`: versioned finite policy, correction bounds and clocks.
- `pd-plan/src/local_clearing.rs`: sealed template family and pure scalar
  handoff braking-room estimate; no terrain query or physical acceptance.
- `pd-eval/src/planner_flight/`: current request/preflight, canonical initial
  source fitting, state-derived airborne acquisition, terminal realization and
  phase/contact geometry, extracted from historical research orchestration.
- `pd-eval/src/waypoint_v2.rs`: input preflight, the owned session lifecycle and
  named phases for nominal generation/audit, fixed-prefix proof, direct
  execution, local clearing search and actual E-to-H/certificate execution.
- `pd-eval/src/waypoint_v2/model.rs`: persisted flight records and additive
  session progress, re-exported through the unchanged public paths.
- `pd-eval/src/waypoint_v2/execution.rs`: forward queries, exclusive segment
  ownership, phase-aware guards and original-source replay proofs.
- `waypoint_v2_output.rs` and `waypoint_v2_bundle.rs`: create-only evidence,
  progress/receipt validation and saved-source CLI replay.
- `local_clearing.rs`: bounded proposal search, actual continuation and proof
  helpers; no historical canary orchestration.
- `evidence_io.rs`: exact-byte hashing and create-only writing/reservation;
  caller-specific trust and provenance validation remain with their callers.
- `waypoint_v2_pack.rs` facade and children: models, sealed input expansion,
  execution/aggregation, capture validation, provenance and common presentation.
  `waypoint_v2_acceptance.rs` owns acceptance. Neither supplies archived answers
  to a flight.
- `waypoint_v2_report.rs`, `pd-report` and `pd-cli/src/planner_v2.rs`: shared
  presentation and the optional synchronous CLI adapter, not trajectory selection.

### Reconciled earlier questions

- The floor-cutaway mismatch belongs to V1's straight corridor model. V2's
  executed ballistic clear routes no longer require it; retain the original
  characterization rather than rewriting its historical results.
- Higher terrain-aware direct arcs remain historical research evidence, not a
  fallback that silently avoids the current waypoint policy.
- The old 56-combination selector is retired. Two old family constants remain
  only in the serialized candidate identity contract. Policy 3 uses
  state-derived airborne acquisition. Those old handoff failures
  are not the current policy-3 verdict or a reason to restart nominal research.
- The latest corrected-case CLI failure was piece-origin/E/H validation, not a
  new first-step physics failure. The replacement and its regression tests close
  that adapter defect; the stopped first pass remains truthful historical evidence.
- The one-update terminal completion reserve is a parked separate design, not
  a missing prerequisite. V1, bounded-witness and F6 documents/data remain
  research history; their obsolete executables are retired, not an active V2 backlog.

### Behavior-preserving cleanup checks

Structural cleanup must retain command ordering, state/fuel/clock continuity,
policy values, guards, serialized field order/names, create-only outputs, current
entrypoints and common report paths. Intentional API/test removals are listed in
the [retirement results](planner_retirement_cleanup_results.md). Keep the current capture and its provenance
unchanged; do not relabel an older capture as evidence from refactored sources.

The explicit regression test below executes all 44 bound inputs without writing
or publishing captures and compares complete flight records to an independently
validated accepted baseline. Only the three flight wall-time fields are excluded;
commands, full states/contact, selections, cycles, segments, manifests, outcomes
and replay flags must match. It is an opted-in local numerical regression, not a
new final-source acceptance/publication matrix:

```sh
rtk proxy env PD_V2_PARITY_CAPTURE=outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native \
  cargo test --release -p pd-eval --lib retained_capture_numerical_parity -- --ignored --nocapture
```

The maintained ordinary gate includes workspace/default-off/feature CLI tests,
help/dependency checks, formatting, strict all-target Clippy, local documentation
links and all maintained JavaScript tests:

```sh
rtk proxy node scripts/check-planner-development.mjs
```

Add `--parity-capture PATH` to include the explicit saved-baseline regression.
Normal tracked tests include direct, corrected and multi-handoff coverage without
an external capture. A standalone planner crate, solver rewrite and report
redesign require a concrete separate need.

The records/replay-safety separation and subsequent
[core loop cleanup](waypoint_v2_core_cleanup_results.md) passed exact numerical
parity for all 44 inputs. That earlier final gate passed 1,019 workspace tests
(ten intentionally ignored), final CLI checks, 62 JavaScript tests, formatting
and strict Clippy with the existing `single_element_loop` exception. The
retained-capture test is intentionally opt-in because it depends on locally
saved evidence rather than a tracked CI fixture; it was run separately and
passed. The rebuilt evaluator's saved-site check verified 42 rich payloads,
36 actual handoffs and 46 receipt-bound pages. All 26 recorded fixture/evidence/
report scopes retained their path/content hashes. The selected capture and
published site are unchanged; this is local regression evidence, not a new
accepted source freeze or expanded terrain-coverage claim. The subsequent
[retirement/consolidation results](planner_retirement_cleanup_results.md) supersede
that source layout and test count, not its measured flight or capture provenance.

## Native V2 Acceptance Contract (2026-10-03)

The completed reliability pass adds a versioned, evaluator-owned acceptance
check for the frozen `planner_v2_lab_suite`. Two final-source batches passed;
the [reliability results](planner_v2_reliability_results.md) record provenance,
repeat and report validation. This is not a controller comparison policy or a
new terrain experiment. The following contract was settled before measured runs:

- Bind the complete 44-case input identity to expansion of the tracked pack,
  not merely to mutually consistent hashes supplied by a capture. Require
  policy 3, completed capture status, and unchanged source/input provenance
  during capture. An older capture need not match today's Git commit.
- Require all 36 ordinary cases to have target touchdown, mission success,
  landed planning stop, integrity, and source-replay evidence. The 11 clear
  controls must be terrain-unblocked and use zero corrections. The 25 terrain
  cases must have a blocked initial nominal and at least one correction.
- Keep the eight diagnostics separate. The six supported diagnostics may land
  or end at an honest finite planning stop, but may not crash, land off-target,
  time out in the core, or hide an implementation/unverified-execution failure.
  The two expected unsupported inputs must remain preflight-only, with no
  invented physical outcome, mission outcome, or replay claim. Previously
  difficult diagnostics are allowed to improve.
- Require integrity for all 44 cases and recorded source replay for all 42
  supported cases. Do not fix the total number of handoffs or impose a new
  machine-dependent timing threshold.
- Saved-capture checking is read-only artifact/identity/outcome validation. It
  checks recorded replay evidence; it does not run a fresh physical replay or
  establish arbitrary-terrain or real-time acceptance.
- A failed capture keeps its evidence and rich report, but cannot replace the
  current accepted site. The checked batch workflow returns failure after
  recording the result; collection-only operation distinguishes completion
  from acceptance explicitly.

The completed reliability pass used a separately approved allowance of one
fresh 44-case batch and one conditional repeat (88 attempts). Its execution
budget and commit permissions were local to that pass, not standing authority
for future work. New measured captures need their own scope; freeze source,
retain every miss and do not alter cases, policy or acceptance thresholds in
response to results. Unit tests and read-only comparisons are separate checks.

<a id="optional-v2-runtime-consumer-next-phase-design"></a>

## Optional V2 session and CLI integration

The former optional-runtime design is now implemented at the bounded session
and CLI scope below. The legacy anchor is retained for historical document links.

The [accepted session and CLI replacement](waypoint_v2_session_repair_results.md)
provides `pd-eval::WaypointV2Session` and a default-off `pd-cli` feature
`planner-v2`. Native, real CLI and CLI-repeat matrices each pass the 44-case
gate, with exact retained-baseline, evaluator/CLI and repeat flight/report
parity. All 42 supported saved flights also replay through the real CLI.
The corrected progress validator distinguishes piece origin, intervention E
and actual H, and requires the selected proposal binding. The new accepted
native capture is published through the existing common report workflow.
The [stopped first pass](waypoint_v2_session_integration_results.md) remains
historical evidence rather than being retroactively accepted.

The session owns one full live plant, original deadline and command ledger.
One advance includes nominal approach to E and correction to actual H; the
next call starts at that retained H. Queries and continuation certificates
remain private clones. Correction counts advance only after the existing
proofs pass. Finalization performs final source replay and can override an
earlier landed progress result. Snapshots remain query-only, and no commands
can be issued after a terminal stop.

The optional dependency is an explicit lab adapter, not an evaluator-free game
library or a new `ControllerSpec`. Ordinary `pd-cli run`, controller defaults
and default-off dependencies remain unchanged. A matching saved-prefix replay
does not imply landing. The supported vehicle/Earth/120-60 Hz setup, terrain-blind
nominal choice and local-clear-then-replan logic are unchanged; hard real-time,
asynchronous planning and generic game integration remain separate decisions.

## Maintained Gates

The counts below are retained outcomes for the maintained controller/guidance
corpora, with freshness and source scope defined by their capture records; they
are not a recapture at current HEAD and do not represent the separate native V2
44-case batch. See the [V2 activation results](waypoint_v2_eval_activation_results.md)
for that active evaluator baseline.

The guidance regression set is:

- terminal bot-lab and trajectory-error smoke packs
- `transfer_route_angle_radius_suite`
- paired waypoint turn and ordered smoke landing/contract packs
- paired full-seed nominal waypoint landing/contract packs
- paired all-radius waypoint landing/contract packs

Current schema-34 primary evidence is:

- clean terminal: `171 / 189` physical successes, with `9` scored failures and
  `9` analytic invalidations
- trajectory-error terminal: `694 / 756` physical successes, with `26` scored
  failures and `36` analytic invalidations
- direct transfer: `297 / 297`
- all-radius turn landing and contract: `405 / 405` for both
- all-radius ordered landing and contract: `135 / 135` for both
- focused generated-route landing: `54 / 54` with zero invalidations
- focused generated-route handoff/ordered contract: `36 / 36` with zero
  invalidations

Supporting full-seed nominal waypoint evidence remains `540 / 540` for turn
landing/contracts and `180 / 180` for ordered landing/contracts. Bounded final
authority-recovery search closes the former
`single_gentle_bend_v1/full/r-30/short/seed 02` landing residual without
changing waypoint contracts or adding route/profile branches.

The report catalog also retains the direct-transfer solved-region and focused
`r+80` full-seed captures as supporting schema-32 evidence (`1080 / 1080` and
`108 / 108`). They remain valid outcome history but do not carry every
schema-34 waypoint/terminal-recovery field; recapture them only when current
schema evidence is needed.

## Experimental Boundary

Pathwise boost scoring, recoverability-weighted boost scoring, and the
no-terrain terminal alias remain reproducible diagnostics. They are not
maintained guidance modes and must not influence default controller behavior.
Their code is isolated from the maintained path, and their comparison packs use
the `diagnostic` expectation tier plus `experimental` tags. Reopen them only
when a new hypothesis justifies another experiment.

## Consolidation Rule

The 2026-07-13 guidance consolidation was behavior-preserving at the
then-current schema `33`: it did not change thresholds, candidate ordering,
route geometry, phase transition conditions, or artifact contracts. Schema
`34` was added later for explicit terminal-recovery evidence. Future structural
cleanup follows the same rule; behavioral changes require a separate controller
change with fresh evaluation evidence.
