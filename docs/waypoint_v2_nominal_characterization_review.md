# Unified nominal characterization primary review

## Decision

**No-go for runtime replacement; complete the research checkpoint.** The
[measured results](waypoint_v2_nominal_characterization_results.md) establish
useful airborne construction and supplied-command agreement, but only 2/8
clear ground starts pass. The existing uncut 8/8 ground constructor remains the
working control. This is a separate primary contract review, not an independent
agent review or full planner acceptance.

The architecture remains appropriate: construct from actual state without
interior terrain, preserve acceptable motion, realize a bounded proposal, audit
terrain, locally clear a conflict, and replan from actual H. A unified nominal
contract does not require one identical actuator template for grounded and
airborne starts. It does require shared target/entry/result semantics, finite
acquisition continuity and mode-specific support constraints.

## Contracts supported by this pass

- Reproducing all 39 airborne states through complete ordinary source prefixes
  gives exact state agreement. No snapshot restoration, new fuel, clock reset or
  grounded source exception is introduced at H.
- Constant discrete acquisition with gravity, turning and paired-throttle fuel
  realization agrees closely with the plant in the witnessed family. The
  existing affine-bridge mathematics also agrees for the constant segment.
- Zero acquisition and natural vertical profiles are first-class choices.
  No future apex is demanded for an already acceptable descending state.
  Vertical preservation wins ranking before additional upward impulse and cost.
- Geometry and combined landing demand matter together. A measured entry angle
  alone is not an energy/attitude/fuel/deadline certificate. Actual powered
  arrival and virtual ballistic intercept are different clocks.
- The four former NoNominal handoffs have actual supplied-command recoveries
  on their real terrain. The old finite miss is not physical impossibility.
- Terrain does not affect seed/rank choice. Real terrain still blocks some
  first-ranked witnesses; those are legitimate future local-clearing demands,
  not evidence that nominal selection should loft until obstacles disappear.
- Every materialized witness passes independent command/endpoint replay and
  raw ordinary/neutral consumed-prefix contact parity. The two accepted artifacts
  are byte-identical. Finite rejection and corrupted evidence remain distinct.

## Contracts not settled for runtime

The ground adapter and finite acquisition family are not adequate for the eight
clear controls. Neither a global entry envelope nor general estimator margins
are calibrated by 133 accepted witnesses. Agreement on accepted inputs does not
prove completeness of the screen or its 0.925 conservative bound. The steep
synthetic's shaping cannot be generalized into a rule that all steep states need
correction, or used as proof that no preserving maneuver exists.

The time/fraction seeds, four entry samples and constant vector acquisition are
research choices. Their boundedness is established; their efficiency and coverage
in a complete loop are not. Terminal construction still invokes the existing
physical backend. No timing claim follows from having fewer analytical seeds.

The original complete 32-case gate, new-policy terrain twins, repeated actual
handoff loop and historical whole-matrix preservation would still be required
after implementation approval. None is replaced by the 39 old-state suffixes
or by choosing any terrain-successful candidate from the research shortlist.
Local acceptance remains clearance, progress and the separate two-second guard,
not direct landing or reaching a terrain feature's far edge.

Do not integrate the studied family as policy 3, relax margins to convert these
misses into passes, replay the old ground generator behind a supposedly unified
interface, revise terrain/denominators, or reopen source contact without new
physical evidence. Preserve old policies and all local behavior.

## Proposed next bounded ground diagnostic

This proposal was subsequently executed under the revised plan. The
[2026-10-02 results](waypoint_v2_ground_diagnostic_results.md) and
[separate primary review](waypoint_v2_ground_diagnostic_review.md) supersede its
next-action status: coupled-state terminal timing is the supported next question;
ground acquisition remains unresolved and runtime replacement remains no-go.
The proposal below records the scope of that completed diagnostic.

The next question is narrower than another search calibration: **does the entry
screen reject proven ground entries, or does this ground constructor fail to
reach acceptable entries?** This phase is proposed, not executed here.

The revised [diagnostic plan](waypoint_v2_ground_diagnostic_plan.md), dated
2026-10-02, uses eight working controls, at most sixteen matched references and
at most three preselected shadow trials. The game setting does not require a
universal analytical feasibility envelope before selected candidates can be
checked in the existing simulator. Physical safety checks and honest finite
misses remain mandatory.

1. Reproduce the eight uncut controls through their original full commands.
   Extract actual source and terminal boundaries, retaining clock, fuel and
   attitude; never execute a saved snapshot.
2. At each actual terminal entry, compare the original duration with the
   unchanged research braking-time duration. Separate reference shape, exact
   versus conservative demand, reserve, alignment and forward motion. The
   braking formula is one construction heuristic, not the safe-entry definition.
3. If conservative screening remains a plausible blocker, select one existing
   candidate each for the fixed flat-845, uphill-845 and downhill-845 cases by
   the plan's terrain-blind rules. Retain analytical rejection; bypass only the
   conservative terminal bound for a labeled shadow trial, keeping other checks
   and replay. Report physical landing and actual reserve separately. No seed,
   duration or margin retry is permitted; shadow success is not new coverage.
4. Inspect acquisition histories only if the preceding evidence leaves that
   gap unresolved. Do not reclassify every seed slot or run the former optional
   S-continuation study. Finish with one supported recommendation or an explicit
   unresolved result and a separate primary review, not an automatic fix.

Reuse the research harness rather than require a new CLI/reporting framework.
Shared-helper changes require the unchanged complete characterization; an
isolated diagnostic uses preservation, focused tests and the existing checker.
Later constructor work must retain airborne regressions and reach 8/8 clear
terrain/replay/reserve checks before runtime approval. Full planner acceptance
still requires the unchanged 32-case loop with actual new handoffs, at least
13/16 ordinary landings and the existing family/reference/integrity requirements.
Those are later gates, not deliverables of this diagnostic.

This is worth one focused diagnostic: four previously unexplained states now
have executable recoveries, and the new ground failure is localized before
execution. It is not worth another open-ended timing-grid expansion. If no
small ground acquisition/screen change emerges from the proven controls, retain
the current ground implementation and defer full mathematical unification;
describe that honestly as staged improvement, not a completed unified planner.
