# Retired RL entrypoint

The earlier complete-mission PPO allocation and the subsequent direct-control
series are closed. This directory no longer contains a Python project or
training, evaluation and continuation entrypoints.

The original package is retained byte-for-byte in the
[historical procedural PPO study](../../studies/rl_procedural_ppo/README.md).
Its [original workflow](../../studies/rl_procedural_ppo/README.historical.md.txt)
documents the old commands, 17-value native observation order, features,
rewards, dependency lock and evidence contracts. Those commands refer to the
former layout; restore that layout in an isolated recovery checkout for exact
historical reproduction.

The existing ignored `.venv` stays at its original path. No environment or
native installation was rebuilt or removed by retirement. The
[retirement record](../../docs/rl_code_retirement_results.md) identifies source
recovery, verification readers and the separately retained RL worktree.

Use [development](../../docs/development.md#adding-an-approach) for current
Agent/Episode integration and the [controller implementation plan](../../docs/maneuver_controller_implementation_plan.md)
for the proposed next software pass. New research needs its own approach and
allocation rather than restarting this closed package.
