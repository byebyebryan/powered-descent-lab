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

The [actuation calibration pass](../../docs/arrival_actuation_plan.md) adds an
explicit `native_pair_v2` profile and independent native witness recipes.
`run_actuation.py ALLOCATION.json` compares those profiles on frozen inputs;
`render_actuation.py OUTPUT RESULT.json [...]` uses the same maintained rich
tree. `test_actuation_study.py` checks native calibration and complete witness/
controller proof. The ordinary constructor and original study scripts retain
the legacy allocation and wire format.

After execution, `audit_actuation.py ROOT --receipt NEW.json` independently
checks source/input hashes, the selected native proofs, measured work and the
fly-through outcome/tick from native observations. It also checks exact legacy
and direct-control fingerprints. `analyze_actuation.py ROOT --receipt NEW.json`
separates populations, transitions and allocation diagnostics without executing
new flights. Both readers use the same `PYTHONPATH` as the campaign.

The actuation renderer consumes a completed `analysis.json` beside its new
output directory. It requires Matplotlib for the standalone calibration figure;
use an isolated presentation environment rather than changing the pinned
simulation environment. Its comparison excludes repeated pilot observations
from population totals while retaining every pilot capture in the rich tree.
`check-actuation-report.mjs ROOT NEW_QA_DIR EXACT_URL` exercises desktop/phone
browsing and rich details against an explicitly owned disposable Chrome with
its profile and PID recorded under `ROOT/browser-qa/`. It never starts a report
server or selects the user's browser.

The [arrival composition pass](../../docs/arrival_composition_plan.md) uses
`run_composition.py ALLOCATION.json`. Witness programs may explicitly request
landing; their unchanged command prefix ends at first arrival resolution and
the existing Poly selector starts at that retained tick. `composition_study.py`
checks old/new native observations through the handoff and commands before it,
alongside complete reconstruction and replay proof. Original witness-only
programs still continue their unchanged observation tail.
