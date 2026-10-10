# Ballistic feedback construction pass — 2026-10-08

[Documentation home](README.md) · [Reviewed plan](ballistic_aim_correction_plan.md)

Follow-up: the separately authorized [paired 1k diagnostic](ballistic_feedback_sweep_results.md)
is now complete on this unchanged candidate: 360/1000 landings, including 125
after waypoint handoffs, versus the retained baseline's 817. The construction
verdict below is retained; broad acceptance and default promotion remain deferred.

## Verdict

Keep the state-based ballistic aiming/correction direction. Do not promote this
first candidate or start its full paired campaign yet. The construction pass is
closed with a bounded negative verdict for waypoint coverage, not for ballistic
aiming itself.

The final opt-in candidate establishes a real engine-off continuation from the
exact retained `715` H1 state in terrain-neutral conditions. Unchanged flat and
uphill core inputs both land without waypoints or a floor cutaway. The real
`715` mission executes two pass-through waypoint handoffs but stops at a later
finite constructor miss. Its complete final record reproduces exactly.

This is not a new arbitrary-terrain pass rate. Development builds differ, cases
repeat, and some runs deliberately ignore terrain after a proven prefix. Do not
combine the 24 attempts into a landing denominator or compare them to 817/1000.
The broad campaign, portable CLI/batch integration, saved-bundle tamper gate and
default promotion remain unexecuted. The maintained policy-3 planner is unchanged.

## What was implemented

- Explicit `pd-eval ballistic-feedback-flight`, never selected by ordinary
  policy-3 or batch commands.
- Pure discrete ballistic aiming and finite coast-turn/burn/coast correction
  math in `pd-plan`; terrain does not choose the destination's vertical profile.
- Live evaluator session with stable goals and absolute arrival/cutoff clocks,
  24-tick aim refreshes, actual two-tick commands, and a 24-tick held-command
  physical/body-reserve check. Desired velocities are never assigned to the plant.
- Admission of already-valid unpowered motion, including descending motion with
  a past apex. Pass-through waypoints allow rising or already-higher motion and
  do not demand zero velocity, a landing angle or a certified landing suffix.
- Explicit adapter to the unchanged terminal controller after an admitted
  destination coast, with descending entry and existing readiness/authority checks.
- Original-source reconstruction, exact complete-state comparison at retained
  origins, exact command replay and deterministic decision reproduction. Final
  attempts also bind source content and executable hashes before/after.
- Common rich reports with desired arc, short-command prediction, current
  ballistic motion, active goal and actually-flown overlays at refresh origins.
  Actual handoffs have separate H markers. Report predictions clone actual
  source-replayed states, not restored snapshots. Accepted reports/navigation and
  server state were not changed; this is local construction evidence only.

## Final-source construction evidence

Evidence root:
[`outputs/research/ballistic-feedback-20261008`](../outputs/research/ballistic-feedback-20261008/).
Final attempts retain unchanged inputs, receipt, preflight, source-prefix proof,
pre-verification execution, command replay, feedback, planning data and rich report.
Earlier development artifacts remain intact with their original schemas/proofs.

| Check | Result | Evidence |
| --- | --- | --- |
| Exact `715` H1, neutral continuation | Corrects, cuts thrust, executes 24 unpowered ticks | [Final neutral report](../outputs/research/ballistic-feedback-20261008/dev-19-repeat-neutral-715/report.html) |
| Original `715`, original source/terrain | Two H; flying finite stop at 26.567 s | [Final flight report](../outputs/research/ballistic-feedback-20261008/dev-17-final-real-715/report.html) |
| Exact full-flight `715` repeat | Complete feedback record identical to final flight | [Repeat evidence](../outputs/research/ballistic-feedback-20261008/dev-20-repeat-real-715/feedback.json) |
| Flat `v2_clear_845`, cap 6 | Landed on target; mission success; zero H; 49.000 s | [Flat report](../outputs/research/ballistic-feedback-20261008/dev-23-final-control-flat/report.html) |
| Uphill `fresh_clear_uphill_805`, cap 6 | Landed on target; mission success; zero H; 48.375 s | [Uphill report](../outputs/research/ballistic-feedback-20261008/dev-24-final-control-uphill/report.html) |

