# Flat direct-leg coupled-thrust audit protocol

This is an opt-in, no-new-flight diagnostic of the three already flown flat
direct profiles. Its question is why the two fixed-launch profiles that land
have less than the declared coupled-thrust robustness reserve. It does not
search profiles, change the launch, relax policy, rerun physics, or promote a
planner/controller/default capability. The original V2 candidates and their
fixed-launch source-bridge wrappers are different objects and must remain
separately identified.

## Frozen input gate

Before recomputing a bridge or writing an artifact, rebuild and verify the
existing baseline, sweep, nominal plant, launch-feasibility, first-contact,
and third-candidate canary artifacts. Their expected semantic identities are
`fnv1a64:d3fa6b24336f7c05`, `fnv1a64:1bcd5a3bd6c6da01`,
`fnv1a64:9e8cbc902ca11fbc`, `fnv1a64:2c7b965ffdc809d6`,
`fnv1a64:f5a6600e99cd283b`, and `fnv1a64:f3fba9290c9073be`.
Retain the nested nominal manifest and launch/contact/canary input-gate
bindings. The only candidate bases are flat native
`fnv1a64:dee613017622ca16`, research-shortest
`fnv1a64:4e6c0b23f9eb1b8f`, and third
`fnv1a64:a18a98ad6e334014`. Each must remain originally V2-certified.
Any identity or binding mismatch stops the pass without a result.

## Analytical reconstruction

For each flat candidate, materialize its *original* exact discrete source
bridge from the reevaluated baseline result. Then independently re-solve its
source bridge from the achieved 60+12-tick launch state stored in the frozen
launch/canary evidence to that candidate's original source handoff, using
its original bridge tick count. Recomputed source-bridge identity, endpoint,
component margins, classification, and reason set must match the frozen
original or wrapper evidence. The 120 Hz and 60 Hz lanes must bind the same
re-solved bridge if their achieved launch states are identical; their later
held-command trajectories remain separate.

For each original/re-solved pair record the start-state position and
velocity offset, exact affine bridge coefficients, first/last requested
thrust vectors and magnitudes, peak tick, derated acceleration limit,
declared robust limit, raw and normalized coupled-thrust reserve, and each
other component margin. Check the maximum against every materialized sample:
the norm of an affine thrust sequence is convex, so a peak must occur at an
endpoint. Decompose the peak-vector change into the position and velocity
start-offset terms of the exact bridge equations. Those terms are analytical
attribution under a fixed handoff and duration, **not** alternate launches
or flown trajectories.

The frozen policy and vehicle imply a `14.4 m/s²` derated acceleration
limit at worst-case mass and a `13.32 m/s²` limit after the declared 7.5%
reserve. Existing stored margins imply peaks near `13.593 m/s²` (native),
`12.886 m/s²` (shortest), and `13.605 m/s²` (third) after launch. These
numbers are expectations to reproduce independently, not thresholds to tune.

## Existing execution cross-check

Using only frozen per-step logs **where retained**, map each re-solved peak
source tick to its absolute physics step after the 72-tick launch. The
original launch-feasibility artifact retains both cadence tick logs and
saturation totals for native and research-shortest profiles; report their
desired/held attitude, commanded and applied throttle, fuel, and saturation
evidence. The third-candidate canary retained outcomes and handoff errors
but **not** complete post-launch per-step logs or saturation totals. Mark
those third-candidate peak-execution fields unavailable instead of
replaying physics or fabricating them. Retain each lane's source-handoff
error and actual first-contact/outcome classification as context. A held
60 Hz command is not assumed to equal the nominal per-tick thrust sample;
actual command saturation and an analytical worst-case-mass reserve are
different measurements. Do not reclassify a successful nominal landing as
robust.

## Stop gate, artifact, and acceptance

The first gate is independent reproduction of the pinned source-bridge
identities and margins. If it fails, stop at an evidence/model mismatch.
If it passes, report whether the reserve loss is accounted for by the
achieved launch state under the fixed handoff/duration and whether existing
logs show a separate commandability issue. Then recommend exactly one
*next* study: a fixed-launch/policy candidate-generation design, a separate
launch/controller investigation, or a separately justified policy study.
This pass itself executes none of them. A finite candidate search cannot
prove physical impossibility; the current flat landings cannot justify
loosening the 7.5% requirement.

