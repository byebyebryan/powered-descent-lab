# Independent checkpoint policy example

This application uses only the shared platform crates. Its all-features normal
dependency tree contains no maintained controller or planner. The JSON checkpoint
contains hand-written demo weights. It demonstrates integration, not a trained
policy or a landing-performance improvement.

The twelve physical features are, in order: target horizontal offset, height above
target, horizontal/vertical velocity, attitude, angular rate, fuel, mass, hull
clearance, remaining scenario time, terrain height ahead relative to current
position, and target pad width. Checkpoint scales and clipping are explicit.
The terrain lookahead stays inside the resolved heightfield domain. No scenario
identity, seed, tag or collection role enters the feature vector.

Two linear outputs map through a logistic throttle and a bounded tanh target
attitude. The core's command semantics apply: zero throttle turns the engine off;
a positive command maps into the vehicle's declared minimum-to-maximum thrust
range. Rotation, fuel, contact and mission progress remain plant-owned. There
is no reward, training optimizer, recovery planner or maintained-controller
fallback inside this example.

```sh
rtk proxy cargo run --release -p pd-policy-example -- evaluate \
  --collection outputs/my-missions \
  --checkpoint examples/checkpoint-policy/checkpoints/linear-demo-v1.json \
  --output outputs/my-policy-evaluation --repeat-index 0 --repeat-index 7
rtk proxy cargo run --release -p pd-policy-example -- replay outputs/my-policy-evaluation/attempt-000000
rtk proxy cargo run --release -p pd-policy-example -- report outputs/my-policy-evaluation/attempt-000000 --output outputs/my-policy-report.html
rtk proxy cargo run --release -p pd-policy-example -- benchmark \
  --collection outputs/my-missions \
  --checkpoint examples/checkpoint-policy/checkpoints/linear-demo-v1.json \
  --output outputs/my-policy-throughput.json --repeats 4
```

The evaluator saves every scheduled attempt and renders each executed capture
through the maintained rich run template. Reports expose physical/mission
outcomes, integrity, complete replay, typed stops and checkpoint configuration.
Outputs are create-only. Replay is read-only and never instantiates the policy.

The benchmark uses the first eight resolved missions, four repeats each, in
summary and captured modes. It compares complete physical summaries, counts
actual physics steps and agent updates, and separately records serialization,
eight full integrity/replay assessments and process peak RSS where available.
Summary mode stores no trace vectors; the benchmark retains only physical
summaries as its comparison oracle. This is an API/cost check, not training or
a landing campaign. All inputs used here are consumed development data.
