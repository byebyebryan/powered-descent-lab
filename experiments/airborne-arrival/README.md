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

`composition_inputs.py ALLOCATION.json` generates the separately frozen
development or confirmation inputs and two native-witness endpoint goals.
Its source/anchor/library hashes and all construction work are recorded; no
failed proposal is replaced. Keep its source snapshot separate from a compiled
runtime epoch when only the input generator is added later.

`audit_composition.py ROOT --receipt NEW.json` checks every selected native
proof, old/new first-resolution prefixes, fresh/development input separation,
and exact work accounting. `analyze_composition.py ROOT --receipt NEW.json`
computes complete confirmation qualification and conditional hindsight fuel
headroom. Neither reader executes native transitions.

`render_composition.py ROOT NEW_OUTPUT` uses the maintained rich tree, consuming
`analysis.json`, `evidence-audit.json` and the checked `developer-gate-final.json`.
It uses the same isolated Matplotlib presentation environment described above.
`check-composition-report.mjs ROOT NEW_QA_DIR EXACT_URL` checks both viewports,
every occurrence, visible lazy previews, comparison controls, plot modes and
detail/tree navigation in an explicitly owned disposable Chrome. These tools
do not refresh old reports or start/stop report servers.

The [timing and goal-selection study](../../docs/arrival_timing_selection_plan.md)
adds `run_timing_study.py ALLOCATION.json` and `timing_study.py`. The optional
native `timing_selection="minimum_fuel_grid_v1"` enumerates the same finite
coast/duration bank and ranks native target-contact forecasts by fuel. Omitted
selection retains the original first-feasible behavior and feedback. Complete
Summary/capture equality, controller/goal reconstruction and replay establish
selected flight evidence; a forecast alone does not. `test_timing_study.py`
checks both default preservation and full new-selector execution proof.

`analyze_timing_study.py ROOT --receipt NEW.json` reports the crossed comparison
and declared learning-release decision without native transitions.
`run_timing_proof.py ALLOCATION.json` completes an explicitly declared existing
Summary roster's proofs; it does not select new inputs. `audit_timing_study.py`
independently checks source/input/seal bindings, full retained-default equality,
arrival predicates/prefixes, timing selection and exact measured accounting.
`render_timing_study.py ROOT NEW_OUTPUT` uses an isolated Matplotlib environment
and the maintained native renderer. Put its output directly below `outputs/`
when both old and new capture roots need relative raw-evidence links; the
renderer requires captures beneath the output's parent. No shared template or
historical capture needs modification.
