# Nominal ballistic-direct flight integration V1

## Objective and boundary

Integrate the committed body-aware nominal ballistic-direct generator with the
ordinary `pd-control::run_controller` / `pd-core::run_simulation` path. A Direct
decision carries a complete launch-to-contact command program, not a V1 chord
route label. This is an explicitly selected nominal flight mode, not a default
planner/controller change or a robustness certificate.

The flight policy is frozen at the checkpoint recorded in
[the body-aware terminal results](waypoint_direct_body_aware_terminal_results.md).
Keep the original four analytical bases, five source-duration offsets, seven
terminal-duration offsets, source fitting, body-aware reference, real attitude
joins, safety thresholds and intended 120 Hz physics / held 60 Hz command clock.
No stored successful schedule is a generator input. Do not alter historical
protocols, fixtures, results, or artifact bytes.

## Decisions and ownership

- `Direct`: the finite input-generated family contains a complete accepted
  nominal witness. Rank accepted witnesses by planned total physics ticks, then
  unchanged stable witness identity. Do not pick the first accepted source row.
- `Unknown`: a supported, valid request has no complete accepted witness in
  this finite family. This is not physical impossibility or waypoint necessity.
- `Unsupported`: a valid input uses unsupported physics, vehicle, policy,
  geometry, source state, or operational waypoint/mission setup.
- `Invalid`: malformed scenario data or inconsistent requested pad bindings.

Unknown, Unsupported and Invalid execute no flight. This slice does not invoke
the existing chord planner or attempt waypoint search. Unknown is the explicit
future escalation boundary; there is no silent controller fallback.

`pd-core` owns neutral serializable timed-program contracts and structural,
context and clock validation. `pd-control` owns the opt-in playback executor,
using the unchanged ordinary runner. `pd-eval` owns generation, acceptance,
provenance, CLI, create-only bundles and observational compute evidence. Do not
make `pd-plan` depend on evaluator or controller code. Do not add this mode to
the built-in default controller registry.

## Program and clock contract

Bind the program to the resolved simulation configuration, immutable world,
vehicle, initial state and mission, explicit pad IDs, generation/terminal
policy identities and selected witness identity. Persist source handoff,
terminal entry, planned end and expected authoritative contact ticks.

Research witness updates use the post-step indices 1, 3, 5, ... . The neutral
program uses ordinary pre-step callback indices 0, 2, 4, ... : subtract exactly
one, without a phase-local clock reset or command recomputation. Retain commands
and phase labels exactly. A source handoff may be off the control clock; terminal
entry must be aligned by the unchanged real coast alignment step.

Programs end at actual first contact, which may precede the planned reference
reserve. Require contiguous command coverage through expected contact, not
extra commands through planned end. Reject unsupported versions, inconsistent
context/pads, missing/extra/off-clock updates, nonfinite or out-of-range
commands and inconsistent phase bounds before ordinary flight execution.
Structural validation alone does not certify physical safety.

Before execution, independently verify the selected serialized generator
witness, including source binding and complete ordinary/neutral physical
replay. Then check that conversion reproduces the exact command program.
Execute with the ordinary controller runner. Compare its actions, authoritative
contact tick, fuel and outcome to the verified witness; independently replay
the ordinary action log and require exact deterministic artifacts. Retain the
neutral incoming contact evidence: ordinary stable-contact velocity zeroing
must not be misread as incoming touchdown velocity.

## Acceptance gates

1. Contract/executor tests run without retained research outputs. Cover
   serialization, initial tick mapping, two-tick command hold, malformed
   coverage/payload/bindings, phase alignment, end mismatch and repeatability.
2. Run all 14 prior development cases and all ten prior fresh cases as 24
   **exposed regression controls**. Compare against the immutable body-aware
   case artifacts, not their old whole-source freeze. Require exact generated
   case/witness payload parity, selected row and commands, normal-run action
   parity, safe target contact at the same tick and independent replay parity.
   No case retuning or new held-out claim is allowed.
3. Negative tests include a supported request with finite-family exhaustion,
   invalid bindings, unsupported policy/physics/vehicle/source state, and
   missing, off-clock or altered commands. An altered self-rehashed program
   must not bypass selected-witness binding or physical acceptance.
4. Repeat the 24-control gate independently. Compare deterministic artifacts
   excluding observational timings and HTML containing those timings. Require
   all controls recorded even if an acceptance check fails. Keep failure
   artifacts and stop at the first divergent integration boundary for diagnosis;
   do not expand terrain sweeps or revise the flight policy.
5. Run focused tests, the workspace test suite, strict all-target workspace
   Clippy, formatting and diff checks. Historical-output-dependent acceptance
   is a separately invoked integration command, not a silently skipped unit
   test. The default tests must work without ignored local output artifacts.

## Cost and artifacts

Measure release-mode generation, selected-witness verification, ordinary
execution/action replay, and artifact/report writing separately. Record
deterministic candidate/attempt counts and known replay tick counts. Existing
source-generator counters are upper bounds, not measured total work: if full
generation tick accounting is unavailable, state that explicitly instead of
labeling a subtotal or bound as actual total physics ticks.

All wall/CPU/per-update timings are observational and excluded from decisions,
program identities and deterministic run identities. Research generation still
performs old-policy comparison work; this checkpoint measures that integrated
backend, not an optimized production planner. No millisecond performance target
or real-time authority is inferred.

The opt-in CLI accepts a complete mission scenario and explicit source/target
pad IDs, with a read-only preflight option. Output roots are create-only. Persist
the resolved request, decision, generated evidence, program, selected witness,
independent safety audit, ordinary run artifacts and a generic run report. Do
not attach a misleading V1 chord plan to the report. Non-Direct bundles contain
the decision but no executed flight.

The regression gate additionally records source/input/protocol hashes (including
the embedded V2 vehicle fixture and original flat-scenario input), archive
hashes before and after, per-case parity checks and measured costs. New source
bindings describe this integration pass; do not relabel or reuse old research
freeze summaries as current authority.

## Exit and non-goals

Close with a reusable opt-in nominal Direct flight capability and a complete
24/24 exact-parity verdict, or a precise bounded integration failure. Cost is
measured, not optimized in this pass. Robustness and useful setup cost remain
gates before default/production selection; millimetre contact margins remain
visible. Waypoint composition and arbitrary incoming waypoint states are
separate capabilities. No commit, push, deployment, roadmap expansion, source
refitting, contact-threshold relaxation or feedback retuning is authorized.
