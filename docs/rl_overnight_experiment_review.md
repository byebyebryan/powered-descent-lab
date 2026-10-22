# Review of the overnight RL experiment

[Experiment plan](rl_overnight_experiment_plan.md) ·
[Shared preparation](controller_rl_preparation_results.md) ·
[Earlier learning recommendation](procedural_learning_next.md)

October 12, 2026. Source-backed self-review against clean main `0e74f24` and
the prepared RL worktree at `51c143c`. This is planning/research evidence, not
an independent-agent review or a launched training allocation.

## Verdict

The plan is suitable for a bounded eight-hour RL experiment. Preparation provides
the isolated native environment and frozen V2 validation assets; the next missing
work is an explicitly versioned V2 sampler/trainer, diagnostics and selection
contract. Launching the historical `train.py` defaults would repeat V1 training.

Proceed through the implementation and pilot gates before committing the larger
allocation. A learner that cannot discover contact on its easier complete missions
should yield a bounded diagnosis, rather than another night of reward/seed repairs.
An experiment with no challenging landings can still complete this research task;
it has not established a replacement controller.

## Findings and incorporated adjustments

| Evidence or counterexample | Disposition in the plan |
| --- | --- |
| Nine earlier allocations and corrected potential accounting still produced zero held-out landings. | Require a new sampler hypothesis, physical telemetry and an early-stop rule; no uniform replay of old defaults. |
| The prepared bridge already supports physical V2 generation/loading and exact command parity. | Use the existing bridge, installed receipt and common evaluation; keep algorithm choices in RL-owned code. |
| Historical `FAMILIES`/`TRAINING_SEEDS` and design imports select V1 environments. | Add named V2 frontdoors, preserving old defaults and frozen design identities. |
| `evaluate.py` and `freeze.py` currently recognize only the old design/freeze shapes. | Extend approach configuration/source/state binding explicitly; do not label new policies as old reward-only revisions. |
| Uniform H-only versus adaptive C0/C1/H changes exposure as well as ordering. | Claim a sampler-package comparison. A static-mixture control is needed later to isolate ordering. |
| Easiest-stage survival or horizontal proximity can select a hovering policy. | Select on challenging native landing summaries; use both-axis separation, speed and contact limits for diagnosis only. |
| Summary selection does not retain saved commands. | Require individual native landing proof before validation eligibility and full fresh replay for validation rates. |
| A representation tested at four million steps cannot be compared causally to an eight-million-step thrust policy. | Choose a common selection horizon; longer checkpoints remain separately labeled development evidence. |
| C0/C1 native proposals can be gentle and are not broad admitted challenges. | Keep separate training/diagnostic banks and verification; never fabricate challenge membership or a qualifying collection manifest. |
| 80/20 reset weights need not produce 80/20 transitions when episode lengths differ. | Record both episode and transition exposure shares. Backend evaluation rates stay separate. |
| Sparse local/route terrain features can alias narrow 4 m physical hazards. | Audit the existing encoder during the pilot, retain the limitation, and hold features fixed for this comparison. |
| The velocity adapter supplies deterministic feedback beyond the learned policy. | Declare its full physical mapping and source ownership as part of the approach; no existing planner/controller calls. |
| Training on fresh seeds does not alone prevent collisions or exposure leakage. | Audit all resolved proposal identities against declared history/reservations, bind banks and log reuse. |
| Validation-driven redesign consumes the validation reservation. | Freeze design/all three checkpoints before a single complete validation pass; no re-ranking or additional variants afterward. |
| A night can run out of time/disk before every capture verifies. | Reserve the final 90 minutes, forecast the full campaign, and retain incomplete accounting rather than reduced denominators. |

## Source and research anchors

- [RL branch handoff](controller_rl_development_handoff.md) binds task, ownership,
  cohorts, worktree environments and final-test access rules.
- [Training entry point](../studies/rl_procedural_ppo/train.py) anchors fixed V1 seeds,
  PPO settings, summary-only selection and current source-binding behavior.
- [Physical environment](../studies/rl_procedural_ppo/pd_env.py),
  [braking potential](../studies/rl_procedural_ppo/pd_env_v2.py), and
  [aligned potential](../studies/rl_procedural_ppo/pd_env_v3.py) anchor the existing
  features, direct actions, launch primitive and objective.
- [Reward tests](../studies/rl_procedural_ppo/test_reward_designs.py) check discounted
  cancellation and unchanged physical execution under identical actions.
