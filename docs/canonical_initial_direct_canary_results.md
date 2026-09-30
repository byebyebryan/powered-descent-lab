# Canonical initial direct transfer results

## Verdict

The sealed evaluator-only protocol passes in two final-source runs. All eight
flat, uphill, and downhill source-rest controls land on unmodified terrain.
Blocking and nonblocking interior-terrain twins retain the complete nominal
search and selected commands. The 900 m late/broad obstruction blocks the fixed
canonical choice even though a freshly regenerated old terrain-aware control
can safely fly a higher arc. All twelve real airborne captures from the new
initial flights regenerate accepted continuations with whole-source replay.

This closes the declared canonical initial-transfer foundation. It does not
implement local obstacle clearing, a multi-waypoint planner, production wiring,
or real-time operation. The [sealed protocol](canonical_initial_direct_canary_protocol.md)
and input-selection manifest preceded implementation and measurements. The
inputs were previously exposed; the four additional endpoints are physical
coverage, not hidden holdouts or completion-reserve experiments.

## Initial flights without a cutaway

| Cohort | Terrain and span | Complete peak COM height, m | Safe contact time, s |
| --- | --- | ---: | ---: |
| Development | Flat 685 m | 178.735 | 30.35 |
| Development | Flat 845 m | 218.748 | 33.60 |
| Development | Uphill 845 m, 75 m rise | 257.690 | 33.60 |
| Development | Downhill 845 m, 75 m fall | 183.134 | 33.60 |
| Additional physical endpoint | Flat 735 m | 191.272 | 31.10 |
| Additional physical endpoint | Flat 915 m | 236.264 | 36.10 |
| Additional physical endpoint | Uphill 915 m, 75 m rise | 275.096 | 33.85 |
| Additional physical endpoint | Downhill 915 m, 75 m fall | 200.506 | 33.85 |

Each generation retains all 140 outer rows. Across these eight inputs, 749 of
1,120 rows produce complete nominal proposals; the other 371 are finite
rejections. All eight winners are row 63: duration multiplier 1, zero source
duration offset, and zero terminal duration offset. Ranking uses lowest
complete-flight peak, then contact tick and stable identity. This is a minimum
within the sealed finite family, not a global optimality result.

Every selected source bridge fits in one correction iteration, with maximum
position error 3.789e-8 m and velocity error 6.590e-10 m/s, below the unchanged
1e-6 tolerances. Commands start at global tick zero, integrate the actual
upright/tilt launch, preserve fuel, and cover every consumed update. Each
selected initial flight contacts at the terminal reference duration: none uses
an after-reference tail, uncovered-command hold, or completion reserve.

Endpoint pad planes are used only locally. Interior terrain contact and
terrain-derived accumulated extrema cannot accept, reject, or rank a nominal
proposal. The independent audit uses ordinary and neutral physics on the real
terrain, with the existing rotated-body clearance policy and pad-transition
exceptions. All eight full final snapshots, complete incoming contact reports,
actions, events, and samples match the independent whole-source replay and
standard action replay. There is no first-step crash or synthetic checkpoint.

## Fixed-choice terrain handling

Each development flight has a blocking triangular twin and a low nonblocking
interior perturbation, with endpoint shelves and every non-terrain input held
fixed. All eight twins have identical complete canonical searches and selected
proposals. All four blocking twins report an actual terrain conflict and all
four low twins still land safely. The low perturbation lowers one interior
point by 1 m; it is not a condition of baseline acceptance or a floor cutaway.

The first blocking-twin conflicts occur during ballistic coast at ticks 1933,
2014, 2134, and 2014 for the four development cases respectively. Their full
states, applied phases, positions, and clearance evidence are retained. A
contact conflict retains its incoming state before touchdown normalization;
an arbitrary unsafe target entry cannot masquerade as terrain blockage.

The separate 900 m discriminator gives the intended distinction:

- Flat and late/broad terrain produce the same complete canonical search and
  selected program, with a whole-flight peak of 232.373 m.
- The flat control lands safely. The late/broad terrain first violates the
  5 m body reserve at global tick 2354, during powered terminal flight: body
  clearance is 2.085 m at COM position (-350.864, 221.131) m.
- The independently regenerated old terrain-aware control selects row 16,
  reaches a measured 120 Hz whole-flight peak of 513.659 m at tick 2518, and
  lands safely at 43.35 s. Full state, incoming contact, and ordinary/standard
  replay evidence agree. Historical commands do not seed either generator.

Thus TerrainBlocked means the canonical choice is obstructed, not that every
higher direct flight is impossible. The canonical policy does not rerank after
terrain rejection. Finite generator exhaustion remains a separate Unknown.

