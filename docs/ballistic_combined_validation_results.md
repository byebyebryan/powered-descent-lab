# Combined safety preservation and fresh terrain results

[Documentation home](README.md) · [Frozen plan](ballistic_combined_validation_plan.md) ·
[Independent changes](ballistic_acquisition_safety_results.md)

The combined opt-in lands **866/1000 (86.6%)** on the original worlds, versus
acquisition-only's 863: three gains, no losses and all 863 earlier successful
ordinary flights exact. On a fresh, seed-disjoint 1k it lands **824/1000 (82.4%)**,
versus acquisition-only's 818: six gains, no losses and all 818 successful flights
exact. All four fresh recipes improve under the terminal safeguard.

The combination works, but the reused original population is optimistic relative
to this new sample. Use **about 82% fresh coverage**, not 86.6% arbitrary-terrain
reliability, as the current expectation. These populations have identical recipe
definitions, not identical terrain. Their rate difference is not a paired loss
or proof of a particular overfitting mechanism. No V17 or policy-3 fresh comparison
was run, so this pass does not measure their fresh landing rates or attribute
acquisition-only's held-out gain over either of them.

## Mechanism and exact control

`acquisition-terminal-safety` only composes the two existing independent opt-ins:
native destination acquisition at the existing coast-query cadence without the
horizontal-room trigger, plus upright coast/support after a terminal reserve
rejection. There is no new trajectory family, waypoint rule, controller tuning,
command choice, physical guard or time allowance. Ordinary policy 3, the ridge
experiment default and standalone coast-terminal execution stay unchanged.

The preceding checkpoint was reviewed and committed as **`dd28de8`**. Its
863/1000 acquisition-only capture is the sealed original reference. New-source
acquisition-only reproduces all **112 control feedback files byte-for-byte**:
104 selected worlds and eight repeats, including 283/327/861. The controls land
69/104; that selected fraction is not a broad terrain estimate.

The fresh input pack freezes 1,000 unique seeds before terrain generation, master
seed 2026101001, with 250 worlds per unchanged recipe. Seeds exclude all earlier
study, sanity, calibration, challenge, original-1k and fresh-100 populations.
Fresh-process reversed generation reproduces every profile exactly. Pylander
source hashes, refinement, 4 m sampling, local pad preparation and supported
vehicle/Earth/120-60 Hz setup are unchanged. No outcome filtering or replacements.

## Paired results, separate populations

| Population | Acquisition-only | Combined | Combined gains / losses |
| --- | --- | --- | --- |
| Original development 1k | 863/1000 | 866/1000 | 3 / 0 |
| Fresh seed-disjoint 1k | 818/1000 | 824/1000 | 6 / 0 |

| Recipe, 250 worlds each | Original acquisition / combined | Fresh acquisition / combined |
| --- | --- | --- |
| Mountains, 4x | 215 / 215 | 201 / 203 |
| Mountains, 8x | 220 / 220 | 211 / 212 |
| Broad massifs, 8x | 224 / 226 | 221 / 223 |
| Successive ridges, 8x | 204 / 205 | 185 / 186 |

Original gains are **538/650/796**. The combined mode changes ten ordinary
flights, issuing 255 fallback frames; 990/1000 complete ordinary flights remain
exact, including every previous success. All three gains follow actual H.

Fresh gains are **087/241/459/583/741/854**, all formerly terminal short-command
stops. The combined mode changes eleven ordinary flights, issuing 421 fallback
frames; 989/1000 remain exact, including all 818 successes. Five changed worlds
still stop later. Three gains follow actual H, and three have no actual H.
All **2,000 paired terminal entry clocks and preterminal command prefixes** match.

Original combined landings include 203 after actual H and 663 without H; fresh
combined has 188 and 636 respectively. Zero-H does not mean an uninterrupted
direct route: waypoint-directed correction or local recovery can occur before
an optional destination acquisition cancels the active waypoint.

Neither population has an executed crash or off-target landing. Every remaining
case is a verified protected airborne/in-progress stop, not proof of physical
impossibility. Against V17 on the original worlds the combined total is 866 versus
829, with forty gains and the same three acquisition losses. Against the older
policy-3 cap-24 reference it is 866 versus 817, with 146 gains and 97 losses:
not strict dominance or accepted replacement evidence for that different planner.

## Remaining failures and the next priority

| Stop group | Original combined | Fresh combined |
| --- | --- | --- |
| No safe recovery command | 69 | 88 |
| Waypoint construction miss | 21 | 27 |
| No ballistic aim | 20 | 21 |
| Local proposal budget | 2 | 10 |
| No local waypoint | 0 | 4 |
| Waypoint continuation rejection | 0 | 2 |
| Recovery episode budget | 0 | 1 |
| Terminal short-command rejection | 8 | 8 |
| Original time budget | 14 | 14 |
| Prediction terrain domain | 0 | 1 |
| Total | 134 | 176 |

Original stops are 112 preterminal and 22 terminal; fresh stops are **153
preterminal and 23 terminal**. Recovery-command exhaustion alone is half of the
fresh remaining population. Even perfect terminal completion would add only
23 fresh landings, reaching 847/1000. Successive ridges remain the weakest recipe.

**Use the combined mode as the opt-in development reference**, while retaining
acquisition-only as the paired comparator. The tiny terminal safeguard is useful
on both populations; it is not the remaining planning solution. Stop optimizing
the original landing headline. Preserve the original worlds as the development
corpus and the now-observed fresh worlds as a locked regression set, not a forever
untouched test. Any later claim of fresh generalization needs new unseen inputs.

