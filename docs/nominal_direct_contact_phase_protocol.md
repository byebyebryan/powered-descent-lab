# Nominal direct first-contact and command-coverage study V1

## Question and scope

Separate physical first-contact sensitivity from the exact-replay contract of
the committed opt-in nominal direct-flight executor. This is a bounded
evaluator-only diagnostic, not a new generator, accepted flight capability,
recovery controller, operational fallback, or robustness certificate.

Baseline: `da0f78993fd65085b69384a52345249b6feb9d56`, the reviewed nominal
direct-flight integration checkpoint. Use its 24 selected programs as frozen
diagnostic inputs. These are exposed regression controls, not new held-out
cases. Saved successes must not become generator seeds.

The reported hull-penetration margin is a contact-predicate margin, not obstacle
clearance. Core permits penetration up to the greater of 0.012 m and one physics
step of contact-closing motion, including the angular-rate hull-radius term.
For the flat-600 baseline, incoming normal speed is 1.5 m/s at 120 Hz: 12.50 mm
allowance minus 5.91 mm penetration leaves 6.59 mm predicate slack. Do not treat
that number as a physical corridor width or prescribe a new margin threshold.

The operational nominal runner also requires contact at the exact expected
physics tick. Its saved commands cover actual first contact, not the entire
planned reference reserve. A safe contact at another tick is a nominal replay
mismatch; missing command coverage is not by itself a physical crash.

## Frozen inputs and preflight

Default input root:
`outputs/research/nominal_direct_flight_integration_20260928/gate_a/`.

Its root `summary.json` is pinned to SHA-256
`58801ab890cc094c30d6466e3b26ef7c667a10dfdcb272c03de5b0d94167e280`,
schema `nominal_direct_flight_regression_v1` / 1, passed status, identity
`fnv1a64:3ea6e074f6de460e`, and all 24 ordered case bindings. An alternate
input root is a relocation of these same inputs, not permission to substitute
another baseline or silently change the case population.

Before any simulation, validate all required input files and their typed
bindings: scenario/request, fixed policies, selected witness/program identities,
context and pad bindings, clock/phase bounds, contiguous commands, and the
post-step witness to pre-step program conversion. Reject malformed, truncated,
tampered, duplicated, missing or path-traversing case bindings. Record used-file
hashes and check them again after execution; never modify or overwrite baseline
artifacts. Bind the current study source and this protocol separately from the
historical baseline source freeze.

Preflight creates no simulation, generation or output. Execution uses a new,
create-only output root and refuses an existing root before simulation. Retain
runtime failure evidence rather than replacing it with a later successful run.

The opt-in CLI is `nominal-direct-contact-phase`; `--preflight-only` and
`--output-dir` are mutually exclusive. `--input-root` optionally supplies an
exact relocation of the pinned baseline root. For example:

```bash
rtk cargo run --release -p pd-eval -- nominal-direct-contact-phase --preflight-only
rtk cargo run --release -p pd-eval -- nominal-direct-contact-phase --output-dir NEW_STUDY_ROOT
```

## Baseline gate and actual entry

Reconstruct each source departure and ballistic coast using the complete saved
commands and unchanged core physics. Preserve the original global 120 Hz
physics / held 60 Hz control clock, command values, fuel, attitude, mission and
physical context. Capture the actual live `SimulationState` at the declared
terminal-entry tick, including its held command and outcomes; do not reseed from
a partial saved pose, reset time, teleport to an endpoint or fit commands.

Prove the source/coast prefix is contact-free and the captured terminal entry
matches the archived witness. Prove ordinary/neutral replay parity and exact
zero-offset baseline command, first-contact audit, physics tick and fuel parity
for **all 24 cases before any nonzero perturbation**. Compare the normal program
execution and its archived deterministic flight artifacts as applicable.
Ordinary successful contact zeroes velocity; preserve incoming contact evidence
from the neutral classification seam rather than reporting that zero as the
landing speed.

A baseline, binding, clock or ordinary/neutral parity mismatch is a harness gate
failure, not a perturbation result. Preserve its evidence and diagnose the first
divergent boundary without filtering cases or changing the flight policy.

## Fixed intervention matrix

At actual terminal entry, clone the full state and change only
`position_m.y += delta_y_m`. Positive offsets mean higher. Ordered offsets:

```text
0, -0.001, +0.001, -0.005, +0.005, -0.010, +0.010, -0.020, +0.020 m
```