## Regeneration from the new flights

| Development flight | Ascending capture tick | Near-apex capture tick | Descending capture tick | Accepted regenerated continuations |
| --- | ---: | ---: | ---: | ---: |
| Flat 685 m | 1814 | 1946 | 2140 | 3 of 3 |
| Flat 845 m | 1814 | 2026 | 2170 | 3 of 3 |
| Uphill 845 m | 1994 | 2216 | 2350 | 3 of 3 |
| Downhill 845 m | 1814 | 1956 | 2170 | 3 of 3 |

The collector executes the newly generated initial program and stops before
consuming its command at each capture. Regeneration receives the retained
in-memory state and original context and deadline, never a serialized state
restore, baseline suffix, saved row, or command seed. The unchanged airborne
family retains all 672 rows across twelve captures, with 187 nominal proposals
and 485 finite rejections. All twelve selected continuations pass actual-terrain
audit and whole-source stitched replay, including complete final and incoming
contact snapshots, global command clock, fuel, actions, events, and samples.

## Repeatability and preservation

Final create-only roots are:

- `outputs/research/canonical_initial_direct_canary_20260930/final_a`
- `outputs/research/canonical_initial_direct_canary_20260930/final_b`

Both pass all six declared gates and have identity
`fnv1a64:33c595b7ef7cecd1`. Complete summary payloads match after excluding only
the declared observational timing fields. All eight final per-case files match
their embedded summary evidence. The 136 bound source/input/protocol files
remain unchanged; closure identity is
`a998d8ad1195c03b0ded74ed4f1d6f47efa44f53be79d14c9db14e979b154a66`.

The earlier `gate_a_draft` root is preserved. Its four complete searches,
selected command programs, and actual-terrain audits match the final runs
exactly. Review changes corrected evidence recording and identity finalization;
they did not tune the flight family after its first measurements.

The old 24-control exact-parity regression passes in
`outputs/research/canonical_initial_direct_canary_20260930/source_rest_preservation`,
identity `fnv1a64:f1bdda88abad9d0a`. Every declared check passes, source closure
is unchanged, and all 26 declared historical archive files remain untouched.
This does not claim a hash audit of every historical output file.

The existing airborne canary passes in
`outputs/research/canonical_initial_direct_canary_20260930/airborne_preservation`,
identity `fnv1a64:ea5a1ab40d37200e`. All four complete case payloads match the
accepted `final_contact_v1_a` evidence, excluding only its declared timings.
Its twelve continuations and twelve obstruction twins retain their original
behavior. Preservation root identities differ from historical roots because
they bind the expanded source tree; physical case evidence does not change.

Final validation passes 842 workspace tests across twelve suites, with one
existing explicit historical-artifact ignore. The twenty added tests cover the
endpoint-only basis, source/audit bindings, harness evidence and stopping,
old-control replay guards, and CLI opt-in boundary. Strict all-target workspace
Clippy, formatting, and whitespace checks pass on the integrated tree.

## Cost, limits, and next boundary

The second final release run records 200.412 to 283.788 ms for initial case
generation plus audit/replay work, median 259.8475 ms. Airborne candidate search
alone takes 13.472 to 33.658 ms, median 23.611 ms. These measure different work;
they are not a like-for-like speedup or worst-case latency guarantee. Planning
pauses the offline simulator. Neither runtime establishes safe motion during
computation or 60 Hz replanning authority.

Coverage remains finite, forward source-pad rest with the supported vehicle,
flat endpoint shelves, and aligned ballistic-coast captures. It does not cover
arbitrary powered redirects, overhangs, disturbances, swept collision, or a
continuous robust operating envelope. Production V1, existing generators,
contact thresholds, and the parked completion reserve remain unchanged.

The next design/implementation slice is one local clearing maneuver for one
demonstrated obstruction. Start from a bound real state early enough to retain
control authority; do not wait until the already-violating conflict snapshot
or restore it as executable state. Derive waypoint position and velocity from
a feasible trajectory, require useful progress and a separately specified
finite safe continuation, then replan from the actual handoff with global time,
fuel, and command cadence intact. Do not require immediate direct landing,
force the literal far edge, or reject a useful first maneuver merely because
another waypoint may be needed. The powered-terminal conflict in the 900 m
case also marks a later arbitrary-powered-state coverage boundary, not a
capability silently supplied by the present airborne canary.

Primary owns the endpoint-only basis, source fit, complete proposal/audit,
independent old-control peak proof, integration review, and final acceptance.
Luna supplies the execution-seam audit and bounded harness implementation and
corrections. No commit, push, deployment, default promotion, or local-clearing
implementation is part of this pass.
