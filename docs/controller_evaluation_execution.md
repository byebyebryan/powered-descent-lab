# Controller characterization execution

[Implementation plan](controller_evaluation_implementation_plan.md) · [Evaluation design](controller_evaluation_design.md)

## Launch and limits

The owner launched this loop on October 21, 2026 with instructions to commit
reviewed checkpoints as work progresses. Start time is October 22 at 04:36:55 UTC.
The implementation and measurement scope is the linked plan, including internal
review/repair and cost-based prospective allocation. No push is included.

The global wall deadline is 16:36:55 UTC, with no new measurements after
15:36:55 UTC and the last hour reserved for closeout. Use one native worker,
40 GiB of new artifacts, 20 GiB of new build/source material and a 50 GiB free
space floor. All proof/reconstruction, retries and local report work count.
The frozen ordinary terminal backend and native plant remain unchanged.

The recovery inventory and complete Git-visible workspace archive are at
`/home/bryan/code/.pd-controller-loop-20261022T043655Z/`. Its initial archive
SHA256 is `e78b128d16bff73a907d23b8656c76774182df1c89e9c14d67149fdc4ce98d91`.
It records 817 files at source checkpoint `8adf2db`; ignored evidence is untouched.
Unrelated research/report files and mixed documentation edits remain preserved.

## Stage ledger

| Stage | Status and acceptance |
| --- | --- |
| A: software | Complete: exact input export/interchange, recipes/audit/sampler, supervised execution and recovery; all 19 maintained checks pass |
| B: input freeze | Complete: prospective declaration, exact candidate plans and native audits sealed before flights |
| C: pilot | Complete: twelve original executions including references, all nine facade results verified; inventory checks pass |
| D: retained parity | Complete: 945/945 exact direct/facade comparisons and proofs; 865 target landings, 80 native crashes retained |
| E: characterization | V1 quarantined; corrected V2 released with all 320 slots |
| F: activation | V1 quarantined; corrected V2 released with all 64 originals / 448 attempts |
| G: confirmation | V1 interrupted and quarantined; sole technical restart released as fresh V2 cohort, fixed N=600 |
| H: closeout | Pending: code review, validation, complete inventory, failure/coverage/cost analysis and bounded new local rich reports |

At the pilot boundary, record immutable recipes and allocations under a new
generated root and update this ledger. Preserve original outcomes, all aliases,
unallocated slots and incomplete attempts. Stop an affected measurement stage
on a proof/parity/tooling fault, then use the plan's bounded repair rules.
Scientific misses do not stop the schedule or authorize backend tuning.

## Acceptance boundaries

Software, local recovery, confirmation, activation value and whole-mission
readiness receive separate verdicts. A finite failed landing with valid proof
is a scientific result. A missing proof or an interrupted schedule is incomplete.
Fresh confirmation uses one prospectively declared local population; if intended
mission handoff requirements are unavailable, its claim remains a synthetic
local reference result. It cannot qualify launch/transfer integration.

No RL training, controller tuning, default promotion, report publication,
selector updates, report refreshes or server lifecycle changes are included.

## Checkpoint one: native input access

The neutral [native input set](native_scenario_inputs.md) and explicit evaluator
exporter are implemented. Four pre-extraction ordered scenario/descriptor
fingerprints remain exact across 4,725 smoke/full source occurrences. The
maintained authored-waypoint input fingerprints also remain exact. The new
controller-free path matches every original terminal scenario and resolved
parameter, including both horizons; it works when the lane config is unavailable
while legacy resolution retains that error.

Four neutral interchange tests cover exact round-trip/aliases, create-only
destinations, incomplete/corrupt sets, invalid inputs, bounded files and symlink
ancestors. Focused resolver tests and strict affected-package Clippy pass.
These are input/software checks; no measured controller flights have started.

## Checkpoint two: recipes and audit

The maneuver application now prepares and checks immutable evaluations without
flights. The recipe fixes source selection or typed coupled local-state sampling,
role/cohort/transform rules, programs, native budgets, resource caps and coverage
bins. Plans retain every proposed case, including native-invalid or unsupported
diagnostics, with exact byte/physical identities and complete attempt schedules.
The sampler uses counter-derived, cohort-separated streams; independent cohorts
forbid transformed copies and diagnostics. The audit records physical and parent
aliases, source roles, observed ranges, joint-cell coverage and supplied exposure
overlap. Unknown history remains explicit.

Four new focused tests cover deterministic preparation, coupled coverage,
transforms/aliases, partial native deadlines, invalid/unsupported dispositions,
exposure rejection and tampered scenarios. No measured allocation is yet released.

## Checkpoint three: supervised execution

