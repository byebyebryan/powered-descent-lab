# Review of shared controller/RL preparation

[Preparation plan](controller_rl_preparation_plan.md) ·
[Architecture](architecture.md) · [Documentation home](README.md)

October 12, 2026. Reviewed against the clean `686a684` checkout, current contracts,
retained development evidence and source implementation. This is a source-backed
self-review of the proposed preparation, not an independent-agent review or an
executed readiness/qualification campaign. No inputs, flights, replay campaigns,
training, worktrees, report editions or server operations were started.

## Verdict

The revised plan is suitable for a bounded shared preparation goal. Most
platform separation is complete; input/exposure freeze and V2 application
interoperability remain. Development branches can split after the stated gates
without waiting for an improved incumbent, an RL landing or complete stress
coverage. Execution remains unlaunched.

The review expanded the earlier metadata-only description of the RL gap,
specified held-out access rules, prevented pooled population scores and made
the fresh validation reference optional. Those adjustments are incorporated
into the plan rather than left as a second task list.

## Findings and disposition

| Finding from current source/evidence | Plan adjustment or preserved boundary |
| --- | --- |
| Neutral runtime/evaluation/report/Python packages already avoid executable incumbent dependencies. | Keep the architecture; add bounded adapters and enforce the maintained dependency gates. |
| Native V2 generation/loading and Python command parity already exist. | Do not rebuild the physical bridge. Exercise the installed build and complete its application consumption. |
| RL training aliases `value-noise` and `layered` still select V1 with old fixed seeds. | Leave historical training defaults intact. Fixed V2 evaluation support is preparation; explicit future sampler/curriculum declarations belong to RL development. |
| RL export uses generic `recipe.generator_version`, index-derived fallback direction/stratum and no measured V2 geometry. | Supply verified backend-specific family identity, physical direction and receipt/geometry metadata through a neutral helper. |
| RL whole-campaign input scope lists accepted entries, while native `immutable_files()` includes excluded V2 proposals. | Reuse that native inventory; fully verify sources and bind all proposals once per campaign. Avoid both missing evidence and whole-collection hashing inside every policy step. |
| Independent Python verification checks input identity and outcome proof but does not currently reconcile V2 coverage geometry. | Include metadata reconciliation with the original versioned input and selection receipt in the preparation gate. |
| RL report rows lack preview/endpoint presentation metadata. | Populate from saved execution and retain common rich details, bounded preview loading and context returns. |
| Current campaign validation rejects rows with neither geometry nor a V1 stratum; failed input loading can leave real geometry unavailable. | Require explicit, truthful failure presentation and full accounting. Cache only metadata verified from sealed inputs; add a narrow unavailable-status representation if needed. Never invent a stratum or zero measurement to satisfy validation. |
| The unified renderer groups rates by terrain family; it does not distinguish broad/stress or validation/final-test populations by itself. | Guard comparison composition by role/population and source declarations. Keep separate comparison views for those populations, each with backend branches. |
| Current calibration and flight inputs are already exposed. Both backends are known to the designer. | Keep current roles/history; use fresh audited namespaces for reserved cohorts. Do not claim an unseen backend or redefine old inputs as held-out. |
| Existing input audit/report tools are designed for inspection and can create exposure receipts/previews. | Final-test preparation uses structural/hash verification only, with no individual audit previews, policy runs or ordinary developer access. |
| Known close-obstacle/tight-crest combinations are sparse; matching panels may overlap. | Use a bounded input-only support pilot, fixed panel rules and a predeclared selection rule. Keep unique unions, missing measurements and shortfalls explicit. Do not chase quota completion. |
| New test/control choices must not alter native clocks, contact, fuel or success. | Freeze physical semantics. Agent decision cadence, launch primitives, internal limits and rewards remain explicit approach choices. |
| Old proposals contain larger flight budgets and completed prerequisites. | This plan declares its own ceiling and defaults; it does not reopen the older qualification campaign or redo the completed incumbent V2 frontdoor. |
| Native build receipts contain absolute source/module paths. | Each worktree owns its environment/build and obtains a matching receipt. Copy inputs with hash checks; preserve old receipts and their paths. |

## Source anchors

- [Architecture](architecture.md) and [independent agents](independent_agents.md)
  define physical observations, execution, proof and normal dependency boundaries.
