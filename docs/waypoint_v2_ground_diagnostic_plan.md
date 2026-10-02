# Waypoint V2 ground diagnostic plan

## Purpose and scope

This revised 2026-10-02 plan is a small diagnostic for the game setting, not a
general feasibility study. Find out whether terminal construction, conservative
screening or acquisition explains the new family's ground misses, then recommend
the smallest useful change. The pass stops at evidence and primary review;
saving this plan does not authorize execution or implementation.

The [measured baseline](waypoint_v2_nominal_characterization_results.md) remains
unchanged: the existing constructor lands all eight uncut clear controls; the
research family lands only the two downhill controls. All eight research launch
preparations succeed. The six misses reject analytically before a physical
witness attempt, so they are neither first-step crashes nor impossibility proofs.
All 39 retained airborne states have free-space landing witnesses, including
real-terrain recoveries at the four former NoNominal handoffs. These are suffix
experiments, not a complete new planner.

## Practical design to preserve

The nominal planner constructs from actual position, velocity, attitude, fuel
and clock, without consulting interior terrain. Its vertical baseline is the
lowest adequate approach, not a curve that already-safe motion must equal.
Preserve acceptable higher or steeper motion; correct unsafe approach, energy
or lateral intercept. A future apex is unnecessary for an already acceptable
descending state.

Use cheap estimates to construct a small candidate set and the existing simulator
to validate selected candidates. We do not need a universal analytical safety
envelope before a usable game planner. Keep physical actuator, fuel, deadline,
contact and reserve checks. Ground and airborne acquisition may use different
adapters while sharing target, entry, preservation and result semantics; do not
hide two unchanged route generators behind a wrapper and call that unification.

The outer loop remains direct-first, executable validation, real-terrain audit,
local clearing and replanning from actual handoff H. Local acceptance requires
clearance, progress and the separate two-second guard, not a complete landing
plan after every waypoint. This pass changes none of those contracts.

## Fixed controls and evidence

Use the eight clear flights in
`outputs/research/waypoint_v2_practical_20261001/final_policy_2_b/runs` on their
original scenarios and 9600-tick deadlines. The accepted canonical archive in
`outputs/research/canonical_initial_direct_canary_20260930/final_b` supplies the
selected program boundaries and original terminal durations.

| Clear case | Canonical control |
| --- | --- |
| `v2_clear_685` | `operational_flat_span_685` |
| `v2_clear_735` | `completion_flat_span_735` |
| `v2_clear_845` | `operational_flat_span_845` |
| `v2_clear_915` | `completion_flat_span_915` |
| `v2_clear_uphill_845` | `operational_uphill_span_845` |
| `v2_clear_uphill_915` | `completion_uphill_span_915` |
| `v2_clear_downhill_845` | `operational_downhill_span_845` |
| `v2_clear_downhill_915` | `completion_downhill_span_915` |

Read-only archive comparison already establishes exact equality of the eight
mapped command lists. It does not establish a new screen or flight result.
Bind original scenarios, full programs, summaries, protocol, evaluator source
and this plan with sorted path/SHA-256 records verified before and after. Check
vehicle, gravity, initial state and pad compatibility; historical scenario names
or horizons must not replace the V2 inputs. Use a separate create-only diagnostic
output root and leave the 47-row corpus and retained evidence untouched.

The bound corpus SHA-256 is
`c570da97f46cdaa5daf70238964d52afaad663107f42e5ef3205a7d186a7c128`.
Both accepted characterization summaries have SHA-256
`0238b3632a71c8ae5f599fabfca0a7ca303584669b91b9c40f99b3f7fa397d0b`.
The canonical summary has SHA-256
`bc60a5547a85a3205b18d3b544319e5757ceb650bb16b6296ec8f8d7eeb49ba5`.
Changed or missing inputs are integrity failures, not permission to substitute
a flight or regenerate old evidence.

## Step one reproduce the eight working landings

Execute each original full program from fresh H0 on its actual terrain. Require
safe target contact and independent consumed-command, endpoint and raw incoming
contact agreement. Capture the live source acquisition endpoint S and terminal
entry T just before the first terminal command, retaining attitude, rate,
position, velocity, held command, mass/fuel and absolute clock. Newly observed
boundaries are replay observations, not previously archived snapshots.

Do not execute from extracted snapshots, reset fuel/time or set velocity
instantaneously. Preserve pre-step update clocks, complete held pairs at S/T
and valid odd first-contact ticks. Compare the old and research states at tick
204 only as different histories at one clock: the old source endpoints are
1812 or 1992, and longer upright launch is not thereby proven to be the fix.
Stop downstream interpretation if the supplied controls fail reproduction.

## Step two compare two references at each actual entry

Hold each actual T fixed and compare the existing terminal reference using its
original duration versus the unchanged research braking-time duration. This
means at most sixteen references, with no duration sweep or changed constants.

The approximation `duration = 2*h/(down_speed + touchdown_speed)` describes a
particular braking construction; the implementation includes a discrete-step correction.
It is not a definition of every safely landable state. A high, slowly descending
entry can produce a long duration where another descent profile might work.
That is a hypothesis to test against these controls, not a measured diagnosis.

Reuse `build_reference`. The canonical archive stores commands and durations,
not the complete terminal reference, so label the rebuilt curve as derived until
its kinematics and paired demands agree with the known flight. A derived-model
discrepancy is not automatically corrupted evidence; disagreement between
executions of the same commands is an integrity failure.

