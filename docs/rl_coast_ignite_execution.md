# Coast and ignition execution

[Plan](rl_coast_ignite_plan.md) · [Documentation home](README.md)

October 22, 2026. The owner requested a goal loop after reviewing the researched
plan. This allocation implements and conditionally runs that bounded experiment.
It does not reopen direct-control RL or generalize the landing controller.

The create-only run root is
`outputs/coast-ignite/loop-20261022T173645Z`. Its workspace baseline records all
pre-existing tracked/untracked bytes, HEAD and dirty status. Preserve them.
Review and commit owned implementation slices locally; no push or server changes.

## Limits and release rules

One native worker, sequential environments, four hours from goal creation;
32 GiB new artifacts and at least 50 GiB free. A phase's limit is subordinate
to the remaining outer allocation. Software work has a 60-minute target.

| Phase | Initial ceiling | Release |
| --- | --- | --- |
| Software | Native unit/parity/proof and Python update tests | Contracts and maintained gate pass |
| Cost pilot | 8 development arcs × 8 schedules; 5 min, 1.5 million native steps, 1 GiB | Measure original/proof/storage cost before later phase limits are frozen |
| Timing map | 12 arcs × 82 schedules; proposed 10 min, 5 million steps, 256 MiB | Native witness results, decision resolution and useful timing windows reviewed |
| Readiness | 256 sampled episodes, then one PPO update; proposed 10 min, 2 million steps, 256 MiB | Positive outcomes on at least two arcs, intact exploration and finite noncollapsed update |
| Learning | 3 seeds × 131,072 decisions; 30 min and 50 million native steps per seed | Fixed validation/checkpoint schedule; source and task freeze |
| Confirmation/report | Proposed 50 min, 60 million steps, 24 GiB captures + 4 GiB presentation | Selected validation packages verified before releasing 600 fresh arcs once |

Freeze exact later phase limits in create-only JSON receipts after the cost
pilot, before execution. Count original physics, admission work, neutral replay,
controller/model reconstruction, optimizer updates and all repeated evaluations.
Faults stop execution; ordinary native misses remain valid measured outcomes.
Failed readiness closes this allocation without training or fresh-data release.

## Progress

- Goal opened at main `b0bfb65`; no source or native campaign changed beforehand.
- Implementing controller-owned coast, same-tick ignition and a separate approach
  binding. Accepted reference/actuation arithmetic and neutral crates stay intact.

Before the first flight, the cost-pilot ceiling was set from the worst-case
64 original episodes plus three neutral replays and two reconstructions each:
1.5 million native steps and 1 GiB, within the unchanged outer limits.

## Software acceptance

The maintained gate passed all 19 checks with `RUST_TEST_THREADS=1`. An initial
parallel run hit the existing report-navigation missing-pinned assertion; that
test passed independently and in the complete serial run. No report-navigation
source was changed. Native coast integration has three geometry/continuity/
rejection tests, and the locked Python approach has four hazard/returns/model/
proof tests. Epoch B also passed all four Python tests against its own bridge.

The accepted reference, paired actuation, checked landing controller and neutral
Python bridge remain byte-identical to the goal baseline. Epoch A was a software
fixture snapshot only. The measured release uses `source-epoch-b/` (329 copied
source/fixture files) and `execution-allocation-v2.json`. The final preflight
changes added compact bound report evidence and byte accounting over the whole
run root; full rich flight views and access to the sealed captures remain.