Both landing controls have the full physical-success, mission-success, integrity,
exact command-source replay and deterministic-decision tuple. Their terrain is
the unchanged ordinary heightfield, not a lowered-floor diagnostic. They enter
maintained landing logic at 12.117 s and 12.683 s respectively; ballistic admission
does not mean the landing controller subsequently coasts all the way to contact.
No final-source downhill flight was collected.

The neutral H1 comparison reconstructs tick 2480 at 20.667 s, position
`(323.709629, 195.536896)` m, velocity `(30.540943, 44.774925)` m/s and
5617.579518 kg fuel, with the original complete state/clocks intact. Correction
ends at tick 2896; verified engine-off state is tick 2920. This is correction
construction evidence, not a terrain-clearance or landing proof.

Final flight-build identities, also recorded in the receipts:

- Executable SHA-256:
  `2ffa627cdc2cc98df224a8aadafc7859e9d6ab1dec32c5afa9a267079e7cc6e7`.
- Rust source-tree/manifests SHA-256:
  `e86de0959ea7e5c84d7a67ceb4992bf2fce8133d500f12f9b03809be19f2ecbf`.
- Report-script source SHA-256:
  `0dd790899fa3aa4078857ad16d16e60ec17b5b775bdaf404645ff1380bd361b0`.
- Base commit: `4267f0b181b688be4638782ebb3a91339c8e658f`, plus dirty source
  content above. The commit alone is not the measured source. Earlier development
  receipts have executable hashes and a Git-status key, not this final content seal.

## Why the waypoint side is not ready

The first waypoint rule places a point one conservative body diameter beyond
the first obstruction, with a terrain-derived height and a two-second drop
allowance. It can produce a clear passage without leaving a useful state for
the next correction. That is the important unresolved problem.

Final `715` H1 occurs at 13.567 s, approximately `(333.31, 208.27)` m with
velocity `(38.56, 42.90)` m/s. H2 occurs at 26.567 s, approximately
`(1162.74, 194.04)` m with velocity `(67.50, -61.91)` m/s. Only 37.26 m remains
laterally to the destination. Even the optimistic zero-turn horizontal stop
distance at available derated thrust is about 132 m; reserving vertical authority
makes it worse. Raising the waypoint does not, by itself, control outgoing velocity.

This does not prove the world impossible. It demonstrates a poor selected exit
state within the candidate's forward-flight model. The H was admitted for local
clearance/continuation, not a complete destination suffix.

The fixed eight-world panel on the preceding build completed without physical
crashes, but with zero landings. It was not rerun on the final endpoint-query
build and must not be described as final-source population validation:

| Cases | Finite stop / mechanism |
| --- | --- |
| `715`, `983` | Executed H, then no target aim; fast, late exit states |
| `280`, `327`, `999` | First generated waypoint arc still blocked |
| `000` | Cannot construct proposed waypoint acquisition from live state |
| `030` | Ideal waypoint arc clears; actual powered correction's short prediction violates the 5 m reserve |
| `565` | Reaches waypoint region; actual two-second coast continuation is not clear |

These are several missing local mechanisms, not evidence that increasing the
hop cap or tweaking one seed will solve them. A blocked active waypoint arc and
failed short-command prediction currently stop rather than performing another
bounded local correction. Multi-H execution in `715` is not multi-H landing coverage.

## Mechanical issues found and fixed

1. First neutral attempt stopped at “safe to turn off” while thrust remained held.
   It is retained but not counted as established engine-off flight. Later checks
   require an issued cutoff and an actual coast interval.
2. Exact-plane waypoint arrival waited past a valid window. Arrival now uses a
   body-sized forward clearance region and actual unpowered continuation.
3. Construction allowed rising waypoint arrivals; acceptance only recognized
   descending roots. Pass-through acceptance now allows rising/higher paths,
   with physical continuation still required.
