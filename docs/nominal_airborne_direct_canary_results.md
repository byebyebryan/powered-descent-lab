# Nominal airborne direct regeneration results

## Verdict

The bounded airborne regeneration canary passes. All twelve real ascending,
near-apex, and descending captures across four uncut-terrain missions produce
new coast-plus-terminal programs that land safely. All twelve obstructed terrain
twins preserve the complete nominal search and commands, then fail the separate
actual-terrain audit. Two final-source runs reproduce the same deterministic
evidence. This closes the declared regeneration foundation, not a production
planner or the complete canonical ballistic and local waypoint design.

The [protocol](nominal_airborne_direct_canary_protocol.md) was written before
measurement. The four physical inputs were already exposed by the previous
operational gate; these results are not new held-out mission coverage.

## Measured population and checks

| Original terrain | Ascending capture tick | Near apex capture tick | Descending capture tick | Regenerated safe landings | Unchanged proposals rejected on obstacle twins |
| --- | ---: | ---: | ---: | ---: | ---: |
| Flat 685 m | 1574 | 1706 | 1900 | 3 of 3 | 3 of 3 |
| Flat 845 m | 1574 | 1786 | 1930 | 3 of 3 | 3 of 3 |
| Uphill 845 m and 75 m rise | 1754 | 1976 | 2110 | 3 of 3 | 3 of 3 |
| Downhill 845 m and 75 m fall | 1574 | 1716 | 1930 | 3 of 3 | 3 of 3 |

Capture states come from ordinary execution of freshly generated accepted
baseline programs. The generator receives the retained live state, original
physical context, and absolute mission budget, but no baseline suffix, reference,
selected-row seed, or saved command schedule. It preserves clock, fuel,
attitude/rate, and held command. Serialized state evidence is never restored
into executable state. Computation pauses the offline simulator.

Each final run retains all 672 original-terrain search attempts: 186 produce
nominal proposals and 486 are finite rejections. The selected twelve programs
pass every declared body-clearance sample and first safe target contact under
ordinary and neutral 120 Hz physics with global 60 Hz held commands. The nominal
shape permits at most one future apex and requires target contact from above.
The original 80 s absolute planning budget is not reset at a capture.

All twelve whole-source stitched replays match the complete live-audit final
snapshot and full incoming contact snapshot before touchdown normalization.
Actions, events, and samples also match the standard replay path. There is no
startup crash, fake checkpoint success, floor cutaway, clock/fuel reset, or
uncovered-command fallback in these accepted executions.

Each twin inserts a tall triangular obstruction ahead of the captured flight,
retaining source and target shelves and all non-terrain inputs. The complete
nominal search payload and selected commands remain identical. Every independent
twin audit records an actual rotated-body clearance violation; no higher arc is
selected to hide the obstruction.

## Repeatability and preservation

Final evidence is retained in create-only local roots:

- `outputs/research/nominal_airborne_direct_canary_20260929/final_contact_v1_a`
- `outputs/research/nominal_airborne_direct_canary_20260929/final_contact_v1_b`

Both summaries are byte-identical, with identity
`fnv1a64:46f616840230427e`. All four complete case payload pairs match after
excluding only the protocol-declared observational wall-time fields. The 129
bound source/input/protocol files are unchanged during both measurements;
their closure identity is
`6571e641aa624b30aece49b2c7ab65d7e64b4dd8aa401348388cd284a6ad01cd`.

Earlier `run_a` and `run_b` roots remain untouched. Review found their contact
comparison covered kinematics but not the entire incoming snapshot. The final
correction exposes the full authoritative snapshot and compares it exactly;
a negative test rejects changed cumulative clearance and held attitude even
when kinematics are unchanged. Every selected proposal is identical before and
after this evidence correction. The earlier roots are not the final acceptance
record, and no flight policy was retuned.

Each final run separately verifies its four complete source-rest baselines.
The additional 24-control regression passes exact generation, selected-command,
contact, ordinary-flight, and action-replay parity. Its retained root is
`outputs/research/nominal_airborne_direct_canary_20260929/source_rest_regression_final_contact_v1`,
identity `fnv1a64:a351ec65ca22471e`; declared source and historical archive checks
remain unchanged. No existing artifact directory is overwritten.

Final validation passes 822 workspace tests across twelve suites, with one
existing explicit historical-artifact ignore; fifteen focused airborne/CLI
tests; strict all-target workspace Clippy; formatting; and whitespace checks.

## Cost and limits

The second final release run records candidate-search costs of 12.583 to
32.603 ms, with a median of 23.259 ms across twelve captures. These are local
observations for the smaller airborne-only search, not a like-for-like speedup
over source-rest generation or a worst-case latency bound. The median already
exceeds one 60 Hz control interval. This synchronous canary proves neither safe
motion during computation nor real-time planning authority.

The input family is running, forward-moving ballistic coast with idle held
throttle and positive fuel. Coverage from arbitrary powered redirects, rotated
high-energy states, repeated waypoints, overhangs, disturbance recovery, or a
continuous operating envelope is not established. Safety uses the existing
discrete body/contact model, not swept collision or perturbation-robust proof.

The existing source-rest generator is retained as a control. It is not promoted
to the desired terrain-blind canonical initial-transfer policy. The new search
begins at the captured airborne states; its finite coast-plus-terminal family
is a regeneration foundation, not the complete canonical ballistic planner.
Production V1/defaults and the design-only completion reserve are unchanged.

## Next bounded pass

First establish the canonical initial transfer using the proven launch and
terminal machinery with interior terrain excluded from nominal selection.
Require uncut flat/uphill/downhill acceptance and identical nominal proposals
across interior-terrain twins. A tall obstacle must block the selected canonical
proposal even when the old terrain-aware family can find a higher direct arc.
Stop there if source-rest compatibility fails; do not tune per obstacle.

Then prototype one bounded local clearing maneuver for the first obstruction.
Derive its handoff position and velocity from a feasible trajectory, require
useful progress and a separately defined finite safe continuation, and replan
from the actual live handoff. Do not require direct landing after that waypoint,
force a literal far-edge coordinate, or optimize the entire route in one step.
Another local waypoint must remain possible. Keep finite generator coverage
failure separate from terrain blockage and from physical infeasibility.
