# Native scenario input sets

[Evaluation workflow](evaluation.md) · [Controller execution](controller_evaluation_execution.md)

`pd-missions` provides a strategy-independent native-scenario interchange format.
It carries complete `ScenarioSpec`s and uninterpreted provenance without requiring
procedural recipe membership, a controller registry or a flight outcome.

## Export retained terminal inputs

Use an explicit terminal-matrix pack and a new destination with an existing
parent. For example:

```sh
rtk proxy cargo run --release -p pd-eval-cli -- export-terminal-inputs \
  fixtures/packs/terminal_bot_lab_full.json --output /tmp/new-terminal-input-set
```

This resolves inputs and writes a new input set. It runs no controller, physics
steps, forecasts, cache operations or reports. It does not open lane controller
configuration files. Mixed/nonterminal packs fail explicitly. The ordinary
pack-running commands retain their existing controller configuration behavior.

Physical construction is shared with the existing resolver. The four maintained
clean/projected-error smoke/full packs have pinned pre-extraction fingerprints
for all ordered descriptors and scenarios. Evaluation horizon overrides,
reachability-horizon provenance, original labels, selectors and numeric
parameters are retained. Export is not a new controller acceptance result.

## Artifacts and identity

A completed set contains:

- `request.json`: schema, producer/input provenance and every occurrence ID.
- `input-NNNNNN.json`: full native scenario, original labels/parameters and lineage.
- `manifest.json`: ordered file/byte/physical identities and all physical aliases.
- `completion.json`: exact request/manifest hashes, written last.

Each original source occurrence remains present, including identical physical
inputs under different labels. `physical_identity` binds every native simulation,
world, vehicle, entry and mission input. It excludes display metadata, seed and
generator membership. Physical aliases are not independent statistical trials.

The producer records its exact executable hash, a versioned resolver identity
and exact source pack/base-scenario bytes. That is separate from a build-time
source ledger; it does not claim the source snapshot proves the compiled producer.
Once resolved, the consumer loads the complete inputs without consulting a
controller config or rerunning the source generator.

`load_scenario_inputs` verifies bounded regular files, path components, schema,
completion, ordered identities, native validity, physical hashes and alias groups.
The maximum is 16,384 occurrences, 16 MiB per input, and 256 MiB of scenario-file
bytes; request/manifest files have separate bounded limits. Existing destinations
are never filled or overwritten. Incomplete exports remain incomplete, and a retry
requires a new destination.
