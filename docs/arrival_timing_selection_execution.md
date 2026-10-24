# Arrival timing and goal selection execution

This pass follows the [frozen plan](arrival_timing_selection_plan.md).
Its create-only root is `outputs/arrival-timing-selection/loop-20261024T171406Z`.
The baseline records head `2cabd70`, 924 Git-visible file hashes and all foreign
changes before edits. The three-hour deadline is Unix 1792872846; limits are
1.2 billion charged steps, 256 captures, 8 GiB retained artifacts, one native
worker, at most 20 closed-form fits, no RL gradient updates and 40 GiB free space.

## Initial declaration

The first-feasible and fuel-ranked continuation comparison, fixed pilot inputs,
capture roles and conditional learning release criteria are declared before any
new campaign outcome. Existing controller/Poly arithmetic, goals, deadlines,
tolerances and historical outcomes remain fixed. The earlier 60-state
confirmation is now exposed comparison evidence.

Implementation, cost release and measured findings are recorded below.

## Opt-in timing implementation

The new complete-bank selector shares the original arrival, prepared coast and
Poly forecast/execution path. It ranks only predicted stable target contacts by
remaining fuel, retaining the earliest grid entry on exact ties. Resource
exhaustion fails and closes the opt-in executor rather than returning a partial
optimum. The original profile retains its serialized feedback and command path.

All four focused Rust composition tests pass, including selected native
fuel/contact equality and resource-limit closure. All seven airborne native
Python tests pass, including the new complete captured/reconstructed timing
execution and exact default info equality. Strict focused Clippy passes.

## Source freeze and pilot cost release

Runtime epoch A is frozen from `bbc4fda`; its native library SHA-256 is
`e237d45a382903e3b1dec65de433a45dfd6da202011c25b0e8078accbcdca50f`.
Its compiled ledger uses retained source paths under `source-epoch-a`.
The fixed pilot completes all 24 Summary executions and six full captures;
every attempt lands and every arrival fulfills its instruction. The original
profile matches retained complete decision info exactly. Charged pilot work is
6,712,412 steps, including reconstruction, goal generation and neutral replay.

`comparison-release.json` freezes 360 crossed Summary executions on the exposed
60 originals, 36 fixed new-selector capture roles plus each original's best
eligible new choice and all failures. Its cap is 300 million steps and 180
captures; all-pass capture count is at most 84. The conditional learning size
ceilings are 120 training, 36 validation and 48 final-test originals, with every
final choice captured. This keeps the all-pass total at most 234 captures inside
the 256 allowance. No outcome-dependent input replacement or goal change occurs.

## Crossed results and learning stop

The 360 crossed executions complete on the same 60 exposed originals. All 180
default-profile decision records match their retained originals exactly. Every
new-selector goal attempt fulfills its instruction and every new-selector
execution has native target success. Arrival feedback remains exactly unchanged.

| Initial strategy | First-feasible mean fuel kg | Fuel-ranked mean fuel kg |
| --- | ---: | ---: |
| Direct | 741.591670 | 589.927633 |
| Support short | 777.912935 | 669.834668 |
| Brake long | 798.952165 | 673.437424 |

Improved direct timing saves 151.664037 kg on average (20.4512%). Under consistent
improved continuation, hindsight goal selection saves only 1.246651 kg (0.2113%),
versus the preceding first-feasible comparison's 36.785215 kg (4.9603%). Direct
wins 53/60 states, support six and braking one. Seven states save more than 1 kg;
maximum residual saving is 31.793852 kg. This is a controlled comparison on
exposed inputs, not new population confirmation or a learned policy.

The predeclared 1% mean-opportunity criterion fails. `learning-stop.json` records
the stop before any fit or training/validation/final-test generation. The other
release conditions pass, but they do not override the failed mean criterion.
No reward, goal, tolerance or release threshold is changed to start learning.

## Complete proof and independent review

