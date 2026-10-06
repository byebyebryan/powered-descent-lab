# Practical waypoint planner V2 plan

> Historical record: superseded executable commands are retired. Use the
> [current documentation](../README.md#docs) for current tooling. Links to retired source
> below point to verified pre-retirement Git revisions, not live modules.

Current next-phase design is the
[unified nominal plan](waypoint_v2_unified_nominal_plan.md): common state-aware
construction, finite acquisition estimates and preservation of already-safe
vertical motion. The [goal amendment](waypoint_v2_goal_amendment.md) records the
updated objective and the stored goal-management limitation.

This file retains the original implementation contract, not the next runtime
policy specification. [Results](waypoint_v2_practical_results.md) record the
complete loop and 12/16 ordinary landings after its single allowed revision.
The 13/16 acceptance floor remains unmet; the old calibration budget is spent.

## Decision and scope

Build an experimental, reusable V2 flight mode for pd-lab's game-like simulator.
It should handle most ordinary terrain layouts with bounded effort, not guarantee
landing from every state. The user confirmed the first scope: the currently
tested vehicle, Earth gravity and 120 Hz physics / 60 Hz control, with broad
heightfield coverage. Other vehicles, gravities, reverse travel, disturbances and
real-time planning are not prerequisites for this version.

Require route-free LandingOnPad input throughout. Existing initial nominal
preflight can accept an empty authored route, but airborne admission cannot;
V2 must reject authored routes up front, not discover that mismatch after flight.
Do not normalize the mission silently.

This is a design checkpoint dated 2026-10-01, not an implemented planner or new
flight result. [The review](waypoint_v2_practical_plan_review.md) records readiness
checks. Implementation, new flight measurements, commits and default promotion
require subsequent authorization. Production V1 remains unchanged.

The practical acceptance bar is eight retained clear-route controls, success on
at least thirteen of sixteen ordinary obstacle cases, and understandable bounded
failures elsewhere. Correct state, commands and reporting remain mandatory;
universal reachability, robust viability and perfect landing do not.

## What the research establishes

The [canonical initial foundation](canonical_initial_direct_canary_results.md)
lands eight uncut flat/uphill/downhill controls. Nominal generation is independent
of interior terrain; its selected commands are then audited against actual terrain.
The [airborne foundation](nominal_airborne_direct_canary_results.md) supports fresh
generation at twelve real captures, without clock, fuel or state reset.

The [local clearing foundation](local_clearing_canary_results.md) proves one useful
correction and independent whole-source replay. Its actual handoff H is tick 2820,
at (-337.771, 382.821) m with velocity (38.313, -10.903) m/s. Fresh nominal generation
works there, but the selected continuation first violates reserve at tick 3120.
The active flight is still at H; the blocked suffix was only audited privately.

The missing capability is repeated composition, not another source-launch study.
The current experiment searches four entries related to the original source
bridge and has a one-correction proof guard. Neither is a general repeated loop.

As a research cross-check, [MIT's trajectory optimization notes](https://underactuated.mit.edu/trajopt.html)
describe planning from current state and executing a prefix repeatedly, while
distinguishing that pattern from guaranteed future feasibility. V2 uses this
piecewise principle, not an MPC optimizer or a recursive-feasibility proof. Our
two-second local continuation remains a useful heuristic filter, not a guarantee
that every subsequent correction exists.

An analytical example supports attempting the next correction with the existing
family. Starting from measured H, a constant upright 13.32 m/s² thrust model with
the existing semi-implicit 120 Hz arithmetic predicts a three-second boost and
one idle hold reaching tick 3182 at roughly (-222.193, 365.943) m, then tick 3422
at (-145.567, 345.169) m after the separate two-second coast. This omits physical
held-pair/fuel validation and full body queries: it is a hypothesis, not an
accepted maneuver, winning-row seed or landing witness. The duration already
belongs to the existing grid; no new optimizer is motivated by this arithmetic.

## Planning and execution loop

Retain one active in-memory flight and its complete consumed-command ledger.
Offline planning pauses simulated motion. Each cycle starts at current boundary C:

1. At source rest, generate the existing canonical initial proposal. At a later
   idle-coast handoff, generate the existing airborne nominal proposal from the
   actual state and original absolute deadline.
2. Select nominally once, without interior-terrain ranking. Independently audit
   that fixed proposal on actual terrain, retaining the first genuine conflict.
3. If Direct, execute the accepted supplied commands to actual target contact
   and replay the whole accumulated flight. No clearing waypoint is added.
4. If TerrainBlocked, search a local correction at safe intervention entries
   before that conflict. Select for local clearance, progress and admission;
   do not generate a suffix or test landing while ranking local candidates.
5. Execute only the chosen safe nominal prefix C through E, then its correction
   E through H. Keep the actual H, append consumed commands, and repeat from H.
6. If a family is exhausted or a limit is reached, stop with a typed incomplete
   result. Do not execute an unsafe nominal suffix or supply implicit commands.

Nominal Unknown and Unsupported are separate from terrain blockage. With no
nominal proposal there is no proven conflict to clear; this first V2 records
NoNominal or Unsupported rather than inventing an obstacle. These are legitimate
coverage misses and remain in the ordinary-suite denominator. A later recovery
primitive is not part of this build.

A valid fixed audit can also reject nominal terminal behavior without finding
an interior obstacle, for example unsafe target contact or no contact before
explicit coverage ends. With intact consumed-prefix/state/contact parity,
classify this as NominalRejected and stop without executing that suffix. Do not
label it TerrainBlocked or ImplementationError just because it did not land.
The existing one-clearing canary's stricter composition gate stays unchanged.

Local handoff is a planning boundary, not mission success, target contact or a
new authored checkpoint. A single long feature may need several corrections;
feature far edges are optional diagnostics, never local acceptance requirements.

## Intervention and local selection

For the initial blocked proposal retain the existing four source-relative entry
formulas, including the first complete idle hold. They are computed from the new
proposal's source handoff, not stored ticks or archived states. Exclude entries
at/after conflict or failing full local admission.

For each subsequent blocked proposal use at most four entries:
`E(f) = C + 2 * floor(f * (F-C) / 2)`, for f in 0, 0.25, 0.5, 0.75, where F is
the new first-conflict tick. Deduplicate and require C <= E < F. C must be an
aligned, supported running handoff. For the reference H/F these entries are
2820, 2894, 2970 and 3044. Starting at C is allowed and may be necessary.

Obtain future E states by forward query propagation of the single selected
nominal proposal from actual C. Keep every preceding state safe under that
proposal's existing phase-specific clearance policy. Never query backwards
before C or restore a snapshot. Execute the selected C-to-E prefix on the active
flight and verify the actual E matches its full query state before switching.

The existing canonical first-conflict helper queries from source rest. Do not
call it with only an airborne suffix: add a live-origin query from C, matched
to the new fixed audit, for conflict position and complete diagnostic state.
An independent whole-source proof includes the entire already consumed ledger.

The initial local grid remains forty-two templates per entry: attitudes -30, 0,
+30 degrees, acceleration factors 0.75 and 1, and powered durations 60, 120, 240,
360, 480, 720, 960 ticks. Keep actual fuel, held-pair throttle conversion and
rate-limited attitude. Then examine the existing 360 aligned handoff boundaries
over at most 720 coast ticks, using one trace and reusable 240-tick windows.

Retain the existing acceptance rules: every local state has full 5 m conservative
rotated-body clearance and no contact; H has unchanged airborne admission; its
x reaches max(entry x, new conflict x) plus one conservative body diameter; a
separate two-second idle/upright continuation preserves clearance and admission.
Do not consume the certificate and then plan from stale H.

Rank locally by latest eligible E, earliest eligible H, least actual fuel burn,
then stable row identity. Do not choose a more expensive row merely because its
fresh direct suffix lands. A later NoNominal result remains a recorded heuristic
miss rather than retrospective end-to-end selection.

Keep the core family initially. After a complete development matrix, allow at
most one explicit uniform V2 policy revision targeting a demonstrated common
failure: entry spacing, maneuver coverage or local ranking. Preserve the first
run, version the changed policy and repeat the same matrix. Do not change its
geometry, remove failures, relax reserve/deadline or modify the sealed V1 canary.
Further revisions need another decision; this is a practical calibration budget,
not a prohibition on ever improving the planner.

## Bounds and stop semantics

Proposed first-version limits are six completed corrections, four entries per
correction, forty-two templates per entry and 360 handoff checks per template.
Thus at most 1,008 local rows and 362,880 handoff checks are possible. Each trace
has at most 960 powered + 720 coast + 240 certificate ticks, before clipping at
the original deadline. These are local-search bounds, not total optimizer work.
Nominal generation also retains its existing finite internal bounds.

Permit the final nominal attempt after correction six; the cap prevents a seventh
correction, not a Direct landing after the sixth. There are at most seven nominal
cycles. Before planning/stepping require remaining original time and positive
fuel. Keep the original min(scenario horizon, 80 s planning budget); no cycle gets
a new horizon. Every completed H must strictly advance global time and satisfy
the body-diameter progress threshold against its current conflict. Identical
feature identity alone is not a loop: progress above a plateau is valid.

Report separate planning stops such as NoNominal, NominalRejected, NoClearing,
CorrectionLimit, Deadline, NoProgress, Unsupported and InvalidInput. Report actual physical
outcome separately: landed, contact/crash, or still flying/incomplete. A finite
NoClearing is not a physical impossibility claim. Incomplete gameplay simulation
is allowed; there is no automatic abort or recovery guarantee.

Unexpected gaps within declared command coverage, invalid bindings, nonfinite
state and replay mismatch are ImplementationError, not ordinary difficulty or
controller edge cases. Preserve
the full partial state/contact/log evidence and stop. Never call an incomplete
prefix a successful mission.

Intentional coverage exhaustion without contact is an incomplete planning result,
not a missing in-bounds command. Use NoProgress as the local-exhaustion subcause
when safe rows fail the progress test; a selected proposal violating its own
validated progress binding is instead ImplementationError.

## Architecture and execution rules

Use a new additive `pd-eval/src/waypoint_v2.rs` runner and
`pd-plan/src/waypoint_v2.rs` pure policy/entry/budget contracts. Keep plant-dependent
generation and auditing in pd-eval for this version. Do not force timed V2
segments through V1's chord-based RoutePlan/all-leg validator or introduce an
evaluator dependency into pd-plan.

Expose the pure module under the existing research feature used by pd-eval;
adding the experimental mode must not change default Planner::plan dispatch.

Extract only the reusable local row search, validation and ordinary segment
execution helpers with narrow crate visibility. Keep all current canary wrappers,
input seals and selection semantics intact. If useful, put shared active-state
and log ownership in `pd-eval/src/segmented_flight.rs`; it calls existing core
transitions and adds no integrator, restore API or contact policy.

The segment ledger records initial nominal prefix, local correction, later
nominal prefix and final nominal suffix, each with start/end ticks, supplied
commands, entry/handoff identity and the applicable audit. Segments own commands
in [start,end), with no duplicate E/H or command at a flying endpoint. Keep
controller ordinals tick/2, the existing sample cadence and actual pre-normalized
IncomingContact evidence. Final contact may occur at an odd tick.

The current LocalGuard assumes everything after first E is local; do not reuse
that assumption for later nominal flight. Whole-source proof must use the actual
segment ledger and command phase. Only the original source nominal prefix may
use the existing launch exception. Every local segment/certificate requires full
5 m reserve. Later nominal flight uses its existing airborne/terminal clearance
rules, with no source exception. Legitimate target-pad terminal contact is not
rejected by a blanket 5 m local guard. Reuse existing queries, not forked equations.

Use existing source-based bounded execution/replay for independent prefix proofs.
Those APIs start at source; continuing the active state uses the extracted live
helper, not repeated calls that reset state. Whole landing replay stitches all
actually consumed segments, not the obsolete canonical program through H.
Early terrain audits compare the consumed prefix, not full nominal completion.

The first execution backend uses the already validated supplied-command
primitives. This is not a claim that legacy `transfer_pdg` tracks every V2 arc.
An optional controller comparison must be reported as a separate lane; do not
attribute deterministic audit/execution disagreement to a tolerated controller
failure. Controller redesign is not an implementation prerequisite here.

Expose reusable `run_waypoint_v2_flight(request, policy)` and an explicit
`waypoint-v2-flight --scenario PATH --source-pad-id ID --target-pad-id ID
--output-dir NEW_ROOT` mode, with input-only preflight analogous to current
nominal mode. These names are proposed, not available commands. Accept scenario
input, not archived programs or expected winning rows. Emit a compact summary,
segment/decision ledger and ordinary trajectory report; full rejected-boundary
diagnostics can be separate opt-in output. Do not invoke preservation canaries
or repeatedly write full traces during normal flight planning.

## Representative mission suite and acceptance

The [input recipes](../fixtures/research/waypoint_v2_practical_suite_plan_v1.json)
pin three existing base manifests by hash and define deterministic new terrain.
The [read-only checker](https://github.com/byebyebryan/powered-descent-lab/blob/5ab3d84281d1ff78fdd73425881e96aa86ebd9ac/scripts/check_waypoint_v2_practical_plan.mjs) resolves
complete scenarios in memory and tests geometry, accounting and existing input
preflight. New vertices add height above the existing flat/sloped base; source
and target shelves, vehicle and clock remain unchanged. This is an exposed
development suite, not held-out coverage or a statistical game-wide guarantee.

| Group | Cases | Acceptance |
| --- | ---: | --- |
| Existing clear flat/uphill/downhill controls | 8 | All land with zero corrections |
| Ordinary ridges | 4 | At least 2 land |
| Ordinary plateaus | 4 | At least 2 land, including unchanged 900 m reference |
| Successive obstructions | 4 | At least 2 land; no prescribed waypoint count |
| Obstructions on uphill/downhill bases | 4 | At least 2 land |
| Diagnostic high/long/near-pad/narrow-pad and unsupported setups | 8 | Honest bounded outcomes; no landing quota |

Require at least 13/16 ordinary landings (81.25 percent), in addition to each
family's floor. Unsupported, NoNominal and NoClearing ordinary cases remain
failures in that fixed denominator. All eight clear controls retain their
declared nominal selection and commands. At least twelve ordinary cases must
actually block the initial fixed nominal proposal, so clear terrain twins cannot
inflate obstacle coverage. Confirm this during the first generation run, not
from preflight. If that discrimination requirement fails, report inadequate
suite coverage; do not tune obstacles after measuring this matrix.

The reference plateau must show repeated correction and eventual landing before
calling this a usable loop. Its first cycle preserves the old local physical
result; do not assert a second winning row or exactly two total corrections.
For a known blocker, do not pass on the basis of a taller terrain-selected
initial direct arc. Report planning coverage, terminal success, correction count,
minimum clearance, fuel, typed failures and timing separately. Optional feedback
controller landings do not replace supplied-command execution evidence.

Measure release planning-only cost on the same host, excluding compilation,
report writing and historical-preservation work. Use all 24 clear/ordinary
attempts in each final repeat, including unsuccessful attempts; exclude the
eight diagnostics from this performance denominator. Use nearest-rank p95 and
report sample count. A provisional interactive goal is median <=2 s and p95
<=5 s per mission attempt. Report actual cost and traces;
this goal is unmeasured, not a real-time deadline. If missed, remove research-only
overhead or optimize measured hotspots before labeling performance usable, rather
than opening another dynamics investigation. Compact output must not hide failures.

Include generation, live-origin queries, terrain audits, local search and selected
proposal validation in planning time. Report active execution, independent replay,
artifact writing and total user wait separately; the planning metric alone does
not establish end-to-end interactive performance.

## Implementation and validation sequence

1. Add pure V2 contracts, later-entry formulas, bounded stops and segment ledger.
   Extract shared execution/search helpers without changing old policies. Test
   aligned clocks, odd conflicts/contact, short intervals, deduplication, checked
   overflow, original budget, six-correction/final-Direct behavior and no reset.
   Test valid non-terrain terminal rejection versus corrupted evidence separately.
2. Build the full generic loop, not a hardcoded second-waypoint script. Start with
   clear control and the unchanged reference plateau. Verify actual C-to-E and
   E-to-H execution, separate certificate and all accumulated source replay. An
   honest finite miss informs coverage; a parity/binding defect stops integration.
3. Add explicit scenario CLI, compact diagnostics and trajectory report. Test
   create-only roots, malformed/unsupported input, no implicit fallback, and
   correct incomplete outcome. No default or V1 schema switch is included.
4. Run all 32 development cases. Keep ordinary misses rather than stopping the
   suite at its first difficult mission; stop on actual implementation/evidence
   defects. Decide on the one allowed uniform revision from recorded causes,
   then rerun the same suite without case replacement.
5. On final source, repeat the suite in a new root; require full deterministic
   decisions, commands, state/contact and replay agreement, excluding only named
   observational timing/path fields. Run workspace tests, strict Clippy, fmt and
   diff checks. Preserve eight canonical controls, twelve earlier airborne
   continuations, the sealed one-clearing canary and 24 source-rest controls;
   expanded source envelopes may differ, physical payloads may not.

Primary owns architecture, local acceptance, integration, review and verdict.
If a worker loop is subsequently requested, bounded ownership can split pure
contracts/tests from CLI/report/suite harness after those interfaces are settled.
Workers must preserve one another's changes. No delegation is required by this
design pass, and no runtime implementation is authorized by saving this plan.

## Exit and investment decision

Call the result an experimental usable V2 for the declared setup only after the
coverage, reference-flight, integrity and measured-performance gates pass. Known
diagnostic failures remain documented; do not imply arbitrary-state or real-world
safety. Default promotion is a separate choice, not an acceptance prerequisite.

If coverage still misses the threshold after the bounded uniform revision, report
the actual dominant gap and decide whether another focused improvement is worth
it. Do not restart launch physics, add a global optimizer or keep producing seam
canaries without a concrete failure motivating that work. The deliverable of the
next implementation phase is the complete reusable loop and its mission results,
not another isolated handoff proof.
