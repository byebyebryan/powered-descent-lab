# Held-cadence diagnostic for the frozen source-duration canary

This opt-in evaluator diagnostic examines the nine already-flown survivors in
the frozen source-duration canary and separates two held-command effects for
one predeclared `-180` row per base. It is a known-outcome diagnostic, not
held-out validation, a certification, a new acceptance gate, or a basis for
changing the planner, controller, simulator/contact rules, V2 policy, F6 path,
or defaults. The source-duration canary and all of its serialized artifacts
remain unchanged.

## Sealed inputs and bounded family

Before constructing a `SimulationState`, the command rebuilds and validates
the upstream source inputs, nested input-gate bindings, coupled-thrust audit,
and the frozen source-duration family. The source summary must have semantic
identity `fnv1a64:dfe0f0feaa15fc24`, the expected schema/family proof, all 15
ordered rows, six analytical-screen skips, and exactly nine 120/held-60
survivors with first-contact records and passing paired replay traces. Its
frozen file SHA-256 is
`a93b728374145d4f12b43b21b7d68e383270db19facdf5964ce91bc47e557e9c`;
verify this separately with `rtk sha256sum` before execution. Runtime preflight
pins the semantic identity and upstream bindings; it intentionally adds no
new hashing dependency solely to compute the file SHA.

The source summary is
`outputs/research/waypoint_direct_source_duration_canary_20260924/run_d/summary.json`.
The upstream relative input paths are the same sealed set used by the source
duration protocol:

| Input | Relative path |
| --- | --- |
| Primitive baseline | `outputs/research/waypoint_direct_contact_contract_20260924/baseline/summary.json` |
| Topology sweep | `outputs/research/waypoint_direct_contact_contract_20260924/sweep/summary.json` |
| Nominal plant | `outputs/research/waypoint_direct_contact_contract_20260924/nominal/summary.json` |
| Launch feasibility | `outputs/research/waypoint_direct_contact_contract_20260924/launch/summary.json` |
| First-contact audit | `outputs/research/waypoint_direct_contact_contract_20260924/audit_a/summary.json` |
| Flat candidate closure | `outputs/research/waypoint_direct_flat_candidate_closure_20260924/canary/summary.json` |
| Coupled-thrust audit | `outputs/research/waypoint_direct_coupled_thrust_audit_20260924/run_a/summary.json` |

All 15 source rows are retained. The six analytical skips are represented
without flight replay. For each of the nine survivors, the diagnostic reruns
the direct-per-tick 120 Hz and combined held-command 60 Hz full flights and
their paired ordinary/neutral core contact replays. Both lanes must exactly
match the frozen canary's launch, reseeded bridge, rollout, contact result,
and replay trace before the diagnostic continues.

Only the native, research-shortest, and third `-180` rows receive the two
additional 60 Hz full-flight modes:

| Mode | Throttle | Attitude target |
| --- | --- | --- |
| `throttle_held_attitude_refreshed_per_tick` | Hold each 60 Hz request for two physics steps | Refresh every physics step |
| `attitude_held_throttle_refreshed_per_tick` | Recompute every physics step | Hold each 60 Hz target for two physics steps |

The original 120 Hz and combined-held 60 Hz lanes are the controls; no third
mixed mode or extra duration is run. A throttle refresh recomputes requested
throttle from that lane's current pre-burn mass and fuel. Each ablation must
complete paired ordinary/neutral core replay parity or the command fails and
writes no completed artifact. Contact classification remains the unchanged
core outcome.

## Evidence and interpretation

For each survivor, signed state residuals are held-60 minus direct-120 at
shared post-step even physics ticks: each sample is the state after the second
physics step in a two-step controller hold, i.e. the end of that held-command
interval, not the tick where the next command is selected. Shared source- and
coast-handoff samples are also labeled. Position and velocity are component
vectors, attitude is a wrapped signed delta, and fuel is signed. Sampling ends
at the common live prefix: neither lane is extrapolated beyond its terminal
state, including post-profile idle. If a source or coast handoff exists in only
one live lane, it is named with the reason the shared residual is unavailable.

Each baseline and ablation lane also has a compact command-hold ledger. It
records the first source-bridge post-step even-tick boundary (the first held
pair endpoint for 60 Hz), then per-phase counts of throttle/attitude refreshes
and held ticks. Per-phase maxima retain the sample that attained the largest
absolute desired-command-minus-held-command fraction, desired-applied-minus
plant-applied fraction, and desired-minus-held target-attitude deltas,
including desired/held/applied values and update-due flags. The command
fraction is compared only with a command fraction; the plant's normalized
applied fraction is compared only with the throttle request's desired applied
fraction, after its minimum-throttle mapping. Throttle demand is recomputed
from that lane's current pre-burn mass and fuel; the ledger summarizes it
without duplicating the full per-step traces.

At each cadence's first contact, the paired core replay records that cadence's
own contact tick, classification, absolute preterminal state, foot states,
and raw minimum hull clearance. It also records signed predicate margins,
with positive values meaning the corresponding threshold is passed: stable
minimum/maximum foot-clearance margins, hull-penetration allowance,
safe normal/tangential speed, attitude and angular-rate margins, and left/right
pad-edge margins. Contact ticks need not match across cadences. No same-tick
contact residual is reported when the other cadence's state at that tick is
not a live preterminal state; no post-terminal ordinary-state values are used.

