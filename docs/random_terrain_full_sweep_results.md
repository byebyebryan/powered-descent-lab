# Full random-terrain sweep results

Date: 2026-10-07. Implements the [repair-and-retry continuation](random_terrain_full_sweep_plan.md).

## Verdict

The full unchanged population completed: **100/100 random cases verified landed
on target**, with integrity and final-source replay passing for every case. All
100 initial nominal routes were clear and executed directly, with zero local
corrections. No random case failed, timed out, stopped early or remained unattempted.

Three separate preservation controls also landed: clear/direct (`v2_clear_685`),
one actual handoff (`v2_ridge_early`), and three actual handoffs
(`v2_plateau_reference_900`). All five predeclared same-source repeats match their
original full non-timing flight records exactly. These eight checks are not added
to the 100-case landing denominator.

This closes the survey collection blocker, not general arbitrary-terrain
reliability. The random recipe produced no blocked nominal path, so the survey
does **not** establish random-terrain waypoint success rates. Corrected-flight
evidence here remains the two separate frozen controls; the accepted 44-case
benchmark and its selected report remain untouched.

## What changed and why it finally ran

The prior numerical repair already removed translation-dependent conservative
body extents and bounded original-pad step-zero cancellation using the existing
admission tolerance. The later stops were cross-source evidence comparison, not
physical launch crashes: first a tiny clearance diagnostic, then selected local
trajectory diagnostics whose content hashes also changed.

Comparison V2 handles that complete dependency rather than ignoring hashes:
each selected proposal is authenticated with the existing native typed
serialization and its executed segment binding checked. Only named clearance
diagnostics within 1e-12 m may differ without crossing zero or required reserve.
All states, commands, policies, selected rows, contacts, outcomes and proof flags
remain exact. Runtime collision thresholds and replay checks were not weakened.

The three controls recorded 18 exceptions in total: 14 clearance scalars (maximum
absolute delta 2.842170943040401e-14 m) and four authenticated derived identity
references. None was a maneuver, state or safety-disposition change. V1 and
absent-contract captures retain their original comparison semantics.

The formerly blocked `random-001`, seed `1634476889`, now lands with both proof
flags passing; its predeclared repeat agrees. Its entire ordinary flight and
executed segments match the original physically landed but proof-failed record
exactly. No further repair or retry was
needed after the new comparator and source freeze.

## Coverage and limits

- The same 100 seeds and raw profiles, all 103 input scenarios and pad patches
  are byte-identical to the original capture. No resampling, terrain lifting,
  route cutaway, strategy tuning or outcome-based selection occurred.
- Terrain relief spans 39.79–150.64 m; endpoint rise spans -99.69–84.30 m.
- Nominal peak COM heights span 267.48–370.69 m in world coordinates; all terrain
  audits passed. Those peaks are not an assertion of uniform clearance above the
  terrain, and audit admission—not a peak-only check—establishes route clearance.
- Random target arrivals span 40.35–41.35 s, on the currently tested vehicle,
  Earth gravity, fixed 1200 m pad separation and 120/60 Hz clocks.
- All 108 measured attempts in this capture have complete passing integrity and
  final-source replay evidence. The five repeats provide same-source determinism
  evidence only for their predeclared five cases, not all possible inputs.

The next coverage decision should target **naturally blocked nominal paths**
through a separately frozen harder-scale procedural population, while retaining
this population as the completed baseline. That would measure whether repeated
local clearing generalizes on procedural terrain. Do not extend this completed
survey or silently tune its recipe to obtain corrections.

## Retained attempts and provenance

| Capture | Measured attempts | Interpretation |
| --- | ---: | --- |
| `capture-random-20261006-v1-reviewed` | 7 | Three controls plus four random attempts; initial-pad replay-envelope failure retained |
| `capture-random-20261006-v2-pad-rest` | 1 | Direct control landed; strict diagnostic comparison stopped collection |
| `capture-random-20261006-v3-comparison` | 2 | Direct control accepted; corrected control landed but hash comparison stopped collection |
| `capture-random-20261007-v4-full` | 108 | Three controls, all 100 random cases, five repeats; complete final-source sample |

There are 118 retained measured invocations across these four attempts, not 118
independent random worlds. Previous outcomes, captures and report bodies were
not rewritten. All 385 protected prior files, including accepted selector/report
bodies and previous survey reports, remain byte-identical.

Final capture: `outputs/eval/planner_v2_random_terrain/capture-random-20261007-v4-full`.
It binds 119 source files at Git HEAD `ffc59ec28613b35327c23edaeae0b586d4cdf085`
plus the recorded uncommitted source snapshot; HEAD alone is not its source identity.

- Evaluator SHA-256: `d48f765c3b7c2523d0b7e9b15f923b576c0d0ea47ea61db6a5fd2d2892eb4442`.
- Manifest SHA-256: `8b694c8f459ffa0e0235a9ffd7bc03bc4d6d2371f1eb9e61be047cadb9d0affa`.
- Receipt SHA-256: `a0592fd865ae8cd85f7214c57fde5bae14dadb7d74738219b1cb226b9aca6e99`.

## Validation and report handoff

Focused geometry/guard and comparison tests pass, including forged hashes,
changed maneuver/state/policy, missing segment binding, threshold crossing and
excessive diagnostic changes. All 42 Python study tests pass. The maintained
11-step gate passes: 217 evaluator library tests plus 12 evaluator CLI tests,
workspace/CLI feature boundaries, formatting, strict Clippy, 52 Node tests and
documentation links. Retained 44-case numerical parity was not rerun or relabeled.

The final survey and all three historical captures pass saved inventory and
projection verification. The native report adapter also validates the complete
final capture before publication, with no new simulation.

Common rich batch/detail report:
`/reports/eval/planner_v2_random_terrain/recheck-20261007-v4-full/`.
From either report home: **Waypoint planning → Full random-terrain sweep**.
The accepted benchmark is still listed separately first; all stopped survey
reports remain reachable as earlier attempts. All 108 details retain the common
trajectory/charts/sample views, with actual one- and three-handoff annotations
on the two corrected controls, not invented waypoints on direct random flights.
Static QA checked 112 pages, all 108 rich detail payloads, four exact handoff
snapshots and 1,012 resolving local links. Existing LAN serving returned HTTP 200
for the root, waypoint topic, full-sweep batch and three-handoff detail. This is
data/navigation validation, not browser visual or human acceptance.

No commit, push, server start/stop or accepted-benchmark promotion was performed.
