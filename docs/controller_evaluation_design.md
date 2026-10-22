# Controller evaluation before RL

[Software implementation](maneuver_controller_implementation_results.md) · [Controller contract](maneuver_controller_design.md) · [Evaluation workflow](evaluation.md)

October 21, 2026. **Proposed next pass; no measured allocation is launched.**
The reviewed software checkpoint is `83303da`.
First characterize the unchanged ordinary landing controller, then establish
whether activation timing leaves a useful planner problem. The controller needs
an explicit, measured operating envelope for the intended handoffs. Exact
adapter parity does not establish that envelope.

## Decisions this evaluation must support

1. From which declared airborne states can the frozen controller reliably
   finish a native target landing, with the actual fuel and remaining deadline?
2. Which coupled states and budget limits produce failures, and how do failures
   differ between touchdown, terrain contact, timeout and implementation errors?
3. On the same original episodes, do activation choices change landing, fuel or
   time enough to justify a planner? A controller that always benefits from the
   same simple activation rule does not establish a need for RL.
4. Does that empirical competence transfer to the intended mission-generated
   entry distribution, rather than only a familiar synthetic landing family?

The first task is local landing recovery on the current plant. A later
whole-mission planner also needs real handoff evidence. These are separate claims.

## Freeze the capability and the question