- [Versioned collection reader](../pd-missions/src/collection_versions.rs) already
  provides role, entries, full immutable file inventory and complete verification.
- [Native Python bridge](../pd-python/src/lib.rs) implements explicit V1/V2
  generation/loading and tests original JSON shapes and command execution.
- [Incumbent presentation adapter](../pd-benchmark/src/presentation.rs) provides
  the working backend/morphology/geometry mapping. Shared extraction must not
  make Python or neutral crates depend on this concrete composition package.
- [RL environment](../experiments/rl/pd_env.py),
  [exporter](../experiments/rl/evaluate.py),
  [independent verifier](../experiments/rl/verify.py) and
  [exposure audit](../experiments/rl/analyze.py) anchor the remaining application work.
- [Campaign projection](../pd-report/src/campaign.rs) and
  [comparison renderer](../pd-report/src/comparison.rs) anchor presentation
  validity and the population-composition guard.
- [V2 population rules](../pd-missions/src/population.rs) use inclusive numeric
  ranges, exclude missing measurements from matches and retain overlapping panels.
- [Generation contract](procedural_generation_v2.md),
  [calibration](procedural_challenge_calibration.md) and
  [baseline](procedural_challenge_baseline.md) establish the physical grid,
  distribution limits and existing measured results.

## Counterexamples checked in the review

- A renamed old collection or a fresh seed with prior exposure is not held-out.
  The plan requires both namespace/exposure reconciliation and physical checks.
- A corrupted excluded proposal must invalidate collection evidence even when
  all selected missions still run. It is included in the native file inventory.
- A malformed selected input must remain in the scheduled accounting without
  invented geometry, direction or flight proof. Source-fixture fault tests are
  required; ordinary valid-input browser checks alone cannot establish this.
- A verified unclassified input is valid geometry without matching labels;
  unavailable metadata is a failure state. They cannot share a fallback label.
- A broad and a stress cohort can share the same backend and disjoint physical
  inputs while still being inappropriate to merge into one rate. Physical parity
  alone does not establish population compatibility.
- A stress proposal can fill several quotas. Summing panel counts exaggerates
  the mission denominator; only the unique selected union counts.
- A zero-landing scripted/PPO smoke can still prove integration. A successful
  smoke does not establish policy learning, feasibility or qualification.
- Moving a native extension into another worktree while editing its old receipt
  is not preserved provenance. New installed-build evidence must be created.
- Reaching the wall ceiling with a partial cohort/campaign is not readiness.
  Preserve its incomplete status instead of extending budgets or weakening gates.

## Budget and sequence check

New proposal ceiling: `128 + 256 + 512 + 512 + 512 + 2048 + 2048 = 6016`.
Every collection is below the native 20,000-index bound; structural attempts
remain separately bounded to eight per proposal. Accepted counts are not targets.
Panel quota ceiling: `4 × 64 = 256` unique selected stress inputs, with overlapping
memberships retained separately. The alternate stress recipe must still pass
native recipe/sampling validation before its pilot; static review is not that gate.

Default measured-study allocation: `16 × 3 = 48` scheduled integration attempts,
at most one independent verification per executed capture, and zero optimizer
transitions/final-test attempts. The optional validation reference adds at most
`512 + 512 = 1024`, for an absolute ceiling of `1072`. It is skipped unless the
launch declaration includes it. No reference outcome chooses input membership.

Source-fixture/software checks remain separate from measured campaign slots.
P4 cannot precede the sealed-cohort and interoperability gates. Worktrees start
from the completed common checkpoint; algorithm development has later contracts.

## What remains to validate during execution

The review does not claim that the exposure inventory is complete, the new
cohorts exist, the stress panels fill, the installed extension is current, missing
metadata is handled, or the smoke passes. These are explicit preparation exits.
Native declaration validation, full collection verification, error/drift/parity
tests and actual rich-report browser checks remain required before the split.

Documentation validation passed: the local-link checker reported no issues,
all 19 selected development-gate/report-presentation Node tests passed, and
`git diff --check` was clean. These are documentation checks, not the P5 runtime
readiness gate. They do not allocate the proposed study budget or regenerate
reports. The implementation goal can be launched against this reviewed plan.
