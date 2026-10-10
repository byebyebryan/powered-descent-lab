# Waypoint entry effort and recovery experiment

[Documentation home](README.md) · [Ridge-waypoint baseline](ballistic_ridge_waypoint_results.md)

Status: complete; see [measured results](ballistic_waypoint_entry_results.md).
All 53 planned invocations are retained. The combined panel is mixed, not adopted.
The preflight version of this protocol is pinned in each capture. This is not
an unattended goal or permission to commit, push or publish accepted reports.

## Two mechanisms, no apex constraint

- Effort: compare the existing 32 bounded waypoint profiles by estimated added
  thrust impulse, then predicted entry speed and stable arrival time. Destination
  construction stays byte-numerically equivalent. Score the finite burn followed
  by its unpowered continuation, not an instantaneous desired departure velocity.
- Recovery: include the upcoming forward x crossing in the same 32-trial budget;
  waypoint construction does not require the descending-height time floor.
  An unsafe coast now can keep a refreshed ongoing correction if its estimated
  cutoff coast and two-second continuation retain the physical body reserve.
  This geometric estimate does not certify the powered phase: ordinary command,
  attitude-slew, body/contact and bounded terrain-recovery checks still govern it.
- Combined: enable both. Ridge remains an explicit same-source control and the
  default of the experimental CLI; maintained policy 3 is unchanged. No hard
  apex/zero-velocity rule, landing suffix, safety relaxation or new recovery family.

Extend the common rich report with the estimated cutoff coast and predicted
waypoint entry velocity/remaining impulse. Predictions are not flown paths.
Keep all existing panels, report trees, source replay and create-only outputs.

## Frozen measurements

1. Unit tests cover effort selection, rising forward reacquisition, safe existing
   overshoot, unchanged destination construction, pending versus current coast,
   deterministic finite bounds and immutable query states. Run relevant report
   and collector tests before release flights.
2. Build one isolated release source/binary. Run ridge, effort, recovery and
   combined on original 349, 006, 807 and 928 plus the three existing direct
   flat/uphill/downhill controls. Repeat 349 per mode: 8 records per mode, 32 total.
   Same-source ridge ordinary flights must match the saved V5 baseline exactly;
   all direct controls must match it in every mode. Keep every failed attempt.
3. Run the unchanged combined source on the prior 16 random worlds, three direct
   controls and repeats of 349/807: 21 records, 19 primary missions. The total
   allowance is 53 native invocations, zero retries or flight-driven tuning.
   Inspect all gains/losses; this selected population is not held-out coverage.
4. Require exact command replay, deterministic native decisions, complete external
   repeats and authenticated scenario/source/binary/old-capture receipts. Run
   the maintained development gate, study/report tests and retained 44-case
   default parity against the explicit October 7 numerical baseline.
5. Reconcile the mechanism and preservation verdict before any 1k campaign.
   No full sweep, default promotion, report-navigation publication or server
   operations in this pass. A mixed/negative outcome is a valid stopping point.

The random preservation IDs are 807, 928, 484, 034, 055, 715, 000, 001, 006,
084, 114, 139, 142, 268, 308 and 349. The direct controls are `v2_clear_845`,
`fresh_clear_uphill_805` and `fresh_clear_downhill_805`. Frozen terrain is copied
from the receipt-bound V5 ridge panel; no snapshot is used to start a flight.
