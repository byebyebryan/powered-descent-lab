# Combined safety preservation and fresh validation

[Documentation home](README.md) · [Independent results](ballistic_acquisition_safety_results.md)

This separately authorized pass composes the existing acquisition gate and
terminal safety fallback. It adds no trajectory algorithm, profile, waypoint
rule, command choice, controller tuning, guard exception or extra landing time.
The maintained policy-3 default, experimental ridge default and accepted reports
remain unchanged. The preceding checkpoint is committed as `dd28de8`.

## Frozen mechanism and evaluation

The explicit `acquisition-terminal-safety` mode inherits acquisition-only and
enables exactly the already-tested upright-coast/upright-support terminal adapter.
Acquisition-only remains independently executable. All physical guards, command
cadences, original budgets, source replay and decision-repeat proofs stay fixed.

Reference: the 863/1000 acquisition-only capture,
`capture-ballistic-acquisition-safety-acquisition-full-20261010-v1`, receipt
`c673075623673007dfd839cccf45a734991a1a5e68fb9fa2e2f838ebce2fd973`, results
`dc75abe4c82975afd1a2ed2c4a7870eddbc51bf4cf6b2facd11d247477babea9`.

Four measured stages use one source/native/reader/renderer/protocol freeze:

1. Exact acquisition-only control: the preceding 101 focused worlds plus
   deadline losses 283/327/861, with eight repeats: 112 attempts.
2. Combined mode on all original 1,000 worlds plus those eight repeats:
   1,008 attempts, compared with acquisition-only's complete saved flights.
3. Acquisition-only on 1,000 fresh worlds plus five repeats: 1,005 attempts.
4. Combined mode on the exact same fresh worlds plus five repeats: 1,005 attempts.

Total: **3,130 attempts**, no retries or outcome-driven tuning. Four workers,
60-second attempt limit and two-hour stage limit. Finite stops or landing
regressions do not cancel the diagnostic sweeps. Source, numerical, collection
or proof failure stops further stages; preserve partial evidence, do not replace
seeds, weaken comparisons or repair and silently restart the measured candidate.

Original repeats are 056/138/538/715/349/283/327/861. Fresh repeats are indices
000/250/500/750/999, outside the 1k denominators. Preserve exact control feedback,
initial arcs, complete command/state replay, deterministic decisions and repeats.
For combination comparisons, bind terminal entry clocks and every preterminal
command; any changes outside terminal ownership are an integrity failure.

## Fresh input contract

Master seed **2026101001**, 1,000 unique seeds, 250 per unchanged recipe:
mountains 4x, mountains 8x, broad massifs 8x and successive ridges 8x. Exclude
the study/sanity/calibration/challenge/original-1k seeds and the earlier fresh
100-world checkpoint. Keep the existing Pylander source hashes, refined recipe,
4 m sampling, local pad shelves/transitions, vehicle/Earth/120-60 Hz setup.
Persist the seed list before generating or viewing terrain. Reproduce complete
profiles in a fresh process with reversed generation order before flying.
No difficulty filtering, corridor clearing, replacement worlds or success quota.

The first fresh acquisition-only run has no earlier same-world baseline. Record
that absence explicitly, not as a manufactured zero-success baseline. Its complete
results become the sealed paired reference for the fresh combined run. Compare
counts across original/fresh populations descriptively, not as paired gains.

## Diagnostics, reporting and closure

Inspect the three saved deadline losses read-only: original budget calculation,
terminal entry/arrival clocks, guidance mode changes, final pose, velocities and
fuel. Do not continue beyond the original budget or claim eventual landing.
The unchanged 69 recovery stops are not part of this implementation.

Retain the common rich batch/detail templates and failure-first tree. Add only
honest optional population/comparison labels for fresh unpaired evidence; do not
regenerate older pages, alter navigation sources or restart the report server.
The already-authorized dynamic library may discover new captures normally.
Protect accepted selectors/pages, preceding captures and existing navigation work.

Run focused Rust/reader/report tests and the maintained developer gate before
measurement. Afterward authenticate source and inventories, all proof tuples,
exact control/repeats, preterminal preservation, gains/losses and recipes. Run
explicit later-source 44-case parity separately; retain the older published
checkpoint's known exact-float discrepancy. Document negative results as well.
No push, default promotion, additional experiments or automatic cleanup follows.
