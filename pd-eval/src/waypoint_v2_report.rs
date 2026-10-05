//! Presentation-only projection and create-only rendering of retained V2 evidence.
//! This module never generates commands, advances a simulator or chooses a route.

use anyhow::{Context, Result, ensure};
use pd_core::{
    EvaluationGoal, MissionOutcome, PhysicalOutcome, ScenarioSpec, SimulationStateSnapshotV1, Vec2,
};
use pd_plan::waypoint_v2::{WaypointV2Policy, WaypointV2Stop};
use pd_report::waypoint_v2::data::*;

use crate::{
    WaypointV2FlightResult,
    waypoint_v2::{WaypointV2CycleDecision, WaypointV2SegmentKind},
};

/// Add executed handoffs to the existing rich report without replacing its
/// telemetry, reference overlays, plots, or statistics. The projection validates
/// that the annotations agree with the recorded flight and its exact clocks.
pub fn render_rich_flight(
    scenario: &ScenarioSpec,
    result: &WaypointV2FlightResult,
    navigation: pd_report::flight_annotations::AnnotationNavigation,
    caption: String,
) -> Result<String> {
    let projected = project_flight(scenario, result)?;
    let mut annotations = executed_annotations(result, &projected)?;
    annotations.navigation = navigation;
    annotations.caption = caption;
    let ordinary = result.ordinary_flight.as_ref().context("missing flight")?;
    let manifest = result.manifest.as_ref().context("missing manifest")?;
    pd_report::render_run_report_with_flight_annotations(
        scenario,
        None,
        manifest,
        &ordinary.events,
        &ordinary.samples,
        &[],
        None,
        None,
        None,
        None,
        Some(&annotations),
    )
}

pub(super) fn executed_annotations(
    result: &WaypointV2FlightResult,
    projected: &FlightReport,
) -> Result<pd_report::flight_annotations::FlightAnnotations> {
    use pd_report::flight_annotations::{ExecutedCorrection, FlightAnnotations, FlightBoundary};
    ensure!(
        result.segments.len() == projected.segments.len(),
        "annotation segment mismatch"
    );
    let boundary = |state: &SimulationStateSnapshotV1| FlightBoundary {
        physics_step: state.physics_step,
        sim_time_s: state.sim_time_s,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
        attitude_rad: state.attitude_rad,
        fuel_kg: state.fuel_kg,
    };
    let corrections = result
        .segments
        .iter()
        .zip(&projected.segments)
        .filter_map(|(segment, display)| {
            display
                .correction
                .as_ref()
                .map(|correction| ExecutedCorrection {
                    number: correction.number,
                    entry: boundary(&segment.entry_state),
                    handoff: boundary(&segment.end_state),
                    reason: correction.reason.clone(),
                    after_handoff: correction.after_handoff.clone(),
                })
        })
        .collect::<Vec<_>>();
    ensure!(
        corrections.len() == result.correction_count as usize,
        "annotation correction mismatch"
    );
    Ok(FlightAnnotations {
        corrections,
        ..Default::default()
    })
}

pub fn policy_version(policy: &WaypointV2Policy) -> Result<u32> {
    policy
        .version()
        .map_err(|_| anyhow::anyhow!("unsupported report policy identity"))
}

fn point(state: &SimulationStateSnapshotV1) -> FlightPoint {
    FlightPoint {
        physics_step: state.physics_step,
        time_s: state.sim_time_s,
        position_m: state.position_m,
        velocity_mps: state.velocity_mps,
    }
}

fn check_point(p: &FlightPoint, hz: u32) -> Result<()> {
    ensure!(
        hz > 0
            && [
                p.time_s,
                p.position_m.x,
                p.position_m.y,
                p.velocity_mps.x,
                p.velocity_mps.y
            ]
            .iter()
            .all(|v| v.is_finite()),
        "nonfinite display state or invalid clock"
    );
    ensure!(
        (p.time_s - p.physics_step as f64 / f64::from(hz)).abs() < 1e-8,
        "display state clock mismatch at {}",
        p.physics_step
    );
    Ok(())
}

