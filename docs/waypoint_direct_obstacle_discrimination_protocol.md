# Frozen direct-generator obstacle discrimination

This evaluator-only pass asks whether the unchanged nominal direct generator
preserves safe obstacle overflight and distinguishes a blocked selected arc
from terminal/contact rejection or bounded family exhaustion. It does not
execute waypoint routes or change `pd_plan::plan()`, controller defaults, core
physics/contact rules, F6, or the historical V2 classifications.

## Frozen inputs and policy

The generation implementation and its default policy remain unchanged: four
V2 seeds, five source-duration offsets each, 72 launch ticks, the existing
paired-command fitter, 120 Hz physics / held 60 Hz commands, current rotated
feet/hull geometry, strict terrain domain, flat whole-body pad corridors,
5 m reserve outside those corridors, unchanged contact thresholds, and
accepted-only planned-time ranking. No case-specific overrides or retuning.

Four development controls use the exact `continuous_flat_r00` full scenario:
uncut flat; the historical centered 160 m overflight obstacle; the historical
440 m first-boundary obstacle; and the historical 1,600 m unknown obstacle.
Obstacle terrain comes only from the raw-pinned first-decision input snapshots
(`ballistic_direct_first_decision_20260925/run_a/summary.json`, SHA-256
`65ad8538d901998ce6473af6d4248de521ef81901382d20d03bca3993247da5d`).
Historical commands and results are never generator inputs. The prior
known-flat summary is used only for byte-preservation comparison after
independent generation (SHA-256
`3e479a9e46c1753c0950076f2434c607db7588003830762c45298fc3e3953e35`).

Eight fresh full scenarios were sealed before implementation or fresh solving
in `fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json`,
SHA-256 `647df7bc94d02a9b48b773c45159e0ffa15bfca232a13b4e21164e2251a8f5b4`.
The original six-case generation manifest remains untouched. Fixed order is
700 m then 900 m, with each span using:

| Profile | Center fraction | Base width fraction | Height fraction |
| --- | ---: | ---: | ---: |
| Flat control | — | — | 0 |
| Low obstacle | .40 | .20 | .20 |
| High obstacle | .60 | .20 | .55 |
| Late broad obstacle | .70 | .25 | .35 |

Fractions are relative to source-to-target horizontal span. A trapezoid's base
edges are center ± width/2 and its top edges center ± width/4. Source and target
surface heights are zero, pads are 36 m wide, strict-domain padding is 160 m,
source center is `(-span, 5 m)` at rest/upright, seed is 7, and vehicle, gravity,
cadence, time budget and landing goal are unchanged. No cutaway or authored
waypoints. Late/broad terrain probes approach constraints, not proven
closed-loop recovery capability.

## Complementary lanes

1. A fixed-command terrain twin replays the corresponding flat span's selected
   accepted witness's exact requested throttle/attitude chronology. It changes
   only terrain and descriptive metadata, never refits commands. Shared logged
   tick extraction and geometry checks are used. It compares motion, fuel and
   held commands until the first terrain-driven contact, and independently
   compares ordinary and neutral core replay (including the ordinary stable
   touchdown velocity-zeroing convention). Terrain reserve loss and contact
   are recorded separately. This is a diagnostic counterfactual, not an
   independently generated accepted planner witness.
2. Regenerated direct evaluates the unchanged input-only generator under the
   new full terrain scenario. It retains every one of twenty rows, including
   skips and failed source/contact/geometry gates. Only complete accepted
   witnesses mean `Direct`; exhaustion means finite `Unknown`, not impossible.

A selected flat arc blocked by terrain does not establish waypoint necessity:
the regenerated family may find a different direct arc. Compare seed duration
multiplier and offset, not terrain-bound raw candidate IDs. If a fresh flat
case has no accepted witness, its obstacle twins explicitly record unavailable;
they must not borrow a witness from another span or invent a counterfactual.

## Gates and stopping rule

Development must reproduce known-flat bytes, retain complete valid ledgers,
accept a complete direct witness for the 160 m positive control, preserve
fixed-command replay prefix parity and ordinary/neutral parity for all twins,
and demonstrate at least one fixed-program terrain block. Reasons must retain
terrain, source, terminal/contact and other limitations distinctly. If any
development criterion fails, publish the four controls and stop before fresh
solving without changing obstacle geometry, generator policy or thresholds.

Only after primary acceptance of development, tests/integration, and a new
implementation freeze may fresh solving start. The freeze covers all workspace
source, Cargo inputs and relevant sealed fixtures, including this manifest;
it is checked before every fresh case and after the run. Evaluate all eight
cases in fixed order, even if an acceptance criterion fails. All must have
valid complete ledgers with no evaluation errors or omissions; both flat and
both low controls must be `Direct`; every accepted witness must pass existing
complete verification; and at least one fixed-program terrain block must be
observed. High/late cases may be `Direct` or finite `Unknown`. No forced waypoint
classification, no tuning after outcomes. A fresh gate miss is published with
all eight outcomes and ends this pass.

Artifacts are create-only, with typed semantic identities excluding paths and
wall-clock timing. Repeat gates and case artifacts at fresh paths and require
byte parity. Preserve all historical raw digests; run workspace tests, format,
strict all-target Clippy and diff checks on the integrated final tree. No
commit, push, deployment, controller promotion or operational waypoint
composition is authorized by this pass.

## Interpretation

Report safe same-arc overflight; selected arc blocked but alternative direct
accepted; terrain-associated finite-family exhaustion; or source/contact/other
limitations, including mixed reasons. A safe twin alongside generator `Unknown`
suggests a family/search limitation, but the twin stays diagnostic unless all
independent complete witness contracts are separately established. Existing
millimetre-scale contact margins remain nominal, not robust certificates.