4. Upright source clearance consumed an earlier turn interval. First tilt now
   reconstructs acquisition from the actual source-cleared state.
5. Ideal endpoint cancellation and paired-tick coast projection labeled the
   destination floor as an obstacle. Desired endpoints use exact target geometry;
   terminal geometric queries end at first touchdown-height crossing, not a
   continuation through the shelf. Actual contact classification, landing
   predicates and powered short-command checks were not relaxed.

## Attempt ledger

All 24 permitted attempts were retained. Two unused neutral-development slots
were reassigned to final-source flat/uphill controls; the total ceiling was not
expanded. Every attempt completed within its 60-second bound and reproduced
its decisions from the same source origin. No historical outcome was rewritten.

| Attempt numbers | Purpose / result |
| --- | --- |
| 01 | Initial neutral `715`; invalid cutoff-construction claim noted above |
| 02, 05, 16 | Neutral cutoff checks across development builds |
| 03, 06, 18 | Original-terrain retained-prefix continuations; finite stops |
| 04, 07 | Original-source `715` development; endpoint/waypoint admission failures |
| 08–15 | Fixed panel `715/280/983/327/999/000/030/565`; finite stops |
| 17 | Final original-source `715`; two H, finite stop |
| 19 | Final neutral construction repeat |
| 20 | Exact final-source full-flight `715` repeat |
| 21–22 | Initial flat/uphill controls; exposed paired-tick terminal query bug |
| 23–24 | Final flat/uphill controls; verified direct landings |

## Validation and limits

All eleven ordinary maintained developer checks pass: workspace/all-feature
tests, CLI boundaries, formatting, strict Clippy, 59 Node tests and docs links.
Focused additions include four aiming-math and eight feedback admission,
cutoff, terrain-blindness and endpoint-query tests, plus optional report fields
and feedback-overlay tests. These are automated data/interaction checks, not
browser pixel acceptance or user approval of the overlays.

The opt-in October-5 retained 44-case exact parity check fails at its first
clearance scalar: `0.0005533556252954597` versus `0.0005533556252945715` m.
This is the exact previously documented
[October-6 geometry-repair difference](random_terrain_survey_recheck_results.md#exact-difference-and-interpretation),
not a new ballistic-candidate outcome. The
[early-exit results](terrain_early_exit_results.md#review-and-retained-identities)
already explain why that older frozen capture is not a full current-source
parity baseline. No comparator, frozen input or historical value was changed.

A separately identified read-only comparison against
`capture-early-exit-20261007-native` passes **44/44 exact complete non-timing
record comparisons** using the current default planner, excluding only its
three already-declared wall-time fields. It creates no new capture and does not
relabel the October-5 source. This establishes preservation of the actual
pre-candidate default checkpoint.

## Recommended next bounded pass

Keep the demonstrated aiming math, ordinary propagation, cutoff invariance,
short physical check and unchanged landing adapter. Do not add a landing-suffix
search or restore the old coast/terminal combination grid.

First review one simple local **exit-state** contract: a waypoint should leave
forward, non-colliding motion with short-term room to change course. Compare a
rising/non-descending exit rule with an acceleration/turn-derived speed bound
as cheap alternatives, not simultaneously as another optimizer. This is not
“prove we can land immediately after the waypoint”; further waypoints remain
allowed. A point that prevents even the next correction is not a useful handoff.

Then handle local failures explicitly: a still-blocked waypoint arc needs a
bounded replacement, and an unsafe actual correction prediction needs a replan
opportunity before commands are committed. Retain a finite stop when neither
can be constructed; do not weaken the body reserve.

Validate with synthetic state/terrain twins, the same `715/983` late-H states,
and `030/565` powered/continuation contrasts. Require flat/uphill/downhill direct
controls and a verified waypoint landing before freezing the full maintained
pack, 1k and separate-100-world ledger. Add saved decision/command tamper checks
and native/CLI campaign plumbing before that freeze. This record identifies the
next investigation; it does not authorize successor tuning, promotion or publication.
