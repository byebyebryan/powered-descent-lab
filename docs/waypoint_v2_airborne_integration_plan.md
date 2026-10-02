# Waypoint V2 airborne integration plan

## Decision and scope

This 2026-10-02 plan recommends the smallest next usability experiment: retain
the working uncut ground constructor and policy 2 local-clearing behavior, and
replace only airborne nominal regeneration with the tested state-aware
acquisition and terminal-time constructor. Add one explicit opt-in policy 3;
policies 1/2 and the default remain unchanged. Saving this plan does not start
implementation, a flight run or a new active goal.

This deliberately narrows the previous recommendation to replace both ground
and airborne construction together. Shared target, state-continuity and safety
semantics do not require identical startup actuator templates. Ground research
8/8 is retained evidence, not discarded work; replacing the already-working
runtime launch is deferred until there is a concrete benefit. A usable
game-oriented loop need not wait for that internal unification.

A legacy-first rescue branch that invokes the new constructor only after old
NoNominal is a viable alternative for minimizing changed flight choices. It is
not selected here: it retains two nominal searches and the old 56 physical
trials before recovery. Airborne replacement gives one bounded family per
airborne origin and a clear comparison against policy 2. If replacement fails
the matrix, that rescue approach can be considered in a later decision; it is
not an automatic fallback or an extra variant in this experiment.

The intended outer logic remains: construct one terrain-blind nominal flight,
validate execution, audit terrain, then either land directly or clear one local
obstruction and replan from actual handoff. A waypoint still does not require a
landing suffix. Current scope is the tested vehicle, Earth gravity, 120/60 Hz,
forward route-free transfers and heightfield terrain, not perfect arbitrary-state
recovery or production/default promotion.

## Research findings supporting the smaller pass

The [terminal-time results](waypoint_v2_terminal_time_results.md) establish 8/8
research ground landings and preserve 39/39 airborne witnesses. The existing
runtime already lands 8/8 uncut clear controls and 12/16 ordinary cases; its four
ordinary misses are NoNominal after locally safe handoffs. All four have
terrain-safe recoveries in the research corpus. This supports integration as
the next experiment, not a prediction of sixteen complete mission successes.

Fresh read-only inspection confirms all 39 retained airborne inputs fit the
existing aligned, running, forward, idle-thrust runtime family. Thirty first
valid proposals use zero acquisition, three use natural-profile lateral
acquisition and six use upward shaping. Eight of the nine powered selections
have no analytically admissible zero-acquisition entry; the remaining row's
first zero-acquisition trial fails the unchanged backend shape check. Do not
delete the powered family to simplify the code.

The first executable proposal appears on attempt one in 38 rows and attempt two
in one. Selecting the first executable proposal would require 40 trials on
those same ledgers instead of the research run's 114, with exactly the same
selected commands. This is a projection from retained results, not a new flight
experiment or measured runtime speedup. Three attempts remain the cap for new
handoffs; do not reduce it to one or two based on this small sample.

Two concrete integration hazards explain why replacing launch simultaneously
is unnecessary risk:

- Initial correction entries assume the canonical 72-tick launch offset and
  source handoff. In one 915 m control, old S is 1992 while the new research
  acquisition ends at 1178, after preparation at 204. Mechanically plugging that
  new endpoint into the old spacing rule changes entries from
  1512/1272/1032/792 to 900/762/624/486. That is an unvalidated local-policy change,
  not a harmless constructor swap or a newly measured flight.
- The old airborne proposal/audit describes only coast then terminal thrust.
  A state-aware proposal can also contain acquisition turn and burn. Its phases
  cannot be hidden inside a larger coast count or retagged as the old policy.
  The audit must use the actual consumed command phase.

## Simplifications to adopt and exclusions to keep explicit

| Area | First usable integration choice |
| --- | --- |
| Pad launch | Keep canonical construction, startup contact handling and policy 2 initial entries unchanged |
| Airborne origin | Accept only actual running, aligned forward/coasting handoffs in the existing supported setup |
| Nominal choice | Keep the tested finite family and ranking; stop at the first executable proposal |
| Physical retries | At most three shortlisted trials; only finite executable rejection permits the next trial |
| Terrain | Audit exactly the selected proposal; never retry a higher arc because terrain blocks it |
| Local clearing | Keep four entries, forty-two templates, existing progress test, two-second certificate and six-correction cap |
| Recovery coverage | Leave reverse transfers, target overshoot recovery, arbitrary powered replanning origins, other vehicles and gravities outside this pass |
| Backend shape edge cases | Preserve the current rejection and bounded fallthrough; do not add a dive-and-recover solver |
| Reporting | Extend existing CLI, flight ledger and suite runner; no new simulator, optimizer or reporting framework |

Near-zero lateral speed retains the tested baseline-only behavior; it does not
gain another duration family. Scope exclusions cannot remove ordinary missions
from the frozen denominator. An unsupported or exhausted handoff is an honest
partial mission failure, not evidence of physical impossibility.

