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
    let mut display_samples = ordinary.samples.clone();
    if display_samples
        .last()
        .is_some_and(|s| s.physics_step < ordinary.final_state.physics_step)
    {
        // A proven finite planning stop may fall between sampling ticks. The
        // exact saved endpoint is a display-only observation, never an added
        // physics step, command, raw sample or replay-proof component.
        let context = pd_core::RunContext::from_scenario(scenario).map_err(anyhow::Error::msg)?;
        let state = ordinary.final_state.to_simulation_state();
        display_samples.push(pd_core::SampleRecord {
            sim_time_s: state.sim_time_s,
            physics_step: state.physics_step,
            observation: state.build_observation(&context),
            held_command: state.held_command,
        });
        annotations.caption.push_str(" · Final displayed observation comes from the exact saved endpoint between sampling ticks; the raw sample ledger is unchanged.");
    }
    let html = pd_report::render_run_report_with_flight_annotations(
        scenario,
        None,
        manifest,
        &ordinary.events,
        &display_samples,
        &[],
        None,
        None,
        None,
        None,
        Some(&annotations),
    )?;
    let html = with_planning_review(&html, result)?;
    let diagnostics = clearing_diagnostics(result);
    Ok(if diagnostics.is_empty() {
        html
    } else {
        html.replacen("  </main>", &format!("{diagnostics}  </main>"), 1)
    })
}

