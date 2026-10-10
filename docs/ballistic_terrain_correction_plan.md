# Ballistic terrain-aware correction pass

This is the opt-in follow-up to [bounded replanning](ballistic_replan_results.md),
not a policy-3/default replacement. The user approved implementation and a goal
loop on October 8, 2026. Old scenarios, captures and accepted reports stay intact.
The [completed results](ballistic_terrain_correction_results.md) record 401/1000,
all 385 previous successful flights preserved, and explicit append-only collector
recovery/reader repair. This is a completed protocol, not a new flight allowance.

## Contract

Keep the next destination/waypoint and its terrain-blind ballistic aim separate
from command-level clearance protection. A geometrically blocked arc can select
or replace a waypoint. An unsafe acquisition command cannot do so by itself.
The controller must first attempt bounded clearance recovery toward the same goal,
then rebuild the correction from the actual resulting state. This is feedback
flight, not constant route search, terrain-following or terrain-aware apex search.

The immediate 24-tick command guard stays mandatory. In a pre-terminal,
source-cleared flight only, a rejected command opens a bounded recovery episode.
Try four full-thrust attitude choices, in order: the requested correction angle,
the current angle, upright, and 30-degree braking tilt opposite horizontal motion.
Duplicate commands are skipped. Each choice is propagated through the ordinary
plant, including attitude slew, gravity, mass/fuel and contact. Its prediction
must retain the existing reserve for the estimated turn time plus 24 ticks,
capped at the existing 240-tick continuation horizon. This is a local command
check, not landing feasibility or a whole-leg safety certificate.

During recovery, keep the exact goal/revision. Reconsider commands at the
existing two-tick command cadence. Resume ordinary acquisition when a fresh
same-goal command passes the response-horizon check. Invalidate old burn/arrival
clocks after overrides. Cap each episode at 600 physics ticks (five seconds),
with the original mission/fuel/wall-time bounds also unchanged. No safe command
or exhausted recovery is a finite planner/controller stop, not physical
unrecoverability. Source upright clearance, terminal control, actual reserve,
waypoint continuation and handoff admission are unchanged.

An arc-valid waypoint proposal need not have an immediately safe acquisition
command: the same mandatory command guard and recovery controller protect it
before any physical execution. Accepted proposals, recovery episodes and actual
handoffs remain distinct. Saved decisions expose rejected/selected commands and
prediction horizons in the maintained rich report, with complete source replay.

## Bounded implementation and validation

1. Implement the command predictor/controller and separate short-command danger
   from waypoint replacement. Unit-test actual-state immutability, deterministic
   choice/bounds, goal preservation, query-error propagation and ownership limits.
2. Run at most eight development attempts before the focused freeze, retaining
   every attempt. Use recorded 034/055 plus direct and waypoint successes; do not
   change fixture geometry, constructor profiles, thresholds or waypoint recipes
   in response to their outcomes. Repairs to integrity/report/implementation bugs
   are distinct from behavior tuning and must be recorded.
3. Freeze a focused panel: 034/041/066/055, earlier finite 032/030/020/000/715,
   successful 001/006/142/084/268/308/349, and the three original flat/uphill/downhill
   controls. Repeat 034/055 independently. Native attempts also reproduce every
   decision and replay the full original command ledger.
4. If collection/integrity is sound, freeze one paired 1k diagnostic (plus the
   same two independent repeats), comparing with 385/1000 and separately the
   older 817/1000 experiment. No sweep-driven tuning or default promotion.
   A negative/regressing result is a valid verdict; retain it honestly.
5. Run the maintained development gate, explicit retained 44-case default parity,
   study/report tests and saved-capture verification. Reconcile current docs and
   report decision navigation. No accepted-site publication or server operations.

The larger controller/testing-pack acceptance campaign remains a prerequisite
for a usable replacement claim, not something inferred from this development
sweep. No commits, pushes or delegation are authorized by this pass.
