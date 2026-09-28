# Frozen terminal-admissibility and execution-isolation results

The two problems are now separated: some retained terminal references cannot
land safely even with essentially exact tracking; another group has safe ideal
contact geometry but loses it under the frozen held-command execution. This
is evaluator-only diagnosis under the unchanged core, not a planner/controller
fix or a new accepted-witness ledger. The
[protocol](waypoint_direct_terminal_admissibility_protocol.md) fixes the inputs,
selection, replay conventions and evidence limits.

## Input preservation and integrity

The saved eight-case obstacle gate retains all 160 rows: 84 analytical skips,
76 complete schedules, 34 accepted witnesses and 42 target-contact rejections.
All eight full generation summaries passed pinned raw SHA-256, recomputed
semantic identity, scenario/policy, ordered basis/row, complete-wrapper and
selector checks before physics. No generator or fitter was invoked.

All 76 authoritative full-program replays and terminal held-60 Hz baselines
reproduced the frozen contact/log results. Reference reconstruction bindings,
synthetic pose matching and evaluator/core contact-mirror parity passed for
all 76. No helper error or mandatory row-guard failure occurred. All reference
airborne clearance scans and reference slew checks passed. Worst frozen-log
binding residuals were `1.91e-14 rad`, `1.09e-11 m` and `8.74e-13 m/s`.

Original global clock phases were preserved: 60 terminal entries were aligned
to the 60 Hz update and 16 were off-clock. Every paired physical comparison
cloned the same complete entry state, including its held command and clock.
Ordinary/neutral contact parity passed in both modes for all 19 comparisons.
The neutral evidence retains incoming contact velocity/rate; frozen ordinary
stable-touchdown logs use the existing zeroing convention.

The original `34 accepted / 42 rejected` ledger is unchanged. The 120 Hz tails
below are counterfactual diagnostic executions, not newly generated or fully
accepted witnesses. Core, controller, planner, generator, mission inputs and
default behavior were not changed. No source-first-step crash was encountered.

## Reference contact audit: all 76 schedules

| Original complete witness | Rows | Admissible ideal pose path | Inadmissible ideal pose path |
| --- | ---: | ---: | ---: |
| Accepted | 34 | 34 | 0 |
| Rejected | 42 | 10 | 32 |

Every ideal reference has a first contact and passes the normal-speed
predicate. All 32 inadmissible references fail maximum-foot clearance at
first contact; the ten admissible rejected rows are flat-700 and low-700
rows 10–14. This finding is conditional on the declared thrust-aligned body
pose: the first reference sample assumes alignment and zero rate, and later
rates follow successive angles. Synthetic idle pre-state inversions submit
these poses to unchanged core classification; they are not physical flights,
entry-attitude joins, fuel proofs or commandability certificates.

The nominal affine terminal bridge checks its planned endpoint, but the craft
makes body contact earlier. A safe endpoint velocity and the ordinary attitude
limit therefore do not imply admissible first contact. Both touchdown feet
must satisfy the current stable geometry predicates. With the retained 8 m
foot span, their height difference on a flat pad is `8 sin(abs(theta))`:
the high-case contact tilt of about `0.06 rad` produces a roughly half-metre
height difference, despite passing the broader `0.15 rad` attitude limit.
Translating the endpoint height alone does not remove that separation.

## Matched-entry cadence comparison: nineteen fixed rows

| Selection | Rows | Frozen 60 Hz stable target contact | Terminal-only 120 Hz stable target contact |
| --- | ---: | ---: | ---: |
| All retained high-700/high-900 failures | 6 | 0 | 0 |
| Flat-700/low-700 near-margin failures | 10 | 0 | 10 |
| Accepted flat-700, flat-900, late/broad-900 controls | 3 | 3 | 3 |

The 120 Hz lane changes only terminal command update frequency. It recomputes
the same reference's throttle inverse from current mass at each physics tick
and commands its same desired attitude, keeping the actual entry, source/coast,
terrain, core slew, minimum throttle and contact predicates fixed. It is not
the production controller and is not a proposal to change its configuration.

All nineteen 120 Hz lanes passed pointwise body clearance and had zero
below-minimum or above-maximum thrust saturations. Their maximum airborne
reference errors were only `5.90e-8 m` and `1.35e-9 m/s`: tracking was essentially
exact at the retained numerical source-handoff error scale. This makes the
high-case remaining failure a reference/contact construction problem, not a
need for still more tracking accuracy on those tails.

| Representative row | Ideal upper foot / normal speed | Held 60 Hz upper foot / normal speed | Terminal 120 Hz upper foot / normal speed |
| --- | --- | --- | --- |
| High-700 row 16 | 0.518830 m / 1.649124 m/s | 0.430243 m / 3.051174 m/s | 0.518830 m / 1.649124 m/s |
| High-900 row 16 | 0.475409 m / 1.665538 m/s | 0.412881 m / 3.334370 m/s | 0.475409 m / 1.665538 m/s |
| Flat-700 row 10 | 0.143023 m / 1.634504 m/s | 0.165940 m / 1.862187 m/s | 0.143023 m / 1.634504 m/s |

