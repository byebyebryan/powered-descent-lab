# Bounded queued-flight recovery experiment

[Documentation home](README.md) · [797/1000 reference](ballistic_phase_transition_results.md)

This authorized pass isolates recovery timing and selection. It does not change
destination acquisition, waypoint placement or landing control. The preceding
read-only investigation found genuinely overlapping earlier warning/alternative
evidence in 84 of 96 recovery failures. Those native short queries are not
landing promises. In the narrower 0.4–0.7-second window, clipping warnings to the
alternatives' measured horizon leaves 65 cases, rather than the earlier 80.

## Fixed mechanism and boundaries

- Compare the actual queued turn/burn/coast program and the same four existing
  full-throttle recovery choices over one common response horizon. Clip the
  queued diagnostic to its actual paired commands through that horizon, including
  known turn/burn/coast switches; retain the ordinary live 24-tick guard without
  alteration. Do not forecast stale powered commands past their known cutoff.
- On the existing 24-tick planning cadence, interrupt a warned queued program
  only when an existing response passes. A warning without a passing response
  does not itself stop an otherwise clear live prefix.
- Select the first passing existing choice at the common horizon, including
  during recovery. Keep the existing goal, choice order, tilt, 600-tick episode
  limit and 240-tick maximum lookahead. No new commands or terrain-aware arc search.
- Resume only when a correction or accepted coast exists and its queued native
  prefix passes. Do not resume merely because an empty proposal's idle command
  is briefly clear. Replan from the unchanged actual state toward the same goal.
- Preserve initial construction, 32 profiles, four local proposals, physical
  reserves/contact, clocks, fuel, deadline, cap 24, terminal configuration,
  default policy 3 and the standalone coast-to-terminal branch.

## Frozen evaluation

Reference: `capture-ballistic-phase-transition-full-20261009-v2`, receipt
`d3c4571f9e0b6ba54e83c7339b51560c532aaac8a2176e1a532e03bca9766833`, results
`e2f3f8b67b150d897a1728926219f063feb05ee6817adf345cc6711e739ffd9d`.

Use the existing 29 transition subjects, plus recovery subjects 016, 099, 139,
218, 330, 433, 446, 739 and regression 407, and successful recovery controls 070,
114, 116, 124, 138, 164, 423, 591, 944 and 977: 48 unique primary worlds.
Repeat 020, 407 and 114 separately. The bounded-comparison probe must reproduce
every complete ordinary reference flight. Run the active candidate on the same
48 worlds and three repeats. Preserve all direct control landings; report any
changed ordinary flights explicitly.

Then run the full original 1000 worlds and repeats 020, 407, 114, 715 and 349,
regardless of honest focused regressions. Four workers, 60 seconds per attempt,
two-hour stage collection bound, create-only captures and no retries. Total:
1,107 declared attempts. Freeze source/native/renderer/reader/protocol before
the probe. No outcome-driven tuning or source changes between stages. A numerical,
integrity or collection failure stops that stage for investigation and remains
retained, never rewritten as a valid result.

## Review and acceptance

Verify original input bytes, complete physical/mission/integrity/source-replay
tuples, deterministic decisions, exact repeats, focused/full overlap, bounded
warning clocks, selected-command bindings, unchanged initial arcs and protected
captures/navigation. Preserve the rich common batch/detail templates. Run native
unit tests, terrain-tool tests and the maintained gate with explicit retained
44-case parity. Record gains, losses, all remaining stop groups and 407's outcome.

A positive total is development evidence, not held-out reliability or default
replacement acceptance. No commit, push, accepted-site/root publication, external
report refresh, server lifecycle change or edits to the three existing navigation
files are authorized by this pass. The destination-gate question remains separate.
