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

Implementation, cost release and measured findings will be recorded here.

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