Nine offsets times 24 cases gives 216 rows, including the 24 zero-offset rows.
Repeat the complete study independently: 432 case/offset observations. The
offsets are fixed diagnostic probes spanning more than one nominal contact
step, not a promised operating tolerance or continuous sensitivity certificate.
They represent terminal-entry state-error counterfactuals, not complete flights
whose launch physically produced those errors.

No velocity, attitude, angular-rate, fuel, combined disturbance, random seed,
terrain or candidate-family expansion is allowed in this pass. No feedback,
reference recomputation, source refitting or changed contact thresholds.

## Two evidence lanes

Both lanes retain the same real global control clock and stop at actual first
authoritative core contact. They may share their identical replay prefix, but
their outcomes and command-coverage claims must remain separate.

1. **Saved-program coverage.** Replay only the accepted saved commands. If no
   contact has occurred when the next globally due command is missing, stop
   before inventing an idle or held-command fallback. Label the row
   coverage-limited. Audit safe target contact separately from equality with
   the baseline expected tick; safe early contact is not a crash.
2. **Diagnostic continuation.** After saved commands end, repeat the final
   saved command value at the unchanged global 60 Hz update ticks, only through
   the original `program.planned_end_physics_step` and never beyond the scenario
   horizon. This planned end already includes the frozen four-tick reference
   reserve: do not append another four ticks. All current selected cases have
   contact four ticks before planned end, giving at most 33.3 ms continuation.
   Record continuation use and update count. This hypothetical extension
   creates no accepted program/witness and does not bypass nominal acceptance.

If the continuation cap is reached without contact, report coverage-limited
evidence, not a physical crash, accepted recovery, physical impossibility or
waypoint necessity. The continuation is deliberately too bounded to establish
general disturbance recovery.

## Evidence and checks

For each case/offset and lane, retain:

- The intervention, actual entry binding, unchanged command identity, coverage
  boundary, simulation/control ticks, continuation count and stop reason.
- First-contact classification, incoming pose/velocity/fuel, all core predicate
  mirror values and margins, including dynamic penetration allowance and minimum
  hull clearance, plus signed contact-tick shift from nominal.
- Strict body/terrain-domain validity, pointwise clearance evidence and
  ordinary/neutral parity. Keep airborne path clearance separate from allowable
  penetration at first contact; neither is a swept-path certificate.
  Audit the shifted entry pose itself without advancing physics, separately
  from the archived prefix/full scan so nominal scan parity stays exact.
- Explicit stable-safe target, unsafe/off-target, coverage-limited and harness
  invalidity outcomes. An exact-tick mismatch is a separate contract observation.

Nonfinite state, domain escape, unexpected fuel/mission/horizon termination or
parity failure must not be silently counted as a safe contact. Report the
earliest failing invariant and retain completed evidence.

Provide typed JSON summaries and inspectable case/row evidence with baseline
checks, source/protocol/input hashes before and after, contact outcome counts,
tick-shift counts and margin ranges. Deterministic identities exclude all
observational compute timings. Compare both complete runs and require identical
deterministic results and unchanged inputs/source during measurement.

## Validation and exit

Artifact-free focused tests cover baseline parity, actual-entry preservation,
safe changed-tick contact versus genuine crash, missing command coverage,
bounded final-command continuation, malformed/off-clock/truncated/tampered
inputs, path validation, read-only preflight and create-only output/CLI modes.
The retained 24-case study is an explicit integration run, not a default test
silently dependent on ignored outputs.

Run focused tests, the workspace test suite, strict all-target workspace Clippy,
formatting and diff checks. Do not repeat full suites without a relevant source
change or unresolved risk. Record the actual validation scope and failures.

Finish this fixed matrix and make one bounded decision:

- Safe physical contacts with changed timing or coverage: the next design
  boundary is operational completion semantics, not obstacle clearance.
- Genuine unsafe first contact: isolate the violated predicate and propose a
  targeted terminal-control change, without changing it in this study.
- Contact unresolved at the finite cap: retain incomplete evidence and identify
  the coverage boundary; do not infer physical fragility or waypoint demand.

Successful study execution means complete valid diagnostic evidence, not
general robustness. Stop at this verdict without adding further axes or sweeps.
`pd-core`, `pd-control`, `pd-plan`, generator policies, default behavior and
historical artifacts remain unchanged. No commit, push, deployment, roadmap
expansion or robustness promotion is part of this authorization.