The runner now executes one bounded child at a time, binds the outer schedule,
preserves ordinary create-only bundles and records every scheduled slot. Native
misses continue; proof, parity, resource and infrastructure faults retain evidence
and mark the remaining schedule unstarted. A child guard reaps workers on error.
Metadata is sealed for incomplete phases as well as completed ones. Output-byte
reservations precede writes, and all original/reference/proof work is accounted
for with unavailable counters explicit.

Continuation requires exactly matching inputs, source paths and executable.
Completed native failures can be reused after supervised fresh proof checks,
without new original flights. Incomplete attempts are not replaced. Saved-result
inventory checks verify payload byte bindings and aggregate accounting without
extra native replay. Confirmation criteria are frozen recipe data and incomplete
samples cannot receive a passing confidence result.

The software panels exercise real subprocesses, exact direct/facade parity,
failed landings, unsupported diagnostics, interruption/reaping, storage caps,
input drift, payload tampering, continuation, a fixed negative confirmation and
infrastructure failure before a flight. The maintained gate also checks the new
explicit exporter frontdoor and rejects overlarge expansion before source I/O.

Final software acceptance passes all 19 maintained developer checks, including
17 maneuver library tests, two ordinary CLI tests, four batch CLI tests, all
workspace tests, strict Clippy, Node/docs checks and dependency boundaries.
The final log is
`/home/bryan/code/.pd-controller-loop-20261022T043655Z/software-gate-r5.log`.
The 174 inventoried plant/controller/planning/fixture files remain byte-identical
to the starting inventory. No campaign landing result is implied by these checks.

Before freezing measurement inputs, review found that local generation inherited
the anchor's descriptive tags and resolved/expectation metadata. Generated cases
now receive their own sampler/task/cohort/index labels. A regression check keeps
the anchor unchanged and rejects those stale labels; native physical identities
are unchanged. The earlier detached software candidate is retained unused, and
the first measured source epoch will include this correction. No measurement
repair allowance has been consumed.

## Input freeze and pilot release

The first measured epoch uses source `0378812`, retained at
`epochs/epoch-2-source`, and its release executable at
`epochs/epoch-2-bin/pd-maneuver-example` under the loop root. Executable SHA256 is
`996fb0da0fb9eca5c7d73d8a09744332cf14e59da5f293f066eb23e0af17c5be`.
All 19 maintained checks passed again after the provenance correction.

`declarations/local-reference-declaration-v1.json` supersedes only the ordering
of the earlier proposal. Its SHA256 is
`199d0a702621433eef78411d3e595206df303670c5665a050ba8ad212c2fa9fd`.
Development grid order is `j = (21*i) mod 64`, with four development parents and
one diagnostic parent per block, each retaining all four transforms. Prefixes
80/160/320 therefore preserve the 4:1 role ratio and primary factor levels.
The data-only retained adapter copies exact native payload bytes and round-robins
15 equal source groups; no physical generator or controller is reimplemented.
Its manifest SHA256 is
`0679f7ed3b517034bb8b0e91ace275a71ad0087fdde50afb2c804e8d372cbbb7`.

The frozen Rust loader and audit accept these complete candidate schedules:

| Candidate | Cases / attempts | Input dispositions and identity |
| --- | --- | --- |
| Retained parity | 3,780 / 3,780 | All supported; 3,672 distinct physical states, source aliases retained |
| Structured local | 320 / 320 | 256 development, 64 diagnostics; 312 supported and 8 explicitly unsupported gravity diagnostics; 240 distinct physical states |
| Paired activation | 64 / 448 | All supported identity originals; seven fixed programs per original |
| Independent confirmation | 600 / 600 | All supported; 600 distinct physical states and parents; zero overlap with declared development and retained identities |

The exact recipes, scenarios, programs, coverage cells, limits and hashes are in
`plans/candidates-v1/` and `declarations/input-freeze-receipt-v1.json`. Input
inspection exposure is recorded separately. Historical exposure is incomplete;
the independent cohort is fresh relative to the declared audited inventory.

`declarations/pilot-declaration-v1.json` releases only three identity cases,
indices 0/104/312, at low/middle/high primary grid coordinates. Each receives
immediate, never and tick-120 activation; immediate also has one direct reference
execution per case. This totals twelve original executions, with all additional
proof work counted. Each of the three phase caps is 600 seconds and 512 MiB;
the attempt cap is 180 seconds, bundle cap 96 MiB and free-space floor 50 GiB.
Only pilot wall/byte measurements determine later allocations. All later sample
sizes remain unallocated until that cost boundary.

## Cost boundary and measurement allocation

