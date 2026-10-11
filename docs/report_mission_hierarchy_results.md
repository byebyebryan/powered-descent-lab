# Mission hierarchy presentation correction

[Presentation workflow](report_presentations.md) · [Previous refinement](report_presentation_refinement_results.md) · [Documentation home](README.md)

The owner found that the report refinement changed the tree too much. Mission
structure is more important than controller comparison, in both report trees
and report home/library navigation.

The prior synthetic trees express mission → arrival/route family → condition
set → applicable vehicle/profile/arc/velocity axes → cases. These trees remain
intact. The procedural view had flattened family/direction/stratum into sixteen
concatenated labels and foregrounded controller comparison; its hierarchy now
uses the recorded input axes as separate levels:

```text
Complete procedural missions
  Terrain family
    Travel direction
      Terrain stratum
        Mission and saved-flight preview
          Controller attempts
          Optional paired preview comparison
```

Each mission row displays the saved preview, outcome, endpoint and evidence for
the selected approach. The default is the first declared approach, without
selecting a winner. Family/direction/stratum summaries count matched mission
occurrences; expanded missions preserve every controller attempt. Controller
outcome bars and comparison controls are optional disclosures. Coverage focus,
search, repeated inputs, physical identity pairing and detail return/navigation
retain their original semantics. Mission and coverage identities stay stable.
Single-campaign companions use the same three input levels and the maintained
common tree controls.

The report home leads with terminal control, direct transfers and complete
procedural terrain. Authored routes and planner probes are separate diagnostics.
The library groups discovered reports by mission family, with report editions
inside those groups. Existing topic/type/search/availability/order filters still
compose; recent reports and approach/subject navigation remain available.
Grouping uses known collection/pack paths rather than controller labels or
outcomes; unknown reports remain explicitly separate. The presentation layer
composes with the existing catalogue without changing its discovery or server.

The edition is create-only and uses the same frozen held-out comparison:
2,048 physical missions, 10,240 scheduled attempts, 9,216 executed details and
1,024 unsupported statuses. No agent execution, new physics or new replay is
part of this correction. Saved seals, proof and rich payload bindings remain
required by the renderer. Existing report editions and captures remain retained.

Validation evidence is retained under
`outputs/report-refinement/hierarchy-20261011T021102Z/`.

## Delivered edition and validation

[Report home](http://192.168.1.110:8000/) leads with mission families. The
[corrected full mission edition](http://192.168.1.110:8000/reports/eval/procedural-learning/mission-hierarchy-20261011T022234Z/report.html)
is retained at
`outputs/reports/eval/procedural-learning/mission-hierarchy-20261011T022234Z/`.
The earlier presentation and inspection edition remain available as history.

- All 18 maintained development checks passed. Current report tests pass 47/47;
  focused presentation/library/tree tests pass 12/12. Formatting, strict report
  Clippy, documentation links and diff checks pass.
- `browser-final/browser-check.json` passes at 1440 × 900 and 390 × 844, with
  seventeen screenshots and no ordinary browser errors. Family/direction/stratum
  expansion, visible mission previews, secondary comparisons, exact coverage
  focus, search, detail return/next navigation and campaign companions work.
  Home/library grouping, ordering, phone containment and blocked-Plotly static
  fallback pass. Two additional default overview screenshots were inspected.
- Five warm trials per viewport include two animation frames. Median branch/filter
  times are 51.5/57.2 ms on desktop and 57.0/51.4 ms on phone emulation; this is
  local Chrome evidence, not a cross-device latency guarantee.
- `preservation-final.json` confirms identical campaign projections, stable
  mission/repeat identities, and identical physical, saved-payload and preview
  bindings for all 9,216 executed attempts. All 37,948 original source files retain
  their sizes and modification times. Original outcome denominators stay intact.
- The original navigation/server work remains unstaged and byte-preserved after
  removing the four-line presentation hook from the comparison. Only that owned
  hook and the new mission-navigation component join this correction's commit.
  The existing server remains running; its local evaluator was rebuilt and its
  navigation refreshed without a server lifecycle change.
