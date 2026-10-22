# Retired procedural PPO experiment

[Original workflow](README.historical.md.txt) · [Original results](../../docs/procedural_learning_results.md) · [Cleanup record](../../docs/rl_code_retirement_results.md)

The earlier complete-mission PPO allocation is closed. Its 17 Python modules,
dependency manifest/lock, ignore rules and original README are retained here
byte-for-byte. `retirement.json` records their original paths and hashes.
Training, feature/reward revisions and continuation commands belong to that
historical allocation; this package is not a current approach template.

The original README is stored as `README.historical.md.txt`. It is raw source text, including links to the separately retained research
records. Its commands describe
the old `experiments/rl` layout and are historical text. Restore that layout in
an isolated recovery checkout when reproducing its exact source identities;
do not repoint an existing run or installed environment at this archive.
The old `experiments/rl/.venv` is retained in place.

The checkpoint-free campaign verifier remains in `verify.py`; it imports the
native bridge but no learner. Readers, exposure checks, checkpoint formats and
the full legacy implementation remain available alongside it. Native replay
does not reconstruct a policy or recover a previously retired flight recording.

`research-source-inventory.json` inventories the separate RL worktree's source,
including all 28 study families and its terminal-input export. Those files stay
at their original paths because later readers and source bindings depend on
them. Their allocation sequence is historical; E24's unfinished evaluation stays
incomplete. Existing models, inputs, runtimes, receipts, captures and reports
remain in place.

Validate the checked-in archive without local environments or saved runs:

```sh
rtk proxy node scripts/check-rl-retirement.mjs
```

An explicit read-only check also compares the isolated worktree against its
recorded source inventory:

```sh
rtk proxy node scripts/check-rl-retirement.mjs \
  --research-worktree /home/bryan/code/powered-descent-lab-rl-v2
```

New approaches use the neutral Agent/Episode interfaces described in
[development](../../docs/development.md#adding-an-approach). The
[controller implementation plan](../../docs/maneuver_controller_implementation_plan.md)
owns the next proposed software slice.