fn stop_label(stop: WaypointV2Stop) -> &'static str {
    match stop {
        WaypointV2Stop::Landed => "landed",
        WaypointV2Stop::NoNominal => "no feasible nominal trajectory found",
        WaypointV2Stop::NominalRejected => "nominal trajectory rejected",
        WaypointV2Stop::NoClearing => "no clearing maneuver found",
        WaypointV2Stop::CorrectionLimit => "correction limit reached",
        WaypointV2Stop::Deadline => "flight deadline reached",
        WaypointV2Stop::NoProgress => "no forward progress",
        WaypointV2Stop::Unsupported => "unsupported setup",
        WaypointV2Stop::InvalidInput => "invalid input",
        WaypointV2Stop::ImplementationError => "execution or evidence error",
    }
}

pub fn outcome(result: &WaypointV2FlightResult) -> String {
    if result.ordinary_flight.is_none() {
        return "Not simulated".into();
    }
    if result.planning_stop == WaypointV2Stop::Landed {
        return match result.correction_count {
            0 => "Landed directly — no corrections".into(),
            1 => "Landed after one correction".into(),
            n => format!("Landed after {n} corrections"),
        };
    }
    let prefix = if result
        .ordinary_flight
        .as_ref()
        .is_some_and(|f| f.final_state.physics_step == 0)
    {
        "Stopped before departure"
    } else {
        "Stopped in flight"
    };
    format!("{prefix}: {}", stop_label(result.planning_stop))
}

pub fn friendly_title(id: &str) -> String {
    let words = id.strip_prefix("v2_").unwrap_or(id).replace('_', " ");
    let words = words
        .replace("diag ", "Diagnostic: ")
        .replace("clear ", "Clear terrain: ");
    let mut chars = words.chars();
    chars.next().map_or_else(
        || "Waypoint flight".into(),
        |c| c.to_uppercase().collect::<String>() + chars.as_str(),
    )
}