Write a separate deterministic semantic-identity artifact, excluding paths
and wall-clock data and refusing to overwrite an existing summary. Run the
no-physics input gate, focused formula/identity tests, and two fresh-path
runs whose summaries are byte-identical. Verify the old sealed hashes stay
unchanged, then run workspace format, tests, strict lint, and diff-integrity
checks on the integrated tree. No production planner, simulator, controller,
contact, orientation, F6, or default-selection behavior changes.

## Recorded result

The no-physics gate accepted all six pinned source artifacts and exactly the
three frozen flat candidates. Independent exact source-bridge reconstruction
matched the stored original and re-solved bridge identities, component
margins, classification, and reasons. The new input-gate identity is
`fnv1a64:099d25787a2694f1`; the audit artifact is
`fnv1a64:59cbe1bd2dd9b8d2`.

| Candidate | Original peak / normalized reserve | Fixed-launch re-solved peak / reserve | Re-solved robust-cap deficit | Flown outcome |
| --- | ---: | ---: | ---: | --- |
| Native | `13.0268 m/s²` / `0.09536` | `13.5928 m/s²` / `0.05606` | `0.2728 m/s²` | Landed |
| Research-shortest | `13.0658 m/s²` / `0.09265` | `12.8859 m/s²` / `0.10514` | `0` | Crashed at first contact |
| Third | `13.0260 m/s²` / `0.09542` | `13.6053 m/s²` / `0.05518` | `0.2853 m/s²` | Landed |

The re-solved peak lies at the *last* source-bridge sample for all three
candidates (physics step 1,812 for native/shortest and 1,992 for third),
not at launch. At that frozen endpoint, the achieved launch state adds
`0.7014 m/s²` of requested vertical thrust for native/shortest and
`0.6323 m/s²` for third, relative to each original bridge at the same
sample. Exact affine-coefficient attribution assigns about `0.6599` and
`0.5981 m/s²`, respectively, to the launch velocity offset; the position
offset accounts for about `0.0415` and `0.0341 m/s²`. This explains the
margin movement *within the fixed handoff and duration equations*. It does
not prove that a different launch would be safe or that one part of the
offset is an independently flyable trajectory.

Both older flat profiles' sealed 120 Hz and 60 Hz logs show no throttle,
fuel-burn, or fuel-exhaustion saturation; their applied throttle at the
re-solved peak was approximately `0.735` (native) and `0.698` (shortest).
Those physical-command values do not replace the worst-case-mass analytical
reserve. The third candidate's peak-tick command and saturation fields are
unavailable because its sealed canary did not retain those per-step logs;
its handoff errors, first contact, and landing outcome remain available.
All three held 60 Hz lanes miss strict nominal source handoff, a separate
execution finding.

Three fresh output paths at
`outputs/research/waypoint_direct_coupled_thrust_audit_20260924/run_a/summary.json`
and `run_b/summary.json` and `run_c/summary.json` are byte-identical, SHA-256
`4a40e971c2b67f6ac395ad8651c99cfff9fa6ae7c9c4611a55a7987a5bff4226`.
Preflight and six focused tests passed. The final integrated tree passed
644 workspace tests across 12 suites, formatting, strict Clippy, and diff
integrity. The original launch, contact-audit, and third-canary summaries
retained their sealed SHA-256 hashes after integration.

The audit's stop gate is satisfied: this is not a bridge-identity or margin
calculation mismatch, and the logged older lanes show no physical command
saturation. The recommended **next design study** is a bounded,
launch-aware candidate-generation/selection experiment with the 60+12-tick
launch and 7.5% robustness policy held fixed, screening actual first-contact
geometry and reporting 60 Hz held-command behavior separately. If that
fixed-launch study cannot produce a candidate meeting both analytical and
flown-contact conditions, then investigate the launch rule as a separate
choice. Do not change the threshold, contact rule, or default selection on
the strength of this outcome-known flat set.