Use the ordinary default terminal backend, predicted terrain clearance enabled,
all experimental flags disabled, and the existing engine-setting/attitude
semantics. Bind its complete resolved configuration and compiled source/executable
identity. Keep the current 120/60/10 Hz physics/controller/planner schedule.
Vehicle geometry, engine, servo, contact limits and Earth gravity retain the
[implemented capability](../examples/maneuver-policy/README.md#supported-inputs).
Valid mass/fuel, entry state, terrain, pads and deadline may vary within a
declared population. Admission remains a semantic check, not a predicted-success
filter. Do not add a readiness gate or tune the controller during characterization.

Write down the intended local task and population before running it. Broad
characterization bounds and the desired operational core have different roles.
Define the desired core from mission requirements and physical input conditions
before its first characterization, and retain its original verdict. Development
results may suggest a revised envelope, but that is an explicit new task version:
retain the wider failures, check that the narrower task still serves the intended
missions, and confirm it independently on fresh inputs after freezing its bounds.
A collection of successful cases selected after a sweep is not an independently
qualified operating envelope.

This is robustness with respect to declared inputs inside the native model.
Wind, sensor noise, actuator failures and different hardware require an explicit
model/capability extension; this pass supplies no real-vehicle certification.

## Evaluation populations

| Population | Contents and role | Acceptance use |
| --- | --- | --- |
| Software regression | Eleven exact adapter cases plus lifecycle, native-clock, integrity and reconstruction fault tests | Preserve implementation and physical behavior; no reliability estimate |
| Retained controller regression | Complete existing clean and projected-error terminal packs, with original reference/core/stress roles and contemporaneous direct/facade execution | Recover existing controller coverage and expose adapter discrepancies; no fresh generalization claim |
| Structured characterization | Deterministic interiors, boundaries and selected coupled corners over the factors below | Map failure mechanisms and observed recovery regions; coverage is reported explicitly |
| Frontier and unsupported diagnostics | Very low fuel, short deadlines, ascending/below-target entries, narrow pads, demanding terrain, plus unsupported mission/hardware inputs | Retain honest outcomes/dispositions separately; no requirement to land every finite state |
| Fresh confirmation | Independently sampled, previously unobserved physical missions from a frozen joint distribution, with frozen controller and activation policy | Estimate landing reliability for that distribution and its declared strata |
| Activation and mission-entry checks | Several declared activation programs on the same airborne episodes; later, separately integrated real mission prefixes | Test decision value and distribution transfer; counterfactual programs are correlated comparisons |

Do not merge these into one success percentage. A stress failure, unsupported
input, missing measurement and core failure are different dispositions.
Every scheduled slot and every exclusion remains in the inventory. Valid core
failures cannot be removed or reassigned after seeing outcomes.

The versioned recipe and resolved-case manifest must bind the following:

| Contract | Required content |
| --- | --- |
| Recipe identity | Schema/version, capability and backend configuration, joint sampler/levels/correlations, transform rules, role/stratum definitions and weights |
| Case identity | Original native scenario and physical SHA256, source-pack/resolver or generator identity, seed as provenance, and parent mission/lineage where applicable |
| Attempt identity | Physical case plus program/configuration hash, declared absolute activation tick, original native horizon and command budget |
| Sampling ledger | Every proposal, input-validation/support disposition and duplicate check; development/fresh/training exposure is explicit |
| Frozen acceptance | Per-role requirements, reliability/confidence and multiplicity rules, fixed final sample count and release conditions |
| Results | Every scheduled attempt, native outcomes/failure and remaining fuel/time, proof dispositions and actual cost; absent evidence stays missing |

Use existing native scenario and physical-identity contracts. Labels describe
evaluation roles; they must not become planner/controller inputs. Do not choose
a case's role or change its budget based on its resulting landing outcome.

The existing clean/full pack uses three payload variants, seven arc points,
three velocity bands and twelve seed perturbations. The projected-error/full
pack adds four undershoot/overshoot conditions. These are structured tests, not
independent random missions. Their ballistic construction couples position and
velocity; most other physical parameters inherit the flat-terminal fixture.
More seed perturbations alone do not cover arbitrary local control.
See the [pack definitions](../fixtures/packs/terminal_bot_lab_full.json),
[projected-error definitions](../fixtures/packs/terminal_traj_err_full.json)
and [current resolver](../pd-eval-cli/src/resolution.rs).

## Cover the coupled state, not only its seed

| Factor | Required variation and interactions |
| --- | --- |
| Clearance and descent | Actual foot/hull clearance, height relative to the target, descending speed and near-contact settling; couple height with descent rate and fuel |
| Lateral recovery | Signed target offset, lateral velocity both toward and away from the target, and projected overshoot/undershoot; couple offset with velocity and usable pad margin |
| Attitude authority | Attitude and angular rate with both signs; couple them with low clearance, lateral braking and the rate-limited servo |
| Vehicle state | Empty/half/full dry masses and independent valid fuel levels, including exhaustion during execution; couple mass, fuel, braking demand and remaining time |
| Time | Original horizon, elapsed native time and remaining deadline; include late activation and partial final command intervals |
| Terrain and pad | Pad width/height and local clearance, both approach directions, approach-side obstacles, overshoot backstops and terrain beyond the pad |
| Invariances | Physically equivalent translations/reflections with appropriately transformed terrain, pads, velocities and attitudes; check names/seeds/tags cannot affect decisions |

Freeze units, numeric ranges, levels, joint correlations, strata, weights,
proposal/rejection rules and case count in a versioned recipe before flights.
Derive candidate bounds from input-only inspection of the retained packs and
the intended airborne mission requirements. A naive full Cartesian product
will contain many extreme combinations; record why each class is core,
characterization or diagnostic before evaluation. Necessary physical checks
are not a controller-success oracle or a general feasibility certificate.

Use designed interaction coverage plus selected higher-order corners where
braking, attitude, terrain and fuel couple. Report which combinations were
covered and which were not. NIST's [combinatorial testing guidance](https://csrc.nist.gov/pubs/sp/800/142/final)
provides methods for managing interaction coverage. Applying it here is a test
design choice; pairwise coverage cannot certify continuous closed-loop dynamics.

Inspect the native physical inputs before flying: invalid geometry, initial
contact, unsupported cadence/hardware and accidental duplicate physical inputs
must be visible in the inventory. Do not reject cases using the controller's
predictions. Mirrors and nearby perturbations are useful diagnostics, but they
are not additional independent samples for statistical confirmation.

## Separate local competence from activation value

For local competence, freeze immediate activation as the initial controller-only
policy. Measure the full landing attempt, including settling, attitude, fuel
and contact. This isolates what the landing implementation can do after a fair,
declared entry; it does not establish that immediate activation is best overall.

For activation value, group comparisons by original physical mission identity.
Run immediate, never, and a small predeclared set of absolute fixed ticks on the
10 Hz decision grid. Select ticks from input-only timing requirements before
examining flight outcomes; fix the number of programs and their total cost.
Each program starts from the same original airborne episode and actually coasts
to its activation boundary. Preserve elapsed time, attitude-servo motion, state,
fuel and the original deadline. A requested tick after native termination simply
does not fire; retain that attempt and its actual physical outcome.

Record the observed successful activation ticks rather than assuming one
monotonic, contiguous safe window. A best-of-grid result is a hindsight upper
bound over the tested programs, not an executable planner or a continuous-time
oracle. Select any fixed rule using development data and freeze it before fresh
confirmation. Repeated programs on one mission do not multiply its independent
sample count.

Before proposing RL, require evidence of both a reliable controller region and
a meaningful decision opportunity: different states favor different actions,
or a declared fuel/time objective leaves useful tradeoffs after landing
requirements are met. Compare against simple deterministic policies using the
same backend, inputs, budgets and information. If one simple policy is adequate,
prefer it until a concrete remaining problem warrants learning.

The current facade requires an airborne, route-free initial scenario and starts
its lifecycle at tick zero. It cannot directly qualify a launch/transfer prefix
from a grounded original mission. That later integration needs an explicit
prefix/command-owner adapter with original clock/fuel/context preservation and
its own parity tests. A newly reset standalone landing scenario derived from a
handoff snapshot can be a local diagnostic, but cannot prove original-episode
handoff behavior or whole-mission success.

## Evidence and statistical acceptance

A scored landing requires native target touchdown, mission success, unchanged
execution evidence and authoritative complete command replay. The composition
also requires valid byte binding and controller/planner reconstruction. Summary
mode may support development/cost exploration; it cannot supply a verified
landing numerator. Final scored attempts retain captures and proof for failures
as well as successes. Report fuel at contact, contact speed/attitude/rate,
terrain clearance, native stop/failure, activation tick and controller cost.

Choose the required reliability and confidence before final evaluation. A useful
working proposal is a one-sided 95% lower confidence bound of at least 99% for
each promised core population, subject to an explicit cost/power decision.
For illustration, with independent Bernoulli mission trials and zero failures,
the exact lower bound is `0.05^(1/N)`; 299 successes give approximately 0.99003.
This is a sample-size illustration, not an allocation or a claim about the
eleven parity cases. Use [exact binomial intervals](https://itl.nist.gov/div898/software/dataplot/refman2/auxillar/exacbino.htm)
for the declared sampling assumptions. Multiple simultaneous stratum claims
need a declared multiplicity treatment. A weighted aggregate alone can hide a
weak stratum.

Fix the final sample count and release rules; do not keep sampling until a
confidence threshold happens to pass. Structured grids, adaptive frontier
searches and repeated policies are not IID trials. Keep development, fresh
confirmation and future RL training inputs separate by recipe lineage and
physical identity, not only filename or seed. Once results are inspected, record
the exposure; a later tuned backend cannot call the same cases fresh again.
No finite grid proves uniform reliability throughout an untested continuous box.

NASA's [Monte Carlo design and requirements-analysis publication](https://ntrs.nasa.gov/citations/20100038453)
emphasizes input distributions, simulation testing, run-count planning and failed
run analysis. The proposed separation of designed coverage, confirmation and
counterfactual activation trials is this project's application of those concerns.

## Concrete next implementation and execution order

1. Add evaluator-owned, output-free access to resolved terminal inputs. Compare
   exact physical identities against the current resolver for clean and
   projected-error families, including original horizons and selectors. Export
   native scenarios plus provenance; the maneuver example must not import
   `pd-eval-cli`, copy its generator, or select a controller while reading inputs.
2. Implement the versioned characterization recipe and manifest: cases, strata,
   roles, program hashes, original scenario identity, native budgets and exposure
   history. Validate the proposed population without flights. Review and freeze
   its numeric bounds and success criteria before execution.
3. Declare a small cost pilot on development inputs. Measure controller and
   proof wall time, retained bytes and native/backend/planner counters; mark
   unavailable internal forecast work as unavailable. Budget actual replay and
   reconstruction passes, not only scheduled flight attempts. The current CLI
   run path performs one original episode plus seven proof/reconstruction passes.
4. Use that cost evidence to declare hard limits for retained-pack parity,
   structured characterization and activation comparisons. Keep all misses;
   make any changed backend a separate version rather than tuning inside a run.
5. Lock the prospectively declared operational core and selected policy for
   confirmation. A revised core needs an explicit new task version; keep the
   original failures. Release a fixed fresh allocation and report strata,
   confidence, failure mechanisms, proof and coverage limits. If it fails, record
   failure or insufficient evidence; do not silently shrink the scored set.
6. Decide the first RL question only after that review. Full mission-entry
   integration remains a separate prerequisite for a whole-mission planner claim.

This document defines the next design and tooling work. It does not authorize
training, a measured flight campaign, report publication, selector changes,
server operations or changes to frozen controller/plant behavior.