For each comparison record geometry, duration/finish clock, forward-velocity
reversal, first-thrust alignment and fuel demand. Separate the current combined
bound, exact discrete vector demand, held-pair demand and actual applied demand.
Compare each with the unchanged 0.925 incoming-mass limit and changing-mass raw
physical limits, in acceleration units. Do not combine a new reference with the
old flight's mass history and call it executed. Retain all applicable predicates
and the current screen's first rejection separately; mark undefined quantities
as not evaluated. Scanning a bounded reference is diagnostic, not a new runtime
optimizer. Do not charge an already completed attitude turn again.

## Step three use at most three shadow probes if needed

Only if the comparison leaves conservative screening as a plausible blocker,
test a few existing research candidates through the physical realizer. A shadow
probe is a diagnostic exception to one analytical gate, not an admitted proposal.
Keep the original screen rejection and nominal acceptance unchanged.

The predeclared cases are `v2_clear_845`, `v2_clear_uphill_845` and the positive
control `v2_clear_downhill_845`. For each failing case, use only its sealed ledger:
among acquisition-passing entries whose first terminal rejection is the finite
coupled-thrust bound, choose the smallest bound/recorded-limit ratio, then stable
seed identity and entry index. Such rows have already passed descending geometry,
deadline and forward-motion checks. Use the downhill control's existing
first-ranked entry. Selection must not consult real terrain or probe outcomes.

Retain that seed, acquisition, coast and terminal duration. Check remaining
alignment and fuel predicates without changing them; if they fail, report that
additional blocker rather than replacing the seed. Missing eligible entries
are not evaluated, with no substitute case. If screening is not a useful next
question, record this entire step as not needed.

For a probe that reaches realization, reproduce the research ground preparation
and actual acquisition/coast from fresh H0. Do not execute its predicted entry
snapshot. Use the existing realizer's rate-limited attitude, paired throttle,
changing mass, fuel, absolute deadline and raw safe target-plane contact checks.
Record live versus predicted entry and demand. The conservative terminal bound
is the only gate eligible for a shadow exception; source exceptions remain
ground-only and all other physical checks remain active.

Report target-plane outcome, independent replay, actual demand/reserve and
separate real-terrain audit. A raw-limit landing that violates the declared
reserve is not a reserve-safe success. A shadow success neither changes the
stored 2/8 nor certifies a usable ground constructor. A physical rejection is
also a useful result. Budget: at most one selected trial per case, three new
shadow trials per artifact, with no retries using different parameters.

## Step four inspect acquisition only if still unresolved

If the matched entries and applicable probes leave acquisition as a remaining
gap, compare each affected control's complete working acquisition with its
existing research history and ledger. Include tilt/source bridge, turning,
powered displacement, velocity change, fuel, entry attitude and remaining time;
do not reduce this to a velocity difference or presume one cause for all cases.

Use recorded rejections to localize the gap. Do not exhaustively recompute all
416 seed/entry slots or run the former optional 24 continuation witnesses from
known S states. No new seeds, time grid, burn shape or margin calibration belongs
to this pass. If the evidence cannot discriminate the cause, retain an explicit
unresolved result and recommend one smaller follow-up question.

## Implementation and validation after execution approval

Reuse the existing `pd-eval` characterization, reference and replay helpers.
Add only the thin diagnostic adapter/output needed for these comparisons; a
new standalone CLI, reporting framework or production generator is not required.
No `pd-plan`, controller, local policy, terrain or default changes are in scope.
The primary owns selection definitions, interpretation and review. If a later
worker loop is requested, delegate only bounded work with explicit ownership.

Test duration provenance, actual-state extraction, held-pair/odd-contact clocks,
exact versus conservative norms, force/acceleration units, changing mass,
alignment ownership and shadow-versus-acceptance classification. Retain tamper,
binding and independent replay checks. Unexpected nonfinite calculations,
command gaps and claimed replay/identity mismatches are integrity failures;
finite physical or analytical rejection is a normal diagnostic outcome.

Two final-source artifacts must agree on comparisons, chosen trials, commands,
states/contact and classifications. The per-artifact bounds are eight control
replays, sixteen references and three shadow trials; repeats are reproducibility,
not additional independent cases. Preserve sources, fixtures and old evidence.
If existing shared helpers are factored or changed, rerun the complete unchanged
characterization and require its original payload. Otherwise use input/source
preservation, focused tests and the existing checker. Run appropriate workspace,
Clippy and formatting checks for any Rust adapter added. The full mission matrix
and historical integration suite belong to a later constructor change.

## Deliverable and next decision

Deliver a compact eight-row comparison/classification table, applicable probe
records, reproduction commands, bound repeatable artifact and separate primary
review. Name duration/reference mismatch, screen conservatism, reserve-contract
difference, acquisition gap or unresolved evidence as supported. More than one
cause may apply; complete attribution of every miss is not a completion condition.

Recommend one smallest supported next change: for example, a bounded alternative
terminal construction, tighter demand screen retaining reserve, or mode-specific
acquisition adapter. Do not automatically implement it, relax margins or expand
the search. If no small change is supported, keep the working ground constructor
and state what remains unresolved; mathematical unification is not worth an
open-ended detour from a usable game planner.

Later changed-family validation must preserve airborne behavior and meet 8/8
clear terrain/replay/reserve checks before runtime review. Full planner acceptance
still requires the actual repeated-handoff 32-case loop, at least 13/16 ordinary
landings and the existing family/reference/integrity gates. No cutaway, scenario
rewrite, budget reset, commit, push or default promotion is implicit. The prior
characterization goal is complete; this planning revision creates no active goal.

## Planning checks

The existing frozen-suite structural/input checker passes with the unchanged
8/16/8 population; flight acceptance is not evaluated. All 45 relative links in
the four revised documents resolve and whitespace checks pass. Before/after
hashes match for 213 monitored source/input/script/protocol files and the four
bound evidence artifacts above. These checks validate this documentation pass,
not the proposed comparisons, shadow flights or workspace tests.
