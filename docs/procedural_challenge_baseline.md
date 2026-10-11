# Challenging V2 development controller baseline

[Calibration](procedural_challenge_calibration.md) ·
[Fresh-input comparison proposal](procedural_challenge_comparison_plan.md) ·
[Evaluation workflow](evaluation.md)

October 11, 2026: the owner requested trying the unchanged current controller on
the new challenge set. This first run measures the **existing exposed calibration
inputs**. It is a development baseline, not the larger proposed fresh-input
qualification/stress campaign. No new terrain, admission rule, controller setting
or learning allocation is introduced.

## Declaration before flight

- Reference: `IncumbentAdapterV1`, acquisition plus terminal safety, correction
  cap 24; raw and global-reflection adapters on identical physical inventories.
- Physical input: sealed 4 m terrain polylines, existing plant/fuel/90 s horizon.
  The seven finer-reference warnings remain explicit calibration diagnostics;
  this run makes no continuous-terrain or resolution-independence claim.
- Inputs: the admitted missions among proposal positions 0–2047 in each of
  `run-20261011T093222Z-pilot/layered-broad` and `value-broad`. Exclusions retain
  their original selection accounting; no replacement proposals are drawn.
- Integration gate: eight predetermined admitted inputs per backend, both
  adapters, at most 32 attempts. Requests are serialized before any flight;
  integration inputs remain part of the subsequent development population,
  with their repeated exposure stated explicitly.
- Main budget: at most 8,192 scheduled attempts across four campaigns; one
  attempt per admitted input per adapter. No outcome-dependent retries.
- Wall limit: two hours for integration, main campaigns and independent replay.
  Interrupted campaigns retain partial files and are reported incomplete.
- Every executed capture is independently replayed in a fresh neutral verifier
  process. Unsupported and preparation failures remain scheduled dispositions.
- Layered Perlin and value noise have separate reports and rates. No combined
  qualification headline, hidden direction exclusions or best-repeat selection.

The [declaration and frozen requests](../studies/mission_coverage/incumbent_v2/README.md)
bind input identities, selected indices, adapter configuration and executable
identities. Controller/planner/plant source remains unchanged. CLI composition
now accepts strict versioned collections; V2 reports use measured morphology
and chord-barrier bands rather than fabricated V1 terrain strata. Existing rich
batch/detail templates and bounded paired browsing are retained.

## Results

Pending the declared execution. The fresh-input/stress comparison proposal
remains a separately scoped follow-up; this development run does not freeze it.