Do not cut ordinary safety contracts while cutting unusual recovery modes.
Odd touchdown endpoints, command ownership at phase boundaries, source contact
versus airborne reserve, changing mass, original deadline and raw safe contact
all remain required. No snapshot restore, invented velocity, fuel/clock reset,
instant attitude change, lowered floor or relaxed landing margin is allowed.

## Runtime construction contract

Provide one live-airborne facade taking the existing context, actual
`SimulationState` and original absolute deadline. It must not load a corpus,
saved prefix, research summary or expected answer in normal runtime use.
Retained fixtures are validation inputs only. Ground preparation is never called
through this facade.

Use the existing seed construction, acquisition estimates, timed entry screening
and rank semantics without another numerical policy. Bounds remain thirteen
seeds, three turn-consistency updates, four coast identities, three durations
per coast and three physical trial attempts. The analytical ceiling is 156
reference constructions per nominal call. Valid baseline times remain untouched.
Do not append the old 56-row airborne search as a fallback; that would increase
work and make failures harder to attribute.

Separate shared command realization and free-space target-plane validation from
the research wrapper's real-terrain reporting. A narrow helper extraction is
permitted; duplicating its numerical implementation or copying the research
corpus runner is not. Keep the old research wrapper's observable outcomes and
old runtime identities unchanged, verified against retained payloads rather
than assuming a refactor is harmless.

Rank eligible seeds once, then try at most the first three in that order. A
proposal is executable only after safe target-plane contact, complete global
command coverage, endpoint agreement and independent neutral replay pass.
Return the first executable proposal immediately. A finite backend rejection
permits the next ranked trial; any integrity failure stops the search. Never
use terrain landing/reserve results as this selection predicate.

The runtime proposal must bind its own constructor identity, dynamics, actual
origin, original deadline, complete commands and expected raw endpoint. It must
cover acquisition turn/burn, coast and terminal phases without impersonating
the legacy coast-only proposal. Reuse existing audit result and cycle fields
where possible; a small typed program wrapper is sufficient. A new strategy
plugin system or generalized arbitrary-start interface is out of scope.

## Terrain audit and actual flight contract

Audit the selected complete command schedule from the actual incoming state.
Validate its identity/origin/deadline and held-pair coverage before propagating.
Use the phase of the command consumed by each transition, not elapsed time
compared with the old coast count. Explicit supported airborne phase labels are
`nominal_acquisition_turn`, `nominal_acquisition`, `ballistic_coast` and
`terminal_bridge`; reject malformed phase schedules rather than granting a pad
exception accidentally. Omitted zero-length phases are allowed.

Acquisition and coast require the existing full body reserve. Only descending
terminal motion inside the target pad gets the existing terminal corridor;
airborne planning never gets a source-pad exception. The existing segment guard
already owns post-step boundaries by the last consumed command. Keep that rule
consistent in the new audit. Raw contact inside an odd final held pair remains
a legitimate terminal endpoint, not a reason to round or move the state.

Feed the resulting fixed audit into the existing Direct/TerrainBlocked logic.
An early terrain contact need not consume the hypothetical landing suffix;
prove its consumed prefix, state, raw contact and ordinary/neutral parity.
Only a genuine contact or reserve conflict enables local clearing. A free-space
constructor miss is NoNominal; a valid nonterrain landing rejection is
NominalRejected; unsupported scope is Unsupported. Binding, nonfinite state,
in-bounds command gaps and replay disagreement are ImplementationError with
failed integrity. Reuse these existing stop types rather than expanding the
public taxonomy.

This admission restriction applies to replanning origins, not speculative local
intervention E. Preserve existing local entry admission, which can accept an
actual powered state using its query-only eligibility check. Never replace that
actual state with the temporary idle-thrust query used by the check.

All queries remain forward clones from actual C. Execute only the chosen safe
prefix to E and clearing commands to actual H; do not execute the diagnostic
unsafe suffix or the continuation certificate. Replan at H with its real
position, velocity, attitude, angular rate, held command, fuel and clock. Keep
the accumulated source/official replay proof for complete and partial flights.

## Implementation responsibilities

The primary owns the live-constructor facade/shared realization seam, new
command-program audit and `pd-eval/src/waypoint_v2.rs` integration. Keep the
canonical initial branch and existing local search/rank unchanged. Any helper
visibility/extraction in the body-aware terminal/characterization modules must
be narrow and behavior-preserving; no unrelated core or controller refactor.

The pure named policy 3 addition belongs in `pd-plan/src/waypoint_v2.rs`, with
exact policy validation, the same maximum corrections and policy 2 initial
entries. CLI/parser and existing suite version allowlists/fake-harness tests
must explicitly admit version 3 while keeping default version 1. Existing
report output should need little or no change. These bounded policy/CLI/suite
tasks may be delegated during a subsequently approved worker loop; coupled
construction and review stay with the primary. No agent implementation starts
in this planning pass.

## Staged validation after approval