- [V2 recipe validation](../pd-missions/src/recipe_v2.rs),
  [generation](procedural_generation_v2.md) and
  [collections](procedural_collections_v2.md) anchor stage controls, physical
  sampling bounds and controller-independent broad admission.
- [Policy exporter](../studies/rl_procedural_ppo/evaluate.py),
  [checkpoint-free verifier](../studies/rl_procedural_ppo/verify.py) and
  [comparison guard](../pd-report/src/comparison.rs) anchor configuration,
  source/native binding, accounting and role/population separation.
- [Earlier results](procedural_learning_results.md) establish the measured
  negative result and hover endpoints; the V2
  [development baseline](procedural_challenge_baseline.md) establishes the
  distinct incumbent reference.

Research was checked against primary sources:

- [Procgen](https://proceedings.mlr.press/v119/cobbe20a.html) supports diverse
  procedural training and separate generalization measurements. It does not
  validate our stage sizes, categories or arbitrary-terrain coverage.
- [SB3 2.9.0 guidance](https://stable-baselines3.readthedocs.io/en/v2.9.0/guide/rl_tips.html)
  matches the locked SB3 version and supports multiple seeds, normalized actions,
  physical environment debugging and simpler discovery problems. The pinned
  CPU runtime remains unchanged; a newer optimizer is not a prerequisite.
- [Potential shaping](https://ai.stanford.edu/~ang/papers/shaping-icml99.pdf)
  supports discount-consistent state-potential differences. Our finite intrinsic
  90 s horizon is part of the observed native task, rather than an external
  collector time limit; keep its terminal semantics and zero terminal potential.
  Policy invariance does not guarantee exploration or invariance to additional
  time/fuel penalties. The earlier native tests provide implementation evidence.

## Budget and static recipe review

The optimizer ceiling is
`2 × 262144 + 6 × 8388608 + 6 × 4194304 = 76,021,760`.
Optional representation signal checks are inside its six allocations. All main
counts/checkpoint intervals are exact multiples of the 8,192-transition rollout.
Pilots are real optimization, separately counted and excluded from selection.
No more than twelve main allocations or two sampler pilots are available.

New proposals total
`2 × (1024 + 1024 + 4096) + 2 × (32 + 32) = 12,416`.
Native verification by one regeneration makes at most 24,832 generation requests;
eight structural attempts remain the per-request native ceiling. Proposals and
generator executions are different counters. H entry counts depend on admission,
not refill. Missing/exhausted diagnostic proposals retain their scheduled failure
within the declared 256-slot development panel.

The changed recipe ranges fit the inspected static native guards: C0's minimum
150 m travel exceeds the maximum 128 m combined shelf/transition extent; unchanged
three-octave controls give a minimum nominal wavelength about 52 m, above the
16 m sampling guard, and the minimum feature radius remains 32.4 m. Actual native
validation, geometry characterization and sealed-bank checks still belong to
execution; these calculations are not generated-input or solvability evidence.

The complete capture path is
`3 × (424 + 420) + (424 + 420) + 3 × 50 + 128 = 3,654`.
It fits the 4,096 capture/replay ceilings and keeps broad/stress separate.
Recurring development checks at eight primary/four optional checkpoint intervals
use `6 × 8 × 256 + 6 × 4 × 256 = 18,432` summary attempts, leaving headroom
under 32,768 for declared pilot/final checks. Every additional check consumes that
ledger; no hidden summary sweeps are authorized.

Phase allowances sum to 480 minutes. They include implementation and reports,
and are ceilings rather than required utilization. Old timing supports feasibility
only weakly because V2 mission lengths/geometry and capture volumes differ.
The pilot must measure throughput, disk use and source/native stability before
recording the actual main allocation. Final tests have zero allocated exposure.

## Gates still belonging to execution

This review has not generated or sealed banks, reserved the proposed seed namespaces,
implemented new interfaces, tested continuation, optimized a policy or executed a
campaign. Native recipe/physical verification, collision/exposure reconciliation,
new configuration and source-binding tests, objective/control diagnostics, actual
timing and the maintained gate are required before main training.

Do not infer progress from reward alone. Read all seed results and actual native
failure modes before the conditional adapter decision. Check full accounting,
frozen-source/configuration equivalence, rich report/browser behavior and preservation
at the end. Keep accepted selectors, historical outcomes, final-test reservations
and the user's server unchanged.

Documentation validation passed: 227 Markdown files and 1,580 local links checked
with zero issues; all 13 focused docs/development-gate Node tests passed; the diff
whitespace check was clean. These are planning-document checks and allocate no
training, generated missions or measured flights.
