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