For each `-180` row, the interaction is computed at common live post-step
control boundaries as `(both-held - direct) - (throttle-only - direct) -
(attitude-only - direct)` for signed state residuals. It is descriptive
nonadditivity among these particular known-outcome lanes. It does not establish
a unique cause, including when the prior source artifact labels a divergence
`unclassified`.

Each of the six mixed ablations additionally records its strict source-handoff
position/velocity errors and booleans, plus signed position, velocity,
attitude, and fuel residuals at shared source- and coast-handoff states versus
that row's direct-120 control. If either lane terminated before a handoff,
the missing residual is explicit; no terminal state is synthesized. These
handoff comparisons, rather than contact classification alone, are the
primary diagnostic for how the two command components relate to the existing
60 Hz strict-handoff miss.

The artifact schema is
`waypoint_direct_source_duration_held_cadence_diagnostic_v1`. Its deterministic
semantic identity is computed with the repository's stable digest after
clearing its own identity field; output paths and wall-clock values are not
included. Output creation uses `create_new` and refuses an existing
`summary.json`. A `--preflight-only` invocation validates identities, gates,
and row coverage without creating any simulation state or advancing physics.

## Result

The final `run_e` and `run_f` outputs are byte-identical. They contain all 15
rows (six explicit analytical skips), nine baseline 120/60 pairs with frozen
flight and replay parity, six full-flight mixed ablations with passing paired
replay parity, signed residuals/contact margins, ablation handoff evidence,
and per-lane command-hold ledgers. Final identity is
`fnv1a64:3c0c861c0a902b95`; both summaries have SHA-256
`5c1dde3178ec1d25c3504e9742346dba9f38c5a29aef076cd714397b51a83ece`.
Earlier `run_a` through `run_d` outputs
are preserved as provisional diagnostics; `run_e` and `run_f` are the
accepted schema-v1 pair.

For the three `-180` representatives, the measured absolute source-handoff
errors are:

| Base | Lane | Position error (m) | Velocity error (m/s) |
| --- | --- | ---: | ---: |
| Native | Both-held 60 Hz | 0.179464 | 0.027454 |
| Native | Throttle held, attitude refreshed | 0.025256 | 0.001621 |
| Native | Attitude held, throttle refreshed | 0.175008 | 0.026803 |
| Research-shortest | Both-held 60 Hz | 0.239568 | 0.036657 |
| Research-shortest | Throttle held, attitude refreshed | 0.048104 | 0.003315 |
| Research-shortest | Attitude held, throttle refreshed | 0.227788 | 0.034949 |
| Third | Both-held 60 Hz | 0.150322 | 0.020603 |
| Third | Throttle held, attitude refreshed | 0.023610 | 0.001958 |
| Third | Attitude held, throttle refreshed | 0.147233 | 0.020261 |

All six mixed ablations still miss the unchanged strict `1e-6 m / 1e-6 m/s`
handoff tolerance. In these three known-outcome representatives, the
throttle-held/attitude-refreshed mode has smaller absolute handoff errors than
the attitude-held/throttle-refreshed mode; the latter remains close to the
combined-held control. This is descriptive evidence that attitude refresh is
associated with smaller handoff misses in this bounded set, not a unique-cause
claim. The native and third mixed lanes made stable target contact; both
research-shortest mixed lanes crashed, matching their baseline classifications.
Signed component residuals, first-contact states, raw clearances, and signed
predicate margins remain available row-by-row in the artifact.

## Reproduction

From the repository root, verify the frozen file digest and preflight first:

```sh
rtk sha256sum outputs/research/waypoint_direct_source_duration_canary_20260924/run_d/summary.json
rtk cargo run -p pd-eval -- waypoint-direct-source-duration-held-cadence-diagnostic \
  --source-duration-summary outputs/research/waypoint_direct_source_duration_canary_20260924/run_d/summary.json \
  --baseline-summary outputs/research/waypoint_direct_contact_contract_20260924/baseline/summary.json \
  --sweep-summary outputs/research/waypoint_direct_contact_contract_20260924/sweep/summary.json \
  --nominal-summary outputs/research/waypoint_direct_contact_contract_20260924/nominal/summary.json \
  --launch-summary outputs/research/waypoint_direct_contact_contract_20260924/launch/summary.json \
  --contact-audit-summary outputs/research/waypoint_direct_contact_contract_20260924/audit_a/summary.json \
  --flat-canary-summary outputs/research/waypoint_direct_flat_candidate_closure_20260924/canary/summary.json \
  --coupled-audit-summary outputs/research/waypoint_direct_coupled_thrust_audit_20260924/run_a/summary.json \
  --output-dir outputs/research/waypoint_direct_held_cadence_diagnostic_20260925/run_e \
  --preflight-only
```

Remove `--preflight-only` to run and write the artifact. The final fresh runs
used `run_e` and `run_f`; use a fresh output directory for any further run,
because the writer will not overwrite a prior summary. The paired final runs
had identical semantic identity and serialized bytes.