1. Record the dirty worktree and preservation hashes before edits. Do not commit
   or clean unrelated work implicitly. Add policy dispatch and the narrow facade,
   then focused tests for first-valid selection, finite fallthrough, fail-closed
   integrity, actual origin/fuel/deadline, terrain independence and phase ownership.
2. Reconstruct all 39 retained airborne inputs through their full ordinary
   prefixes. Require the adapter's selected commands, origin and target-plane
   endpoint to match each retained first valid proposal exactly. All four former
   NoNominal rows must retain terrain/reserve/replay recovery. Verify that the
   one finite shape failure consumes one attempt and the next trial succeeds;
   never rerank against terrain. Stop before mission studies if this gate fails.
3. Run four new-policy integration canaries: `v2_clear_845`, `v2_ridge_late`,
   `v2_successive_plateaus` and `v2_plateau_reference_900`. The clear flight must
   preserve canonical commands and land with no correction; late ridge must
   exercise acquisition after actual handoff and land. Successive/reference
   canaries exercise finite fallthrough/repeated clearing and must retain honest
   complete or partial evidence. A finite coverage miss there may proceed to the
   complete matrix; integrity, active safety or replay failure may not. Verify
   both unsupported vehicle/gravity diagnostics reject before simulation.
4. Freeze the implemented policy and source, then run one complete 32-case
   development matrix with the existing runner. Keep every ordinary failure in
   its denominator and report changed subsequent handoffs. If the usability gate
   fails, stop numerical/policy tuning and report the remaining gap; no automatic
   additional grid, ranking or fallback experiment follows.
5. If that matrix passes, run one final-source policy 2 preservation matrix and
   two final-source policy 3 matrices. Policy 2 must retain the old 8/8 clear,
   12/16 ordinary and reference behavior, with physical per-case payloads matched
   under only the existing named timing/path exclusions. Cross-build provenance
   is recorded separately, not declared identical. Policy 3 repeats must match
   the existing full-payload comparison and source/binary/runner provenance rules.
6. Complete workspace/focused/fake-suite/corpus/format/Clippy/preservation gates
   and separate primary acceptance or rejection review. A shared-realizer
   extraction also requires one complete sealed 55-row research reproduction:
   compare retained/synthetic row evidence, traces, metrics and controls against
   the accepted terminal-time result. Only new source bindings and artifact
   identity may differ; do not mask physical or decision differences.

The flight budget is one fixed new policy, at most two passes of the four-case
canary batch, one complete new-policy development matrix, one old-policy
preservation matrix and two new-policy final matrices. An adapter-correctness
retry cannot change acquisition, times, margins, entry spacing, local ranking
or selection rules. There is no policy 4 or numerical revision allowance in this
phase; the earlier policy 2 revision budget remains spent. Retain all failed
roots. Additional complete matrices or coverage tuning require another decision.

## Acceptance and stop conditions

The unchanged complete-loop gate remains:

- All eight clear controls land with zero corrections; initial commands and
  launch/entry behavior remain canonical.
- At least thirteen of sixteen ordinary cases land and each family lands at
  least two of four; no scope-based removal from the denominator.
- The reference plateau lands after at least two corrections; at least twelve
  ordinary initial nominal proposals are genuinely terrain blocked.
- All complete/partial supported flights satisfy integrity, full source/official
  replay, segment continuity, raw contact and safety guards. Diagnostics have
  no landing quota, not an exemption from integrity.
- Planning median remains at most 2 s and nearest-rank p95 at most 5 s over all
  twenty-four clear/ordinary attempts, including failures. Output, execution and
  replay timings remain separate; the research debug row timings are not this
  latency measurement.

If all gates pass, close the first usable opt-in V2 checkpoint for this supported
game-oriented setup. Document the remaining finite misses and exclusions rather
than turning them into another mandatory recovery project. This does not grant
GUI/controller/default promotion, perturbation guarantees or universal coverage.
If a gate fails, preserve the existing usable ground path and policy 2 and report
whether the gap is construction, terrain/local coverage, performance or evidence.
A bounded implementation checkpoint may close with an honest rejection; the
usable-planner objective must not be marked achieved on that basis.

## Current planning validation boundary

This plan is based on source inspection and retained evidence, not a new
simulation. Both accepted terminal-time summaries still match byte for byte and
all 252 source/input bindings match the current checkout. Read-only ledger
analysis establishes the 38/1 first-valid attempt distribution, 30/3/6 selection
kinds and launch-landmark differences; those do not prove future mission results.

The frozen 8/16/8 structural/input checker, corpus/tamper checks and fake-CLI
suite checks pass. All 1734 monitored source/script/fixture/evidence files keep
their pre-pass fingerprint, `c2ac70a9d343aec3e1121453100e3d8c52586f853ee6ed4c1a9a6ff4f1d1ad99`.
Document links and whitespace are checked. Workspace tests and Clippy are not
rerun for this documentation-only pass; the preceding 904-test result and its
disclosed baseline lint exception remain historical evidence. No source, fixture,
flight policy, runtime behavior, active goal, commit, push or deployment changes.