The limits remain `0.15 m` upper-foot clearance and `3 m/s` normal closing
speed. For all six high failures, per-tick execution removes the normal-speed
violation but still fails upper-foot clearance, matching the ideal reference.
High-700 row 16 contacts at `64.883333 s`, versus `63.425 s` in held execution;
high-900 row 16 contacts at `68.6 s`, versus `67.191667 s`. Comparing their
independent contact events avoids treating different contact ticks as the
same state. Cross-lane signed residuals stop at the common live airborne prefix.

The ten near-margin cases land under 120 Hz: their upper foot falls from about
`0.16594 m` to `0.143023 m`, leaving only about `6.98 mm` of upper-foot margin.
This is real nominal cadence sensitivity, not a robustness result. These ten
entries are already aligned with the global 60 Hz clock; their failure is
not merely the one idle tick at an off-clock terminal phase boundary. All six
selected high entries are off-clock, but fixing that timing and achieving
near-exact tracking still cannot make their existing reference land safely.
This combined-frequency experiment does not separately attribute throttle,
attitude, mass-update or entry-delay contributions.

Source-duration repetitions share a tail/reference and clock parity. The six
high rows are two distinct high-case tail geometries, and the ten near-margin
rows are the matching flat/low tail geometry with five duration offsets each;
they are not sixteen independent landing designs.

## Verdict and smallest next decision

Keep the earlier conclusions: the opt-in nominal generator accepts uncut
flat/uphill/downhill flights; safe obstacle overflight can remain Direct; a
blocked chosen arc can have another direct arc; finite Unknown is not proof
that a waypoint is necessary. The production V1 chord model remains unchanged.

Close this causal diagnostic here. A cadence-only fix is insufficient for the
high cases, and source fitting is not the next problem: the unchanged source
programs replay faithfully and per-tick terminal tracking is already nearly
exact. There are two distinct follow-on responsibilities, in this order:

1. Design a generic body-aware terminal first-contact contract and a reference
   construction with enough freedom to satisfy it. Audit actual first contact,
   not just the nominal endpoint. Use these exposed cases only as development
   controls; do not weaken core thresholds or treat filtering the 32 bad tails
   as solving the high-case direct route.
2. For admissible references, design a commandable executor at the intended
   global 60 Hz cadence and independently verify the complete launch-to-landing
   witness. A staged capture/uprighting tail or added terminal degrees of
   freedom are design options, not decisions implemented by this pass.

A separately approved policy-change pass should preserve accepted controls
and seal new held-out inputs before implementation. Robust contact margins,
arbitrary incoming waypoint states, waypoint composition and default integration
remain separate open capabilities. More obstacle sweeps, source refits or
waypoint expansion would not resolve the isolated reference geometry problem.

## Evidence and reproduction

Create-only outputs are under
`outputs/research/waypoint_direct_terminal_admissibility_20260927/run_a/` and
`run_b/`. Each retains a compact root summary and eight full case summaries;
selected rows carry full terminal traces and common-prefix signed residuals.
The diagnostic checkpoint source/protocol binding is
`fnv1a64:fb3309e31a1afa7b`, distinct from the historical production-input freeze.
Protocol SHA-256:
`ba129bfc735443d0ebe3e076ff10eddff923bc94248a1ef93d229b1a75e27d58`.

Both root summaries and all eight case summaries are byte-identical across
the independent output paths. Result identity: `fnv1a64:55268bb4947877fc`;
root summary SHA-256:
`891ba3e5645de6c8d3ccfd2f8b7608e40d05180667c772aff9d1f27b18570c78`.
All 92 bound source/manifest files and the protocol match the measurement
binding after evaluation; all eight frozen input and output digests verify.
Validation passed 721 workspace tests in twelve suites, focused module/CLI
tests, strict all-target workspace Clippy, formatting and diff checks.

That all-source runtime binding is historical once a follow-on evaluator is
added. The original output bytes, result identity and validation above remain
preserved; they are not relabeled as a run of the newer source tree.

Run through `rtk cargo run -p pd-eval --`:

```text
waypoint-direct-terminal-admissibility --preflight-only
waypoint-direct-terminal-admissibility --output-dir EXISTING_PARENT/NEW_DIR
```

The default input root is the retained September 25 `fresh_run_a` family.
`--input-root` can locate an exact byte-identical copy, not substitute missions.
Existing output roots, changed raw/semantic input bindings, missing wrappers,
changed selectors and source/protocol drift stop evaluation. The preflight
compares wrapper provenance against its analytical policy identity separately
from the enclosing generation-policy identity; these are different contracts.

The primary owned replay/probes, protocol, integration, physical evaluation,
review and acceptance. Luna owned the bounded runner/preflight/refusal tests
and a separate read-only helper review. That review led to explicit stable
ordinary-zeroing versus neutral-incoming-state regression coverage. No commit,
push, deployment, controller/planner/default promotion or follow-on policy
implementation was performed.
