# Waypoint V2 terminal time construction plan

## Purpose and decision

This 2026-10-02 plan proposes one bounded research implementation: improve
terminal landing time without changing launch/acquisition, the reference builder,
the physical controller or the 0.925 thrust budget. The question is whether
removing the demonstrated timing restriction recovers clear ground starts while
preserving airborne behavior. Saving this plan does not start implementation,
flight experiments, a new active goal or a runtime policy revision.

The [completed diagnostic](waypoint_v2_ground_diagnostic_results.md) reproduces
8/8 old uncut ground flights. At those actual terminal entries, all original
durations pass but all research braking-time references reverse lateral motion.
Flat/uphill shadow landings use approximately 0.993 thrust, so their success does
not justify relaxing the budget. The research family's admitted ground coverage
remains 2/8. Its acquisition compatibility remains an open question.

The outer planner remains terrain-blind nominal construction, executable
validation, terrain audit, local clearing and replanning from actual handoff.
This change must not use terrain to raise the nominal arc over an obstacle or
require a landing plan after each waypoint. A new future apex is not required
for an already acceptable descending state.

## A cheap bound from the existing reference

The following is a local derivation for the existing quadratic-horizontal
acceleration reference, not a universal vehicle feasibility envelope. Orient
the horizontal axis toward the target. Let `d` be remaining horizontal distance,
`v` be initial velocity toward the target, `dt` the physics step, `n` the terminal
tick count and `k` an integer tick. For `d > 0`, `v > 0`, `n > 2`, the builder's
zero final lateral velocity and zero final lateral acceleration imply:

```text
vx(k) = (n-k) * (n-1-k) * (A*k + B)
B = v / (n * (n-1))
A = 12 * (d/dt - v*(n-2)/3) / (n*(n+1)*(n-1)*(n-2))
```

At integer ticks, velocity is nonnegative throughout exactly when:

```text
n <= 4*d/(v*dt) + 3
```

The continuous approximation is `T <= 4*d/v`; the extra three ticks come from
this velocity-first discrete construction. Keep the existing forward-velocity
validator authoritative, especially after rounding and at tolerances. This
bound says nothing by itself about vertical motion, thrust, attitude, fuel,
body clearance or actual landing. It is a cheap construction limit, not an
acceptance certificate.

Read-only verification against all sixteen retained references checked 38320
integer velocity samples. The largest difference from the existing coefficient
calculation was `1.123e-13 m/s`; the bound agrees with all sixteen existing
forward-motion classifications. No new candidate flight was run.

| Actual entry | Original time s | Largest even no reversal time s | Research formula time s |
| --- | ---: | ---: | ---: |
| Clear 685 | 12.500 | 17.283 | 18.383 |
| Clear 735 | 13.000 | 17.983 | 19.350 |
| Clear 845 | 15.500 | 21.517 | 30.617 |
| Clear 915 | 17.000 | 23.650 | 41.783 |
| Uphill 845 | 14.000 | 19.517 | 27.033 |
| Uphill 915 | 13.500 | 18.650 | 20.633 |
| Downhill 845 | 15.500 | 21.517 | 24.700 |
| Downhill 915 | 15.000 | 20.650 | 20.700 |

These are exact discrete reference-family limits rounded down to complete
held pairs, not maximum safe flight durations. Every working time lies below
the limit; every rejected research time lies above it.

## Proposed duration rule

Use at most three durations at each unchanged predicted coast entry. Try the
unchanged vertical braking-time candidate first. If it passes all existing
analytical predicates, retain it without adding alternatives at that entry.
This preserves a valid existing choice instead of optimizing it unnecessarily.
Unexpected nonfinite calculations are integrity failures, not retry signals.

Check time-independent entry geometry and direction once before constructing
references. An entry below the required height, not descending, or initially
moving away from the target cannot be repaired by choosing a different terminal
duration; retain its finite rejection without trying alternatives.

If the baseline has a finite rejection and `d > 0`, `v` exceeds the existing
forward-velocity tolerance, try two horizontal shape anchors:

| Anchor | Unrounded tick count | Why this shape |
| --- | --- | --- |
| Zero initial lateral acceleration | `2*d/(v*dt) + 1` | Starts the terminal lateral demand at zero and finishes with zero lateral velocity and acceleration |
| Affine lateral acceleration | `3*d/(v*dt) + 2` | Removes the quadratic acceleration term and decelerates toward zero velocity |

These are derived shape choices, not duration factors fitted to individual
missions. Their continuous approximations are `2*d/v` and `3*d/v`. Thirty
read-only algebra checks at five durations and three velocities confirm the
stated unrounded properties. Rounding or clipping changes those exact shape
properties; rebuild and validate the resulting reference rather than assuming
the rounded candidate retains them.

Round each alternative upward to an even tick count, then clip to the available
even upper bound: the smaller of the no-reversal bound and the remaining
absolute deadline after acquisition, coast and the existing four terminal extra
ticks. Require at least four ticks for this positive-distance horizontal branch;
an empty interval is a finite miss. Deduplicate against the baseline and other
alternative. Never retry neighboring ticks or consult archived successful times.

For zero or near-zero forward velocity, keep the unchanged baseline candidate
only; avoid division and do not invent another seed family in this pass. This
is an explicit coverage limitation. Motion away from the target must first be
handled by existing acquisition; terminal duration cannot hide its initial
negative forward velocity. Near-zero horizontal distance and near-zero velocity
retain the existing vertical-only candidate. Do not classify an excluded branch
as physical impossibility or insert a waypoint to conceal a constructor miss.

