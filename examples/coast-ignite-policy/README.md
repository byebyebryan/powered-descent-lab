# Coast and ignition composition

This optional approach owns one ordinary native Episode and a controller-owned
coast/landing mode. Python chooses wait or ignite; it never emits physical
commands. `pd-control` owns coast hold and the checked Poly Landing backend.
The neutral `pd-python` bridge and accepted Policy 3 are unchanged.

See the [plan](../../docs/rl_coast_ignite_plan.md),
[execution allocation](../../docs/rl_coast_ignite_execution.md), and
[Python approach](../../experiments/coast-ignite/README.md).

The build ledger binds native dependency sources, approach code and Python
entrypoints. Captured execution checks those bytes against the compiled ledger.
Rebuild after editing any participating source. The Python environment is
optional for the ordinary workspace tests; PyO3 uses the normal linkable Python.

```sh
rtk proxy cargo test -p pd-coast-ignite-example
```
