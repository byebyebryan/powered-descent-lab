# Planner retirement and consolidation results

The approved [eight-phase batch](planner_retirement_cleanup_plan.md) is complete.
Policy 3 is the sole maintained planner execution path and the Rust policy
default. The current runtime no longer depends on the historical research tree.
Complete flight numerics, current input identities and saved evidence are preserved.
The [follow-up housekeeping round](#follow-up-housekeeping-round-october-5-2026)
below closes the stranded dependencies found after that first batch.

This is retirement and housekeeping, not a solver change or a new terrain
acceptance campaign. The starting checkpoint was `e1d9363`; the integrated
runtime/retirement checkpoint is `93aa7c7`. Documentation reconciliation follows
that code checkpoint. The accepted selected capture still names its measured
source `561b6e5`, not the cleanup source.

## Current ownership

| Responsibility | Maintained owner |
| --- | --- |
| Pure discrete bridge/coast and canonical-initial math | `pd-plan/src/ballistic.rs` and its child |
| Sealed current policy, correction bounds and clocks | `pd-plan/src/waypoint_v2.rs`, `local_clearing.rs` |
| Request/preflight, initial source fitting, state-derived airborne acquisition, terminal realization and contact geometry | `pd-eval/src/planner_flight/` |
| Owned piecewise session, live execution, records and source/prefix/certificate proofs | `pd-eval/src/waypoint_v2/` |
| Current local-clearing proposal, admission, execution and evidence helpers | `pd-eval/src/local_clearing.rs` |
| Exact-byte hashing and create-only output reservation/writing | `pd-eval/src/evidence_io.rs` |
| Native pack models, expansion, aggregation, capture validation, provenance and presentation | `pd-eval/src/waypoint_v2_pack/`, with its existing facade |
| Common batch/tree and rich-detail presentation | `pd-report`, current evaluator report adapters |
| Current report/browser checks shared by maintained workflows | `scripts/planner-v2-report-checks.mjs`, `planner-v2-browser-helpers.mjs` |
| Ordinary and explicit retained-baseline developer gates | `scripts/check-planner-development.mjs` |

Pure planning math is always available; the old research Cargo feature is gone.
`pd-report` no longer depends on `pd-plan`. No new crate or generic framework was
introduced. Caller-specific trust, path and provenance checks remain with their
callers rather than moving into generic I/O helpers.

The flight contract remains terrain-blind nominal construction, fixed-command
terrain audit, bounded local clearing and replanning from actual H. Candidate
ordering/ranking, all policy values, commands, full states/contact, fuel,
attitude/rate, original deadline and command cadence are unchanged. A private
continuation certificate is not executed live state or a mandatory landing suffix.

## Keep, extract and retire inventory

### Kept

- Current 44-case pack and all its frozen research inputs, expanded scenario
  identities and source ordering.
- Normal terminal, direct-transfer and authored-waypoint guidance; ordinary
  `pd-cli run` remains unchanged.
- The owned synchronous `WaypointV2Session` and optional `planner-v2` CLI
  flight/replay adapter. The feature remains default-off, with no normal
  `pd-eval` dependency when disabled.
- Current V2 flight, bundle, progress, batch and report schemas/IDs, including
  serialized legacy-looking V1/V2 DTO names and fields used by current records.
- Known saved policy-1/2 identity recognition, historical controller fields and
  descriptor IDs, saved route plans, common historical report readers and
  ordinary saved-action replay. Reading metadata does not admit retired execution.
- Common rich plots, statistics, sample inspection, actual-H annotations,
  batch review trees and home/topic navigation.
- Every file under `fixtures/research`, saved captures, original and published
  pages, current selector and historical receipts. Removed source is recoverable
  through Git; captured evidence was not deleted.
- Six retired pack JSON files as archival/navigation metadata: the two
  `planner_generated_route_*_smoke` packs, the two
  `transfer_route_angle_{pathwise,recoverability}_compare` packs and the two
  `transfer_bot_lab_{pathwise,recoverability}_compare` packs.

### Extracted before retirement

- Exact bridge/kinematics and canonical-initial basis math from the analytical
  research module into `pd-plan::ballistic`.
- Current nominal request/preflight and identity hashing, initial generation and
  source fitting, airborne acquisition, terminal realization, phase-aware contact
  and terrain helpers into `pd-eval::planner_flight`.
- Current local-clearing execution and proof helpers from canary orchestration.
- SHA-256 subprocess behavior, pretty JSON plus one newline, raw-byte writing and
  create-only reservation into neutral evidence I/O. Error contexts and caller
  validation order are preserved.
- Current JS comparison, projection, annotation, common-template and browser/CDP
  helper bodies from pinned historical command scripts into maintained helpers.
- Current tests from research corpus runners to tracked input adapters. The CLI
  clear-flight test retains its 685 m physical base through `v2_clear_685`.

### Retired executable code and frontdoors

- V1 chord/visibility search, `pd_plan::plan`, candidate exposure and V1-only
  generated-route scenario resolution. The serialized `PlannerMatrix` variant
  remains readable but is rejected before resolution/execution, including when
  a caller renames the pack.
- Policy-1/2 execution and the old 56-combination airborne selector. Exact named
  saved identities remain recognizable. Two old family constants remain only
  because current serialized candidate identity includes them; they no longer
  drive a selector.
- Frozen pathwise/recoverability boost scorers, experimental weights/mode code
  and built-in execution aliases. Historical flags still decode and default
  false; enabling either fails explicitly rather than silently using normal scoring.
- Test-only fixed-gate ballistic spike and its dedicated tests.
- Historical source-transition, route-capability and route-execution research;
  bounded-witness/proposal, progress-envelope, physical/executor comparison,
  terrain-equivalence, candidate-replay and final-landing audit orchestration.
- Conservative-ballistic setup, integration, held-out, handoff, F5/F6 and
  controller-shadow execution/report families after extracting their live math.
- Direct-route characterization/primitive/topology/controller-comparison,
  generation, obstacle-discrimination, terminal-admissibility and body-aware
  research orchestration; the old nominal-plant research tree, operational/canary
  wrappers and obsolete canary-comparison example.
- Standalone 32-case compact/rich/navigation-preview generators, dedicated
  display-only suite-card DTOs and the old `waypoint-v2-report` command. The
  current 44-case common batch/detail generator and site navigation readers remain.
- Superseded JS practical/fresh-terrain/corpus runners and tests, pinned
  batch/common/tree command entrypoints, compact/preview report checkers and their
  historical browser gate. Maintained workflow, integration and browser scripts
  import the extracted helpers, not those executables.

The evaluator command registry is now exactly these eight commands:
`run-pack`, `check-planner-v2`, `publish-planner-v2`, `report`, `refresh-reports`,
`refresh-navigation`, `promote-cache`, and `waypoint-v2-flight`. The regression
checks registry membership directly, not an incomplete invocation that could
fail merely because required arguments were absent.

This reduces the tracked tree by roughly 128,000 net lines after adding the
maintained replacement modules/tests. No blanket fixture or output cleanup was
performed. The exact source inventory is recoverable with
`git diff --name-status e1d9363..93aa7c7`.

### Intentional test removals and retained coverage

Tests dedicated to retired V1 search/exposure, experimental transfer scorers,
fixed-gate/analytical research and historical canary/report generators were
removed with those implementations. Their old count is not a regression target.
The ignored 39-row airborne acquisition research-adapter gate is replaced by
the complete 44-case current-flight parity gate; its nine useful acquisition
unit tests moved with the current runtime.

The migrated runtime has 25 passing focused tests covering terrain-blind
selection, canonical source fitting, current acquisition, terminal-time bounds,
deadline and clock admission, phase ownership, invalid/nonfinite proof behavior
and baseline-retaining duration selection. Three pure ballistic/canonical tests
remain in `pd-plan`. A normal tracked multi-handoff regression verifies actual-H
state/fuel/clock continuity, exclusive segment ownership, correction count,
original deadline, integrity and source replay. It does not require an archive.
Retired policy/pack/controller rejection is tested separately from historical
identity/descriptor decoding. Current common rich-report tamper/annotation and
create-only evidence tests remain.

## Validation results

| Check | Result |
| --- | --- |
| Maintained developer runner, including explicit retained parity | All 11 steps passed |
| Workspace all-features tests | 469 passed, 4 intentionally ignored |
| Feature-enabled and default-off CLI tests/help/dependency checks | Passed |
| Formatting and diff whitespace | Passed |
| Strict workspace/all-target/all-feature Clippy | Passed with `-D warnings`; old `single_element_loop` allowance removed |
| All maintained JavaScript tests | 49 passed |
| Complete-flight numerical comparison | All 44 exact, excluding only `planning_s`, `execution_s`, `replay_s` |
| Explicit retained input-expansion and report-projection gates | Both passed |
| Rebuilt evaluator's selected-capture acceptance and saved-site checker | 44 cases, 42 rich payloads, 36 actual H, 46 receipt-bound pages passed |
| Historical native/CLI/repeat and saved-replay receipt checks | Passed; 42 supported receipts and 252 component digests bound |
| Fresh read-only replay using rebuilt current CLI on saved bundles | All 42 passed; 252 actual component digests matched, no artifact writes or new planning |
| Protected path/content inventory | All 28 scopes / 3,280 entries unchanged |

The numerical preservation gate passed before migration, at extraction
checkpoints and after integrated retirement. It compares full flight records,
not just outcomes: commands, proposals, states/contact, cycles, segments,
manifests and proof flags remain equal. Only the three existing flight timing
fields are excluded. Existing JS cross-root comparison exclusions and derived
timing-dependent receipt checks were preserved, not broadened.

The verdict is unchanged: 36 mandatory landings (11 direct, 25 corrected), two
diagnostic landings, four zero-step diagnostic stops and two unsupported inputs.
All 44 retain integrity and all 42 supported flights retain source replay. The
diagnostics are not added to the ordinary landing denominator.

Final integration caught two stale CLI imports and an obsolete research example;
all were fixed before the successful workspace gate. Independent bounded Luna
reviews checked JS/trust helper extraction, Rust entrypoint/feature boundaries,
and exact preflight/identity/terrain helper preservation. The primary integrated
the coupled runtime extraction, reviewed retirement boundaries, strengthened the
command-registry test and owned final acceptance/commits.

The protected baseline is
`outputs/validation/planner_retirement_cleanup_20261005/baseline.json` (local,
ignored validation evidence). Its sorted per-scope file/link inventory hashes
cover frozen inputs, accepted and historical captures, selectors, published
planner/terminal/transfer reports and session receipts. No captured source was
relabelled. Current guide docs are reconciled; dated historical result documents
retain their original measured scope, even where they name now-retired commands.

## Maintained checks and next boundary

Ordinary tracked development checks:

```sh
rtk proxy node scripts/check-planner-development.mjs
```

Explicit complete-flight regression against the local accepted capture:

```sh
rtk proxy node scripts/check-planner-development.mjs \
  --parity-capture outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native
```

Read-only saved-site acceptance/report check after building the evaluator:

```sh
rtk proxy node scripts/check-planner-v2-workflow.mjs
```

There is no required remaining migration item in this batch. The useful next
product decision is whether to validate seeded procedural terrain on the same
vehicle/setup or integrate this owned synchronous session into a concrete game
host. Neither is authorized or established by housekeeping. Arbitrary external
airborne starts, other vehicles/gravities, asynchronous/per-tick planning,
disturbance recovery and random-terrain reliability remain outside the accepted
envelope. No push, publication, server restart or new acceptance campaign occurred.

## Follow-up housekeeping round: October 5, 2026

The bounded follow-up from clean `90a2228` is complete. Its final Rust checkpoint
is `f9e85b0`; subsequent result documentation does not change that source. The
selected accepted capture still names `561b6e5`, its original measured source.
This round removes about 3,000 net Rust lines, including replacement tests, without
changing flight selection, commands, controller behavior or report presentation.

### Removed and retained boundaries

- Removed the uncalled 961-line `pd-control` complete-program playback module,
  strict/operational playback exports, and its exclusive core validation/factory
  helpers. Live `FlightProgramUpdateV1` and historical program/binding DTOs retain
  their serialized shapes and strict unknown-field decoding. Current V2 owns its
  schedule/proof validation; ordinary simulator and saved-action replay remain.
- Removed obsolete V1 route validation, authority and endpoint-shaping math,
  plus its exclusive exact segment-corridor query. The live exact point-centred
  clearance implementation is byte-identical to the starting source. Nine checked
  historical planning declarations retain their shapes, including custom
  infinity/null decoding and existing policy snapshot values.
- Removed unreachable new-run V1 planner metrics and route-plan/compute artifact
  writing. Maintained resolvers already produce no V1 plans. Saved descriptors,
  historical cache completeness checks and plan-aware report refresh/rendering
  remain; a compatibility regression injects and refreshes a saved plan bundle.
- Retired execution of `transfer_waypoint_{contract_,}rpos80_{smoke,full}` and
  `transfer_waypoint_sequence_late_bend_diagnostic`. Both `single_dogleg_v1` and
  `late_bend_v1` reject explicitly even in renamed/custom packs and direct helper
  admission. Their fixture JSON, metadata, selector strings and saved reports
  remain readable. Exclusive geometry/constants/tests are gone; maintained bend,
  turn and double-bend generation and all supported handoff envelopes remain.
- Removed unused `PreviewSeries` / `build_multi_run_preview_svg`. The live
  `AggregatePreviewSeries` and trajectory aggregate preview remain. Extracted
  the existing common rich HTML/CSS/JS into `pd-report/src/rich_template.rs`:
  both raw-string hashes and all other facade source match the original extraction
  boundary. No charts, inspection views, data or handoff annotations were removed.
- Corrected nonexistent `transfer/scoring.rs` guide references, a stale research
  wrapper comment and active-versus-archival catalogue/navigation wording. Current
  ownership docs distinguish historical decoding from execution. These are source
  changes only: published pages were not refreshed or overwritten.

Intentional API/test removals cover only the retired implementations. Replacement
tests distinguish archival decoding from execution admission, cover original and
renamed pack rejection, retain handoff-goal coverage on a maintained bend, and
round-trip both retired selector strings. The pre-retirement fingerprint baseline
was committed separately as `ffc7f37` before any profile removal. It binds the
complete ordered scenario/descriptor serialization for all 20 surviving authored
waypoint packs: 3,366 resolved inputs, unchanged after retirement.

### Follow-up validation

| Check | Result |
| --- | --- |
| Maintained developer gate | All 11 steps passed |
| Workspace all-features tests | 446 passed, 4 intentionally ignored |
| Feature-enabled/default-off CLI tests, help and normal dependency boundary | Passed |
| Formatting, diff whitespace and strict all-target/all-feature Clippy | Passed |
| Maintained JavaScript tests | 49 passed |
| Final-source complete-flight parity | All 44 exact; only `planning_s`, `execution_s`, `replay_s` excluded |
| Maintained authored-waypoint input fingerprints | 20 packs / 3,366 inputs unchanged |
| Explicit retained expansion and report-projection tests | Both passed |
| Rebuilt evaluator acceptance and saved report-site checks | 44 cases, 42 rich payloads, 36 actual H, 46 receipt-bound pages passed |
| Historical native/CLI/repeat and saved-replay receipts | Passed; 42 supported receipts / 252 component digests |
| Rebuilt current CLI, fresh read-only replay of saved bundles | All 42 passed / 252 actual component digests matched; no new planning or artifact writes |
| Rich template and checked historical DTO/live point-query source preservation | Passed |
| Protected evidence inventory | 31 scopes / 3,867 file/link/directory entries unchanged |

Integration found two test-only issues before final acceptance. Moving a handoff
probe off the retired dogleg initially kept its old legacy envelope; the maintained
bend needs its existing `continuation_pass_through_v1` envelope. That correction
preserves handoff-goal coverage without tuning production geometry or guidance.
Strict Clippy then found a needless borrow in the new fingerprint test. After its
one-line correction, that focused test passed again and the last five developer
gate steps were resumed; the already-passing workspace/CLI steps were not repeated.
The 198 evaluator library tests pass, with four explicit archive checks ignored in
ordinary runs. The final numerical regression and saved compatibility checks run
on the corrected source, without expanded exclusions.

Two bounded Luna assignments handled retirement and the independent template
extraction. The primary owned scope/compatibility decisions, diff and source
preservation review, docs, the final lint correction, integration acceptance and
all reviewed local commits. No required cleanup item remains in this round.

Local ignored evidence is under
`outputs/validation/planner_housekeeping_round2_20261005/`: the initial inventory,
source/template preservation receipts, initial and resumed developer logs,
historical receipt checks and final structured result. All pack/scenario/research
fixtures and saved evidence retain their original contents and identities. The
frozen verdict remains 36 mandatory landings (11 direct, 25 corrected), two diagnostic landings, four
zero-step diagnostic stops and two unsupported inputs; all 44 pass integrity and
all 42 supported flights retain source replay.

Remaining V1-looking names and data readers are deliberate compatibility, not
an executable old planner or failed experiment to revive. Broader refactoring is
not required for the next product pass. Seeded terrain validation or a concrete
game-host adapter remains a separate decision; neither is established by cleanup.
No push, report publication, server restart or new acceptance campaign occurred.
