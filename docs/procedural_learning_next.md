# Next procedural learning experiment

[Run record](procedural_learning_results.md) · [Previous execution contract](procedural_learning_plan.md) · [Ownership](architecture.md)

Status: recommendation for the next loop, not a launched allocation. The October
10 run's final comparison remains authoritative in its run record. This proposal
does not reopen its frozen checkpoints, rewards or held-out selection.

## What the completed optimization tells us

Nine allocations completed 72,032,256 PPO transitions on complete procedural
missions. None of the selected development panels or broad confirmations found
a native target landing. The original distance reward favored fast crashes;
the braking revision mostly learned horizon stops. Correcting discounted
terminal potential did not produce a landing within the declared allocation.

This bounds the tested combination of exploration, reward, control mapping,
network and mission distribution. It does not establish that RL cannot solve
the task. Another long uniform run of the same setup needs a new hypothesis.
Horizontal offset is also a weak secondary selection metric: an agent can be
near the pad horizontally while remaining above it until timeout.

## Proposed work order

1. Establish a learning diagnostic before another large allocation. Inspect
   reward components, actual commanded throttle/attitude, touchdown separation,
   speed, attitude, fuel and native stops on a small declared development panel.
   Terminal and simple-transfer synthetic tasks can identify missing physical
   control. They neither replace the complete-mission benchmark nor require
   authored waypoint phases.
2. Test a sampler curriculum over complete procedural missions. Begin with
   shorter transfers and lower relief in both terrain families, then mix in and
   advance toward the original envelope. Preserve rest-on-source-pad starts,
   target-pad contact, native fuel/horizon and both travel directions throughout.
   Recipes, stage populations, mixing weights and advancement rules must be
   declared before their outcome evaluations. Retain every structural rejection;
   do not admit missions because a controller happens to solve them.
3. Keep the corrected v3 terminal-potential accounting as the objective baseline.
   Compare the curriculum against a uniform sampler with the same approach and
   equal bounded optimization budgets. Development advancement may depend on
   aggregate native success across families/directions; checkpoint rewards or
   horizontal progress alone cannot advance the curriculum.
4. If the diagnostic shows that direct throttle/attitude exploration remains the
   limiting issue, consider one separately declared control parameterization.
   A general learned setpoint policy or a small independently owned feedback
   adapter is a candidate, not a current implementation. Compare it while holding
   sampling and objective fixed. Any deterministic control must be documented
   as part of that approach, with no incumbent planner/controller fallback.
5. Freeze the chosen design and all seed checkpoints, then run the unchanged
   common benchmark on new seed-disjoint held-out collections. Report each seed,
   both families, directions and relief strata, along with unsupported inputs
   and the complete native landing/integrity/replay tuple.

Do not change sampler, objective and control mapping together in the first
comparison. Reserve final evaluation time and cap the number of revisions.
Choose exact stage recipes and transition/wall budgets from a timing and
exploration pilot before launching the next contract; this recommendation does
not silently allocate another day of training.

## Ownership and acceptance

`pd-missions` owns versioned physical recipes, resolved inputs and collection
management. The experiment owns curriculum scheduling and exposure bookkeeping.
`pd-run` and `pd-python` expose ordinary physical execution; learning reward and
curriculum rules stay outside them. `pd-eval` and `pd-report` remain usable by
every approach. The incumbent adapter stays an explicitly named comparator.

Maintain output-independent generator, bridge and dependency-boundary checks.
Before accepting a changed learning interface, verify identical native execution
under identical commands and captured/summary agreement. Reward changes require
objective checks; changing features/actions requires explicit versioned contracts.
None can alter native contact, clocks, resource limits or success semantics.

The now-observed October 10 held-out inputs may become a fixed regression panel
for later experiments. Keep their original role and exposure history intact,
and generate fresh held-out seeds for a new generalization claim. Failure
examples may diagnose a population-wide issue; mission IDs, coordinates and
saved failure clocks must not become control rules.

Accept learning competence only from frozen native captures on the full declared
distribution, with independent replay and honest denominators. An easier-stage
success is evidence for that stage. It does not establish broad terrain landing
or justify changing the current controller, default selector or published site.
