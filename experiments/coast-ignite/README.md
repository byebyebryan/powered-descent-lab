# Coast and ignition experiment

An optional categorical PPO approach over a fixed controller. The policy sees
ten scaled physical/context features and chooses wait or ignite at 10 Hz.
Ignition executes the actual native burn to completion inside the same episode.
There is no cached stopping payoff, duration search or physical action from RL.

[Design](../../docs/rl_coast_ignite_plan.md) and
[allocation](../../docs/rl_coast_ignite_execution.md) own the hypothesis, staged
release rules and limits. Installation or tests do not release a campaign.
The retired `experiments/rl` environment stays independent.

The subsequent [stopping study](../../docs/rl_coast_ignite_stopping_plan.md) uses
the existing frozen checkpoints with zero training. Its
[execution record](../../docs/rl_coast_ignite_stopping_execution.md) owns the
allocation and outcome. `run_stopping.py` requires a declared root and uses
create-only outputs; explicit post-validation continuation retains prior work
and is rejected after fresh release. `render_stopping.py` creates separate
established trees for populations with different policy sets, with the primary
comparison first. Neither command changes the original experiment or defaults.

```sh
rtk proxy uv sync --project experiments/coast-ignite --locked
rtk proxy uv run --project experiments/coast-ignite maturin develop --release \
  --manifest-path examples/coast-ignite-policy/Cargo.toml
rtk proxy uv run --project experiments/coast-ignite python -m unittest discover \
  -s experiments/coast-ignite -p 'test_*.py'
```

`run_loop.py` requires an explicitly new output directory and absolute outer
deadline. It runs a create-only cost pilot, timing map and readiness before
conditionally training three seeds and releasing fresh evaluation. Never rerun
it over existing outputs. Frozen captures need the compiled source epoch,
native artifact, exact runtime lock and selected checkpoint for reconstruction.
The report uses the maintained mission tree and complete rich flight template.

The completed allocation's final edition is a display-only projection of its
sealed comparison, retained miss diagnostics and resource-accounting supplement:

```sh
rtk proxy python experiments/coast-ignite/render_result.py \
  --learning outputs/coast-ignite/loop-20261022T173645Z/campaign-c/learning \
  --output outputs/coast-ignite/loop-20261022T173645Z/campaign-c/learning/NEW_EDITION
```

Choose a new destination. This command groups the four retained attempts under
each physical arc and adds result tables/curves without simulating or changing
the shared tree browser. It requires this study's retained inputs and receipts;
the ordinary native/Python tests do not.
