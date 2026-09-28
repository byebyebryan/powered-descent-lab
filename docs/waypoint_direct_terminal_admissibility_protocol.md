# Frozen terminal-admissibility and execution-isolation protocol

This evaluator-only diagnostic is a continuation of the
[obstacle-discrimination checkpoint](waypoint_direct_obstacle_discrimination_results.md),
not a new generator policy, controller fix, or waypoint experiment. The
retained missions, launch/source programs, tail references, clearance rules,
vehicle, core contact predicates and accepted-witness ledger stay unchanged.

## Questions and stopping rule

1. Does the ideal thrust-aligned terminal reference itself reach admissible
   first body contact under the existing discrete core predicates?
2. From the identical replayed terminal-entry state and original global clock,
   what changes if terminal commands update every physics tick instead of
   following the frozen global 60 Hz clock?
3. Do the retained accepted controls survive that diagnostic substitution?

Stop after this comparison and its integrity/validation gates. A changed
cadence result is not a new accepted complete witness. An inadmissible reference
does not prove there is no other direct arc. No result authorizes weakened
contact thresholds, waypoint demand, a new source fit, sealed-input retuning,
or production/default integration.

## Frozen input ledger

Use the exact retained eight-case fresh gate under
`outputs/research/waypoint_direct_obstacle_discrimination_20260925/fresh_run_a/`.
The raw root summary SHA-256 is
`3282a7e533809009a9e5cf377ad8cbbfea598a2b4840df533fbe696015409baa`,
and its semantic identity is `fnv1a64:a484e83d323f3e02`.
The sealed manifest is
`fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json`,
SHA-256 `647df7bc94d02a9b48b773c45159e0ffa15bfca232a13b4e21164e2251a8f5b4`.
The runner pins every full generation summary's raw digest, recomputes its
semantic identity, and checks scenario, policy, basis and ordered row bindings.

All input checks must finish before output creation or physics. Preserve the
exact order of flat, low, high and late/broad at 700 m, then the same four at
900 m. The input ledger is 160 rows, 84 analytical skips, 76 complete schedules,
34 accepted witnesses and 42 rejected target-contact crashes. Audit all 76
scheduled rows; retain the skips in the preflight counts, not as fabricated
physical observations. No generator or fitter is called.

Run the full paired cadence comparison on these nineteen explicit keys:

| Case ID suffix | Rows | Purpose |
| --- | --- | --- |
| high_obstacle_span_700 | 16, 17, 18, 19 | All four high-case failures |
| high_obstacle_span_900 | 16, 17 | Both high-case failures |
| flat_control_span_700 | 10, 11, 12, 13, 14 | Five near-margin failures |
| low_obstacle_span_700 | 10, 11, 12, 13, 14 | Five matching near-margin failures |
| flat_control_span_700 | 16 | Accepted control |
| flat_control_span_900 | 10 | Accepted control |
| late_broad_span_900 | 16 | Accepted alternative-direct control |

Each full ID starts with `fresh_`. Row selectors, original acceptance, complete
wrapper and first-contact-only rejection bindings are checked before physics.
These repeated duration offsets share tail references and clock phase; they
are not nineteen independent mission geometries.

## Reference contact audit

Reconstruct the exact discrete affine terminal bridge from the generated
basis's original source handoff, unchanged coast duration, terminal duration,
target position and terminal downward-speed fraction. Bind its desired angles
and ordinary replay position/velocity error magnitudes to the frozen logs.
Use the existing post-burn-mass throttle inverse in physical lanes.

Reference body attitude is thrust-aligned. The first sample assumes alignment
and zero reference angular rate; subsequent signed rates are consecutive
shortest-angle differences divided by the physics timestep. This reference
pose study does not certify the entry-attitude join or commandability.

For each reference post-pose, invert an idle synthetic pre-state and submit
one neutral step to the unchanged authoritative core contact classifier.
Check that the returned position, velocity, attitude and rate match the
requested reference pose; independently require evaluator predicate-mirror
parity. Stop at first core contact, retain incoming velocity and all signed
contact margins, including both feet, hull penetration, normal/tangential
speed, attitude, angular rate and target-pad containment. Exceeding a slew
limit invalidates the pose path, rather than hiding it in the synthetic probe.
Unpowered terminal samples are explicitly unsupported in this retained audit.

Apply the unchanged pointwise exact rotated-body terrain-domain and clearance
scan to airborne poses: 5 m reserve outside the existing whole-body flat-pad
transition corridors and 0 m inside them. Contact uses the core penetration
predicates, not a contradictory positive airborne-clearance requirement.
No-contact within the stored reference is not success. "Admissible reference
pose path" means these geometric/contact checks passed; it is not a physical
flight, fuel proof, continuous swept-path proof or robustness certificate.

## Matched-entry physical lanes

Authoritatively replay each complete frozen program and require its original
log/contact result. Replay the contact-free prefix again to the exact state
immediately before the first terminal physics tick. Retain global step/time,
position, velocity, body attitude/rate, fuel, held command, extrema and original
controller-clock phase. Clone this same state into both lanes; never reset time
or realign a phase-local clock.

- **Frozen held 60 Hz:** on the original global update ticks use the stored
  requested throttle and desired attitude, holding the existing command on
  other ticks. Require terminal log, first-contact state/predicates, and ordinary
  versus neutral replay parity with the historical result.
- **Terminal-only 120 Hz:** every terminal physics tick recompute throttle from
  the unchanged reference thrust magnitude and that lane's current mass, and
  command the unchanged reference attitude. Core slew, minimum throttle,
  integration and contact predicates remain identical. Source/coast are not
  replayed with a different cadence.

Stop each lane at its own first core contact or the reference end. Report full
per-tick command/pose/signed-error traces, contact predicates/margins, thrust
saturation counts and body-clearance evidence. Ordinary stable touchdown
zeroes velocity/rate; retain neutral incoming contact velocity for causal
comparison. Compare signed cross-lane residuals only over their common live,
airborne global ticks, never extrapolating one lane past its contact event.

## Integrity and exit gate

Create-only outputs carry frozen input bindings, explicit selection, current
diagnostic source/protocol hashes, case evidence and compact aggregate results.
Verify the source binding is unchanged across evaluation. Run independently
into two fresh directories; compare root and all eight case summaries byte for
byte. Identities exclude output paths and wall-clock time. The prior production
freeze belongs to the historical input, not the additive diagnostic code.

Mandatory integrity gates are all 76 original baseline/contact reproductions,
reference bindings, reference-pose/core-mirror checks, matched-entry identity
and global-phase preservation, and ordinary/neutral comparison parity. Empirical
reference and 120 Hz contact outcomes are observations, not enforced success
counts. Preserve all historical input bytes, production planner/controller/core
code and the original 34/42 acceptance ledger.

Run focused probe/clock/refusal and CLI tests, the integrated workspace tests,
strict all-target workspace Clippy, formatting and diff checks. Report the
causal limit and the smallest justified next decision without implementing it.
No commit, push, deployment or automatic follow-on policy experiment.
