# Airborne arrival characterization

Opt-in native characterization of a single optional waypoint arrival followed
by a named deterministic Poly Landing selector. No learner or optimizer is
included. See the [design](../../docs/airborne_arrival_plan.md) and
[allocation/results](../../docs/airborne_arrival_execution.md).

Use the pinned Python environment in `experiments/coast-ignite/.venv`. Build
`pd-coast-ignite-example` in a retained source epoch, expose its cdylib as
`pd_coast_native.so` on `PYTHONPATH`, then run `test_arrival.py`. The build ledger
binds these Python scripts and all native execution/report source files.

`run_arrival.py ALLOCATION.json` consumes a prewritten create-only allocation
with original input paths, fixed choice lists, captures and resource limits.
It retains every Summary result and selected full captures. Captures require
complete physical replay, controller reconstruction, regeneration of the
input-defined goal, and exact Summary/Captured equality. Faults get receipts
and stop the run. Native plant steps, local model steps and selection forecasts
are accounted separately; algebraic actuation scans are exposed in feedback.

`render_arrival.py OUTPUT RESULT.json [...]` projects sealed captures through
the maintained rich tree/detail templates and adds choice comparisons. The
output is new; existing reports and server lifecycle are not modified.
