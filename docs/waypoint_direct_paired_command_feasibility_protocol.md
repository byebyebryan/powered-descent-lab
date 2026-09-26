# Paired-command feasibility for the frozen source-duration canary

This is an opt-in evaluator experiment over the frozen source-duration canary.
It asks whether a bounded schedule of commands held for two 120 Hz physics
ticks can hit the same frozen source handoff from the same held-60 Hz launch,
at the same source duration and with the same unchanged coast/terminal tail.
It is known-outcome feasibility evidence, not held-out validation, a landing
certificate, or authority to change the planner, controller, simulator,
contact contract, policy, F6 path, or defaults.

## Inputs and preflight

The command rebuilds the upstream identities and nested input gates, binds the
coupled-thrust audit, and validates the frozen source-duration family before
constructing a simulation state. It requires all 15 ordered rows: six explicit
analytical skips and nine survivors. The source summary has identity
`fnv1a64:dfe0f0feaa15fc24` and expected file SHA-256
`a93b728374145d4f12b43b21b7d68e383270db19facdf5964ce91bc47e557e9c`.
Verify the file digest separately with `rtk sha256sum`; preflight pins semantic
identity and upstream bindings without adding a runtime file-hashing
dependency.

The frozen source summary and seven upstream paths are the same sealed inputs
listed in
[`waypoint_direct_held_cadence_diagnostic_protocol.md`](waypoint_direct_held_cadence_diagnostic_protocol.md#sealed-inputs-and-bounded-family).
`--preflight-only` validates these gates and row coverage without creating a
simulation state, advancing physics, or creating output.

## Predeclared experiment

All nine survivor rows first rerun both frozen controls: direct-per-tick 120 Hz
and combined-held 60 Hz. Each control must match its frozen canary evidence and
pass ordinary/neutral core replay parity. A mismatch aborts before writing an
artifact.

The first schedule gate is the predeclared `-180` row for each base: native
(0-based row index 1), research-shortest (index 6), and third (index 11). For
each, the source bridge is reconstructed from that row's actual held-60 launch
state to its unchanged frozen handoff, over its unchanged frozen duration. The
unchanged coast and terminal tail are retained for final full-flight replay.

Adjacent reference thrust-acceleration vectors are averaged into one command
per pair of physics ticks. At the first nonzero paired-mean pair, its magnitude
is retained while its direction is aligned to the duration-specific reseeded
bridge's first powered direction; an initially unpowered pair is located and
kept distinct. The frozen launch-tilt target is unchanged. Later powered target
angles are derived from their corrected vectors. A deterministic four-parameter
correction applies constant and normalized linear-in-time x/y acceleration
offsets, ramped from zero at the first powered pair. Central finite differences
use `1e-4 m/s^2`; damped
least squares has at most six iterations and eight descending line-search
scales per iteration. Each coefficient is bounded to `[-0.25, 0.25] m/s^2`.
The initial damping is `1e-3`, reduced by `0.3` after accepted steps and
multiplied by `10` after an exhausted line search. An incomplete Jacobian,
unreached handoff, singular system, or exhausted budget is recorded as a
solver outcome, not evidence that the endpoint is unreachable.

Every trial uses the actual evaluator/core two-tick rollout, including
attitude slew, throttle mapping, current mass/fuel, and plant command
saturation. The schedule preserves the first powered pair's reference
direction; any earlier unpowered pair retains the frozen launch target.
The schedule has separate source screens for robust coupled thrust, minimum
throttle and saturation, powered slew, the original first-powered-direction
source-attitude margin, launch boundary, the unchanged V2 clearance mirror, aggregate fuel/time, and
authoritative core no-contact through the source phase. The V2 mirror is
reported separately; it is not treated as a replacement for core contact.
These physical screens are used during line search; strict endpoint tolerance
is a separate final gate so a nonzero residual can improve iteratively.

The minimum-throttle evidence is the minimum positive mapped
`desired_applied_throttle_frac` (after command-to-throttle mapping), compared
with the vehicle's declared minimum. It is not the raw command throttle
fraction and is not the plant's `plant_applied_throttle_frac`; the latter is
recorded separately for both physics steps in each command ledger row.

A witness requires source handoff within `1e-6 m` and `1e-6 m/s`, every
source-specific screen, and an exact match between the scheduled source-gate
rollout and the source prefix of its scheduled full-flight run. Prefix matching
checks per-step rollout evidence, state and command samples, handoff endpoint
and errors, source contacts, launch evidence, and reseeded bridge. The final
full flight then replays the logged scheduled commands through ordinary and
neutral core states; both parity checks and an authoritative first-contact
record are required. Contact classification is recorded, not optimized or
required to be stable.

The schedule-specific launch-boundary screen records the frozen launch target,
actual post-launch attitude, reference first-powered angle, and scheduled first
powered angle. It requires the scheduled angle to retain the reference direction
and recomputes the one-physics-tick slew margin from actual launch-end attitude
with the unchanged declared robustness margin. The four-parameter correction
is zero through the first powered pair so it cannot rotate that boundary
command. The source-attitude screen preserves the existing convention: its
robust margin is computed for the first powered scheduled target only. That
target angle and its margin are recorded along with the actual attitude after
the first physics tick of its held pair. Later source angles are not treated as
pad-departure attitude; they remain subject to powered slew, clearance, and
authoritative contact screens.

Only if all three representatives pass that complete gate does the runner
apply the same unretuned algorithm to the remaining six survivors. Otherwise
those rows are preserved as `deferred_after_first_gate_failure`, and execution
stops at the predeclared first gate. The six analytical skips remain explicit
and are never simulated. Every row records whether it is an analytical skip,
first-gate representative, second-phase survivor, or deferred survivor.

The paired-command ledger labels the two completed physics steps that used
each command (the first source pair is steps 73 and 74 after the 72-tick launch).
It records requested acceleration and attitude, held throttle request, and
both per-step applied throttle fractions. The artifact also retains solver
iterations and line-search trials, signed endpoint residual and full endpoint
state, schedule screens, full-flight rollout, replay parity, and first
authoritative contact with its signed first-contact predicate margins.

## Artifact and acceptance

Schema `waypoint_direct_source_duration_paired_command_feasibility_v1` uses a
stable semantic identity with its own identity field cleared. It excludes
output paths and wall-clock values, creates `summary.json` with create-only
semantics, and verifies its serialized identity round trip. Use two fresh
output directories and require byte-identical summaries before accepting the
determinism check. The baseline control lanes and frozen canary/tail are never
rewritten.

The primary result is the first-gate outcome and its strict witness/screens.
A passing result establishes only that a bounded schedule was found for these
known frozen rows under this evaluator. A solver failure is not proof of
infeasibility. No result here certifies a landing or supports held-out,
population-wide, production, or default-behavior claims.

## Result

The accepted `run_d` and `run_e` outputs are byte-identical. Both retain all
15 ordered rows (six explicit analytical skips), nine frozen baseline pairs,
three passing `-180` representatives, and six completed second-phase
survivors. All nine scheduled source handoffs pass `1e-6 m / 1e-6 m/s`, all
schedule-specific screens, scheduled source-prefix equality, and ordinary /
neutral replay parity. Across the nine survivors, position error ranges from
`3.7755e-8` to `6.2144e-8 m`; velocity error ranges from `6.5887e-10` to
`1.2805e-9 m/s`.

| `-180` representative (row index) | Position error (m) | Velocity error (m/s) | Full-flight first contact |
| --- | ---: | ---: | --- |
| Native (1) | `5.8359e-8` | `1.1339e-9` | Stable touchdown on target |
| Research-shortest (6) | `5.4116e-8` | `1.0665e-9` | Crash |
| Third (11) | `4.8352e-8` | `8.3877e-10` | Stable touchdown on target |

Over all nine scheduled full flights, four made stable on-target contact and
five crashed. Positive signed predicate margins pass the corresponding
threshold; negative margins fail it. The minimum first-contact margins across
those nine contacts are:

| Predicate margin | Minimum | Minimum row index |
| --- | ---: | --- |
| Stable minimum foot clearance | `+0.059482 m` | Research-shortest (5) |
| Stable maximum foot clearance | `-0.168500 m` | Research-shortest (5) |
| Stable hull penetration | `+0.004796 m` | Third (12) |
| Safe normal speed | `+1.029661 m/s` | Research-shortest (5) |
| Safe tangential speed | `+1.968316 m/s` | Research-shortest (9) |
| Safe attitude | `+0.108991 rad` | Research-shortest (5) |
| Safe angular rate | `+0.244620 rad/s` | Research-shortest (5) |
| Left touchdown-pad edge | `+13.556176 m` | Research-shortest (9) |
| Right touchdown-pad edge | `+14.224296 m` | Third (11) |

These contact margins are recorded outcomes, not schedule acceptance gates; the
negative stable-clearance margin occurs on a crash contact and is not a stable
landing claim. Semantic identity is `fnv1a64:080e8e9867b02d97`; both summaries
have SHA-256
`6477a947537208872056d46e82c7b6fc0c7aa6801f14ca9ccb8b340f4da86247`.
The accepted files are
`outputs/research/waypoint_direct_paired_command_feasibility_20260925/run_d/summary.json`
and
`outputs/research/waypoint_direct_paired_command_feasibility_20260925/run_e/summary.json`.

Temporary `/tmp` run-b was provisional because it applied the first-powered
source-attitude limit to later source angles; temporary run-c passed the
feasibility gate but predates persisted first-contact margins. Neither is part
of the accepted byte-identical pair.

## Reproduction

From the repository root, verify the frozen source digest and validate inputs:

```sh
rtk sha256sum outputs/research/waypoint_direct_source_duration_canary_20260924/run_d/summary.json
rtk cargo run -p pd-eval -- waypoint-direct-source-duration-paired-command-feasibility \
  --source-duration-summary outputs/research/waypoint_direct_source_duration_canary_20260924/run_d/summary.json \
  --baseline-summary outputs/research/waypoint_direct_contact_contract_20260924/baseline/summary.json \
  --sweep-summary outputs/research/waypoint_direct_contact_contract_20260924/sweep/summary.json \
  --nominal-summary outputs/research/waypoint_direct_contact_contract_20260924/nominal/summary.json \
  --launch-summary outputs/research/waypoint_direct_contact_contract_20260924/launch/summary.json \
  --contact-audit-summary outputs/research/waypoint_direct_contact_contract_20260924/audit_a/summary.json \
  --flat-canary-summary outputs/research/waypoint_direct_flat_candidate_closure_20260924/canary/summary.json \
  --coupled-audit-summary outputs/research/waypoint_direct_coupled_thrust_audit_20260924/run_a/summary.json \
  --output-dir /tmp/pd-paired-command-preflight-only \
  --preflight-only
```

Remove `--preflight-only` to run the experiment. Use a fresh output directory
for each run; the writer refuses to overwrite an existing `summary.json`.
