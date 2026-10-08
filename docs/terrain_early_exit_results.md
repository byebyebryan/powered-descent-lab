# Bounded early-nominal exit results

Status: **completed bounded implementation and validation, 2026-10-07**.
The [approved plan](terrain_early_exit_plan.md) closes nine diagnostic missions
and one separate normal 44-case pack. No wider sweep or held-out sample ran.

## Verdict

The delayed-handoff mechanism is now an executed fix in the three selected
clear-audit subjects, not just a counterfactual. `967`, `955` and `024` all
land on the target after the earlier actual exit, with mission success,
integrity and complete original-source replay. `983` and `516` decline their
terrain-blocked early queries and retain the original complete ordinary flights
and finite `NoNominal` stops. Neither stop is a crash.

This establishes a useful bounded capability, **not a new random-terrain success
rate**. The five subjects were selected for diagnosis. Other nominal failures,
154 clearing exhaustions in the old cap-24 population, and cap promotion remain
separate questions. The candidate is in the working source; the ordinary release
binary, accepted report selector, served pages and server were not changed.

## Implementation and evidence boundaries

- The existing local search/ranking chooses the same complete clearing proposal.
  Its original H and continuation remain untouched planning witnesses.
- One optional query uses the first earlier settled, upright, command-aligned
  coast boundary. Forward clones start at actual E; no saved-state restoration
  occurs. The same five-metre body reserve and supported family apply throughout
  the two-second selected-coast continuation.
- The unchanged nominal constructor runs once at that state. A passed actual
  terrain audit permits the earlier exit; a finite rejection keeps the old H
  fallback. Ordinary waypoint clearing still does not require a landing suffix.
- Actual clearing-prefix and private continuation source proofs precede commit.
  The exact checked nominal is queued for the next piece, bound to the complete
  actual state and original deadline. The early exit counts as one correction.
- Optional saved metadata distinguishes actual piece end from original witness
  H. Bundle, replay, batch and rich-report validators bind it to the consumed
  prefix and the next exact nominal. Legacy records decode without that field.
  Common report plots/navigation remain intact; unused queries are not markers.

No controller/nominal tuning, per-seed conditions, global row reranking, reserve
relaxation, clock/fuel reset or default cap change was made.

## Frozen nine-mission matrix

Capture: `outputs/eval/planner_v2_random_terrain/capture-early-exit-20261007-v1`.
The [frozen contract](../studies/terrain_profiles/early_exit_plan.json) reuses the
same seven scenario byte identities and two declared repeats. This capture's
isolated binary explicitly identifies experimental cap 24; it is not the
ordinary cap-6 release. All nine missions passed integrity and source replay.

| Case | Early query step | Original witness H | Executed result |
| --- | ---: | ---: | --- |
| 000 | none | none | Unchanged direct landing; zero corrections |
| 271 | 4152 on final correction | 4226 | Verified landing; four corrections |
| 983 | 2428 | 3112 | Terrain-blocked query; exact old-flight fallback |
| 516 | 2274 | 2756 | Terrain-blocked query; exact old-flight fallback |
| 967 | 2882 | 3218 | Earlier actual exit and verified landing |
| 955 | 2964 | 3150 | Earlier actual exit and verified landing |
| 024 | 2828 | 3062 | Earlier actual exit and verified landing |

The 271 control also retained an earlier blocked query and two maneuvers with
no earlier eligible boundary. Each selected maneuver has at most one optional
query. Its final exit advances replanning by 0.617 seconds; the three subjects
advance it by 2.8, 1.55 and 1.95 seconds respectively.

Full selected proposals, early states, nominal searches and audits match the
previous counterfactual evidence for all five subjects. Original ordinary
actions/events/samples match through each early boundary. The declined subjects
match complete prior flights except declared root policy/input identities,
wall timings and independently checked optional query metadata. Repeats of
967 and 983 match every non-timing flight field. No additional attempts ran.

The native missions all completed successfully, but the collector's final
saved verifier initially failed with a Python `NameError`: a live-state check
had been placed in the read-only verifier. Only the collector and its tests
were corrected. The original closed capture, source snapshot and receipt were
not rewritten, and no flights were retried. The corrected read-only verifier
passes on those original bytes. Separate review evidence binds both tool versions
and confirms identical Rust source; collection is not relabeled as having used
the corrected collector.

## Normal cap-6 acceptance pack

Capture: `outputs/eval/planner_v2_lab_suite/capture-early-exit-20261007-native`.
The separately retained normal-policy binary ran exactly 44 cases with
`--enforce-regression-policy --no-publish`. Native saved acceptance also passes.

- **36/36 core target landings:** 11 clear/direct and 25 blocked/corrected.
- **44/44 integrity passed; 42/42 supported source replays passed.**
- Diagnostics: three verified landings, three honest finite `NoClearing` stops,
  two unsupported inputs. Zero crashes and no lost prior landings.
- `v2_diag_near_target` now lands; it remains outside the core denominator.

The frozen October-5 acceptance baseline and all input bytes remain unchanged.
It predates intervening fallback/logging and derived-clearance work, so it is
not a complete current-source numerical-parity baseline. Supplemental read-only
motion comparison uses the existing October-7 handoff-room benchmark, explicitly
identified separately: 22 cases preserve complete ordinary evidence and consumed
piece commands/states/clocks; the other 22 preserve original selected schedules,
states and ordinary prefixes up to the early exit. All 11 direct core controls
are unchanged. This is not a claim that all historical unexecuted query metadata
or derived proposal hashes are identical.

## Review and retained identities

`outputs/eval/planner_v2_random_terrain/early-exit-review-20261007-v1` retains
matrix admission, collector-only source differences, normal binary, benchmark
process logs and exact motion comparison under a separate closed receipt.

- Matrix manifest SHA-256:
  `54d2e2cf6d818764347089ee8175f6504c35ec5eee1493ed054fbf94ec137b05`.
- Matrix receipt SHA-256:
  `f71f22e19f8a13e248771d2019df98077500451dee12c2d7d421f1810c9a213a`.
- Experimental executable SHA-256:
  `54ffd58ac4b4e4a4e9a3d557725cfd03238f40a2bb226b8edbbdb85141271509`.
- Normal benchmark summary SHA-256:
  `db4057e0f098fb7a6a5167b63b41b33fd41ec401a1188fcccff17a19a101b177`.

The maintained eleven-step developer gate passes, including workspace/CLI tests,
formatting, strict Clippy, 52 Node tests and documentation links. All 99 terrain
study tests pass, including five collector contract/saved-verification tests.
Tracked native coverage tests actual-state queue binding, exclusive piece clocks,
unsupported/missed-reserve/no-earlier fallback, rejected-query classification,
integrity-error paths, historical decoding and actual-versus-witness annotations.
The 1,064 protected existing files retain their pre-pass aggregate identity.

## Next decision

The bounded candidate is ready for review, then a separately authorized paired
rerun of the same cap-24 1,000-world population. Require all 748 previous landings
preserved, explicit early-exit/fallback counts, honest stops and unchanged clear
routes. Only then consider a separately frozen fresh 100-world test sample.
Do not tune on the held-out sample or extrapolate the three diagnostic successes.
Default-cap promotion and report publication remain separate decisions. No
commit, push, publication or broader campaign was authorized in this pass.
