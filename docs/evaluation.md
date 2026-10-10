# Evaluation workflow

[Documentation home](README.md) · [Development](development.md) · [Reports](reports.md)

Run the examples from the repository root. Native V2 planner captures and
controller-pack caches are different workflows: the planner section below uses
its frozen acceptance contract; controller packs use cache and baseline comparisons.
Commands that run missions create local evidence and may update report indexes.
Use the saved-capture checks for read-only inspection.

## Planner Evaluation (V2 Default)

The latest opt-in [acquisition/safety checkpoint](ballistic_acquisition_safety_results.md)
records acquisition-only 863/1000 versus 829 (37 gains, three deadline losses)
and terminal-only 833/1000 (four gains, no losses). Both full paired sweeps,
focused controls, exact repeats and proofs verify. These are separate development
experiments, not a combined candidate or the default planner used by commands
below. Their [frozen plan](ballistic_acquisition_safety_plan.md) is complete;
read-only checks are listed in the results and
[terrain tooling workflow](../studies/terrain_profiles/README.md#ballistic-feedback-paired-diagnostic).
New measured runs, a combination and held-out validation need a separate decision.

The [ballistic aim and correction plan](ballistic_aim_correction_plan.md) proposes
a feedback planning revision and full paired regression: the maintained 44-case
pack, portable diagnostics, easy/harder 100-case sets, all original 1k worlds and
the separate observed 100-world validation set. The opt-in
[construction candidate](ballistic_feedback_results.md) is implemented but not
accepted; its separate [1k diagnostic](ballistic_feedback_sweep_results.md)
lands 360/1000. The subsequent [bounded replan pass](ballistic_replan_results.md)
lands 385/1000, preserving all 360 candidate successes. The subsequent
[terrain-aware correction pass](ballistic_terrain_correction_results.md) lands
401/1000, preserving all 385 earlier complete flights and gaining 16. These are opt-in diagnostics;
the full candidate acceptance campaign is not executing.
The later [waypoint-clearance panel](ballistic_waypoint_clearance_results.md)
lands 13/19 selected primary missions (plus two finite-stop repeats); 807/928
progress beyond their original stops but do not land. It is not another full 1k
estimate, and ordinary planner defaults remain unchanged.
The later [ridge-aware panel](ballistic_ridge_waypoint_results.md) lands 807 but
loses three previous selected landings: 8/16 random worlds versus 10/16, with
three unchanged direct controls. It is not ready for a wider campaign or adoption.
The preceding [waypoint-entry ablations](ballistic_waypoint_entry_results.md)
land 349/006 with either lower effort or higher-energy recovery. The combined
preservation panel stays at 8/16, with three gains and three losses, plus three
exact direct controls. Its fixed 53 invocations are complete; the opt-in modes
do not change the maintained planner or the experimental ridge default.
The preceding [local waypoint ablations](ballistic_local_waypoint_results.md)
complete 65 planned records: early-target alone lands 715, but local height alone
does not, and the combined preservation panel falls from 8/16 to 6/16 with
084/142 lost. Direct controls remain exact. Early-only has seven focused random
comparisons, not full preservation validation; no wider campaign or adoption
follows this mixed result. All new modes are explicit opt-ins.
The subsequent [landing-duration experiment](ballistic_landing_duration_results.md)
completes 44 more bounded records: combined 715/084/142 enter landing but stop
later, while 268 is gained without losses (7/16 versus 6/16). All previous
successful ordinary flights, direct controls and early-only 715 remain exact.
An instantaneous admitted fit is not sustained braking or a verified landing.
Its allowance is closed; no 1k campaign or default adoption follows.
The [retained countdown follow-up](ballistic_landing_countdown_results.md)
completes 48 records with no new landings (7/16). The arrival clock is retained,
but thrust/tilt/terrain revalidation releases it before landing. Initial
acceleration admission does not establish full-profile braking feasibility.
All old landing outcomes, direct controls and early-only 715 are preserved;
268's successful ordinary flight changes. Defaults remain unchanged.
The [084 coast-through diagnostic](ballistic_terminal_coast_results.md) then
records four fixed setup comparisons. Preserving H1 coast and using standalone
terminal defaults at a scheduled checkpoint lands without W2; transfer setups
remain airborne at the original budget. This saved-prefix capability example
does not select automatic entry timing or change maintained planner commands.
The subsequent [automatic branch](ballistic_coast_terminal_results.md) completes
22 fixed comparisons: 084 selects its crest-clear/terminal continuation from
runtime state and lands with H1 only. Selected random outcomes move from 3/6 to
4/6, with no losses, all other ordinary flights exact and three exact direct
controls. The focused allowance is closed. The subsequently authorized
[full preservation and 1k pass](ballistic_coast_terminal_validation_results.md)
completes 1023 records: 8/16 panel landings without losses, then 562/1000 versus
401, with 200 gains and 39 losses. The current candidate combines several
refinements; this is not the isolated coast-branch increment or held-out coverage.
All records/repeats/proofs verify, with no actual crashes. The accepted
policy-3 site and defaults are unchanged; its retained 817/1000 remains stronger.
Frozen inputs, separate denominators, candidate command/replay proofs
and native/CLI/repeat agreement remain required; historical full-flight parity
is not acceptance for an intentional behavior change. Current commands below
still execute the existing planner, not the proposed loop.

Read-only saved diagnostic verification (no new flight or publication):

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_feedback_sweep.py verify \
  outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1
rtk proxy python3 -B studies/terrain_profiles/ballistic_feedback_sweep.py verify \
  outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-r2
rtk proxy python3 -B studies/terrain_profiles/ballistic_feedback_sweep.py verify \
  outputs/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1
rtk proxy python3 -B scripts/resume-ballistic-feedback-sweep.py --verify \
  outputs/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1
rtk proxy python3 -B studies/terrain_profiles/waypoint_clearance_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-waypoint-clearance-panel-20261008-v1
rtk proxy python3 -B studies/terrain_profiles/ridge_waypoint_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-ridge-waypoint-panel-20261008-v1
rtk proxy python3 -B studies/terrain_profiles/waypoint_entry_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-waypoint-entry-preservation-20261008-v1
rtk proxy python3 -B studies/terrain_profiles/waypoint_local_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-waypoint-local-preservation-20261008-v1
rtk proxy python3 -B studies/terrain_profiles/coast_terminal_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-coast-terminal-focus-20261009-v1
rtk proxy python3 -B studies/terrain_profiles/coast_terminal_validation.py verify \
  outputs/eval/planner_v2_random_terrain/capture-coast-terminal-preservation-20261009-v1
rtk proxy python3 -B studies/terrain_profiles/ballistic_feedback_sweep.py verify \
  outputs/eval/planner_v2_random_terrain/capture-coast-terminal-sweep-20261009-v1
```

The [bounded early-exit results](terrain_early_exit_results.md) retain a separate
nine-mission cap-24 diagnostic and normal 44-case cap-6 acceptance pack. Both
allowances are closed; accepted selectors and report publication are unchanged.
Read-only saved checks for those captures are:

```sh
rtk proxy python3 -B studies/terrain_profiles/early_exit.py verify \
  outputs/eval/planner_v2_random_terrain/capture-early-exit-20261007-v1
rtk proxy outputs/eval/planner_v2_random_terrain/early-exit-review-20261007-v1/bin/pd-eval \
  check-planner-v2 --dir outputs/eval/planner_v2_lab_suite/capture-early-exit-20261007-native
```

The second command uses the retained normal-policy candidate binary, not the
untouched ordinary release executable. It checks saved evidence without flights
or publication. General batch commands below are not authorization to reopen
the completed campaign.

The subsequent [paired early-exit 1k rerun](terrain_early_exit_sweep_results.md)
is also complete: 817/1000 verified landings, 69 gains, no lost prior landings.
It uses the exact admitted experimental cap-24 binary and unchanged original
worlds, not the normal acceptance denominator. See
[saved sweep verification](#paired-early-exit-1000-world-rerun) below.

The [fresh checkpoint and departure probe](terrain_departure_probe_results.md)
are also closed: 77/100 new-world landings with the retained early-exit/cap-24
binary, followed by a negative 13-mission fixed departure experiment. Its runtime
was retired; no default or accepted selector changed. Read-only checks are:

```sh
rtk proxy python3 -B studies/terrain_profiles/fresh_validation.py verify \
  outputs/eval/planner_v2_random_terrain/capture-fresh-early-exit-20261008-v1
rtk proxy python3 -B studies/terrain_profiles/departure_probe.py verify \
  outputs/eval/planner_v2_random_terrain/capture-departure-probe-20261008-v1
```

These checks authenticate saved native/operator sources, full flights, controls,
repeats and receipts without new flights, replay or publication. The fresh
edition also needs its retained baseline for reference authentication. The
rejected departure runner fails before output reservation when its runtime is
absent; it is not a supported ordinary planner command.

Run the current planner lab suite with the normal batch entrypoint:

```bash
rtk proxy cargo run --release -p pd-eval -- run-pack --workers 4 --enforce-regression-policy
```

Omitting the pack selects
[`planner_v2_lab_suite`](../fixtures/packs/planner_v2_lab_suite.json). It evaluates
the unchanged 32 practical cases and 12 additional terrain cases using V2
policy 3. Explicit controller packs, such as `terminal_bot_lab_suite`, keep their
existing evaluation and cache/comparison workflow.

Planner batches use fresh, create-only captures beneath
`outputs/eval/planner_v2_lab_suite/`. Each capture contains grouped batch results,
full flight/summary/scenario evidence, original rich reports, and annotated rich
mission detail pages. Clear/direct landings, terrain/waypoint landings, finite
planning stops, and unsupported diagnostics are shown separately. The eight
diagnostics are not added to the ordinary landing-success denominator.

The checked command distinguishes a completed capture from an accepted batch.
It requires all 36 ordinary target landings, direct clear controls, corrected
blocked-terrain cases, and integrity/source-replay evidence, while permitting
honest finite diagnostic stops. Failure returns a nonzero exit status after
retaining the capture and its rich reports. A failed batch cannot replace the
current accepted report. Omitting `--enforce-regression-policy` keeps
collection-only exit semantics, but still prints the verdict and cannot publish
a failed batch as current.

Add `--no-publish` to retain a new native capture without changing the selected
report, even if the batch passes. This still executes missions and writes a
create-only capture; it is not a read-only check.

Check a saved capture without running missions or changing reports:

```bash
rtk proxy cargo run --release -p pd-eval -- check-planner-v2 --dir CAPTURE_DIRECTORY
rtk proxy cargo build --release -p pd-eval
rtk proxy node scripts/check-planner-v2-workflow.mjs
```

Replace `CAPTURE_DIRECTORY` with an existing capture directory. The second
command resolves the selected capture automatically, checks it with the
compiled release evaluator, and verifies the common batch/detail pages against
their raw evidence and render receipt. Use `--compare-dir OTHER_CAPTURE` to
also compare a same-source repeat. These read-only checks validate recorded
replay evidence; they do not perform a fresh physical replay. See the
[acceptance contract](guidance.md#native-v2-acceptance-contract-2026-10-03)
for the exact gates and evidence boundaries.

The [reliability results](planner_v2_reliability_results.md) record two
same-source accepted captures, report validation and the remaining runtime
integration boundary.

Start at the report home, then **Waypoint planning -> Current V2 batch -> Mission
detail**. The stable batch page is
`/reports/eval/planner_v2_lab_suite/index.html`. Every simulated mission detail
keeps the existing trajectory views, charts, sample inspection, and statistics,
and adds exact executed waypoint handoffs and return/previous/next navigation.
Unsupported cases have diagnostic pages, not fabricated flight plots.
Terminal, transfer and V2 now share the same batch/detail templates; V2 adds
executed handoffs without replacing the established plots or review tree. See
the [common-template checkpoint](planner_v2_common_report_templates_results.md).

Policy 3 is the sole executable planner policy and the Rust policy default.
`--policy-version 1` or `2` is rejected; known saved policy identities remain
readable without keeping their selectors executable. Frozen research fixtures,
captures and report pages retain their original identities and provenance.
Native V2 batches have their own truthful schema, not forged V1/controller
records. They do not use controller cache/Git-ref baseline comparisons;
unsupported comparison flags fail before flight execution. For native V2,
`--enforce-regression-policy` applies the frozen pack's acceptance contract,
not a legacy controller baseline comparison.

This default covers planner evaluation/reporting on the supported vehicle,
Earth gravity, and 120/60 Hz setup. It does not change ordinary `pd-cli run` or
terminal/transfer controller defaults. See the
[activation contract](waypoint_v2_eval_activation_plan.md).
The [earlier activation results](waypoint_v2_eval_activation_results.md)
retain the 2026-10-03 capture, evidence comparisons, navigation/browser checks
and runtime-integration boundary at that checkpoint. The reliability results
linked above retain earlier captures; the session/CLI replacement supplies the
current selected capture. Cleanup validation does not relabel that capture's source.

Run the maintained developer gate without creating captures or publishing reports:

```bash
rtk proxy node scripts/check-planner-development.mjs
```

For the explicit read-only retained-baseline regression, add
`--parity-capture outputs/eval/planner_v2_lab_suite/capture-early-exit-20261007-native`.
It executes all 44 frozen inputs locally and requires exact complete-flight
parity except the three existing wall-time fields. The ordinary gate uses tracked
direct, corrected and multi-handoff tests and needs no locally saved capture.
This is the explicitly named later local numerical checkpoint, not a new
accepted-site selection; the published October-5 capture predates the documented
geometry-scalar repair. See [retained parity scope](development.md#optional-retained-flight-regression).

## Single Planner Flight

`pd-eval waypoint-v2-flight` runs the current piecewise planner from a full
scenario: terrain-blind nominal construction, fixed terrain audit, local
clearing where feasible and replanning at actual H. Supply exact pad
IDs from that scenario and a new output directory:

```bash
rtk proxy cargo run --release -p pd-eval -- waypoint-v2-flight \
  --scenario SCENARIO.json --source-pad-id SOURCE_PAD_ID \
  --target-pad-id TARGET_PAD_ID --output-dir NEW_FLIGHT_ROOT
```

Replace the uppercase arguments with real values. Use `--preflight-only`
instead of `--output-dir` for read-only validation. Output includes complete
flight records, segment/proof evidence, ordinary replay artifacts and the common
rich `report.html` with actual waypoint handoffs. Unsupported and invalid requests
are rejected before simulator execution; a finite planning stop is not a landing
or an impossibility proof. Output roots are create-only. The old standalone
nominal-direct command is retired; its live realization helpers now belong to
`pd-eval::planner_flight`.

### Optional V2 session CLI

The default-off `pd-cli` feature exposes the same owned session, with its own
adapter argument names. Validate a scenario without creating a simulator or bundle:

```sh
rtk proxy cargo run --release -p pd-cli --features planner-v2 -- waypoint-v2-flight \
  SCENARIO.json --source-pad SOURCE_PAD_ID --target-pad TARGET_PAD_ID --preflight-only
```

To execute instead, replace `--preflight-only` with `--output-dir NEW_CLI_BUNDLE`.
The new root is create-only. The bundle includes progress, segment/receipt evidence
and a rich report; this command does not publish the native batch site. Unlike
the evaluator's collection-oriented single-flight exit, this CLI requires the
full successful landing gate for exit zero. Inspect the planning, physical and
mission outcomes together when either command stops.

Replay a saved CLI bundle from its recorded commands, without new planning:

```sh
rtk proxy cargo run --release -p pd-cli --features planner-v2 -- waypoint-v2-replay \
  --bundle-dir EXISTING_CLI_BUNDLE
```

Replay executes the saved source simulation and verifies evidence; it is not a
resume from a restored airborne snapshot. The synchronous adapter remains a lab
workflow, not an externally driven per-tick game controller.

## Batch Eval

### Procedural-terrain diagnostic pack

The opt-in [ten-world diagnostic pack](../fixtures/research/terrain_diagnostics_v1/README.md)
turns inspected 1k failures into departure, late-airborne-acquisition and useful-
progress tests, with successful comparisons and a clear-direct control. Its
[selection plan](terrain_diagnostics_plan.md) and
[frozen contract](../studies/terrain_profiles/diagnostics_plan.json) keep exact
source inputs, original identities and baseline observations. No large saved
survey is required to use the checked-in inputs. This is selected development
data, not a fresh coverage estimate or a change to the default 44-case suite.

The collector uses native input-only preflight and `waypoint-v2-flight`, retains
the common rich detail reports, and never publishes or changes a selector.
For a separately authorized collection, use a new root:

```sh
rtk proxy python3 -B studies/terrain_profiles/diagnostics.py run \
  --binary target/release/pd-eval \
  --output outputs/eval/planner_v2_random_terrain/NEW_DIAGNOSTIC_CAPTURE
```

Default baseline mode requires the contract's frozen executable and complete
non-timing equality for all ten missions, followed by three fixed exact repeats:
13 measured attempts, no retry. Matching six expected finite failures validates
the diagnostics; it does not count as six successful landings. Future behavior
work can explicitly use `--candidate` with another source-frozen executable:
changed failure results are recorded against the baseline, successful comparisons
must still land, the clear control must remain direct, and repeats remain exact.
That mode needs its own authorization; it is not another phase of a closed run.

Saved verification executes no flight, fresh physical replay or report write:

```sh
rtk proxy python3 -B studies/terrain_profiles/diagnostics.py verify \
  outputs/eval/planner_v2_random_terrain/DIAGNOSTIC_CAPTURE
```

The verifier checks frozen source/input bytes, preflights, ordered attempts,
native full/compact results, source/protected-file stability, family observations,
repeat equality and the exact artifact receipt. Compact observations explain
actual H states; the final cycle state is a planning origin, not necessarily the
eventual touchdown. Keep the complete landing tuple when interpreting results.
For a committed early exit, observations use the actual earlier handoff; the
unflown original witness's braking-room estimate is not attributed to that state.

### Isolated correction-cap diagnostic

The [cap-sensitivity plan](terrain_cap_probe_plan.md) tests whether the two
mid-flight `CorrectionLimit` cases simply need more permitted waypoints.
The [contract](../studies/terrain_profiles/cap_probe_plan.json) fixes caps 12/24,
subjects `030/280`, preservation controls `000/253`, and two repeats per stage.
Cap 24 is conditional on a subject still hitting cap 12. Maximum 12 flights;
no new terrain, fuel, deadline, maneuver, ranking or guard settings.
The [completed result](terrain_cap_probe_results.md) used six cap-12 attempts:
`030` lands with seven corrections; `280` reaches later `NoClearing` after eight.
Cap 24 was not admitted. Production remains cap 6; the allowance is closed.

The [collector](../studies/terrain_profiles/cap_probe.py) generates isolated
source copies with explicit probe policy IDs and builds their binaries in a
separate task-specific target directory. It changes only the copied policy cap
and identity, not tracked Rust, the sealed production policy admission, ordinary
release evaluator, default pack or report site. These experimental captures are
not accepted production policy-3 batches.

For a separately authorized new capture:

```sh
rtk proxy python3 -B studies/terrain_profiles/cap_probe.py run \
  --output outputs/eval/planner_v2_random_terrain/NEW_CAP_PROBE_CAPTURE
```

Read-only saved verification performs no build, flight, fresh replay or report write:

```sh
rtk proxy python3 -B studies/terrain_profiles/cap_probe.py verify \
  outputs/eval/planner_v2_random_terrain/CAP_PROBE_CAPTURE
```

Verification binds source transformations, binaries, immutable inputs, the
original six-correction prefix, next nominal audit, full native result tuple,
controls, exact repeats, conditional stage admission and the complete receipt.
Flights start from the original pad source; continuation means an identical
executed prefix followed by more planning, not arbitrary airborne-state restore.

### Paired 1,000-world relaxed-cap sweep

The [paired sweep plan](terrain_cap_sweep_plan.md) reruns all original 1k saved
worlds under an isolated cap of 24, with the three original controls and seven
fixed repeats: 1010 measured attempts. This is development-population cap
sensitivity, not new random terrain or fresh held-out validation. The
[contract](../studies/terrain_profiles/cap_sweep_plan.json) leaves all other
flight settings and production defaults unchanged.
The [completed paired result](terrain_cap_sweep_results.md) preserves all 733
old landings and adds 15 for 748/1000; no case reaches cap 24. All 1010 attempts
verify and the allowance is closed. Production remains at six corrections.

For an explicitly authorized new capture:

```sh
rtk proxy python3 -B studies/terrain_profiles/cap_sweep.py run \
  --output outputs/eval/planner_v2_random_terrain/NEW_PAIRED_CAP_CAPTURE
```

Saved verification is read-only and requires the authenticated original 1k
capture to remain available. It checks complete flights for old non-cap stops,
exact executed prefixes for old cap stops, probe identity/source/binary,
inputs, ledgers, controls, repeats, source stability and complete receipts:

```sh
rtk proxy python3 -B studies/terrain_profiles/cap_sweep.py verify \
  outputs/eval/planner_v2_random_terrain/PAIRED_CAP_CAPTURE
```

Do not use the ordinary native survey checker to relabel experimental probes
as accepted policy 3. Both native full/compact records retain their explicit
cap-probe identity. Collection does not publish reports or alter navigation.

### Paired early-exit 1,000-world rerun

The [fixed plan](terrain_early_exit_sweep_plan.md) and
[contract](../studies/terrain_profiles/early_exit_sweep_plan.json) reuse the exact
admitted early-exit executable, all 1000 original scenario bytes, three controls
and seven repeats. The [completed result](terrain_early_exit_sweep_results.md)
records 817/1000 landings, preserving all 748 baseline landings and all 387 clear
direct flights. All 1010 attempts verify. This closed allowance supplies paired
development evidence, not a fresh held-out estimate or default-cap promotion.

The initial collector serialization error required a new create-only capture,
not new flights: seven authenticated executions and original error ledgers are
retained, then 1003 previously unattempted missions run. No input, native binary,
trajectory or numerical comparison tolerance changed. The final saved verifier
checks complete parsed native scenario values while input bytes stay pinned.

Read-only saved verification, requiring the authenticated cap-24 baseline:

```sh
rtk proxy python3 -B studies/terrain_profiles/early_exit_sweep.py verify \
  outputs/eval/planner_v2_random_terrain/capture-early-exit-sweep-20261008-v2
```

It checks full fallback parity or exact original selected proposals/ordinary
prefixes to the earlier actual exit, queued nominal bindings, all landing tuples,
repeats, continuation accounting and the closed receipt. It launches no flights,
replay processes or report publication. Do not relabel the isolated probe as
accepted ordinary policy 3 or reuse this completed execution allowance.

### Bounded handoff timing/selection diagnostic

The [fixed diagnostic plan](terrain_handoff_probe_plan.md) compares five saved
`NoNominal` cases at their original H, an earlier settled coast point on the
same maneuver and the preselected max-room accepted alternative. Seven original
queries include airborne/rest controls; two fixed repeats close the 19-probe
allowance. This is counterfactual construction/audit evidence, not a new flight
sweep, executed waypoint/landing claim or production-policy change.
The [completed result](terrain_handoff_probe_results.md) closes all 19 queries:
earlier same-maneuver queries construct five nominals, three clear and two
terrain-blocked; max-room alternatives are clear in two cases. Original controls
and exact repeats pass. The retained root is
`outputs/eval/planner_v2_random_terrain/capture-handoff-probe-20261007-v1`.
The subsequent [bounded early-exit implementation](terrain_early_exit_results.md)
and [paired 1k rerun](terrain_early_exit_sweep_results.md) are now complete under
their separate execution allowances; the original diagnostic remains query-only.

For an explicitly authorized capture, build the diagnostic evaluator into an
isolated target directory, leaving the ordinary release binary unchanged:

```sh
rtk proxy cargo build --release --locked -p pd-eval \
  --target-dir target/handoff-probe-20261007-v1
rtk proxy python3 -B studies/terrain_profiles/handoff_probe.py run \
  --binary target/handoff-probe-20261007-v1/release/pd-eval \
  --output outputs/eval/planner_v2_random_terrain/NEW_HANDOFF_PROBE_CAPTURE
```

Saved verification authenticates retained inputs/source/binary, original
comparisons, source-prefix proofs, audits, repeats, ledgers and receipts, without
executing any new query/replay or publishing reports:

```sh
rtk proxy python3 -B studies/terrain_profiles/handoff_probe.py verify \
  outputs/eval/planner_v2_random_terrain/HANDOFF_PROBE_CAPTURE
```

### Random procedural-terrain survey

The separate [100-case survey plan](random_terrain_survey_plan.md) uses fresh
procedural profiles rather than altering the sealed planner benchmark. The
[terrain runner](../studies/terrain_profiles/README.md#separate-random-terrain-flight-survey)
documents explicit preparation, execution and saved verification. Its landing
rate, observed nominal conflicts and completed corrections are exploratory
coverage; sentinels and repeats keep separate denominators. It does not change
the default pack, policy or accepted report selector.
The [completed full sweep](random_terrain_full_sweep_results.md) verified 100/100
random direct landings, all three controls and five exact non-timing repeats.
All random nominal paths were clear; this is not random blocked-path coverage.
The [repair-and-retry continuation](random_terrain_full_sweep_plan.md) supersedes
the earlier no-retry policy without changing the frozen inputs. Each retry must
use a new create-only source-frozen capture and preserve previous attempts.

The [harder-terrain challenge](terrain_challenge_plan.md) uses separate source-bound
calibration/held-out plans with four global physical-scale recipes and explicit
repeat indices. Its [completed results](terrain_challenge_results.md) record
21/64 blocked-route landings, 36/36 clear-route landings and 43 finite planning
stops across 100 held-out cases. Calibration/control/repeat denominators remain
separate. This is a frozen development challenge, not a changed default pack or
accepted-benchmark promotion. Do not rerun a completed capture or select new
seeds according to its outcomes.

The completed [failure-only timing pass](intervention_fallback_results.md) reuses
the same saved worlds, rather than generating another population. Its
[collector](../studies/terrain_profiles/fallback_pass.py) authenticates exact input
bytes, source snapshots, bounded query work, preserved successes and same-source
repeats. Read-only verification, with no flights or report refresh:

```sh
rtk proxy python3 -B studies/terrain_profiles/fallback_pass.py verify \
  outputs/eval/planner_v2_random_terrain/capture-fallback-20261007-focus
rtk proxy python3 -B studies/terrain_profiles/fallback_pass.py verify \
  outputs/eval/planner_v2_random_terrain/capture-fallback-20261007-challenge
rtk proxy target/release/pd-eval check-planner-v2 \
  --dir outputs/eval/planner_v2_lab_suite/capture-fallback-20261007-benchmark
```

The benchmark was captured with `--no-publish`. These roots retain their actual
source identities; saved verification does not claim that they came from later
code. The bounded pass is closed; a new campaign or publication needs a separate
decision, not reusing a completed root or its expired allowance.

The completed [handoff-room preference pass](handoff_room_results.md) binds the
same original inputs separately from the current fallback motion baseline.
All 171 planned evaluations completed; 68/100 challenge landings preserve all
65 previous successes. Its [collector](../studies/terrain_profiles/handoff_room_pass.py)
also checks actual accepted-handoff estimates and the one-sided selection rule.
Saved verification is read-only and does not generate another campaign:

```sh
rtk proxy python3 -B studies/terrain_profiles/handoff_room_pass.py verify \
  outputs/eval/planner_v2_random_terrain/capture-handoff-room-20261007-challenge
rtk proxy target/release/pd-eval check-planner-v2 \
  --dir outputs/eval/planner_v2_lab_suite/capture-handoff-room-20261007-benchmark
```

The current benchmark was also captured with `--no-publish`; all 38 prior
benchmark landings are complete-flight exact. These captures and the shadow
retain distinct truthful source identities. Wall timings and additive query
diagnostics are the only cross-source motion-comparison exclusions; same-source
repeats retain diagnostics. The pass is closed, not a standing retry allowance.

### Fresh 1,000-case terrain validation

The separately scoped [1k validation plan](terrain_validation_1k_plan.md) uses
1000 unseen worlds, 250 per unchanged challenge recipe, plus three controls and
five fixed repeats. The current planner is frozen; there is no mid-campaign
tuning, filtering or automatic retry. It estimates coverage under the same
generator, not paired improvement over an older planner. This allowance does
not authorize report publication or selector changes.

The [completed results](terrain_validation_1k_results.md) record 733/1000
verified landings, 346/613 blocked-route landings and exact controls/repeats.
All 1008 attempts are verified and the allowance is closed; the commands below
inspect retained evidence, not permission to collect another population.

Read-only saved verification, with no fresh flights, replay or report writes:

```sh
rtk proxy python3 -B studies/terrain_profiles/survey.py verify \
  outputs/eval/planner_v2_random_terrain/capture-validation-1k-20261007-v1
rtk proxy target/release/pd-eval check-terrain-survey \
  --capture-dir outputs/eval/planner_v2_random_terrain/capture-validation-1k-20261007-v1
```

### Controller batches

`pd-eval` owns scenario packs, scenario-family expansion, seed sweeps, and
native multithreaded execution.

Example:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/terminal_bot_lab_suite.json --workers 4
```

`run-pack` now writes stable review output to `outputs/eval/<pack>/`, but stores
the actual batch artifacts under:

- `outputs/eval/cache/<workspace-or-commit-key>/<batch-stem>/`

By default it will:

- reuse a complete candidate cache when the resolved pack digest matches
- try `--compare-ref auto`
- on a dirty workspace, compare against the clean `HEAD` cache if it exists
- on a clean workspace, compare against the previous clean commit cache if it
  exists

Add `--enforce-regression-policy` when a run should exit nonzero if the
resolved compare target fails the default regression gate. That flag requires
an explicit or cached compare baseline.

After a dirty run becomes the new checkpoint, promote it into the clean commit
key:

```bash
rtk proxy cargo run -p pd-eval -- promote-cache fixtures/packs/terminal_bot_lab_suite.json
```

Run the same matrix with the full seed tier:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/terminal_bot_lab_full.json --workers 8
```

Run the trajectory-error matrix:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/terminal_traj_err_suite.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/terminal_traj_err_full.json --workers 8
```

Run the experimental terrain backstop diagnostics:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/experimental_terrain_backstop_suite.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/experimental_terrain_backstop_full.json --workers 8
```

Run the first transfer-guidance smoke matrix:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_bot_lab_suite.json --workers 8
```

Run the nominal-radius route-angle diagnostic matrix:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_route_angle_suite.json --workers 8
```

Run the transfer radius-tier diagnostics:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_radius_tier_suite.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_route_angle_radius_suite.json --workers 8
```

Run the full-seed transfer reliability and frontier packs:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_route_angle_radius_full_solved.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_route_angle_radius_frontier_full.json --workers 8
```

Run the route-wide waypoint landing and contract packs:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_route_angle_smoke.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_contract_route_angle_smoke.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_route_angle_smoke.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_contract_route_angle_smoke.json --workers 8
```

The two legacy `planner_generated_route_*_smoke` packs are retained archive
metadata, not runnable planner gates. Use `planner_v2_lab_suite` for planner
evaluation; authored waypoint-guidance packs below remain supported.

Run the full-seed nominal and all-radius waypoint closure packs:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_route_angle_full.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_contract_route_angle_full.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_route_angle_full.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_contract_route_angle_full.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_route_angle_radius_smoke.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_contract_route_angle_radius_smoke.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_route_angle_radius_smoke.json --workers 8
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_contract_route_angle_radius_smoke.json --workers 8
```

Force a rerun and skip cache reuse if needed:

```bash
rtk proxy cargo run -p pd-eval -- run-pack fixtures/packs/terminal_bot_lab_suite.json --workers 8 --no-reuse
```

Use `terminal_bot_lab_suite` as the primary controller workbench. It is the
smoke-tier Earth `half_arc_terminal_v1` matrix over:

- `condition_set = clean`
- `vehicle_variant = empty`
  - `pylander`-aligned Earth baseline hardware with empty payload
- `vehicle_variant = half`
  - the same hardware with half payload
- `vehicle_variant = full`
  - the same hardware with full payload
- `arc_point x velocity_band`
- one maintained `current` controller lane
- optional cached comparison against an earlier result pack; comparison role is
  report provenance, not another physical-case axis

The maintained terminal baseline now matches the core `pylander`
vehicle/engine envelope closely enough to reason about directly:

- `8m x 10m` hull
- `7200kg` dry mass
- `6300kg` max fuel
- `240000N` max thrust
- `25%` ignited minimum throttle
- `90 deg/s` max rotation rate

The one intentional simplification is fuel use:

- `pd-lab` does not yet model `pylander` overdrive or the nonlinear burn
  penalty above nominal thrust
- fuel burn currently scales linearly between minimum and maximum thrust

In maintained terminal packs, `current` resolves to `terminal_pdg_v1`. Batch
reports prefer cached current-controller history when a compatible clean cache
exists. The older heuristic `baseline_v1` controller remains available for
explicit comparison fixtures, but it is not executed by the maintained clean
or trajectory-error packs and is not the project progress signal.

Use `terminal_bot_lab_full` when the same matrix should run with the full
seed tier for spread measurement. The `terminal_compare_*_fixture` packs are
only for smoke-testing pack-vs-pack compare output.

Use `terminal_traj_err_suite` and `terminal_traj_err_full` when the same
Earth/payload matrix should exercise projected miss conditions. These packs use
current-lane-only runs over:

- `traj_undershoot_small`
- `traj_undershoot_large`
- `traj_overshoot_small`
- `traj_overshoot_large`

The clean matrix keeps small seed-level radial/speed jitter. The trajectory
error matrix instead owns the lateral miss as a condition-set perturbation:
undershoot stays short on the approach side, overshoot crosses to the far side,
and the configured small/large projected miss magnitudes are recorded in each
resolved run.

Use `experimental_terrain_backstop_suite` and
`experimental_terrain_backstop_full` only for non-blocking terrain diagnostics.
They run the same Earth terminal matrix machinery, but they are explicitly not
part of the maintained terminal guidance scorecard. These packs are
current-lane-only over `empty` and `half` payload tiers, use the `diagnostic`
expectation tier, and include condition sets:

- `terrain_backstop_wall`
- `terrain_backstop_slanted`

The two backstop variants are shape variants, not low/medium height bands: both
use a `400m` terrain rise so they behave more like a wall or cliff than a small
obstacle.

The experimental terrain packs intentionally prune terrain-blind high-arc cells:
backstop entries keep `a70/a80`. Both `terrain_clip` and backstop containment
are parked as terminal-controller objectives until terrain work is reframed as
approach-corridor validation, waypoint planning, or collision-course warning.

The terminal controller contract is narrower: given a reachable, terrain-valid
approach corridor or target, land safely. Terrain condition metadata is for
diagnostics and reports, not controller mode switches.

`transfer_bot_lab_suite` is the first Phase 3 source-to-target matrix and the
fast transfer smoke gate. It uses `transfer_matrix =
signed_route_arc_transfer_v1`, the default nominal `800m` route radius,
route-angle labels from `r-60` through `r+60` in smoke tier, and the staged
`transfer_pdg_v1` controller. Transfer reports label the matrix axes as route
and radius instead of terminal arc and velocity band.

`transfer_route_angle_suite` runs the same controller, payload tiers, and fixed
nominal radius, but expands to all 11 signed route angles from `r-80` through
`r+80` with smoke seeds. It is the nominal-radius route-shape diagnostic pack.

`transfer_radius_tier_suite` keeps the smoke route-angle set and expands across
`short = 400m`, `nominal = 800m`, and `long = 1200m` radius tiers. It is the
fast distance-sensitivity gate.

`transfer_route_angle_radius_suite` combines all 11 route angles with all three
radius tiers for a 297-run wide smoke diagnostic.

`transfer_route_angle_radius_full_solved` expands the solved direct-transfer
partition to full seeds: all route angles from `r-80` through `r+60`, all three
radius tiers, and all three payload tiers. The separate `r+80` partition now has
its own focused steep-uphill regression; the old partition names remain.

`transfer_route_angle_radius_frontier_full` retains the historical name for the
separate `r+80` full-seed partition across all radius and payload tiers. It is now
a focused steep-uphill regression, not an unresolved controller failure region.


That writes:

- `pack.json`
- `resolved_runs.json`
- `summary.json`
- `report.html`
- optional `compare.json`
- per-run bundles under `outputs/eval/<pack>/runs/`

Batch output keeps stable semantic run directories for inspection while also
recording stable digests for the resolved pack and resolved run set.

The batch report is intentionally compare-friendly:

- a selector-aware review tree as the main drill-down surface
- explicit report context near the top of the page:
  - standalone
  - lane compare
  - external compare
  - compare basis
  - scope resolution
  - compare status
- a regression-policy panel and overview chip for compare runs
- optional candidate-vs-baseline deltas over shared run IDs
- stable links back to per-run detail reports and bundles

At this point the batch/single-run reporting stack and cache workflow are good
enough for real controller iteration. The evaluator can now:

- reuse and promote batch caches
- prefer cached current-lane history compare by default
- classify analytically impossible terminal runs separately from scored
  failures
- annotate low-thrust/high-energy frontier cells without removing them from
  scoring
- evaluate a default thresholded regression policy over compare runs, scoped to
  the preferred current controller lane when both reports contain one
- record transfer handoff diagnostics in per-run review metrics, including
  terminal entry kind, handoff gate, handoff height/speed, handoff projected
  `dx`, handoff angle, boost-cutoff quality/projected `dx`, and
  Pylander-inspired shape metrics
- render transfer-specific `Transfer Handoff Triage` and `Transfer Shape
  Triage` sections ahead of the Review Tree so transfer tuning starts from
  handoff/gate/cutoff quality before visual shape

## Retained controller checkpoints

The following values are preserved controller snapshots from the former root
README, not new evaluations on the documentation source. Interpret them with
[Terminal Suite Design](terminal_suite.md), [Transfer Suite Design](transfer_suite.md)
and the selected saved reports. Do not merge their denominators with the current
V2 planner pack or treat dated timing measurements as a current host guarantee.

Checkpoint on the maintained Earth payload tiers:

- `terminal_bot_lab_suite`
  - `current`: `171 / 180` scored successes, `9` scored failures,
    `9` impossible warnings, `12` frontier annotations
- `terminal_bot_lab_full`
  - `current`: `686 / 720` scored successes, `34` scored failures,
    `36` impossible warnings, `48` frontier annotations
  - by vehicle tier:
    - `empty`: `252 / 252`
    - `half`: `252 / 252`
    - `full`: `182 / 216` scored, `34` fail, `36` impossible warnings,
      `48` frontier annotations

Trajectory-error checkpoint:

- `terminal_traj_err_suite`
  - `current`: `694 / 720` scored successes, `26` frontier failures,
    `0` core failures, `36` impossible warnings, `48` frontier annotations
- `terminal_traj_err_full`
  - `current`: `2772 / 2880` scored successes, `107` frontier failures,
    `1` core failure, `144` impossible warnings, `192` frontier annotations
  - by condition:
    - `traj_undershoot_small`: `698 / 720` scored, `22` frontier failures,
      `36` impossible warnings, `48` frontier annotations
    - `traj_undershoot_large`: `708 / 720` scored, `12` frontier failures,
      `36` impossible warnings, `48` frontier annotations
    - `traj_overshoot_small`: `684 / 720` scored, `36` frontier failures,
      `36` impossible warnings, `48` frontier annotations
    - `traj_overshoot_large`: `682 / 720` scored, `37` frontier failures,
      `1` core failure, `36` impossible warnings, `48` frontier annotations
  - by vehicle tier:
    - `empty`: `1008 / 1008`
    - `half`: `1007 / 1008`, `1` core failure
    - `full`: `757 / 864` scored, `107` frontier failures,
      `144` impossible warnings, `192` frontier annotations

Experimental terrain diagnostic snapshot:

- `experimental_terrain_backstop_suite`
  - `current`: `57 / 72` scored successes, `15` scored failures
  - `3.24s` wall clock with `8` workers
- `experimental_terrain_backstop_full`
  - `current`: `228 / 288` scored successes, `60` scored failures
  - `12.73s` wall clock with `8` workers
  - the first generic terrain-clearance candidate constraint is in place
  - `terrain_clip` is parked until it can test localized avoidance without
    forcing route-level replanning
  - the backstop packs are also parked outside the maintained terminal guidance
    scorecard

Transfer route-angle checkpoint:

- `transfer_route_angle_radius_suite`
  - `current`: `297 / 297` successes, `0` crashes, `0` invalidations
  - the maintained smoke matrix is clean across all route angles, radius tiers,
    payloads, and seeds
- `transfer_route_angle_radius_frontier_full`
  - `current`: `108 / 108` successes and `0` invalidations
  - the historical frontier name now denotes a focused steep-uphill regression,
    not a current failure region
- `transfer_waypoint_turn_contract_smoke`
  - `current`: `81 / 81` pass-through handoff successes
  - every normalized gentle, medium, and sharp waypoint contract passes without
    route/profile controller branches
- `transfer_waypoint_turn_smoke`
  - `current`: `81 / 81` final landings
  - retained terminal horizons now release when their attitude-aware vertical
    braking margin is exhausted; waypoint contract quality remains `81 / 81`
- `transfer_waypoint_sequence_smoke`
  - `current`: `27 / 27` final landings across the maintained double-bend corpus
- `transfer_waypoint_sequence_contract_smoke`
  - `current`: `27 / 27` ordered sequence successes
  - all `54` handoffs satisfy the planned tangent/energy contract at window
    entry and resolve as `contract_pass`
- `transfer_waypoint_turn_contract_route_angle_smoke`
  - `current`: `135 / 135` handoff successes over
    `r-60 | r-30 | r00 | r+30 | r+60`
- `transfer_waypoint_turn_route_angle_smoke`
  - `current`: `135 / 135` final landings with `0` invalidations
  - final-waypoint states are ranked by terrain-blind terminal recoverability
    after satisfying the waypoint contract
- `transfer_waypoint_sequence_contract_route_angle_smoke`
  - `current`: `45 / 45` ordered sequence successes
- `transfer_waypoint_sequence_route_angle_smoke`
  - `current`: `45 / 45` final landings with `0` invalidations
  - all `90` ordered handoffs and all `45` final landings complete cleanly
- full-seed nominal waypoint closure:
  - turn landing and contract: `540 / 540` for both goals
  - ordered landing and contract: `180 / 180` for both goals
- all-radius waypoint closure:
  - turn contract and paired landing: `405 / 405` for both goals
  - ordered contract and landing: `135 / 135` for both goals
  - bounded final authority-recovery search closes the former
    `single_gentle_bend_v1/full/r-30/short/seed 02` landing residual
- `transfer_waypoint_sequence_late_bend_diagnostic`
  - archived capture: `27 / 27` final landings and complete route telemetry
  - `27 / 54` handoffs enter the capture radius outside the envelope, then
    recover before the waypoint plane; this profile is diagnostic, not a gate
- smoother `r+80` bend reset:
  - landing: `15 / 27` smoke and `54 / 108` full
  - handoff contract: `21 / 27` smoke and `89 / 108` full

The waypoint corpus uses fixed route-frame geometry rather than silently lifting
waypoints in world Y. Maintained ordered waypoints carry an explicit handoff
tangent: the normalized inbound/outbound angle bisector. Spatial radius entry
opens an acceptance window; guidance keeps the active leg until the contract
passes or the craft reaches the waypoint plane. Maintained handoff envelopes
also cap energy and reject fixtures whose optimistic stopping-distance ratio
exceeds `0.75`. Schema `34` reports the immutable plan tangent, window-entry
snapshot, final resolution reason, window duration, and final-terminal
recoverability separately. Full-seed nominal closure is `540 / 540` for turn
landing/contracts and `180 / 180` for ordered landing/contracts. All-radius
contracts are `405 / 405` turn and `135 / 135` ordered; paired landings are
`405 / 405` and `135 / 135`. Final-state ranking is terrain-blind and uses
terminal braking authority rather than route/profile labels. Controller compute
remains below the `1ms` p99 budget.

Execution of the four old `single_dogleg_v1` packs and the full-matrix
`late_bend_v1` pack is retired, including profile reuse in renamed/custom packs.
Their fixture metadata and saved reports remain readable diagnostic history;
they are not acceptance gates or executable experiment frontdoors.
Terrain-blind waypoint guidance v1 is closed over the preplanned maintained
corpus. The former chord-based V1 planner is retired; its schema-36 contracts and
saved `54 / 54` landing and `36 / 36` handoff evidence remain readable history.
The active planner is V2's terrain-blind nominal / local-clear / actual-H replan
loop described above. Broader terrain coverage and game-host integration remain
separate follow-ups, not capabilities established by this cleanup.
The guidance implementation now follows the ownership boundaries in
`docs/guidance.md`: terminal and transfer are separate modules, pure waypoint
geometry is isolated from controller lifecycle state, telemetry emission is
separate from control decisions, controller tests live outside production
modules. Rejected boost scorers are retired executable modes; their saved
diagnostics remain readable.
Detailed checkpoint history lives in `docs/progress.md`,
`docs/transfer_suite.md`, and `docs/terminal_suite.md`.