Every candidate must pass the same geometry, absolute finish, forward motion,
coupled thrust, first-thrust alignment, minimum-throttle and estimated-fuel
checks. No shadow exception is permitted. If the baseline fails and multiple
alternatives pass, use the existing estimated-fuel, finish-clock ordering, then
stable duration-candidate index. Keep outer seed/entry ranking and terrain
independence unchanged. A shorter time is not presumed safer or lower demand.
Alignment still uses the acquisition-end attitude and the already allocated
coast interval; do not charge a completed turn again or assume instant rotation.

## Scope and finite budget

Keep the current ground preparation, at most thirteen acquisition seeds, three
turn-consistency updates, four coast locations and three physical witness
attempts per row. The only added freedom is at most two terminal times after a
finite baseline rejection. Maximum analytical reference constructions are
`13 * 4 * 3 = 156` per row; no extra physical attempts are authorized. These
are limits, not measured planning latency or a new default runtime budget.

Keep all previously accepted baseline times available. New candidates can
change global ranking despite that preservation, so regressions must still be
measured rather than ruled out by assertion. Do not filter or reorder candidates
using terrain success, a later landing result or case identity.

Implement as an explicit new opt-in research identity using existing helpers,
not a change to the old characterization or policies 1/2. Reuse the reference
builder, screen components, materializer and reporting conventions. Maintain
four coast-entry identities and record each time candidate's provenance,
rounded/clipped duration and rejection; select at most one passing duration per
coast entry before existing outer ranking. Avoid a new general optimizer,
standalone CLI, reporting framework or mandatory dependency.

## Execution stages after approval

1. Add the bounded duration helper, candidate traces and focused tests. Cover
   discrete algebra, signed direction, near-zero motion, small/deadline-clipped
   intervals, even rounding, deduplication, nonfinite failures, baseline retention,
   maximum three candidates and terrain independence. Preserve old behavior
   under its old identity; do not silently change existing helpers or evidence.
2. Compare the rule at all eight proven actual terminal entries, using fresh
   original prefixes for state extraction. Require at least one fully screened
   reference per entry under the unchanged budget. Original durations are
   comparison controls only, never candidate inputs. This is an analytical
   gate: a different first-thrust direction at fixed T is not automatically
   executable from the old attitude. Do not restore snapshots or rotate instantly.
   If this gate fails, report the specific remaining predicate and stop before
   the broad constructor run; no timing-grid expansion follows.
3. If the matched-entry gate passes, run the same sealed 47 retained inputs and
   eight separately labeled synthetic conditions using the new opt-in identity.
   Ground rows begin at fresh H0; all 39 airborne states are reconstructed through
   their full original prefixes. Use actual acquisition/coast/terminal realization
   and the original clock, fuel and deadline. Replay every claimed witness and
   audit terrain separately without changing rank. Compare old and new outcomes,
   demand, entry, vertical acquisition impulse and analytical/physical cost.
4. Produce two matching final-source artifacts with verified sorted input/source
   bindings, preservation checks and workspace/focused/format/Clippy gates.
   Finish with a separate primary review and explicit go/no-go for further
   runtime work. No full mission matrix or runtime integration is implicit.

## Acceptance and stop decisions

The target for the new constructor checkpoint is 8/8 clear actual-terrain
landings with phase-specific body reserve, unchanged thrust budget, deadline,
safe raw contact and independent replay. All 39 retained airborne rows must
retain free-space witnesses; the four former NoNominal handoffs must retain
terrain/replay/reserve success. Report all other first-ranked terrain outcomes
and synthetic conditions honestly, including regressions; do not substitute a
terrain-successful lower-ranked candidate. Keep zero/natural acquisition choices
and their preservation priority; no new apex or profile-equality requirement.

If ground coverage stays below 8/8, retain the existing working ground constructor
and isolate the remaining reason. A thrust-bound miss is not proof that the
bound is falsely conservative; an acquisition miss is not grounds to enlarge
the timing set. Recommend at most one later mode-specific acquisition question,
or defer unification if no small change is supported. Do not change acquisition
or margins during this pass, and do not reset the spent runtime revision budget.

The research task can finish with measured rejection and review; that does not
complete the usable-planner objective. Even a successful constructor checkpoint
still needs separately approved runtime integration and the unchanged complete
32-case repeated-handoff gate, including 8/8 clear, at least 13/16 ordinary and
the family/reference/integrity requirements.

## Planning review and validation boundary

Primary design review finds a bounded question with attributable controls,
derived candidate times, unchanged safety gates and explicit degenerate cases.
The discrete bound and anchor algebra are verified against local calculations;
candidate admissibility, actual landing coverage and performance are not yet
measured. The two alternatives are a proposed small family, not guaranteed
solutions. This is ready for a separately approved research implementation,
not a runtime promotion or an independent-agent review.

This planning pass changes documents only. Source, scripts, fixtures and all
monitored evidence bytes remain unchanged. Existing input-structure and link/
whitespace checks validate readiness, not new flight acceptance. Workspace
tests and strict Clippy are not rerun for this documentation pass; the preceding
diagnostic's disclosed baseline lint exception remains applicable. No commit,
push, deployment, simulator run or new active goal is included.

Before/after hashes match for all 1724 monitored source/script/fixture/evidence
files. All 247 completed-diagnostic bindings still match the checkout; the frozen
8/16/8 structural checker passes without evaluating flight acceptance. All 81
relative links in the four changed documents resolve and whitespace checks pass.