The initial fixed-plus-best roster has 84 new comparison captures. Before
additional execution, `proof-completion-allocation.json` declares all 96 remaining
new-selector choices for complete proof, reserving 150 million steps. These are
the original scheduled inputs and unchanged outcomes, not new cases or policy
selection. All 180 new choices now have complete physical/mission/integrity/replay
evidence, controller and regenerated-goal reconstruction, and Summary equality.

There are 186 new verified landing captures including six repeated pilot roles,
372 reconstructions and 124 exact native arrival-prefix checks. The read-only
independent audit validates all schedules, canonical scenario identities, sealed
hashes, selected timing minima, physical goal-resolution ticks, prefixes and work.
Its initial reader compared default-expanded native scenario JSON to sparse input
JSON; it now compares the complete canonical scenario to the sealed original and
independently preserves exact input hashes. No flight/input/proof was changed.

Incoming native contact speeds span 1.305–2.674 m/s. These are measured from
`incoming_contact.state`; post-contact final velocities have already been reset
and are not touchdown-speed measurements. Frozen Summary fields are preserved.

Total charged work is 148,935,601 steps: 2,887,015 native, 75,118,905 local model,
69,533,679 timing forecast and 1,396,002 neutral replay steps. Goal construction/
reconstruction contributes 407,596 native steps. Campaign phases total 137.08 s;
build, implementation and review time are separate. There are 570 original
execution occurrences, one native worker, zero closed-form fits and zero RL
gradient updates. The 60 comparison originals remain the population denominator.

Within the crossed Summary panel, complete timing search uses 16,225,500 forecast
steps versus 1,343,870; arrival model work is identical at 14,376,977 steps per
profile. Aggregate episode process CPU is 23.73 versus 11.46 s. Developer checks
ran concurrently during some collection, so wall measurements are descriptive;
per-update latency and an isolated real-time budget remain unqualified.

The useful result is improved timing selection, with a much smaller residual
goal-menu opportunity. Further timing-search optimization/amortization or a new
capability envelope needs its own hypothesis and fresh evaluation. The current
pass does not authorize a different learning task after its release gate failed.

## Validation and HTML delivery

The [complete HTML mission tree](http://starship:8000/arrival-timing-selection-report-20261024T171406Z/report.html)
contains all six strategy/timing executions for every original, plus the six
separate pilot captures: 366 rich views, 60 comparison states and 366 focused
previews. The [published findings](http://starship:8787/reports/9dacc45f867ad355453e68ba15cd014c4c75ee4665ee56be6463f578f150a6eb/)
provide the crossed table and standalone figure, linking the complete tree.
The rich edition is 926,447,433 bytes, above the shared publisher's bundle limit;
its canonical raw-capture links stay on the existing project report service.

The final maintained gate passes all 19 checks. Browser acceptance at 1440×1000
and 390×844 browses every occurrence, exercises the 60-state/six-choice picker,
seven rich details per viewport, plot modes and next/back navigation. There are
no final application errors or page overflow. The published figure and complete
report link pass at both sizes. The connected Browser skill uses task-owned
tabs; those tabs are closed, viewport overrides reset and user tabs preserved.

Visual review caught a checker paint race: a loaded SVG could precede its visible
paint. The checker now requires the actual flight SVG URL, successful decode and
two animation frames. A separate desktop/phone preview pass verifies and visually
inspects all six visible pilot images after painting. Earlier checker artifacts
are retained; report bytes and shared templates were not changed for this fix.

All 1,469 rich report files, 366 complete native capture URLs and four publication
assets pass visible-hostname HTTP byte verification: 2,732,951,045 bytes in total.
Publication reuses the existing shared report server. Earlier report/capture
paths, selectors and both report-service lifecycles remain preserved.

The final closure receipt binds resource limits, heads, audit/analysis/delivery
hashes, exact foreign-file preservation and local commits. No default promotion,
push or new learning campaign is performed.
