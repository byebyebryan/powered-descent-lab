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
- Layered Perlin and value noise retain separate population rates and proofs. No combined
  qualification headline, hidden direction exclusions or best-repeat selection.

The [declaration and frozen requests](../studies/mission_coverage/incumbent_v2/README.md)
bind input identities, selected indices, adapter configuration and executable
identities. Controller/planner/plant source remains unchanged. CLI composition
now accepts strict versioned collections; V2 reports use measured morphology
and chord-barrier bands rather than fabricated V1 terrain strata. Existing rich
batch/detail templates and bounded paired browsing are retained.

## Results

The fixed campaign completed on October 11, 2026 within the two-hour native
budget. The unchanged cap-24 reference, with global reflection for leftward
inputs, landed **77.0% on layered Perlin and 77.4% on value noise**. Both
normalized campaigns executed every scheduled admitted input; there were no
unsupported inputs, preparation failures or failed replay/integrity checks.

| Backend | Verified landings | Scheduled rate | Wilson 95% interval | Stopped airborne | Stopped before departure |
| --- | ---: | ---: | --- | ---: | ---: |
| Layered Perlin | 1,266 / 1,645 | 77.0% | 74.9–78.9% | 370 | 9 |
| Value noise | 1,290 / 1,666 | 77.4% | 75.4–79.4% | 366 | 10 |

Every nonlanding endpoint remained physically `flying`; no normalized attempt
ended in a recorded crash or off-target landing. These finite
controller stops still count as mission misses. Replay validates the recorded
execution, including a stop; it does not turn that stop into a landing.

The original 2,048 proposal slots per backend yielded 1,645 layered and 1,666
value inputs. The other **403 layered and 382 value proposals** failed challenge
admission and were retained in selection accounting without replacements.
The two admitted inventories contain **3,311 distinct physical inputs** with no
cross-backend duplicates. The main campaigns scheduled **6,622 attempts** and
executed **4,953**; the 32 predetermined integration attempts are separate,
including 24 executed captures on inputs repeated in the main campaigns.

The raw adapter landed **618/1,645 (37.6%)** on layered and **647/1,666 (38.8%)**
on value. Its 836 layered and 833 value leftward inputs were explicitly
unsupported. Conditional on its supported rightward inputs, rates were
**618/809 (76.4%)** and **647/833 (77.7%)**. Those conditional rates do not replace
the full scheduled rates.

Reflection left the saved rightward controller frames, commands, samples,
final states and stop results identical on all 809 layered and 833 value
inputs, excluding measured compute duration. Normalized leftward rates were
**648/836 (77.5%)** and **643/833 (77.2%)**. This supports treating direction as a
diagnostic filter inside shared terrain groups.

## What the challenge exposes

Mixed morphology accounts for much of each broad population and has the lowest
well-populated navigation-group rate: **764/1,058 (72.2%)** layered and
**696/982 (70.9%)** value. Smaller morphology groups are navigation aids with
their own denominators, rather than guarantees of category coverage.

| Measured chord barrier | Layered landings | Value landings |
| --- | ---: | ---: |
| Below 100 m | 125/139 · 89.9% | 272/295 · 92.2% |
| 100–below 200 m | 193/235 · 82.1% | 205/250 · 82.0% |
| 200–below 400 m | 416/535 · 77.8% | 356/450 · 79.1% |
| At least 400 m | 532/736 · 72.3% | 457/671 · 68.1% |

Inputs with low chord barriers qualify through other measured challenge paths;
this band is not a flat-terrain curriculum. The largest recorded nonlanding
stop is `terrain_recovery_no_safe_command`: **155 layered and 153 value** cases.
Next are waypoint aim construction misses (66 layered) and the original
controller budget (86 value). These records identify useful failure clusters;
they do not establish whether every admitted mission is physically solvable.

The earlier [gentle V1 held-out reference](procedural_learning_results.md) reached
1,973/2,048 (96.3%). The new
rate measures a different, more challenging distribution, with unchanged
controller behavior. It is not a paired regression estimate. The Wilson
intervals describe these development draws conditional on geometric admission;
unmeasured terrain, stress panels and generalization remain outside this result.
No RL training, controller tuning, fresh-input qualification, benchmark freeze
or accepted-default publication occurred. The
[fresh-input/stress comparison proposal](procedural_challenge_comparison_plan.md)
remains a separately scoped follow-up.

## Reports and retained verification

The latest [unified browsing edition](../outputs/procedural-challenge/baseline-20261011T181000Z/combined-review-r2/report.html)
puts both terrain families in one mission tree with separate results and
provenance. Its [validation record](report_unified_procedural_results.md) retains
the [focused browsing refinement](report_mission_browsing_qol_results.md) and
verifies unchanged campaign evidence. The prior
[layered edition](../outputs/procedural-challenge/baseline-20261011T181000Z/layered-review-r5/report.html),
[value edition](../outputs/procedural-challenge/baseline-20261011T181000Z/value-review-r5/report.html)
and original verified editions below remain retained.

- [Layered Perlin mission review](../outputs/procedural-challenge/baseline-20261011T181000Z/layered-review-r2/report.html)
- [Value-noise mission review](../outputs/procedural-challenge/baseline-20261011T181000Z/value-review-r2/report.html)
- [Layered metrics and input accounting](../outputs/procedural-challenge/baseline-20261011T181000Z/layered/metrics.json)
  and [value metrics](../outputs/procedural-challenge/baseline-20261011T181000Z/value/metrics.json)
- [Independent layered verification](../outputs/procedural-challenge/baseline-20261011T181000Z/layered/independent-verification.json)
  and [independent value verification](../outputs/procedural-challenge/baseline-20261011T181000Z/value/independent-verification.json)
- [Rightward command/state parity](../outputs/procedural-challenge/baseline-20261011T181000Z/rightward-controller-parity.json)
- [Report completion receipt](../outputs/procedural-challenge/baseline-20261011T181000Z/report-completion.json)
- [Layered browser checks](../outputs/procedural-challenge/baseline-20261011T181000Z/browser-layered-full/browser-result.json)
  and [value browser checks](../outputs/procedural-challenge/baseline-20261011T181000Z/browser-value-full/browser-result.json)

All four fresh neutral verifiers finished before runtime source changed. The
flight source checkpoint is `9a6b9ec`; controller/planner/plant/evaluator and
mission-generator implementation files match the prior `f7b0f0f` checkpoint.
The initial report refinement then failed because saved native details already
had static charts. Its failed edition and original `run-result.json` remain
retained. That receipt honestly reports an incomplete reporting stage, despite
completed verified flights.

Report-only commit `97d7fb1` makes static enrichment repeatable and preserves
the saved rich payload, chart modes and inspection tools. Fresh `-review-r2`
editions bind the original verification receipts and the new renderer identity;
report recovery added no campaign attempts or replays. Both reports keep the
terrain-family → morphology → chord-barrier → mission hierarchy. Desktop and
mobile checks cover zero initial preview images, one terrain band at a time,
12 mission rows per page, paging/reload, direction filtering, paired preview
bounds, rich details and return navigation, with no browser errors or horizontal
page overflow. The largest exercised bands contain 678 layered and 632 value
missions. The maintained 18-check gate also passed.