The pilot completed all nine scheduled facade attempts plus three direct
references. Its inventories verify, and all three direct comparisons are exact.
The largest supervised child took 0.281197 seconds; the largest complete attempt
directory used 5,237,210 bytes. Every original, reference and proof pass remains
in the separate resource counters. CPU time and internal backend forecast work
remain explicitly unavailable.

At 05:59:49 UTC, `declarations/allocation-declaration-v1.json` froze every later
count using resource fields only. The conservative per-attempt estimates are
0.562394 seconds and 10,539,956 bytes, including a doubled metadata allowance.
After input/pilot bytes, 512 MiB is reserved for further preparation, analysis
and bounded rich reports; 900 seconds is reserved at the measurement boundary.
The remaining wall/artifact allowances are split 30/20/20/30 percent.

| Stage | Fixed allocation | Wall cap | Output cap, bytes |
| --- | --- | --- | --- |
| D | 945 direct/facade pairs, seeds 0/1/6 | 10,117 s | 12,695,507,726 |
| E | 256 development + 64 diagnostic occurrences | 6,745 s | 8,463,671,817 |
| F | 64 originals times seven activation programs | 6,745 s | 8,463,671,817 |
| G | 600 independent originals, immediate policy | 10,117 s | 12,695,507,726 |

The full 3,780-pair option exceeds D's prospective storage share under the frozen
two-times cost estimate; its other 2,835 source slots remain unallocated. The
selected 945-case panel retains every arc, velocity band and source group. E/F/G
receive their largest declared panels. Their exact plans and passing audits are
in `plans/allocated-v1/`. G additionally binds all inspected development physical
and parent identities and has zero overlap. No count or policy can expand after
observing performance; interruptions remain incomplete panels.

## Retained parity checkpoint

D completed all 945 selected, physically distinct source cases in 286.872402
seconds and 5,137,844,183 bytes. Every direct/facade comparison, neutral replay,
controller reconstruction, planner reconstruction and outer integrity check
passes. Its sealed inventory check also passes; no repair or retry occurred.

The immediate controller lands 865/945 (91.53%). All 80 crashes occur in the
full-mass high-speed stratum. Empty/reference and half/core each land 315/315;
full/stress lands 235/315. These are retained source labels and a designed panel,
not a new operational-core declaration or an independent reliability estimate.
The wrapper preserves the original backend's successful and failed behavior.

The median supervised child cost is 0.301218 seconds, p95 0.441729 seconds and
maximum 0.742951 seconds. Maximum bundle size is 15,740,160 bytes. Those maxima
exceed the pilot's doubled point estimates, which were allocation estimates;
the hard caps remain authoritative and the complete phase fits its allocation.
`declarations/analysis-v1/D-retained-analysis.json` and its rows retain the
stratified outcomes, incoming contact states and timing/storage distributions.
No later allocation changes in response to these outcomes or unused resources.

## Technical invalidation and repair one

Inspection of the below-target diagnostic exposed a real generator defect:
sampler v1 raised `pad.surface_y_m` independently but left the underlying terrain
at zero. The unchanged native contact classifier uses the terrain for collision
and the pad's horizontal footprint for on-target classification
([source](../pd-core/src/sim.rs)). It can therefore report a native target landing
at ground level despite a declared elevated pad. Native proof and reconstruction
correctly reproduce that model; they cannot repair an inconsistent scene.
This is a geometric input-validity fault, not an outcome-based envelope change.

Every v1 local primary case has the mismatch. Its pilot capability outcomes,
structured results, activation results and partial confirmation are quarantined;
none will enter corrected local rates or confirmation. E completed 320 slots
(312 flights, eight unsupported rejections), F completed 448 flights and G
retained 226 completed results plus one interrupted worker and 373 unstarted
slots. G has no completion seal or confidence verdict. Its interrupted work
counters are unavailable, rather than counted as zero. Old inputs, captures,
source/executable epoch and analyses remain at their original paths.
`declarations/technical-invalidation-v1.json` and its accounting-correction
receipt preserve the inventory. D remains valid because its original pad and
terrain are consistent; its exact parity and measured failures are unchanged.

Repair one changes only local input construction and its preflight audit:
sampler v2 places the terrain baseline at the declared pad surface, with obstacle
heights relative to that baseline. The audit measures the maximum surface error
over pad endpoints and every interior heightfield vertex, and refuses a
non-diagnostic local panel with inconsistent surfaces. Diagnostics retain the
deliberately inconsistent below-target scene as a separate model-integrity probe.
Native validation, contact semantics, controller configuration and control logic
remain frozen. Regression coverage checks geometry and transformed copies,
including an interior vertex that endpoint-only checks would miss.