/// Execution and planning are separate: an airborne endpoint is not a crash,
/// and a rejected audit may describe an obstruction far beyond that endpoint.
pub(crate) fn execution_status(result: &WaypointV2FlightResult) -> (&'static str, &'static str) {
    let Some(ordinary) = &result.ordinary_flight else {
        return ("not_simulated", "Not simulated");
    };
    match ordinary.final_state.physical_outcome {
        PhysicalOutcome::Crashed => ("crashed", "Physical crash"),
        PhysicalOutcome::LandedOnTarget => ("landed", "Target touchdown"),
        PhysicalOutcome::LandedOffTarget => ("off_target", "Off-target touchdown"),
        PhysicalOutcome::Flying if ordinary.final_state.physics_step == 0 => {
            ("not_started", "Not launched")
        }
        PhysicalOutcome::Flying => ("in_progress", "Stopped mid-flight"),
        PhysicalOutcome::TimedOut => ("timed_out", "Physical timeout"),
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn nominal_conflict_phase(cycle: &crate::WaypointV2Cycle) -> &str {
    match cycle
        .audit
        .as_ref()
        .and_then(|a| a.clearance_scan.first_violation.as_ref())
        .map(|v| v.phase.as_str())
    {
        Some("source_bridge" | "source_clearance") => "Departure",
        Some("ballistic_coast") => "Coast",
        Some("terminal_bridge" | "terminal_descent") => "Approach",
        Some(phase) => phase,
        None => "No recorded clearance conflict",
    }
}

/// Add a read-only planning summary to a common rich page or authenticated saved
/// copy. Existing plots/scripts and the embedded numeric payload are untouched.
pub(crate) fn with_planning_review(html: &str, result: &WaypointV2FlightResult) -> Result<String> {
    let marker = "id=\"planner-review\"";
    ensure!(
        html.matches(marker).count() <= 1,
        "duplicate planning review"
    );
    if html.contains(marker) {
        return Ok(html.into());
    }
    ensure!(
        html.matches("</header>").count() == 1,
        "missing rich report header"
    );
    let (status, label) = execution_status(result);
    let explanation = match status {
        "not_started" => {
            "Planning stopped before takeoff. No physics steps were executed; the source-pad marker is not a crash."
        }
        "in_progress" => {
            "Planning stopped at the saved airborne endpoint. This is not a crash or a landing; no subsequent flight is implied."
        }
        "crashed" => {
            "The executed flight records a physical crash. This is distinct from collisions predicted in rejected planning queries."
        }
        "not_simulated" => "No executed simulator trajectory is available.",
        _ => "Physical outcome, planning result and verification are separate evidence below.",
    };
    let endpoint = result.ordinary_flight.as_ref().map_or_else(|| "No simulator endpoint".into(), |o| {
        let s = &o.final_state;
        format!("Executed endpoint: {:.3} s · step {} · x {:.2} m · y {:.2} m · {} commands · {} completed handoffs", s.sim_time_s, s.physics_step, s.position_m.x, s.position_m.y, o.actions.len(), result.correction_count)
    });
    let mut rows = String::new();
    for cycle in &result.cycles {
        let audit = cycle.audit.as_ref();
        let violation = audit.and_then(|a| a.clearance_scan.first_violation.as_ref());
        let obstruction = violation.map_or_else(
            || {
                if audit.is_some_and(|a| a.passed) {
                    "Clear audited route".into()
                } else if matches!(cycle.decision, WaypointV2CycleDecision::NoNominal) {
                    "No nominal available — not an obstacle finding".into()
                } else {
                    "No recorded clearance conflict — see Flight JSON".into()
                }
            },
            |v| {
                let position = cycle.conflict_state.as_ref().map_or_else(
                    || "position unavailable".into(),
                    |s| format!("x {:.2} m · y {:.2} m", s.position_m.x, s.position_m.y),
                );
                let hz = result.manifest.as_ref().map_or(120, |m| m.physics_hz);
                format!(
                    "{} · {}<br>Proposed t {:.3} s · step {} · {}",
                    html_escape(nominal_conflict_phase(cycle)),
                    html_escape(&v.phase),
                    v.physics_step as f64 / f64::from(hz),
                    v.physics_step,
                    position
                )
            },
        );
        let search = cycle.local_search.as_ref().map_or_else(
            || "Not requested".into(),
            |s| {
                let mut text = format!(
                    "{} candidates · {} accepted",
                    s.row_count, s.accepted_row_count
                );
                if let Some(exit) = &s.early_exit {
                    let disposition = serde_json::to_value(exit.disposition)
                        .expect("early exit disposition serialization");
                    text.push_str(&format!(
                        "<br>Optional early query: {}{}<br>Original witness H: step {} ({})",
                        html_escape(disposition.as_str().expect("early exit disposition string")),
                        exit.query_state
                            .as_ref()
                            .map_or_else(String::new, |q| format!(
                                " at {:.3} s / step {}",
                                q.sim_time_s, q.physics_step
                            )),
                        exit.witness_handoff_physics_step,
                        if exit.disposition == crate::WaypointV2EarlyExitDisposition::Committed {
                            "query only; not flown"
                        } else {
                            "ordinary fallback; early query not flown"
                        }
                    ));
                }
                text
            },
        );
        let decision = serde_json::to_value(&cycle.decision)?
            .as_str()
            .context("cycle decision")?
            .to_string();
        rows.push_str(&format!("<tr data-review-cycle=\"{}\"><td>{}<br>Actual {:.3} s / x {:.2} m</td><td>{}</td><td>{obstruction}</td><td>{search}</td></tr>", cycle.cycle_index, cycle.cycle_index, cycle.current_state.sim_time_s, cycle.current_state.position_m.x, html_escape(&decision)));
    }
    let open = if matches!(status, "not_started" | "in_progress" | "crashed") {
        " open"
    } else {
        ""
    };
    let cycles = if rows.is_empty() {
        String::new()
    } else {
        format!(
            "<details{open}><summary>Planning cycles · {} · proposed routes, not executed flight</summary><p class=\"muted\">The obstruction time/location belongs to the audited proposal. It may be well beyond the executed endpoint above. A predicted collision in a rejected query is not an executed crash.</p><div style=\"max-width:100%;overflow-x:auto\"><table><thead><tr><th>Cycle / executed start</th><th>Decision</th><th>First proposed-route clearance conflict</th><th>Local clearing search</th></tr></thead><tbody>{rows}</tbody></table></div></details>",
            result.cycles.len()
        )
    };
    let panel = format!(
        "<section class=\"panel wide\" id=\"planner-review\" data-execution-status=\"{status}\"><h2>{label} · {}</h2><p>{explanation}</p><p>{endpoint}</p><p>Planning reason: {}<br>Recorded integrity: {} · Recorded source replay: {}</p><p class=\"muted\">Plots show executed samples. The dashed idealized reference is illustrative, not the audited planner proposal or an executed route.</p>{cycles}</section>",
        stop_label(result.planning_stop),
        html_escape(
            result
                .reason
                .as_deref()
                .unwrap_or("No stop reason recorded")
        ),
        result.integrity_passed,
        result.final_source_replay_passed
    );
    // This additive V2-only extension also works with authenticated old rich
    // pages. Their embedded reportData and original scripts remain unchanged.
    let banner = r#"<style>
.banner.planning-stop{background:rgba(178,107,0,.10);color:#8b5500;border-color:rgba(178,107,0,.25)}
#planner-review{margin:12px 0;min-width:0}
#planner-review p{margin:8px 0;line-height:1.45}
#planner-review details{margin-top:12px}
#planner-review table{width:100%;min-width:680px;border-collapse:collapse;text-align:left}
#planner-review th,#planner-review td{padding:8px;vertical-align:top;border-bottom:1px solid var(--line)}
</style><script>
document.addEventListener('DOMContentLoaded', () => {
  const review = document.getElementById('planner-review');
  const banner = document.getElementById('outcome-banner');
  if (!review || !banner) return;
  banner.textContent = review.querySelector('h2').textContent;
  banner.classList.toggle('planning-stop', ['not_started', 'in_progress', 'not_simulated'].includes(review.dataset.executionStatus));
  if (['not_started', 'in_progress'].includes(review.dataset.executionStatus)) {
    const set = (id, text) => { const e = document.getElementById(id); if (e) e.textContent = text; };
    set('key-quality-label', 'Landing');
    set('key-quality-value', 'Not reached');
    set('key-quality-meta', 'No touchdown recorded');
    set('quality-title', 'Pre-landing diagnostics');
    const grid = document.getElementById('quality-grid');
    if (grid) {
      const note = document.createElement('p');
      note.className = 'muted';
      note.textContent = 'No touchdown recorded. Saved endpoint/envelope diagnostics below are not measured landing or impact quality.';
      grid.before(note);
      for (const label of grid.querySelectorAll('.label')) {
        if (label.textContent === 'Offset') label.textContent = 'Endpoint offset';
        if (label.textContent.startsWith('Impact ')) label.textContent = label.textContent.replace('Impact ', 'Endpoint ');
      }
    }
  }
}, {once: true});
</script>"#;
    Ok(html.replacen("</header>", &format!("</header>\n{panel}{banner}"), 1))
}

fn clearing_diagnostics(result: &WaypointV2FlightResult) -> String {
    let escape = |value: &str| {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let mut html = String::new();
    for cycle in &result.cycles {
        let Some(search) = cycle
            .local_search
            .as_ref()
            .filter(|s| !s.row_diagnostics.is_empty())
        else {
            continue;
        };
        let has_room = search
            .row_diagnostics
            .iter()
            .any(|row| row.eligible_handoff.is_some());
        let room_header = if has_room {
            "<th>Accepted handoff / braking room estimate</th>"
        } else {
            ""
        };
        html.push_str(&format!("<section class=\"panel wide\"><details><summary>Clearing query diagnostics · cycle {} · {} rows</summary><p class=\"muted\">Queries, not executed flights. A trace cutoff after a completed handoff certificate does not invalidate that handoff. Full row diagnostics are in Flight JSON.</p><div style=\"overflow-x:auto\"><table><thead><tr><th>Row / entry s</th><th>Exact trace cutoff</th><th>Cutoff t / x</th><th>vx / vy</th><th>Required x</th><th>Min reserve</th><th>First handoff / continuation rejection</th>{room_header}</tr></thead><tbody>", cycle.cycle_index, search.row_diagnostics.len()));
        for row in &search.row_diagnostics {
            let pose = row.stop_state.as_ref().map_or_else(
                || "not propagated".into(),
                |s| format!("{:.3} s / {:.2} m", s.sim_time_s, s.position_m.x),
            );
            let velocity = row.stop_state.as_ref().map_or_else(
                || "—".into(),
                |s| format!("{:.2} / {:.2} m/s", s.velocity_mps.x, s.velocity_mps.y),
            );
            let reserve = row
                .minimum_clearance_m
                .map_or_else(|| "—".into(), |c| format!("{c:.3} m"));
            let continuation = row
                .first_continuation_rejection
                .as_ref()
                .and_then(|b| b.reason.as_deref())
                .unwrap_or("none recorded");
            let room_cell = if has_room {
                let value = row.eligible_handoff.as_ref().map_or_else(
                    || "—".into(),
                    |h| {
                        let room = h.braking_room.as_ref().map_or_else(
                            || "estimate unavailable".into(),
                            |r| {
                                format!(
                                    "{:.2} m room (heuristic, not landing proof)",
                                    r.remaining_room_m
                                )
                            },
                        );
                        format!(
                            "H {:.3} s / x {:.2} m<br>vx / vy {:.2} / {:.2} m/s<br>{room}",
                            h.state.sim_time_s,
                            h.state.position_m.x,
                            h.state.velocity_mps.x,
                            h.state.velocity_mps.y
                        )
                    },
                );
                format!("<td>{value}</td>")
            } else {
                String::new()
            };
            html.push_str(&format!("<tr><td>{}<br>{:.3}</td><td>{}</td><td>{pose}</td><td>{velocity}</td><td>{:.2} m</td><td>{reserve}</td><td>{}</td>{room_cell}</tr>", escape(&row.row_id), row.entry_physics_step as f64 / 120.0, escape(row.stop_reason.as_deref().unwrap_or("none")), row.minimum_progress_x_m, escape(continuation)));
        }
        html.push_str("</tbody></table></div></details></section>");
    }
    html
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
    crate::waypoint_v2::early_exit::validate_records(result)?;
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
        samples.first() == Some(&start),
        "missing or contradictory first actual sample"
    );
    if samples.last() != Some(&finish) {
        let interval = scenario
            .sim
            .sample_interval_steps()
            .context("missing sample cadence")?;
        let last = samples.last().context("missing last actual sample")?;
        ensure!(
            result.integrity_passed
                && result.final_source_replay_passed
                && manifest.physical_outcome == PhysicalOutcome::Flying
                && manifest.mission_outcome == MissionOutcome::InProgress
                && manifest.end_reason == pd_core::EndReason::Running
                && matches!(
                    result.planning_stop,
                    WaypointV2Stop::NoNominal
                        | WaypointV2Stop::NominalRejected
                        | WaypointV2Stop::NoClearing
                        | WaypointV2Stop::CorrectionLimit
                        | WaypointV2Stop::Deadline
                        | WaypointV2Stop::NoProgress
                )
                && !finish.physics_step.is_multiple_of(interval)
                && last.physics_step == finish.physics_step / interval * interval,
            "missing or contradictory final actual sample"
        );
    }
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
                    && search.actual_handoff_state() == Some(&s.end_state),
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
            let mut after_handoff = match next.map(|c| &c.decision) {
                Some(WaypointV2CycleDecision::Direct) if landed => "Replanning from this actual state found a direct continuation, which landed on the target.".into(),
                Some(WaypointV2CycleDecision::LocalCleared) => "Replanning from this actual state found another obstruction, so a further local correction followed.".into(),
                Some(_) | None => format!("After this handoff, the recorded flight ended with: {}.", stop_label(result.planning_stop)),
            };
            if let Some(exit) = &search.early_exit
                && exit.disposition == crate::WaypointV2EarlyExitDisposition::Committed
            {
                after_handoff = format!(
                    "A clear nominal was audited at this earlier actual handoff and that exact checked program was executed next. The original clearing witness at step {} was query evidence, not an executed waypoint. {after_handoff}",
                    exit.witness_handoff_physics_step
                );
            }
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
    fn planning_review_keeps_queries_separate_from_execution_and_escapes_text() {
        let (_, mut result) = synthetic(false);
        let original = result.clone();
        let base = "<header>rich header</header><div>unchanged plots</div><script>const reportData = {\"x\":1.2345678901234567};</script>";
        let html = with_planning_review(base, &result).unwrap();
        assert!(html.contains("Not launched · no clearing maneuver found"));
        assert!(html.contains("0 commands") && html.contains("not a crash"));
        assert!(html.contains("Recorded integrity: false"));
        assert!(html.contains("const reportData = {\"x\":1.2345678901234567};"));
        assert_eq!(
            result, original,
            "presentation cannot mutate saved evidence"
        );
        assert_eq!(with_planning_review(&html, &result).unwrap(), html);
        assert!(with_planning_review("missing header", &result).is_err());
        assert!(with_planning_review(&(base.to_owned() + base), &result).is_err());
        result.reason = Some("<script>unsafe & reason</script>".into());
        let html = with_planning_review(base, &result).unwrap();
        assert!(html.contains("&lt;script&gt;unsafe &amp; reason&lt;/script&gt;"));
        assert!(!html.contains("<script>unsafe"));

        let (_, mut airborne) = synthetic(true);
        airborne.planning_stop = WaypointV2Stop::NoNominal;
        let html = with_planning_review(base, &airborne).unwrap();
        assert!(html.contains("Stopped mid-flight · no feasible nominal trajectory found"));
        assert!(html.contains("Executed endpoint: 0.108 s · step 13"));
        airborne
            .ordinary_flight
            .as_mut()
            .unwrap()
            .final_state
            .physical_outcome = PhysicalOutcome::Crashed;
        assert_eq!(execution_status(&airborne).0, "crashed");
        airborne.ordinary_flight = None;
        assert_eq!(execution_status(&airborne).0, "not_simulated");
    }

    #[test]
    fn planning_review_does_not_label_an_approach_conflict_as_a_departure_failure() {
        let (_, mut result) = synthetic(false);
        let snapshot = result.ordinary_flight.as_ref().unwrap().final_state.clone();
        let mut conflict = snapshot.clone();
        conflict.physics_step = 4110;
        conflict.sim_time_s = 34.25;
        conflict.position_m = Vec2::new(1150.9456, 316.1128);
        let audit = serde_json::from_value(serde_json::json!({
            "passed":false,"safe_target_contact":true,"ordinary_neutral_parity":true,"commands_match":true,
            "first_contact":null,"incoming_contact":null,"final_state":conflict,
            "rejection_reasons":["synthetic clearance failure"],
            "clearance_scan":{"poststep_state_count":4110,"airborne_state_count":4109,
            "source_corridor_state_count":0,"terminal_corridor_state_count":0,"exact_clearance_query_count":4109,
            "all_airborne_states_passed":false,"minimum_airborne":null,
            "first_violation":{"physics_step":4110,"phase":"terminal_bridge","reason":"synthetic fixture",
            "clearance_m":4.8,"required_clearance_m":5.0,"corridor":"none"}}
        })).unwrap();
        result.cycles.push(crate::WaypointV2Cycle {
            cycle_index: 0,
            current_state: snapshot,
            nominal_search_identity: "synthetic".into(),
            nominal_proposal_identity: None,
            nominal_peak_com_height_m: None,
            nominal_attempt_status_counts: Default::default(),
            nominal_rejection_reason_counts: Default::default(),
            nominal_updates: vec![],
            audit: Some(audit),
            fixed_consumed_prefix_proven: false,
            decision: WaypointV2CycleDecision::NoClearing,
            conflict_state: Some(conflict),
            conflict_incoming_contact: None,
            local_search: None,
        });
        let base = "<header>rich header</header>";
        let html = with_planning_review(base, &result).unwrap();
        assert!(html.contains("Not launched") && html.contains("Executed endpoint: 0.000 s"));
        assert!(html.contains("Approach · terminal_bridge"));
        assert!(html.contains("Proposed t 34.250 s") && html.contains("x 1150.95 m"));
        assert!(html.contains("<details open>"));
        result.cycles[0].audit = None;
        result.cycles[0].decision = WaypointV2CycleDecision::NoNominal;
        let html = with_planning_review(base, &result).unwrap();
        assert!(html.contains("No nominal available — not an obstacle finding"));
        assert!(!html.contains("Proposed t"));
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
    fn proven_finite_off_cadence_endpoint_is_displayed_without_changing_evidence() {
        let (scenario, mut result) = synthetic(true);
        result.integrity_passed = true;
        result.final_source_replay_passed = true;
        result.planning_stop = WaypointV2Stop::NoNominal;
        let ordinary = result.ordinary_flight.as_mut().unwrap();
        ordinary.final_state.physics_step = 14;
        ordinary.final_state.sim_time_s = 14.0 / 120.0;
        ordinary.samples[1].physics_step = 12;
        ordinary.samples[1].sim_time_s = 0.1;
        ordinary.samples[1].observation.physics_step = 12;
        ordinary.samples[1].observation.sim_time_s = 0.1;
        result.segments[0].end_physics_step = 14;
        result.segments[0].end_state = ordinary.final_state.clone();
        let manifest = result.manifest.as_mut().unwrap();
        manifest.physics_steps = 14;
        manifest.sim_time_s = 14.0 / 120.0;
        let original = result.clone();
        let display = project_flight(&scenario, &result).unwrap();
        assert_eq!(display.finish.physics_step, 14);
        assert!(!display.landed);
        let html =
            render_rich_flight(&scenario, &result, Default::default(), String::new()).unwrap();
        assert!(html.contains("exact saved endpoint between sampling ticks"));
        assert_eq!(result, original, "display must not mutate raw evidence");
        let mut bad = result.clone();
        bad.final_source_replay_passed = false;
        assert!(project_flight(&scenario, &bad).is_err());
        let mut bad = result.clone();
        bad.ordinary_flight.as_mut().unwrap().samples.pop();
        assert!(project_flight(&scenario, &bad).is_err());
        let mut bad = result;
        bad.ordinary_flight.as_mut().unwrap().samples[1].physics_step = 14;
        bad.ordinary_flight.as_mut().unwrap().samples[1].sim_time_s = 14.0 / 120.0;
        bad.ordinary_flight.as_mut().unwrap().samples[1]
            .observation
            .physics_step = 14;
        bad.ordinary_flight.as_mut().unwrap().samples[1]
            .observation
            .sim_time_s = 14.0 / 120.0;
        bad.ordinary_flight.as_mut().unwrap().samples[1]
            .observation
            .position_m
            .x += 1.0;
        assert!(project_flight(&scenario, &bad).is_err());
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