The next high-yield pass should diagnose original recovery failures using actual
queued correction, warning, rotation and response sequences: distinguish late
warning, insufficient response room and a held-command prediction veto from a
genuinely exhausted response family. Freeze a small portable diagnostic panel
before changing behavior. Do not shorten the guard, increase arc/profile counts,
add seed-specific waypoint rules or retune against this new test population.
Default/CLI/session integration and a ballistic-candidate benchmark remain
separate readiness work; this is not a game-loop acceptance or automatic promotion.

## Deadline regressions: diagnosis, not an extra-time repair

283/327/861 keep their exact acquisition-only ordinary flights in the combination.
The source policy is 90 s maximum minus a 10 s mission-time reserve: an 80 s
original planning deadline. The loop stops at 79.8 s to retain one 24-tick query
interval. This is explicit existing policy, not a reset clock or fuel exhaustion;
the three stopped flights still have approximately 4.1–4.3 tonnes of fuel.
The 10 s reserve is not currently a separate spendable terminal-flight allowance.

| Case | V17 / new terminal entry, s | V17 / new height above target, m | V17 / new downward speed, m/s |
| --- | --- | --- | --- |
| 283 | 33.00 / 30.65 | 844.2 / 918.6 | 36.26 / 14.50 |
| 327 | 34.50 / 29.87 | 867.9 / 1061.9 | 57.80 / 14.70 |
| 861 | 49.80 / 40.73 | 313.7 / 545.8 | 51.48 / 13.24 |

The new entries are earlier, higher and descending more slowly. Latest-safe
descent lasts 42.80/46.67/33.47 s respectively, versus 37.68/34.95/8.30 s before.
All three new traces lack an active retained terminal arrival plan throughout;
this is not a continuously extended countdown. In 283/327, initial local
touchdown clearance is only 19.1/109.4 m despite height above target of 918.6/1061.9 m.
The terminal rate policy uses current touchdown clearance; cautious early control
near terrain can therefore prolong a much longer descent to the lower target.
861 has much larger local clearance and substantial lateral speed, so the local
clearance observation is not a universal explanation for all three cases.

This supports separating terminal-entry shape/time cost from waypoint geometry.
It does not prove that a later entry or extra time safely lands them. No stopped
mission was continued, reserve spent, guard relaxed or trajectory tuned here.
Keep the losses in the original comparison and defer any time-policy change.

## Evidence, reporting and validation

All **3,130 planned attempts** complete and verify, including 26 external repeats.
Native integrity, physical/mission tuples, decision reproduction and complete
source-command replay remain mandatory. Independent audits authenticate all four
stage inventories, repeats, full replay states, original comparisons, both paired
preterminal prefixes, all 225 frozen source files and 68 protected paths. The
original 112 control feedback files additionally match exact bytes. Existing
navigation work, accepted selectors/pages and retained captures are unchanged.

Scoped feedback tests pass **56/56**, terrain-tool tests **207/207**, and batch
example tests **3/3**. The maintained eleven-check developer gate passes,
including 69 Node tests, strict Clippy, formatting and documentation links.
All 44 default-planner inputs retain exact numerical parity against the later
`capture-early-exit-20261007-native` checkpoint. This is default preservation,
not a new 44-case ballistic-candidate acceptance run; the older published
checkpoint's known exact-float discrepancy remains explicit and unmodified.

Common rich reports and failure-first review trees remain. Optional population
and comparison labels distinguish genuinely unpaired fresh acquisition evidence
from paired results; no zero-success baseline is manufactured. This is automated
artifact/data checking, not manual browser acceptance. The running dynamic report
library discovers the new captures normally; no server restart, manual root
publication or historical report rewriting was performed.

Collection uses four workers per stage, sequentially. Original combined takes
605.04 s, fresh acquisition 539.35 s and fresh combined 585.07 s; the slowest
attempt is 4.98 s. These include native proof/report collection costs and do not
establish a game-controller latency or a controlled speedup benchmark.

Read-only saved checks:

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_combined_validation.py verify original-combined
rtk proxy python3 -B studies/terrain_profiles/ballistic_combined_validation.py verify heldout-acquisition
rtk proxy python3 -B studies/terrain_profiles/ballistic_combined_validation.py verify heldout-combined
```

Capture roots are `capture-ballistic-combined-validation-{inputs,control,original-combined,heldout-acquisition,heldout-combined}-20261010-v1`
under `outputs/eval/planner_v2_random_terrain/`.

```text
native:            145c5ed7bcdec899782c561d2452ba369f8beb8c767117e7d20d6197ad2f6e2b
original receipt:  0dd8aea93fbfa90b0ee4da1fec8fe8e46eac1a8b00aa212717ccf5e9fdd88e51
original results:  fd3d1df41d3069cf83037b4eae17aebb12850ac02ed22a414116f43ae3f0c620
fresh A receipt:   bf75b93fcae7d42e46a881f4f5bb619a9f54cd586be841407756418a1641b392
fresh A results:   f1b4ca6fd5434fd76b37bb7bf03fde06bab5c814232478df52ad4c1a8582fb14
fresh AB receipt:  a1d70356153e92539c99733ab89f5f4d5b10f5b6b81d4b7d5b5a746aa6e7e154
fresh AB results:  286d9cdd1265817c67bc4a3adf3317d558871ec6fb0339877a8a8d4b27acd97a
```

The measured pass performed no push, default promotion or further campaign.
Subsequent review commits do not relabel its frozen source or regenerate its
evidence. Default promotion and new campaigns remain separate decisions; the
pre-existing navigation changes are outside this checkpoint.