The corrected task will receive a new source epoch and local task/cohort/seed
identities. All inspected v1 candidates, including the full 600 confirmation
inputs, enter its exposure inventory. E/F keep counts 320/64 and their original
programs; fresh G keeps N=600 and the same 99%/95% criterion and immediate policy.
This consumes one of two measurement repair epochs and the sole confirmation
technical-invalidation restart. No extra repair is available for another fresh
confirmation fault, and no failed state is being removed from its task version.

## Corrected local input freeze

Repair source `e2d2482` passes all 19 maintained checks, including 18 maneuver
library tests, two ordinary CLI tests and four batch CLI tests. The complete log
is `/home/bryan/code/.pd-controller-loop-20261022T043655Z/software-gate-repair1.log`.
Source is retained at `epochs/epoch-3-source`, with release executable at
`epochs/epoch-3-bin/pd-maneuver-example`, SHA256
`8e1499abfac8e1424ff34dcf7daf15565e92ad6743eb2c3032abeb24777f4a6c`.

Before replacement flights, `declarations/allocation-declaration-v2.json` binds
the new task `local-reference-v2`: development cohort `development-v2`, seed
2026102203; confirmation cohort `confirmation-v2`, seed 2026102204. The original
numeric bounds, primary factor ordering, diagnostic roles, programs, counts,
caps and confirmation criterion are unchanged. Fresh G has 600 unique physical
states/parents and zero overlap with corrected development, every inspected v1
candidate and the retained inventory. Earlier global exposure remains unknown.

All three corrected plans pass native input/support audits. Maximum pad/terrain
surface error is exactly zero across every non-diagnostic input. The complete
controller descriptors equal their pre-repair descriptors. Deliberate mismatched
diagnostics remain visibly separate. The plans are in `plans/allocated-v2/`;
their fixed stage names are E2/F2/G2. Including all old artifacts, every new phase
cap and a 512 MiB overhead reserve gives 39,429,015,893 bytes, below the global
40 GiB ceiling. Old cost evidence remains available without making a capability
claim for its inconsistent scenes. No extra pilot or enlarged sample is released.

## Prospective local reference population

Before any campaign flights, `declarations/local-reference-proposal.json` under
`outputs/controller-characterization/loop-20261022T043655Z/` records this proposed
task. It is a synthetic airborne local reference, not the distribution of actual
launch/transfer handoffs. Its source-independent generator and all physical
case/program schedules will be frozen before the cost pilot.

| Factor | Prospective reference bounds |
| --- | --- |
| Initial clearance | 5 to 800 m relative to the target, with attitude-aware foot placement |
| Lateral entry | Signed offset up to 0.75 times clearance; velocity toward target over `sqrt(2h/g)+2` s plus -6 to +6 m/s residual |
| Descent | 0.1 to 0.85 times `sqrt(2 * net upright acceleration * clearance)` at original mass/fuel |
| Attitude / rate | -0.25 to +0.25 rad; -0.15 to +0.15 rad/s |
| Dry mass / fuel | 7,200 / 9,450 / 11,700 kg; independently varied positive fuel 1,000 to 6,300 kg |
| Pad / original horizon | Width 24 to 60 m, surface 0 to 20 m; 60 to 90 s original horizon |
| Terrain | Flat; approach ridge 12 m high; overshoot backstop 18 m high, with explicit placement/width rules |

Generated terrain endpoints contain the entry, pad and declared obstacle with
80 m padding and at least 120 m on each side. Native flat extrapolation is
unchanged; the local extent keeps maintained rich report views readable.

This couples braking/offset to clearance through native algebra without using
the controller's predictions to admit winners. It extends the retained packs'
fixed fuel/pad/deadline and zero-attitude coverage. The unchanged native validator
and facade own valid/support dispositions. The native vehicle family is fixed.

The maximum development panel has 256 development occurrences and 64 separately
scored diagnostic occurrences. Four explicit transforms share each parent:
identity, reflection, physical translation and label-only control. They are
designed comparisons, not independent trials. Diagnostics cover near-empty fuel,
partial deadlines, ascending entry, a narrow pad, unsupported gravity, a target
above the entry, near-contact settling and attitude/rate stress. The 64 activation
originals select identity cases from different development parents before flights.

The prospective independent cohort instead uses separate counter-derived
cohort/seed streams, continuous scalar draws, three mass levels and three terrain
profiles, with identity transform only. Its policy is immediate activation.
The working criterion is a one-sided 95% exact-binomial lower bound of at least
99% for this one mixture; no per-stratum or whole-mission guarantee follows.
The exact 600/300 allocation will be selected from pilot cost alone. All original
retained inputs, development/pilot exposure and unknown historical coverage stay
visible; no failed state will be silently removed or relabeled as a diagnostic.