/// Shared by future captures and archived rendering. Exact boundary snapshots are
/// inserted for display only; neither samples nor source evidence are modified.
pub fn project_flight(
    scenario: &ScenarioSpec,
    result: &WaypointV2FlightResult,
) -> Result<FlightReport> {
    let version = policy_version(&result.policy)?;
    ensure!(
        !result.input_identity.trim().is_empty(),
        "missing flight input identity"
    );
    scenario
        .world
        .terrain
        .validate()
        .map_err(anyhow::Error::msg)?;
    let ordinary = result
        .ordinary_flight
        .as_ref()
        .context("no simulated flight to render")?;
    let manifest = result
        .manifest
        .as_ref()
        .context("simulated flight is missing manifest")?;
    ensure!(
        manifest.scenario_id == scenario.id
            && manifest.physics_hz == scenario.sim.physics_hz
            && manifest.controller_hz == scenario.sim.controller_hz,
        "manifest/scenario disagreement"
    );
    let finish = point(&ordinary.final_state);
    check_point(&finish, scenario.sim.physics_hz)?;
    ensure!(
        manifest.physics_steps == finish.physics_step
            && manifest.sim_time_s == finish.time_s
            && result.physical_outcome.as_ref() == Some(&manifest.physical_outcome)
            && result.mission_outcome.as_ref() == Some(&manifest.mission_outcome)
            && ordinary.final_state.physical_outcome == manifest.physical_outcome
            && ordinary.final_state.mission_outcome == manifest.mission_outcome
            && ordinary.final_state.end_reason == manifest.end_reason,
        "final outcome/manifest disagreement"
    );
    let landed = manifest.physical_outcome == PhysicalOutcome::LandedOnTarget
        && manifest.mission_outcome == MissionOutcome::Success;
    ensure!(
        landed == (result.planning_stop == WaypointV2Stop::Landed),
        "landing label contradicts physical/mission outcome"
    );
    let start = FlightPoint {
        physics_step: 0,
        time_s: 0.0,
        position_m: scenario.initial_state.position_m,
        velocity_mps: scenario.initial_state.velocity_mps,
    };
    check_point(&start, scenario.sim.physics_hz)?;
    let samples = ordinary
        .samples
        .iter()
        .map(|s| {
            ensure!(
                s.physics_step == s.observation.physics_step
                    && s.sim_time_s == s.observation.sim_time_s,
                "sample/observation clock mismatch"
            );
            let p = FlightPoint {
                physics_step: s.physics_step,
                time_s: s.sim_time_s,
                position_m: s.observation.position_m,
                velocity_mps: s.observation.velocity_mps,
            };
            check_point(&p, scenario.sim.physics_hz)?;
            ensure!(
                p.physics_step <= finish.physics_step,
                "sample beyond actual endpoint"
            );
            Ok(p)
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        samples.first() == Some(&start) && samples.last() == Some(&finish),
        "missing or contradictory first/final actual sample"
    );
    ensure!(
        samples
            .windows(2)
            .all(|w| w[0].physics_step < w[1].physics_step),
        "samples are not strictly chronological"
    );
    for (i, cycle) in result.cycles.iter().enumerate() {
        ensure!(cycle.cycle_index == i, "nonconsecutive cycle indices");
        check_point(&point(&cycle.current_state), scenario.sim.physics_hz)?;
        if i == 0 {
            ensure!(
                point(&cycle.current_state) == start,
                "first cycle does not start at actual source"
            );
        }
    }
    let clearing_cycles = result
        .cycles
        .iter()
        .filter(|c| c.decision == WaypointV2CycleDecision::LocalCleared)
        .collect::<Vec<_>>();
    let correction_segments = result
        .segments
        .iter()
        .filter(|s| s.kind == WaypointV2SegmentKind::LocalCorrection)
        .count();
    ensure!(
        result.correction_count as usize == correction_segments
            && clearing_cycles.len() == correction_segments,
        "correction count/segment/cycle mismatch"
    );
    ensure!(
        result.segments.is_empty() == (finish.physics_step == 0),
        "missing flown segment evidence or fictitious departure"
    );
    let mut segments = Vec::new();
    let mut correction_index = 0;
    for (i, s) in result.segments.iter().enumerate() {
        let entry = point(&s.entry_state);
        let end = point(&s.end_state);
        check_point(&entry, scenario.sim.physics_hz)?;
        check_point(&end, scenario.sim.physics_hz)?;
        ensure!(
            s.start_physics_step == entry.physics_step
                && s.end_physics_step == end.physics_step
                && s.start_physics_step < s.end_physics_step
                && end.physics_step <= finish.physics_step,
            "invalid segment endpoints"
        );
        if i == 0 {
            ensure!(entry == start, "first segment does not begin at source");
        } else {
            ensure!(
                result.segments[i - 1].end_state == s.entry_state,
                "segment boundary snapshots are discontinuous"
            );
        }
        for boundary in [&entry, &end] {
            if let Some(sample) = samples
                .iter()
                .find(|p| p.physics_step == boundary.physics_step)
            {
                ensure!(
                    sample == boundary,
                    "sample disagrees with exact segment boundary"
                );
            }
        }
        let correction = if s.kind == WaypointV2SegmentKind::LocalCorrection {
            let cycle = clearing_cycles[correction_index];
            correction_index += 1;
            let search = cycle
                .local_search
                .as_ref()
                .context("clearing cycle missing local search")?;
            let selected = search
                .selected
                .as_ref()
                .context("clearing cycle missing selected proposal")?;
            ensure!(
                selected.identity == s.proposal_identity
                    && selected.entry_state == s.entry_state
                    && selected.handoff_state == s.end_state,
                "handoff ownership/proposal mismatch"
            );
            ensure!(
                search.handoff_source_replay_passed && cycle.fixed_consumed_prefix_proven,
                "unproven executed correction boundary"
            );
            let next = result.cycles.get(cycle.cycle_index + 1);
            if let Some(next) = next {
                ensure!(
                    next.current_state == s.end_state,
                    "next planning cycle did not restart at handoff"
                );
            }
            let audit = cycle
                .audit
                .as_ref()
                .context("correction lacks recorded nominal audit")?;
            let reason = if let Some(v) = &audit.clearance_scan.first_violation {
                if let Some(clearance) = v.clearance_m {
                    ensure!(
                        clearance.is_finite()
                            && v.required_clearance_m.is_finite()
                            && clearance < v.required_clearance_m,
                        "invalid recorded clearance violation"
                    );
                    format!(
                        "The proposed direct continuation had {clearance:.3} m body clearance against a {:.3} m reserve at physics step {}. That rejected continuation was not flown.",
                        v.required_clearance_m, v.physics_step
                    )
                } else {
                    format!(
                        "The proposed direct continuation failed its terrain clearance audit: {}. That rejected continuation was not flown.",
                        v.reason
                    )
                }
            } else {
                let contact = audit
                    .incoming_contact
                    .as_ref()
                    .context("correction has no recorded terrain conflict")?;
                ensure!(
                    contact.classification == pd_core::ContactClassification::Crash,
                    "correction audit has no hazardous contact"
                );
                format!(
                    "The proposed direct continuation would contact terrain at physics step {}. That rejected continuation was not flown; the active flight used this correction instead.",
                    contact.state.physics_step
                )
            };
            let after_handoff = match next.map(|c| &c.decision) {
                Some(WaypointV2CycleDecision::Direct) if landed => "Replanning from this actual state found a direct continuation, which landed on the target.".into(),
                Some(WaypointV2CycleDecision::LocalCleared) => "Replanning from this actual state found another obstruction, so a further local correction followed.".into(),
                Some(_) | None => format!("After this handoff, the recorded flight ended with: {}.", stop_label(result.planning_stop)),
            };
            let conflict = cycle.conflict_state.as_ref().map(point);
            if let Some(p) = &conflict {
                check_point(p, scenario.sim.physics_hz)?;
            }
            Some(Correction {
                number: correction_index,
                reason,
                after_handoff,
                not_flown_conflict: conflict,
            })
        } else {
            None
        };
        let mut points = vec![entry];
        points.extend(
            samples
                .iter()
                .filter(|p| {
                    p.physics_step > s.start_physics_step && p.physics_step < s.end_physics_step
                })
                .cloned(),
        );
        points.push(end);
        segments.push(FlownSegment {
            kind: match s.kind {
                WaypointV2SegmentKind::InitialNominal => SegmentKind::InitialNominal,
                WaypointV2SegmentKind::LocalCorrection => SegmentKind::LocalCorrection,
                WaypointV2SegmentKind::AirborneNominal => SegmentKind::ReplannedNominal,
            },
            points,
            correction,
        });
    }
    if let Some(last) = result.segments.last() {
        ensure!(
            last.end_state == ordinary.final_state,
            "segment ledger omits actual endpoint"
        );
    }
    let target_id = match &scenario.mission.goal {
        EvaluationGoal::LandingOnPad { target_pad_id } => target_pad_id,
        _ => anyhow::bail!("V2 report requires a landing goal"),
    };
    let target = scenario
        .world
        .landing_pads
        .iter()
        .find(|p| &p.id == target_id)
        .context("missing target pad")?;
    let source = scenario
        .world
        .landing_pads
        .iter()
        .find(|p| (p.center_x_m - start.position_m.x).abs() < 1e-8)
        .context("missing source pad")?;
    Ok(FlightReport {
        title: friendly_title(&scenario.id), case_id: scenario.id.clone(), outcome: outcome(result), landed,
        correction_count: result.correction_count, elapsed_s: finish.time_s,
        terrain: scenario.world.terrain.points().to_vec(), source_pad: Vec2::new(source.center_x_m, source.surface_y_m), target_pad: Vec2::new(target.center_x_m, target.surface_y_m),
        start, finish, segments, index_href: None, capture_label: "V2 capture".into(), policy_version: version,
        diagnostics: vec![("Planning stop".into(), serde_json::to_value(result.planning_stop)?.as_str().unwrap_or_default().into()),
            ("Recorded reason".into(), result.reason.clone().unwrap_or_else(|| "None".into())),
            ("Input identity".into(), result.input_identity.clone()),
            ("Recorded integrity passed".into(), result.integrity_passed.to_string()),
            ("Recorded final source replay passed".into(), result.final_source_replay_passed.to_string()),
            ("Evidence scope".into(), "Presentation checks retained evidence consistency; it does not rerun or re-prove the flight.".into())],
        source_links: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        local_clearing::LocalClearingOrdinaryEvidenceV1,
        waypoint_v2::{WaypointV2Segment, WaypointV2Timings},
    };
    use pd_core::{EndReason, RunContext, RunManifest, RunSummary, SampleRecord, SimulationState};
    use serde_json::Value;
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    /// Synthetic evidence, not a physical flight. No commands are simulated.
    fn synthetic(odd_endpoint: bool) -> (ScenarioSpec, WaypointV2FlightResult) {
        let mut scenario = crate::test_inputs::planner_request("v2_clear_845").scenario;
        scenario.id = "v2_clear_synthetic".into();
        let context = RunContext::from_scenario(&scenario).unwrap();
        let initial = SimulationState::new(&context).unwrap();
        let mut final_state = initial.clone();
        if odd_endpoint {
            final_state.physics_step = 13;
            final_state.sim_time_s = 13.0 / f64::from(scenario.sim.physics_hz);
            final_state.position_m += Vec2::new(1.0, 2.0);
            final_state.velocity_mps = Vec2::new(1.0, -2.0);
        }
        let snapshot = SimulationStateSnapshotV1::from_state(&final_state);
        let make_sample = |s: &SimulationState| SampleRecord {
            sim_time_s: s.sim_time_s,
            physics_step: s.physics_step,
            observation: s.build_observation(&context),
            held_command: s.held_command,
        };
        let mut samples = vec![make_sample(&initial)];
        if odd_endpoint {
            samples.push(make_sample(&final_state));
        }
        let ordinary = LocalClearingOrdinaryEvidenceV1 {
            final_state: snapshot.clone(),
            incoming_contact: None,
            actions: vec![],
            events: vec![],
            samples,
        };
        let manifest = RunManifest {
            schema_version: 1,
            scenario_id: scenario.id.clone(),
            scenario_name: scenario.name.clone(),
            scenario_seed: scenario.seed,
            scenario_tags: scenario.tags.clone(),
            controller_id: "synthetic_display_only".into(),
            physics_hz: scenario.sim.physics_hz,
            controller_hz: scenario.sim.controller_hz,
            sim_time_s: final_state.sim_time_s,
            physics_steps: final_state.physics_step,
            controller_updates: 0,
            physical_outcome: PhysicalOutcome::Flying,
            mission_outcome: MissionOutcome::InProgress,
            end_reason: EndReason::Running,
            summary: RunSummary::default(),
        };
        let segments = if odd_endpoint {
            vec![WaypointV2Segment {
                kind: WaypointV2SegmentKind::InitialNominal,
                start_physics_step: 0,
                end_physics_step: 13,
                proposal_identity: "synthetic".into(),
                updates: vec![],
                entry_state: SimulationStateSnapshotV1::from_state(&initial),
                end_state: snapshot,
            }]
        } else {
            vec![]
        };
        (
            scenario,
            WaypointV2FlightResult {
                policy: WaypointV2Policy::revision_3(),
                input_identity: "synthetic display test".into(),
                planning_stop: WaypointV2Stop::NoClearing,
                reason: Some("synthetic fixture only".into()),
                correction_count: 0,
                initial_nominal_terrain_blocked: true,
                integrity_passed: false,
                physical_outcome: Some(PhysicalOutcome::Flying),
                mission_outcome: Some(MissionOutcome::InProgress),
                absolute_deadline_physics_step: None,
                cycles: vec![],
                segments,
                ordinary_flight: Some(ordinary),
                final_source_replay_passed: false,
                manifest: Some(manifest),
                failed_local_row: None,
                timings: WaypointV2Timings::default(),
            },
        )
    }

    #[test]
    fn synthetic_odd_actual_endpoint_and_empty_stop_are_not_landing() {
        let (scenario, result) = synthetic(true);
        let data = project_flight(&scenario, &result).unwrap();
        assert_eq!(data.finish.physics_step, 13);
        assert_eq!(data.finish.time_s, 13.0 / 120.0);
        assert_eq!(data.segments[0].points.last(), Some(&data.finish));
        assert!(!data.landed);
        assert!(data.outcome.starts_with("Stopped in flight"));
        let (scenario, result) = synthetic(false);
        let data = project_flight(&scenario, &result).unwrap();
        assert!(data.segments.is_empty());
        assert_eq!(data.elapsed_s, 0.0);
        assert!(data.outcome.starts_with("Stopped before departure"));
    }

    #[test]
    fn rich_batch_report_retains_original_payload_beyond_annotations() {
        let (scenario, result) = synthetic(true);
        let ordinary = result.ordinary_flight.as_ref().unwrap();
        let manifest = result.manifest.as_ref().unwrap();
        let original = pd_report::render_run_report_with_flight_annotations(
            &scenario,
            None,
            manifest,
            &ordinary.events,
            &ordinary.samples,
            &[],
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let annotated = render_rich_flight(
            &scenario,
            &result,
            pd_report::flight_annotations::AnnotationNavigation {
                collection: Some(pd_report::flight_annotations::NavigationLink {
                    label: "Planner V2 batch".into(),
                    href: "../../index.html".into(),
                }),
                ..Default::default()
            },
            "Synthetic batch report test, not flight acceptance".into(),
        )
        .unwrap();
        let payload = |html: &str| {
            let start = html.find("const reportData = ").unwrap() + "const reportData = ".len();
            serde_json::Deserializer::from_str(&html[start..])
                .into_iter::<Value>()
                .next()
                .unwrap()
                .unwrap()
        };
        let mut enriched = payload(&annotated);
        assert!(
            enriched
                .as_object_mut()
                .unwrap()
                .remove("flightAnnotations")
                .is_some()
        );
        assert_eq!(payload(&original), enriched);
        assert!(annotated.contains("Planner V2 batch"));
        assert!(
            annotated.contains("id=\"chart-spatial\"")
                && annotated.contains("id=\"chart-metrics\"")
        );
        let mut bad = result.clone();
        bad.correction_count = 1;
        assert!(render_rich_flight(&scenario, &bad, Default::default(), String::new()).is_err());
    }

    #[test]
    fn corrupt_missing_segments_clocks_counts_and_outcomes_fail_closed() {
        let (scenario, result) = synthetic(true);
        let mut bad = result.clone();
        bad.segments.clear();
        assert!(project_flight(&scenario, &bad).is_err());
        let mut bad = result.clone();
        bad.segments[0].end_state.sim_time_s += 0.1;
        assert!(project_flight(&scenario, &bad).is_err());
        let mut bad = result.clone();
        bad.correction_count = 1;
        assert!(project_flight(&scenario, &bad).is_err());
        let mut bad = result.clone();
        bad.planning_stop = WaypointV2Stop::Landed;
        assert!(project_flight(&scenario, &bad).is_err());
        let mut bad = result.clone();
        bad.ordinary_flight.as_mut().unwrap().samples[0]
            .observation
            .position_m
            .x += 1.0;
        assert!(project_flight(&scenario, &bad).is_err());
        let mut bad = result.clone();
        bad.segments[0].entry_state.position_m.x = f64::NAN;
        assert!(project_flight(&scenario, &bad).is_err());
        let mut bad = result;
        bad.ordinary_flight.as_mut().unwrap().samples.pop();
        assert!(project_flight(&scenario, &bad).is_err());
    }

    #[test]
    #[ignore = "requires pinned retained local V2 capture; presentation only, no mission execution"]
    fn retained_presentation_gate_checks_exact_handoffs_and_shared_projection() {
        for (id, steps) in [
            ("v2_clear_845", vec![]),
            ("v2_clear_uphill_845", vec![]),
            ("v2_clear_downhill_845", vec![]),
            ("v2_ridge_late", vec![2426]),
            ("v2_successive_rising", vec![2006, 2774]),
            ("v2_plateau_reference_900", vec![2820, 3136, 3226]),
            ("v2_diag_near_target", vec![]),
        ] {
            let run = repo().join("outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a").join("runs").join(id);
            let scenario: ScenarioSpec =
                serde_json::from_slice(&fs::read(run.join("scenario.json")).unwrap()).unwrap();
            let result: WaypointV2FlightResult =
                serde_json::from_slice(&fs::read(run.join("flight.json")).unwrap()).unwrap();
            let display = project_flight(&scenario, &result).unwrap();
            assert_eq!(
                display
                    .segments
                    .iter()
                    .filter(|s| s.correction.is_some())
                    .map(|s| s.points.last().unwrap().physics_step)
                    .collect::<Vec<_>>(),
                steps
            );
            for pair in display.segments.windows(2) {
                assert_eq!(pair[0].points.last(), pair[1].points.first());
            }
            if id == "v2_ridge_late" {
                assert_eq!(
                    display
                        .segments
                        .iter()
                        .map(|s| (s.points[0].time_s, s.points.last().unwrap().time_s))
                        .collect::<Vec<_>>(),
                    vec![(0.0, 12.6), (12.6, 2426.0 / 120.0), (2426.0 / 120.0, 34.0)]
                );
                assert!(
                    display.segments[1]
                        .correction
                        .as_ref()
                        .unwrap()
                        .reason
                        .contains("4.612 m")
                );
                assert!(
                    display.segments[1]
                        .correction
                        .as_ref()
                        .unwrap()
                        .reason
                        .contains("not flown")
                );
                let mut bad = result.clone();
                bad.segments[1].entry_state.velocity_mps.x += 1.0;
                assert!(
                    project_flight(&scenario, &bad).is_err(),
                    "discontinuous full boundary accepted"
                );
                let mut bad = result.clone();
                bad.segments[1].proposal_identity = "different owner".into();
                assert!(
                    project_flight(&scenario, &bad).is_err(),
                    "wrong handoff owner accepted"
                );
                let mut bad = result.clone();
                bad.cycles[1].current_state.position_m.y += 1.0;
                assert!(
                    project_flight(&scenario, &bad).is_err(),
                    "replan at wrong state accepted"
                );
                let mut bad = result.clone();
                bad.cycles[0].local_search.as_mut().unwrap().selected = None;
                assert!(
                    project_flight(&scenario, &bad).is_err(),
                    "missing selected correction accepted"
                );
            }
            if id.contains("clear_") {
                assert!(display.landed);
            }
            if id == "v2_diag_near_target" {
                assert!(!display.landed);
                assert!(display.segments.is_empty());
            }
            if result.ordinary_flight.is_some() {
                let rich =
                    render_rich_flight(&scenario, &result, Default::default(), String::new())
                        .unwrap();
                assert!(rich.contains("const reportData = "));
            }
        }
    }
}
