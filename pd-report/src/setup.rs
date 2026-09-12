//! Static, setup-only reports for the feature-gated v2 direct-bridge kernel.
//!
//! This renderer deliberately has no run/simulation input. It renders the
//! exact virtual ballistic arcs and the selected analytical powered bridges
//! carried by `pd_plan`, while keeping the evidence and labels separate from
//! controller and simulator reports.

use std::{fs, path::Path};

use anyhow::{Context, Result, anyhow};
use pd_core::Vec2;
use pd_plan::conservative_ballistic_bridge::{
    AnalyticalBridgeV2, BridgeEnvironmentEvidenceV2, CertificationV2, CoastEvidenceV2,
    ComponentMarginsV2, CorrectionEnvelopeSampleV2, DirectBridgeCandidateV2, DirectBridgePolicyV2,
    DirectBridgeProbeResultV2, DirectBridgeReasonV2, DirectBridgeReportV2, HandoffV2, MarginV2,
    MissionStatusV2, RidgeCanaryEvidenceV2, RouteProgressSegmentV2, VirtualBallisticArcV2,
    WaypointCandidateV2,
};
use serde::Serialize;

const REPORT_TITLE: &str = "Conservative Ballistic Direct-Bridge Setup Report";
const MAX_DISPLAY_POINTS: u64 = 96;

#[derive(Serialize)]
struct SetupHtmlData {
    schema_id: String,
    schema_version: u32,
    report_identity: String,
    fixture_schema_id: String,
    fixture_schema_version: u32,
    fixture_identity: String,
    policy_identity: String,
    vehicle_identity: String,
    evaluation_identity: String,
    cases: Vec<SetupHtmlCaseData>,
    ridge_canary: SetupHtmlRidgeCanaryData,
}

#[derive(Serialize)]
struct SetupHtmlCaseData {
    id: String,
    candidate_count: usize,
    certified_candidate_count: usize,
    selected_candidate_identity: String,
    result_identity: String,
}

#[derive(Serialize)]
struct SetupHtmlRidgeCanaryData {
    source_case_id: String,
    identity: String,
    blocking_lane_valid: bool,
    direct_status: String,
    nominal_candidate_identity: String,
    waypoint_status: String,
    waypoint_candidate_identity: Option<String>,
    global_replan_count: usize,
}

impl From<&DirectBridgeReportV2> for SetupHtmlData {
    fn from(report: &DirectBridgeReportV2) -> Self {
        Self {
            schema_id: report.schema_id.clone(),
            schema_version: report.schema_version,
            report_identity: report.identity.clone(),
            fixture_schema_id: report.fixture.schema_id.clone(),
            fixture_schema_version: report.fixture.schema_version,
            fixture_identity: report.evaluation.fixture_identity.clone(),
            policy_identity: report.evaluation.policy_identity.clone(),
            vehicle_identity: report.evaluation.vehicle_identity.clone(),
            evaluation_identity: report.evaluation.identity.clone(),
            cases: report
                .fixture
                .cases
                .iter()
                .enumerate()
                .map(|(index, case)| {
                    let result = result_for_case(report, case.id.as_str(), index);
                    SetupHtmlCaseData {
                        id: case.id.clone(),
                        candidate_count: result.candidates.len(),
                        certified_candidate_count: result.certified_candidate_count,
                        selected_candidate_identity: result.selected_candidate_identity.clone(),
                        result_identity: result.identity.clone(),
                    }
                })
                .collect(),
            ridge_canary: SetupHtmlRidgeCanaryData {
                source_case_id: report.evaluation.ridge_canary.source_case_id.clone(),
                identity: report.evaluation.ridge_canary.identity.clone(),
                blocking_lane_valid: report.evaluation.ridge_canary.blocking_lane_valid,
                direct_status: mission_status_label(report.evaluation.ridge_canary.direct_status)
                    .to_owned(),
                nominal_candidate_identity: report
                    .evaluation
                    .ridge_canary
                    .nominal_candidate_identity
                    .clone(),
                waypoint_status: report
                    .evaluation
                    .ridge_canary
                    .waypoint_search
                    .selected_candidate
                    .as_ref()
                    .map(|candidate| {
                        mission_status_label_for_certification(candidate.classification)
                    })
                    .unwrap_or("not_found")
                    .to_owned(),
                waypoint_candidate_identity: report
                    .evaluation
                    .ridge_canary
                    .waypoint_search
                    .selected_candidate_identity
                    .clone(),
                global_replan_count: report.evaluation.ridge_canary.direct_global_replans.len(),
            },
        }
    }
}

/// Render a deterministic setup-only HTML page. Callers should validate the
/// projection with `validate_report_artifact_v2` before writing it to a
/// durable output path.
pub fn build_conservative_ballistic_setup_html(report: &DirectBridgeReportV2) -> String {
    let overview = report
        .fixture
        .cases
        .iter()
        .enumerate()
        .map(|(index, case)| {
            let result = result_for_case(report, case.id.as_str(), index);
            render_case_overview_row(case.id.as_str(), result)
        })
        .collect::<String>();
    let case_sections = report
        .fixture
        .cases
        .iter()
        .enumerate()
        .map(|(index, case)| {
            let result = result_for_case(report, case.id.as_str(), index);
            render_case_section(case, result, &report.fixture.policy)
        })
        .collect::<String>();
    let data = json_html(&SetupHtmlData::from(report));
    format!(
        r###"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>{css}</style>
</head>
<body>
  <main>
    <header class="hero">
      <div class="eyebrow">powered descent lab · setup evidence</div>
      <h1>{title}</h1>
      <p class="lede"><strong>analytical setup only</strong> · <strong>no controller</strong> · <strong>no simulation</strong> · four frozen direct-bridge probes plus one bounded ridge canary.</p>
      <p class="lede">Faint blue lines are the four direct virtual ballistic duration candidates; orange segments are the selected exact powered bridges; green is the selected exact coast segment between derived handoffs. Not-certified means conservative rejection, not physical impossibility. The four V2 probe cards perform no waypoint search; the ridge canary below is the only bounded one-waypoint evidence.</p>
      <div class="chips"><span>fixture {fixture}</span><span>policy {policy}</span><span>vehicle {vehicle}</span><span>evaluation {evaluation}</span></div>
    </header>

    <section class="panel">
      <div class="panel-head"><div><div class="eyebrow">v2 overview</div><h2>Four analytical missions</h2></div><span class="badge">no run artifacts</span></div>
      <div class="table-wrap"><table><thead><tr><th>Mission</th><th>Virtual candidates</th><th>Certified</th><th>Selected candidate</th><th>Classification</th></tr></thead><tbody>{overview}</tbody></table></div>
    </section>

    <section class="panel">
      <div class="panel-head"><div><div class="eyebrow">Geometry legend</div><h2>How to read the plots</h2></div></div>
      <div class="legend"><span><i class="swatch terrain"></i>terrain and pads</span><span><i class="swatch initial"></i>initial state</span><span><i class="swatch direct"></i>direct virtual ballistic candidates</span><span><i class="swatch powered"></i>selected powered bridges</span><span><i class="swatch coast"></i>selected exact coast</span><span><i class="swatch reference"></i>release/touchdown references</span></div>
      <p class="muted">Each candidate retains complete component margins, decisive reasons, bridge counters, and attitude/slew evidence. Display polylines are deterministic visual samples; the underlying analytical evidence remains in the reloadable summary.</p>
    </section>

    {ridge_section}

    {sections}

    <section class="panel provenance">
      <div class="panel-head"><div><div class="eyebrow">Provenance</div><h2>Frozen identities</h2></div></div>
      <dl class="facts">
        <div><dt>Report schema</dt><dd>{schema} / v{version}</dd></div>
        <div><dt>Fixture schema</dt><dd>{fixture_schema} / v{fixture_version}</dd></div>
        <div><dt>Report identity</dt><dd>{report_identity}</dd></div>
        <div><dt>Fixture identity</dt><dd>{fixture}</dd></div>
        <div><dt>Policy identity</dt><dd>{policy}</dd></div>
        <div><dt>Vehicle identity</dt><dd>{vehicle}</dd></div>
        <div><dt>Evaluation identity</dt><dd>{evaluation}</dd></div>
      </dl>
      <p class="muted">The report is recomputed from the embedded frozen fixture and carries no controller identity, samples, events, or simulation manifest. A not-certified result is a conservative rejection, not a claim of physical impossibility.</p>
    </section>
  </main>
  <script type="application/json" id="direct-bridge-setup-data">{data}</script>
</body>
</html>"###,
        title = escape_html(REPORT_TITLE),
        css = SETUP_CSS,
        fixture = escape_html(&report.evaluation.fixture_identity),
        policy = escape_html(&report.evaluation.policy_identity),
        vehicle = escape_html(&report.evaluation.vehicle_identity),
        evaluation = escape_html(&report.evaluation.identity),
        report_identity = escape_html(&report.identity),
        schema = escape_html(&report.schema_id),
        version = report.schema_version,
        fixture_schema = escape_html(&report.fixture.schema_id),
        fixture_version = report.fixture.schema_version,
        overview = overview,
        ridge_section = render_ridge_canary_section(
            report,
            report
                .fixture
                .cases
                .iter()
                .find(|case| case.id == report.evaluation.ridge_canary.source_case_id)
                .expect("validated report contains ridge canary source case"),
            &report.fixture.policy,
        ),
        sections = case_sections,
        data = data,
    )
}

/// Render a deterministic SVG small-multiple preview for all four probes.
pub fn build_conservative_ballistic_setup_preview_svg(report: &DirectBridgeReportV2) -> String {
    let width = 980.0;
    let case_height = 360.0;
    let ridge_height = 540.0;
    let height = case_height * report.fixture.cases.len() as f64 + ridge_height;
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.0} {height:.0}" role="img" aria-label="{label}"><title>{label}</title><desc>Analytical setup only; no controller and no simulation. Four direct virtual ballistic candidates with selected powered bridges and exact coast segments.</desc>"#,
        label = escape_html(REPORT_TITLE),
    );
    for (index, case) in report.fixture.cases.iter().enumerate() {
        let result = result_for_case(report, case.id.as_str(), index);
        let offset_y = case_height * index as f64;
        svg.push_str(&render_case_group(
            case,
            result,
            &report.fixture.policy,
            width,
            case_height,
            offset_y,
        ));
    }
    let ridge_case = report
        .fixture
        .cases
        .iter()
        .find(|case| case.id == report.evaluation.ridge_canary.source_case_id)
        .expect("validated report contains ridge canary source case");
    svg.push_str(&render_ridge_canary_group(
        ridge_case,
        &report.evaluation.ridge_canary,
        &report.fixture.policy,
        width,
        ridge_height,
        case_height * report.fixture.cases.len() as f64,
    ));
    svg.push_str("</svg>");
    svg
}

/// Validate and write the setup HTML report.
pub fn write_conservative_ballistic_setup_report(
    path: &Path,
    report: &DirectBridgeReportV2,
) -> Result<()> {
    validate_report(report)?;
    let html = build_conservative_ballistic_setup_html(report);
    write_output(path, html.as_bytes())
}

/// Validate and write the setup SVG preview.
pub fn write_conservative_ballistic_setup_preview_svg(
    path: &Path,
    report: &DirectBridgeReportV2,
) -> Result<()> {
    validate_report(report)?;
    let svg = build_conservative_ballistic_setup_preview_svg(report);
    write_output(path, svg.as_bytes())
}

fn validate_report(report: &DirectBridgeReportV2) -> Result<()> {
    pd_plan::conservative_ballistic_bridge::validate_report_artifact_v2(report)
        .map_err(|error| anyhow!(error))
}

fn write_output(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }
    fs::write(path, bytes).with_context(|| format!("failed to write output {}", path.display()))
}

fn result_for_case<'a>(
    report: &'a DirectBridgeReportV2,
    case_id: &str,
    fixture_index: usize,
) -> &'a DirectBridgeProbeResultV2 {
    report
        .evaluation
        .results
        .iter()
        .find(|result| result.id == case_id)
        .or_else(|| report.evaluation.results.get(fixture_index))
        .expect("validated direct-bridge report has one result per fixture case")
}

fn render_case_overview_row(case_id: &str, result: &DirectBridgeProbeResultV2) -> String {
    let selected = selected_candidate(result);
    let (classification, status_class) = selected
        .map(|candidate| {
            (
                classification_label(candidate.classification),
                classification_class(candidate.classification),
            )
        })
        .unwrap_or(("not certified", "reject"));
    let selected_identity = if result.selected_candidate_identity.is_empty() {
        "none".to_owned()
    } else {
        result.selected_candidate_identity.clone()
    };
    format!(
        r###"<tr><td><a href="#{id}"><code>{id}</code></a></td><td>{candidate_count}</td><td>{certified_count}</td><td><code>{selected}</code></td><td><span class="status {status_class}">{classification}</span></td></tr>"###,
        id = escape_html(case_id),
        candidate_count = result.candidates.len(),
        certified_count = result.certified_candidate_count,
        selected = escape_html(&selected_identity),
        classification = classification,
        status_class = status_class,
    )
}

fn render_case_section(
    case: &pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    result: &DirectBridgeProbeResultV2,
    policy: &DirectBridgePolicyV2,
) -> String {
    let candidate_rows = result
        .candidates
        .iter()
        .map(|candidate| render_candidate_row(candidate, result.identity.as_str()))
        .collect::<String>();
    let selected = selected_candidate(result)
        .map(render_selected_summary)
        .unwrap_or_else(|| {
            "No selected analytical candidate survived the frozen conservative policy.".to_owned()
        });
    format!(
        r###"<section class="panel case" id="{id}">
  <div class="panel-head"><div><div class="eyebrow">mission plot</div><h2>{id}</h2></div><span class="badge">{candidate_count} candidates · {certified_count} certified</span></div>
  <p class="muted">Source pad at ({source_x:.1}, {source_y:.1}) m · target pad at ({target_x:.1}, {target_y:.1}) m · initial state ({initial_x:.1}, {initial_y:.1}) m. Case result identity: <code>{result_identity}</code>.</p>
  <div class="plot-wrap">{plot}</div>
  <div class="grid-two"><div><h3>Candidate classification</h3><div class="table-wrap"><table><thead><tr><th>Candidate and identities</th><th>Classification</th><th>Reasons</th><th>Aggregate margin</th><th>Component margins</th><th>Bridges / fuel</th><th>Attempts, ticks, attitude and slew</th></tr></thead><tbody>{rows}</tbody></table></div></div><div><h3>Selected candidate</h3><div class="callout">{selected}</div><h3>Reference geometry</h3><dl class="facts compact"><div><dt>Release reference</dt><dd>({release_x:.1}, {release_y:.1}) m</dd></div><div><dt>Touchdown reference</dt><dd>({touchdown_x:.1}, {touchdown_y:.1}) m</dd></div><div><dt>Case result identity</dt><dd>{result_identity}</dd></div></dl></div></div>
</section>"###,
        id = escape_html(&case.id),
        candidate_count = result.candidates.len(),
        certified_count = result.certified_candidate_count,
        source_x = case.source.center_x_m,
        source_y = case.source.surface_y_m,
        target_x = case.target.center_x_m,
        target_y = case.target.surface_y_m,
        initial_x = case.initial_position_m.x,
        initial_y = case.initial_position_m.y,
        result_identity = escape_html(&result.identity),
        plot = render_case_svg(case, result, policy, 960.0, 410.0),
        rows = candidate_rows,
        selected = selected,
        release_x = result.release_reference_m.x,
        release_y = result.release_reference_m.y,
        touchdown_x = result.touchdown_reference_m.x,
        touchdown_y = result.touchdown_reference_m.y,
    )
}

fn render_ridge_canary_section(
    report: &DirectBridgeReportV2,
    case: &pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    policy: &DirectBridgePolicyV2,
) -> String {
    let canary = &report.evaluation.ridge_canary;
    let flat = &canary.flat_control;
    let envelope = &canary.correction_envelope;
    let mesa = &canary.mesa;
    let waypoint = canary.waypoint_search.selected_candidate.as_ref();
    let maximum_time_shift_drift_x_m = envelope
        .samples
        .iter()
        .map(|sample| {
            sample
                .nominal_time_shift_drift_bounds_m
                .lower_m
                .x
                .abs()
                .max(sample.nominal_time_shift_drift_bounds_m.upper_m.x.abs())
        })
        .fold(0.0_f64, f64::max);
    let maximum_time_shift_drift_y_m = envelope
        .samples
        .iter()
        .map(|sample| {
            sample
                .nominal_time_shift_drift_bounds_m
                .lower_m
                .y
                .abs()
                .max(sample.nominal_time_shift_drift_bounds_m.upper_m.y.abs())
        })
        .fold(0.0_f64, f64::max);
    let direct_status = mission_status_label(canary.direct_status);
    let direct_status_class = mission_status_class(canary.direct_status);
    let direct_explanation = if canary.direct_status == MissionStatusV2::Red {
        "RED is scoped to the shortest robust nominal direct lane: the blocking-lane contract is valid and that lane is rejected."
    } else {
        "GREEN means the nominal direct lane was not rejected by the declared blocking-lane contract."
    };
    let waypoint_summary = waypoint
        .map(render_waypoint_evidence)
        .unwrap_or_else(|| {
            "<div class=\"callout warning\"><strong>one-waypoint witness: NOT FOUND</strong> · the bounded deterministic search produced no certified witness.</div>".to_owned()
        });
    let diagnostics = canary
        .direct_global_replans
        .iter()
        .map(render_ridge_diagnostic_row)
        .collect::<String>();
    let nominal_source_handoff = flat
        .nominal_candidate
        .source_handoff
        .as_ref()
        .map(|handoff| render_handoff_fact("nominal source handoff", handoff))
        .unwrap_or_else(|| "<div><dt>Nominal source handoff</dt><dd>none</dd></div>".to_owned());
    format!(
        r###"<section class="panel ridge-canary" id="ridge-canary">
  <div class="panel-head"><div><div class="eyebrow">bounded ridge canary</div><h2>One waypoint versus the nominal direct lane</h2></div><span class="status {direct_status_class}">{direct_status} · nominal direct</span></div>
  <p class="muted">This is a controller-neutral analytical certificate for source case <code>{source_case}</code>. It is not a universal infeasibility result and makes no claim about a controller or simulator. The RED/GREEN result is only for the declared nominal direct lane and its optimistic post-commit correction envelope.</p>
  <div class="plot-wrap">{plot}</div>
  <div class="grid-two ridge-grid">
    <div>
      <h3>Declared mission status</h3>
      <div class="callout"><strong>{direct_status} · nominal-direct</strong> · {direct_explanation}<br><span class="muted">Blocking-lane contract valid: <strong>{blocking_lane}</strong> · direct nominal candidate: <code>{nominal_identity}</code> · canary identity: <code>{canary_identity}</code>.</span></div>
      <h3>Flat-twin control and commitment</h3>
      <dl class="facts compact">
        <div><dt>Flat-twin control</dt><dd>GREEN with {flat_certified} certified candidates; flat terrain identity <code>{flat_terrain}</code>.</dd></div>
        <div><dt>Shortest robust nominal</dt><dd>{nominal_multiplier}x · {nominal_duration:.3}s · candidate <code>{nominal_identity}</code>.</dd></div>
        {nominal_source_handoff}
        <div><dt>Commitment boundary</dt><dd>post-source-handoff coast; the envelope starts after this handoff.</dd></div>
      </dl>
    </div>
    <div>
      <h3>Optimistic local correction envelope</h3>
      <dl class="facts compact">
        <div><dt>Display tube</dt><dd>{sample_count} deterministic samples · full-thrust-plus-hull reach {horizontal_bound:.2} m horizontal · {vertical_bound:.2} m vertical at the horizon.</dd></div>
        <div><dt>Time-shift drift</dt><dd>the delayed-crossing allowance also carries the nominal ballistic state forward; sampled drift reaches {drift_x:.2} m horizontal · {drift_y:.2} m vertical and is included in every displayed bound.</dd></div>
        <div><dt>Display envelope bounds</dt><dd>lower ({lower_x:.2}, {lower_y:.2}) m · upper ({upper_x:.2}, {upper_y:.2}) m.</dd></div>
        <div><dt>Exact crossing cut</dt><dd>{cut_samples} physics-tick samples whose reachable x interval contains ridge center {cut_center:.2} m · arc steps {cut_first} .. {cut_last} · bounds ({cut_lower_x:.2}, {cut_lower_y:.2}) .. ({cut_upper_x:.2}, {cut_upper_y:.2}) m.</dd></div>
        <div><dt>Physics assumptions</dt><dd>full derated thrust in any direction at worst-case mass {mass:.1} kg; attitude slew and throttle granularity ignored.</dd></div>
        <div><dt>Fuel/time assumptions</dt><dd>{fuel:.2} kg correction fuel · {fuel_horizon:.2}s fuel horizon · {mission_horizon:.2}s mission horizon · {correction_horizon:.2}s correction horizon.</dd></div>
        <div><dt>Crossing allowance</dt><dd>{crossing:.2}s crossing time · {allowance:.2}s derived allowance · terminal reserve {terminal_reserve:.2} kg · mission reserve {mission_reserve:.2} kg.</dd></div>
      </dl>
    </div>
  </div>
  <div class="grid-two ridge-grid">
    <div>
      <h3>Derived blocking mesa</h3>
      <dl class="facts compact">
        <div><dt>Geometry</dt><dd>base [{base_left:.2}, {base_right:.2}] m · top [{top_left:.2}, {top_right:.2}] m · top y {top_y:.2} m; the top covers the exact crossing cut.</dd></div>
        <div><dt>Source feature</dt><dd>x [{feature_left:.2}, {feature_right:.2}] m · feature top {feature_top:.2} m · identity <code>{feature_identity}</code>.</dd></div>
        <div><dt>Blocking margin</dt><dd>{blocking_margin}; full correction corridor blocked: <strong>{blocks}</strong>.</dd></div>
        <div><dt>Terrain identity</dt><dd><code>{mesa_identity}</code> · geometry is derived from the envelope, with no per-case winning-coordinate tuning.</dd></div>
      </dl>
    </div>
    <div>
      <h3>Global-replan diagnostics</h3>
      <p class="muted">These 0.75x/1.25x/1.5x rows are deliberately global direct-route replans. Higher arcs may remain GREEN; that does not negate the scoped nominal-lane RED.</p>
      <div class="table-wrap"><table><thead><tr><th>Diagnostic</th><th>Classification</th><th>Duration</th><th>Reasons</th></tr></thead><tbody>{diagnostics}</tbody></table></div>
    </div>
  </div>
  <h3>Deterministic one-waypoint witness</h3>
  {waypoint_summary}
  <p class="muted">The witness uses two ballistic legs and three exact analytical bridges: source, intermediate, and terminal. It is a bounded terrain-derived certificate, not a controller command stream.</p>
</section>"###,
        source_case = escape_html(&canary.source_case_id),
        direct_status = direct_status,
        direct_status_class = direct_status_class,
        direct_explanation = direct_explanation,
        plot = render_ridge_canary_svg(case, canary, policy, 960.0, 500.0),
        blocking_lane = if canary.blocking_lane_valid {
            "yes"
        } else {
            "no"
        },
        nominal_identity = escape_html(&canary.nominal_candidate_identity),
        canary_identity = escape_html(&canary.identity),
        flat_certified = flat.certified_candidate_count,
        flat_terrain = escape_html(&flat.terrain_identity),
        nominal_multiplier = format_multiplier(flat.nominal_duration_multiplier),
        nominal_duration = flat.nominal_duration_s,
        nominal_source_handoff = nominal_source_handoff,
        sample_count = envelope.samples.len(),
        horizontal_bound = envelope.horizontal_displacement_bound_m,
        vertical_bound = envelope.vertical_displacement_bound_m,
        drift_x = maximum_time_shift_drift_x_m,
        drift_y = maximum_time_shift_drift_y_m,
        lower_x = envelope.bounds.lower_m.x,
        lower_y = envelope.bounds.lower_m.y,
        upper_x = envelope.bounds.upper_m.x,
        upper_y = envelope.bounds.upper_m.y,
        cut_samples = envelope.crossing_cut.eligible_sample_count,
        cut_center = envelope.crossing_cut.feature_center_x_m,
        cut_first = envelope
            .crossing_cut
            .first_arc_step
            .map_or_else(|| "none".to_owned(), |step| step.to_string()),
        cut_last = envelope
            .crossing_cut
            .last_arc_step
            .map_or_else(|| "none".to_owned(), |step| step.to_string()),
        cut_lower_x = envelope.crossing_cut.bounds.lower_m.x,
        cut_lower_y = envelope.crossing_cut.bounds.lower_m.y,
        cut_upper_x = envelope.crossing_cut.bounds.upper_m.x,
        cut_upper_y = envelope.crossing_cut.bounds.upper_m.y,
        mass = envelope.worst_case_mass_kg,
        fuel = envelope.available_correction_fuel_kg,
        fuel_horizon = envelope.fuel_horizon_s,
        mission_horizon = envelope.mission_horizon_s,
        correction_horizon = envelope.correction_horizon_s,
        crossing = envelope.crossing_time_s,
        allowance = envelope.crossing_time_allowance_s,
        terminal_reserve = envelope.terminal_reserve_fuel_kg,
        mission_reserve = envelope.mission_reserve_fuel_kg,
        base_left = mesa.base_left_x_m,
        base_right = mesa.base_right_x_m,
        top_left = mesa.top_left_x_m,
        top_right = mesa.top_right_x_m,
        top_y = mesa.top_y_m,
        feature_left = mesa.feature_left_x_m,
        feature_right = mesa.feature_right_x_m,
        feature_top = mesa.feature_top_y_m,
        feature_identity = escape_html(&mesa.source_feature_identity),
        blocking_margin = render_margin_inline(mesa.correction_blocking_margin_m),
        blocks = if mesa.blocks_full_correction_corridor {
            "yes"
        } else {
            "no"
        },
        mesa_identity = escape_html(&mesa.identity),
        diagnostics = diagnostics,
        waypoint_summary = waypoint_summary,
    )
}

fn render_waypoint_evidence(candidate: &WaypointCandidateV2) -> String {
    let status = mission_status_label_for_certification(candidate.classification);
    let status_class = certification_status_class(candidate.classification);
    let reason_text = if candidate.reasons.is_empty() {
        "none".to_owned()
    } else {
        candidate
            .reasons
            .iter()
            .map(|reason| escape_html(&reason_label(*reason)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let handoffs = [
        ("source handoff", candidate.source_handoff.as_ref()),
        (
            "intermediate entry handoff",
            candidate.intermediate_entry_handoff.as_ref(),
        ),
        (
            "intermediate exit handoff",
            candidate.intermediate_exit_handoff.as_ref(),
        ),
        ("terminal handoff", candidate.terminal_handoff.as_ref()),
    ]
    .iter()
    .map(|(label, handoff)| match handoff {
        Some(handoff) => render_handoff_fact(label, handoff),
        None => format!("<div><dt>{}</dt><dd>none</dd></div>", escape_html(label)),
    })
    .collect::<String>();
    let bridges = [
        ("source bridge", candidate.source_bridge.as_ref()),
        (
            "intermediate bridge",
            candidate.intermediate_bridge.as_ref(),
        ),
        ("terminal bridge", candidate.terminal_bridge.as_ref()),
    ]
    .iter()
    .map(|(label, bridge)| render_bridge_detail(label, *bridge))
    .collect::<String>();
    let (route_progress_fact, route_progress_table) = candidate
        .route_progress
        .as_ref()
        .map(|progress| {
            let rows = progress
                .segments
                .iter()
                .map(|segment| {
                    format!(
                        "<tr><td>{}</td><td>{:.3} m/s</td><td>{:.3} / {:.3} m</td><td>{}</td><td>{}</td></tr>",
                        route_progress_segment_label(segment.segment),
                        segment.minimum_tangent_velocity_mps,
                        segment.backtracking_distance_m,
                        segment.allowed_backtracking_distance_m,
                        segment.sample_count,
                        if segment.passes { "pass" } else { "fail" },
                    )
                })
                .collect::<String>();
            (
                format!(
                    "<div><dt>Route-progress screen</dt><dd>{} · minimum tangent velocity {:.3} m/s · total local backtracking {:.3} m. Pad-end bridges may use the physical touchdown half-span; coasts and the intermediate bridge use endpoint tolerance only.</dd></div>",
                    if progress.passes { "pass" } else { "fail" },
                    progress.minimum_tangent_velocity_mps,
                    progress.total_backtracking_distance_m,
                ),
                format!(
                    "<div class=\"table-wrap\"><table><thead><tr><th>Route segment</th><th>Minimum tangent velocity</th><th>Backtracking / allowance</th><th>Samples</th><th>Screen</th></tr></thead><tbody>{rows}</tbody></table></div>"
                ),
            )
        })
        .unwrap_or_else(|| {
            (
                "<div><dt>Route-progress screen</dt><dd>missing</dd></div>".to_owned(),
                String::new(),
            )
        });
    format!(
        r###"<div class="callout waypoint-callout"><strong>one-waypoint witness: <span class="status {status_class}">{status}</span></strong> · waypoint ({waypoint_x:.2}, {waypoint_y:.2}) m · candidate <code>{identity}</code> · reasons {reasons}<div class="table-wrap"><table><thead><tr><th>Ballistic leg</th><th>Start</th><th>End</th><th>Duration</th><th>Arc steps</th></tr></thead><tbody><tr><td>source → waypoint</td><td>{source_start}</td><td>{source_end}</td><td>{source_duration:.3}s</td><td>{source_steps}</td></tr><tr><td>waypoint → target</td><td>{target_start}</td><td>{target_end}</td><td>{target_duration:.3}s</td><td>{target_steps}</td></tr></tbody></table></div><dl class="facts compact">{handoffs}{bridges}{route_progress_fact}<div><dt>Total witness</dt><dd>fuel {fuel} kg · time {time} s · identity <code>{identity}</code></dd></div></dl>{route_progress_table}</div>"###,
        status = status,
        status_class = status_class,
        waypoint_x = candidate.waypoint_position_m.x,
        waypoint_y = candidate.waypoint_position_m.y,
        identity = escape_html(&candidate.identity),
        reasons = reason_text,
        source_start = format_position(candidate.source_leg.start_m),
        source_end = format_position(candidate.source_leg.end_m),
        source_duration = candidate.source_leg.duration_s,
        source_steps = candidate.source_leg.steps,
        target_start = format_position(candidate.target_leg.start_m),
        target_end = format_position(candidate.target_leg.end_m),
        target_duration = candidate.target_leg.duration_s,
        target_steps = candidate.target_leg.steps,
        handoffs = handoffs,
        bridges = bridges,
        route_progress_fact = route_progress_fact,
        route_progress_table = route_progress_table,
        fuel = format_optional(candidate.total_fuel_burn_kg),
        time = format_optional(candidate.total_time_s),
    )
}

fn route_progress_segment_label(segment: RouteProgressSegmentV2) -> &'static str {
    match segment {
        RouteProgressSegmentV2::SourceBridge => "source bridge",
        RouteProgressSegmentV2::SourceCoast => "source coast",
        RouteProgressSegmentV2::IntermediateBridge => "intermediate bridge",
        RouteProgressSegmentV2::TerminalCoast => "terminal coast",
        RouteProgressSegmentV2::TerminalBridge => "terminal bridge",
    }
}

fn render_handoff_fact(label: &str, handoff: &HandoffV2) -> String {
    format!(
        "<div><dt>{}</dt><dd>arc step {} · state {} · velocity {}</dd></div>",
        escape_html(label),
        handoff.arc_step,
        format_position(handoff.state.position_m),
        format_velocity(handoff.state.velocity_mps),
    )
}

fn render_bridge_detail(label: &str, bridge: Option<&AnalyticalBridgeV2>) -> String {
    match bridge {
        Some(bridge) => format!(
            "<div><dt>{}</dt><dd>{} · {:.3}s · {:.3} kg fuel · {} · identity <code>{}</code></dd></div>",
            escape_html(label),
            classification_label(bridge.classification),
            bridge.duration_s,
            bridge.fuel_burn_kg,
            format_margin_scalar(bridge.margins.minimum_normalized()),
            escape_html(&bridge.identity),
        ),
        None => format!("<div><dt>{}</dt><dd>none</dd></div>", escape_html(label)),
    }
}

fn render_ridge_diagnostic_row(
    diagnostic: &pd_plan::conservative_ballistic_bridge::RidgeDirectDiagnosticV2,
) -> String {
    let reasons = if diagnostic.reasons.is_empty() {
        "none".to_owned()
    } else {
        diagnostic
            .reasons
            .iter()
            .map(|reason| escape_html(&reason_label(*reason)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "<tr><td><code>{}x</code><br><span class=\"muted\">global replan diagnostic</span></td><td><span class=\"status {}\">{}</span></td><td>{:.3}s</td><td>{}</td></tr>",
        format_multiplier(diagnostic.duration_multiplier),
        classification_class(diagnostic.classification),
        classification_label(diagnostic.classification),
        diagnostic.duration_s,
        reasons,
    )
}

fn format_multiplier(value: f64) -> String {
    let mut rendered = format!("{value:.2}");
    while rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.pop();
    }
    rendered
}

fn format_position(vector: Vec2) -> String {
    format!("({:.2}, {:.2}) m", vector.x, vector.y)
}

fn format_velocity(vector: Vec2) -> String {
    format!("({:.2}, {:.2}) m/s", vector.x, vector.y)
}

fn render_selected_summary(candidate: &DirectBridgeCandidateV2) -> String {
    let source = candidate
        .source_bridge
        .as_ref()
        .map(|bridge| format!("source bridge {:.3}s", bridge.duration_s))
        .unwrap_or_else(|| "no source bridge".to_owned());
    let terminal = candidate
        .terminal_bridge
        .as_ref()
        .map(|bridge| format!("terminal bridge {:.3}s", bridge.duration_s))
        .unwrap_or_else(|| "no terminal bridge".to_owned());
    let coast = candidate
        .selected_coast
        .as_ref()
        .map(|coast| format!("coast {:.3}s", coast.duration_s))
        .unwrap_or_else(|| "no selected coast".to_owned());
    format!(
        "<strong>{}</strong> · minimum normalized margin <strong>{}</strong> · {source} · {coast} · {terminal} · total fuel {} kg · candidate <code>{}</code>",
        classification_label(candidate.classification),
        format_margin_scalar(candidate.margins.minimum_normalized()),
        format_optional(candidate.total_fuel_burn_kg),
        escape_html(&candidate.identity),
    )
}

fn render_candidate_row(candidate: &DirectBridgeCandidateV2, result_identity: &str) -> String {
    let reasons = if candidate.reasons.is_empty() {
        "none".to_owned()
    } else {
        candidate
            .reasons
            .iter()
            .map(|reason| reason_label(*reason))
            .map(|reason| escape_html(&reason))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        r#"<tr><td><code>{candidate_identity}</code><br><span class="muted">result <code>{result_identity}</code></span></td><td><span class="status {status_class}">{classification}</span></td><td>{reasons}</td><td><strong>{aggregate}</strong></td><td>{margins}</td><td>{bridges}</td><td>{evidence}</td></tr>"#,
        candidate_identity = escape_html(&candidate.identity),
        result_identity = escape_html(result_identity),
        status_class = classification_class(candidate.classification),
        classification = classification_label(candidate.classification),
        reasons = reasons,
        aggregate = format_margin_scalar(candidate.margins.minimum_normalized()),
        margins = render_margins(&candidate.margins),
        bridges = render_bridges(candidate),
        evidence = render_candidate_evidence(candidate),
    )
}

fn render_bridges(candidate: &DirectBridgeCandidateV2) -> String {
    let source = render_bridge_summary("source", candidate.source_bridge.as_ref());
    let terminal = render_bridge_summary("terminal", candidate.terminal_bridge.as_ref());
    format!(
        "<div class=\"bridge-summary\">{source}<br>{terminal}<br>total fuel <strong>{fuel}</strong> kg<br>total time <strong>{time}</strong> s</div>",
        fuel = format_optional(candidate.total_fuel_burn_kg),
        time = format_optional(candidate.total_time_s),
    )
}

fn render_bridge_summary(label: &str, bridge: Option<&AnalyticalBridgeV2>) -> String {
    match bridge {
        Some(bridge) => format!(
            "{label} {duration:.3}s / {fuel:.3} kg",
            duration = bridge.duration_s,
            fuel = bridge.fuel_burn_kg,
        ),
        None => format!("{label} none"),
    }
}

fn render_candidate_evidence(candidate: &DirectBridgeCandidateV2) -> String {
    let source = render_environment_evidence("source", candidate.source_environment.as_ref());
    let terminal = render_environment_evidence("terminal", candidate.terminal_environment.as_ref());
    let coast = candidate
        .selected_coast
        .as_ref()
        .map(render_coast_evidence)
        .unwrap_or_else(|| "<div><dt>coast slew evidence</dt><dd>none</dd></div>".to_owned());
    format!(
        r#"<details class="evidence-details"><summary>attempts / ticks / attitude / slew</summary><dl class="facts compact"><div><dt>Bridge attempts</dt><dd>source {} · terminal {}</dd></div><div><dt>Streamed ticks</dt><dd>source {} · terminal {}</dd></div><div><dt>Environment rejections</dt><dd>source {} · terminal {}</dd></div>{source}{terminal}{coast}</dl></details>"#,
        candidate.source_bridge_attempt_count,
        candidate.terminal_bridge_attempt_count,
        candidate.source_bridge_streamed_tick_count,
        candidate.terminal_bridge_streamed_tick_count,
        candidate.source_environment_rejection_count,
        candidate.terminal_environment_rejection_count,
        source = source,
        terminal = terminal,
        coast = coast,
    )
}

fn render_environment_evidence(
    label: &str,
    environment: Option<&BridgeEnvironmentEvidenceV2>,
) -> String {
    let Some(environment) = environment else {
        return format!("<div><dt>{label} environment evidence</dt><dd>none</dd></div>");
    };
    format!(
        r#"<div><dt>{label} ticks</dt><dd>pad corridor {} · free flight {}</dd></div><div><dt>{label} clearance</dt><dd>{clearance}</dd></div><div><dt>{label} attitude</dt><dd>initial {initial} · final {final_margin} · final angular rate {angular}</dd></div>"#,
        environment.pad_corridor_tick_count,
        environment.free_flight_tick_count,
        clearance = render_margin_inline(environment.clearance_margin),
        initial = render_margin_inline(environment.initial_attitude_margin),
        final_margin = render_margin_inline(environment.final_attitude_margin),
        angular = render_margin_inline(environment.final_angular_rate_margin),
    )
}

fn render_coast_evidence(coast: &CoastEvidenceV2) -> String {
    format!(
        r#"<div><dt>Coast slew evidence</dt><dd>angle {:.3} rad · required time {:.3}s · margin {}</dd></div><div><dt>Handoff ticks</dt><dd>source {} · terminal {}</dd></div>"#,
        coast.slew_angle_rad,
        coast.required_slew_time_s,
        render_margin_inline(coast.coast_slew_margin),
        coast.source_handoff.arc_step,
        coast.terminal_handoff.arc_step,
    )
}

fn render_margins(margins: &ComponentMarginsV2) -> String {
    let components = [
        ("coupled thrust", margins.coupled_thrust),
        ("minimum throttle", margins.minimum_throttle),
        ("clearance", margins.clearance),
        ("bridge endpoint", margins.bridge_endpoint),
        ("powered slew", margins.powered_slew),
        ("source attitude", margins.source_attitude),
        ("coast slew", margins.coast_slew),
        ("fuel", margins.fuel),
        ("time", margins.time),
        ("touchdown speed", margins.touchdown_speed),
        ("touchdown attitude", margins.touchdown_attitude),
        ("touchdown angular rate", margins.touchdown_angular_rate),
    ];
    let rows = components
        .iter()
        .map(|(name, margin)| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(name),
                format_margin_raw(*margin),
                format_margin_scalar(margin.normalized),
            )
        })
        .collect::<String>();
    format!(
        r#"<details class="margin-details"><summary>component margins</summary><table><thead><tr><th>Component</th><th>Raw</th><th>Normalized</th></tr></thead><tbody>{rows}</tbody></table></details>"#,
        rows = rows,
    )
}

fn render_margin_inline(margin: MarginV2) -> String {
    format!(
        "raw {} · normalized {}",
        format_margin_raw(margin),
        format_margin_scalar(margin.normalized)
    )
}

fn format_margin_raw(margin: MarginV2) -> String {
    format_margin_scalar(margin.raw)
}

fn format_margin_scalar(value: f64) -> String {
    if value == f64::MAX {
        "unbounded".to_owned()
    } else if value == -f64::MAX {
        "failed".to_owned()
    } else {
        format!("{value:.6}")
    }
}

fn format_optional(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "none".to_owned())
}

fn classification_label(classification: CertificationV2) -> &'static str {
    match classification {
        CertificationV2::Certified => "certified",
        CertificationV2::NotCertified => "not certified",
    }
}

fn classification_class(classification: CertificationV2) -> &'static str {
    match classification {
        CertificationV2::Certified => "accept",
        CertificationV2::NotCertified => "reject",
    }
}

fn mission_status_label(status: MissionStatusV2) -> &'static str {
    match status {
        MissionStatusV2::Green => "GREEN",
        MissionStatusV2::Red => "RED",
    }
}

fn mission_status_class(status: MissionStatusV2) -> &'static str {
    match status {
        MissionStatusV2::Green => "accept",
        MissionStatusV2::Red => "reject",
    }
}

fn mission_status_label_for_certification(classification: CertificationV2) -> &'static str {
    match classification {
        CertificationV2::Certified => "GREEN",
        CertificationV2::NotCertified => "RED",
    }
}

fn certification_status_class(classification: CertificationV2) -> &'static str {
    classification_class(classification)
}

fn selected_candidate(result: &DirectBridgeProbeResultV2) -> Option<&DirectBridgeCandidateV2> {
    result
        .candidates
        .iter()
        .find(|candidate| candidate.identity == result.selected_candidate_identity)
}

fn reason_label(reason: DirectBridgeReasonV2) -> String {
    serde_json::to_string(&reason)
        .unwrap_or_else(|_| "\"unknown\"".to_owned())
        .trim_matches('"')
        .replace('_', " ")
}

fn render_case_svg(
    case: &pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    result: &DirectBridgeProbeResultV2,
    policy: &DirectBridgePolicyV2,
    width: f64,
    height: f64,
) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.0} {height:.0}" role="img" aria-label="Direct bridge {id}"><title>Direct bridge {id}</title>{group}</svg>"#,
        width = width,
        height = height,
        id = escape_html(&case.id),
        group = render_case_group(case, result, policy, width, height, 0.0),
    )
}

fn render_case_group(
    case: &pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    result: &DirectBridgeProbeResultV2,
    policy: &DirectBridgePolicyV2,
    width: f64,
    height: f64,
    offset_y: f64,
) -> String {
    let margin_left = 64.0;
    let margin_right = 18.0;
    let margin_top = 42.0;
    let margin_bottom = 42.0;
    let plot_width = (width - margin_left - margin_right).max(1.0);
    let plot_height = (height - margin_top - margin_bottom).max(1.0);
    let (min_x, max_x, min_y, max_y) = case_bounds(case, result, policy);
    let map = |point: Vec2| {
        let x = margin_left + (point.x - min_x) / (max_x - min_x) * plot_width;
        let y = offset_y + margin_top + (max_y - point.y) / (max_y - min_y) * plot_height;
        (x, y)
    };
    let terrain_points = case
        .terrain_points_m
        .iter()
        .map(|point| {
            let (x, y) = map(*point);
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let source_pad = pad_rect(&case.source, "source pad", map);
    let target_pad = pad_rect(&case.target, "target pad", map);
    let initial_state = initial_state_marker(case.initial_position_m, map);
    let mut svg = format!(
        r###"<g data-case="{id}"><rect x="0" y="{offset_y:.2}" width="{width:.2}" height="{height:.2}" fill="#fffdf8" stroke="#d9cdbc"/><text x="18" y="{title_y:.2}" font-family="sans-serif" font-size="15" font-weight="700" fill="#20211e">{id}</text><text x="{legend_x:.2}" y="{title_y:.2}" font-family="sans-serif" font-size="11" fill="#6d665c">analytical setup only · no simulation</text><polyline points="{terrain}" fill="none" stroke="#20211e" stroke-width="2.4"/><g class="pads">{source_pad}{target_pad}</g>{initial_state}"###,
        id = escape_html(&case.id),
        offset_y = offset_y,
        width = width,
        height = height,
        title_y = offset_y + 23.0,
        legend_x = width - 300.0,
        terrain = terrain_points,
        source_pad = source_pad,
        target_pad = target_pad,
        initial_state = initial_state,
    );

    for candidate in &result.candidates {
        svg.push_str(&polyline_for_points_with_identity(
            &display_arc_points(&candidate.virtual_arc),
            map,
            "#315f86",
            1.5,
            "0.30",
            "",
            "data-arc-identity",
            &candidate.virtual_arc.identity,
        ));
    }

    if let Some(candidate) = selected_candidate(result) {
        if let Some(bridge) = candidate.source_bridge.as_ref() {
            svg.push_str(&polyline_for_points(
                &display_bridge_points(bridge),
                map,
                "#b95024",
                4.8,
                "0.95",
                "",
            ));
        }
        if let (Some(source), Some(terminal)) = (
            candidate.source_handoff.as_ref(),
            candidate.terminal_handoff.as_ref(),
        ) {
            svg.push_str(&polyline_for_points(
                &display_coast_points(&candidate.virtual_arc, source, terminal),
                map,
                "#176b5c",
                4.8,
                "0.95",
                "",
            ));
        }
        if let Some(bridge) = candidate.terminal_bridge.as_ref() {
            svg.push_str(&polyline_for_points(
                &display_bridge_points(bridge),
                map,
                "#b95024",
                4.8,
                "0.95",
                "",
            ));
        }
    }

    svg.push_str(&reference_marker(
        "release reference",
        result.release_reference_m,
        map,
        "#315f86",
    ));
    svg.push_str(&reference_marker(
        "touchdown reference",
        result.touchdown_reference_m,
        map,
        "#315f86",
    ));
    if let Some(candidate) = selected_candidate(result) {
        if let Some(handoff) = candidate.source_handoff.as_ref() {
            svg.push_str(&handoff_marker("source handoff", handoff, map, "#b95024"));
        }
        if let Some(handoff) = candidate.terminal_handoff.as_ref() {
            svg.push_str(&handoff_marker("terminal handoff", handoff, map, "#b95024"));
        }
    }
    svg.push_str("</g>");
    svg
}

fn render_ridge_canary_svg(
    case: &pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    canary: &RidgeCanaryEvidenceV2,
    policy: &DirectBridgePolicyV2,
    width: f64,
    height: f64,
) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.0} {height:.0}" role="img" aria-label="Ridge canary"><title>Ridge canary</title>{group}</svg>"#,
        width = width,
        height = height,
        group = render_ridge_canary_group(case, canary, policy, width, height, 0.0),
    )
}

fn render_ridge_canary_group(
    case: &pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    canary: &RidgeCanaryEvidenceV2,
    policy: &DirectBridgePolicyV2,
    width: f64,
    height: f64,
    offset_y: f64,
) -> String {
    let margin_left = 64.0;
    let margin_right = 18.0;
    let margin_top = 47.0;
    let margin_bottom = 42.0;
    let plot_width = (width - margin_left - margin_right).max(1.0);
    let plot_height = (height - margin_top - margin_bottom).max(1.0);
    let (min_x, max_x, min_y, max_y) = ridge_bounds(case, canary, policy);
    let x_span = (max_x - min_x).max(1.0);
    let y_span = (max_y - min_y).max(1.0);
    let map = |point: Vec2| {
        let x = margin_left + (point.x - min_x) / x_span * plot_width;
        let y = offset_y + margin_top + (max_y - point.y) / y_span * plot_height;
        (x, y)
    };
    let mesa_points = canary
        .mesa
        .terrain_points_m
        .iter()
        .map(|point| {
            let (x, y) = map(*point);
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let envelope_points = envelope_polygon_points(&canary.correction_envelope.samples)
        .iter()
        .map(|point| {
            let (x, y) = map(*point);
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let source_pad = pad_rect(&case.source, "source pad", map);
    let target_pad = pad_rect(&case.target, "target pad", map);
    let initial_state = initial_state_marker(case.initial_position_m, map);
    let nominal = &canary.flat_control.nominal_candidate;
    let mut svg = format!(
        r###"<g data-canary="ridge_probe"><rect x="0" y="{offset_y:.2}" width="{width:.2}" height="{height:.2}" fill="#fffdf8" stroke="#d9cdbc"/><text x="18" y="{title_y:.2}" font-family="sans-serif" font-size="15" font-weight="700" fill="#20211e">ridge canary · {status}</text><text x="{legend_x:.2}" y="{title_y:.2}" font-family="sans-serif" font-size="11" fill="#6d665c">nominal lane + optimistic correction tube + one waypoint · no simulation</text><polygon points="{envelope}" fill="#315f86" opacity="0.14" stroke="#315f86" stroke-width="1.1" stroke-dasharray="4 3"/><polygon points="{mesa}" fill="#b95024" opacity="0.24" stroke="none"/><polyline points="{mesa}" fill="none" stroke="#20211e" stroke-width="2.6"/><g class="pads">{source_pad}{target_pad}</g>{initial_state}"###,
        offset_y = offset_y,
        width = width,
        height = height,
        title_y = offset_y + 24.0,
        legend_x = width - 520.0,
        status = mission_status_label(canary.direct_status),
        envelope = envelope_points,
        mesa = mesa_points,
        source_pad = source_pad,
        target_pad = target_pad,
        initial_state = initial_state,
    );

    for diagnostic in &canary.direct_global_replans {
        let arc = VirtualBallisticArcV2::new(
            policy,
            nominal.virtual_arc.start_m,
            nominal.virtual_arc.end_m,
            diagnostic.arc_steps,
        );
        svg.push_str(&polyline_for_points_with_identity(
            &display_arc_points(&arc),
            map,
            "#315f86",
            1.4,
            "0.28",
            "4 5",
            "data-global-replan-identity",
            &diagnostic.candidate_identity,
        ));
    }
    svg.push_str(&polyline_for_points_with_identity(
        &display_arc_points(&nominal.virtual_arc),
        map,
        "#315f86",
        2.8,
        "0.95",
        "8 5",
        "data-nominal-identity",
        &nominal.identity,
    ));
    if let Some(bridge) = nominal.source_bridge.as_ref() {
        svg.push_str(&polyline_for_points(
            &display_bridge_points(bridge),
            map,
            "#b95024",
            4.6,
            "0.92",
            "",
        ));
    }
    if let (Some(source), Some(terminal)) = (
        nominal.source_handoff.as_ref(),
        nominal.terminal_handoff.as_ref(),
    ) {
        svg.push_str(&polyline_for_points(
            &display_coast_points(&nominal.virtual_arc, source, terminal),
            map,
            "#176b5c",
            4.6,
            "0.92",
            "",
        ));
    }
    if let Some(bridge) = nominal.terminal_bridge.as_ref() {
        svg.push_str(&polyline_for_points(
            &display_bridge_points(bridge),
            map,
            "#b95024",
            4.6,
            "0.92",
            "",
        ));
    }
    if let Some(candidate) = canary.waypoint_search.selected_candidate.as_ref() {
        svg.push_str(&polyline_for_points(
            &display_arc_points(&candidate.source_leg),
            map,
            "#7b4b94",
            2.2,
            "0.78",
            "6 4",
        ));
        svg.push_str(&polyline_for_points(
            &display_arc_points(&candidate.target_leg),
            map,
            "#7b4b94",
            2.2,
            "0.78",
            "6 4",
        ));
        for bridge in [
            candidate.source_bridge.as_ref(),
            candidate.intermediate_bridge.as_ref(),
            candidate.terminal_bridge.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            svg.push_str(&polyline_for_points(
                &display_bridge_points(bridge),
                map,
                "#d16a27",
                4.8,
                "0.97",
                "",
            ));
        }
        svg.push_str(&position_marker(
            "waypoint",
            candidate.waypoint_position_m,
            map,
            "#7b4b94",
        ));
        if let Some(handoff) = candidate.source_handoff.as_ref() {
            svg.push_str(&handoff_marker(
                "waypoint source handoff",
                handoff,
                map,
                "#d16a27",
            ));
        }
        if let Some(handoff) = candidate.intermediate_entry_handoff.as_ref() {
            svg.push_str(&handoff_marker(
                "intermediate entry",
                handoff,
                map,
                "#d16a27",
            ));
        }
        if let Some(handoff) = candidate.intermediate_exit_handoff.as_ref() {
            svg.push_str(&handoff_marker(
                "intermediate exit",
                handoff,
                map,
                "#d16a27",
            ));
        }
        if let Some(handoff) = candidate.terminal_handoff.as_ref() {
            svg.push_str(&handoff_marker(
                "waypoint terminal handoff",
                handoff,
                map,
                "#d16a27",
            ));
        }
    }
    svg.push_str(&reference_marker(
        "release reference",
        nominal.virtual_arc.start_m,
        map,
        "#315f86",
    ));
    svg.push_str(&reference_marker(
        "touchdown reference",
        nominal.virtual_arc.end_m,
        map,
        "#315f86",
    ));
    if let Some(handoff) = nominal.source_handoff.as_ref() {
        svg.push_str(&handoff_marker(
            "commitment / source handoff",
            handoff,
            map,
            "#b95024",
        ));
    }
    svg.push_str("</g>");
    svg
}

fn ridge_bounds(
    case: &pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    canary: &RidgeCanaryEvidenceV2,
    policy: &DirectBridgePolicyV2,
) -> (f64, f64, f64, f64) {
    let nominal = &canary.flat_control.nominal_candidate.virtual_arc;
    let mut points = canary.mesa.terrain_points_m.clone();
    points.extend([
        case.initial_position_m,
        Vec2::new(case.source.center_x_m, case.source.surface_y_m),
        Vec2::new(case.target.center_x_m, case.target.surface_y_m),
        nominal.start_m,
        nominal.end_m,
    ]);
    points.extend(display_arc_points(nominal));
    for diagnostic in &canary.direct_global_replans {
        let arc = VirtualBallisticArcV2::new(
            policy,
            nominal.start_m,
            nominal.end_m,
            diagnostic.arc_steps,
        );
        points.extend(display_arc_points(&arc));
    }
    for sample in &canary.correction_envelope.samples {
        points.extend([sample.bounds.lower_m, sample.bounds.upper_m]);
    }
    if let Some(candidate) = canary.waypoint_search.selected_candidate.as_ref() {
        points.extend(display_arc_points(&candidate.source_leg));
        points.extend(display_arc_points(&candidate.target_leg));
        for bridge in [
            candidate.source_bridge.as_ref(),
            candidate.intermediate_bridge.as_ref(),
            candidate.terminal_bridge.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            points.extend(display_bridge_points(bridge));
        }
    }
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let x_pad = ((max_x - min_x) * 0.04).max(20.0);
    let y_pad = ((max_y - min_y) * 0.08).max(20.0);
    (min_x - x_pad, max_x + x_pad, min_y - y_pad, max_y + y_pad)
}

fn envelope_polygon_points(samples: &[CorrectionEnvelopeSampleV2]) -> Vec<Vec2> {
    let mut points = samples
        .iter()
        .map(|sample| sample.bounds.upper_m)
        .collect::<Vec<_>>();
    points.extend(samples.iter().rev().map(|sample| sample.bounds.lower_m));
    points
}

fn position_marker(
    label: &str,
    position: Vec2,
    map: impl Fn(Vec2) -> (f64, f64),
    color: &str,
) -> String {
    let (x, y) = map(position);
    format!(
        r###"<circle cx="{x:.2}" cy="{y:.2}" r="5.2" fill="{color}" stroke="#fffdf8" stroke-width="1.5"/><text x="{label_x:.2}" y="{label_y:.2}" font-family="sans-serif" font-size="10" fill="{color}">{label}</text>"###,
        x = x,
        y = y,
        label_x = x + 8.0,
        label_y = y - 10.0,
        label = escape_html(label),
        color = color,
    )
}

fn case_bounds(
    case: &pd_plan::conservative_ballistic_bridge::DirectBridgeProbeV2,
    result: &DirectBridgeProbeResultV2,
    _policy: &DirectBridgePolicyV2,
) -> (f64, f64, f64, f64) {
    let mut points = case.terrain_points_m.clone();
    points.extend([
        case.initial_position_m,
        Vec2::new(case.source.center_x_m, case.source.surface_y_m),
        Vec2::new(case.target.center_x_m, case.target.surface_y_m),
        result.release_reference_m,
        result.touchdown_reference_m,
    ]);
    for candidate in &result.candidates {
        points.extend(display_arc_points(&candidate.virtual_arc));
    }
    if let Some(candidate) = selected_candidate(result) {
        if let Some(bridge) = candidate.source_bridge.as_ref() {
            points.extend(display_bridge_points(bridge));
        }
        if let Some(bridge) = candidate.terminal_bridge.as_ref() {
            points.extend(display_bridge_points(bridge));
        }
        if let (Some(source), Some(terminal)) = (
            candidate.source_handoff.as_ref(),
            candidate.terminal_handoff.as_ref(),
        ) {
            points.extend(display_coast_points(
                &candidate.virtual_arc,
                source,
                terminal,
            ));
        }
    }
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let x_pad = ((max_x - min_x) * 0.04).max(20.0);
    let y_pad = ((max_y - min_y) * 0.08).max(20.0);
    (min_x - x_pad, max_x + x_pad, min_y - y_pad, max_y + y_pad)
}

fn pad_rect(
    pad: &pd_plan::conservative_ballistic_bridge::PadInputV2,
    label: &str,
    map: impl Fn(Vec2) -> (f64, f64),
) -> String {
    let (left, surface) = map(Vec2::new(
        pad.center_x_m - pad.width_m * 0.5,
        pad.surface_y_m,
    ));
    let (right, _) = map(Vec2::new(
        pad.center_x_m + pad.width_m * 0.5,
        pad.surface_y_m,
    ));
    let target = label.starts_with("target");
    let label_x = if target {
        left.max(right)
    } else {
        left.min(right)
    };
    let text_anchor = if target { "end" } else { "start" };
    format!(
        r###"<rect x="{left:.2}" y="{surface:.2}" width="{width:.2}" height="6" fill="#176b5c" opacity="0.84"/><text x="{label_x:.2}" y="{label_y:.2}" text-anchor="{text_anchor}" font-family="sans-serif" font-size="10" fill="#176b5c">{id}</text>"###,
        left = left.min(right),
        surface = surface - 3.0,
        width = (right - left).abs().max(3.0),
        label_x = label_x,
        label_y = surface + 18.0,
        text_anchor = text_anchor,
        id = escape_html(label),
    )
}

fn initial_state_marker(position: Vec2, map: impl Fn(Vec2) -> (f64, f64)) -> String {
    let (x, y) = map(position);
    format!(
        r###"<circle cx="{x:.2}" cy="{y:.2}" r="5.0" fill="#7b4b94" stroke="#fffdf8" stroke-width="1.4"/><text x="{label_x:.2}" y="{label_y:.2}" font-family="sans-serif" font-size="10" fill="#7b4b94">initial state</text>"###,
        x = x,
        y = y,
        label_x = x + 8.0,
        label_y = y - 22.0,
    )
}

fn reference_marker(
    label: &str,
    position: Vec2,
    map: impl Fn(Vec2) -> (f64, f64),
    color: &str,
) -> String {
    let (x, y) = map(position);
    let touchdown = label.starts_with("touchdown");
    let label_x = if touchdown { x - 8.0 } else { x + 8.0 };
    let text_anchor = if touchdown { "end" } else { "start" };
    format!(
        r#"<circle cx="{x:.2}" cy="{y:.2}" r="6.0" fill="none" stroke="{color}" stroke-width="1.3" stroke-dasharray="3 3"/><circle cx="{x:.2}" cy="{y:.2}" r="2.6" fill="{color}"/><text x="{label_x:.2}" y="{label_y:.2}" text-anchor="{text_anchor}" font-family="sans-serif" font-size="10" fill="{color}">{label}</text>"#,
        x = x,
        y = y,
        label_x = label_x,
        label_y = y - 7.0,
        text_anchor = text_anchor,
        label = escape_html(label),
    )
}

fn handoff_marker(
    label: &str,
    handoff: &HandoffV2,
    map: impl Fn(Vec2) -> (f64, f64),
    color: &str,
) -> String {
    let (x, y) = map(handoff.state.position_m);
    format!(
        r###"<circle cx="{x:.2}" cy="{y:.2}" r="4.2" fill="{color}" stroke="#fffdf8" stroke-width="1.3"/><text x="{label_x:.2}" y="{label_y:.2}" font-family="sans-serif" font-size="10" fill="{color}">{label}</text>"###,
        x = x,
        y = y,
        label_x = x + 8.0,
        label_y = y + 14.0,
        label = escape_html(label),
    )
}

fn display_arc_points(arc: &VirtualBallisticArcV2) -> Vec<Vec2> {
    let count = arc.steps.clamp(2, MAX_DISPLAY_POINTS) as usize;
    (0..count)
        .map(|index| {
            let step = (index as u64 * arc.steps) / (count as u64 - 1);
            arc.state_at(step).position_m
        })
        .collect()
}

fn display_coast_points(
    arc: &VirtualBallisticArcV2,
    source: &HandoffV2,
    terminal: &HandoffV2,
) -> Vec<Vec2> {
    let reverse = source.arc_step > terminal.arc_step;
    let low = source.arc_step.min(terminal.arc_step);
    let high = source.arc_step.max(terminal.arc_step);
    let count = high.saturating_sub(low).clamp(2, MAX_DISPLAY_POINTS) as usize;
    let mut points = (0..count)
        .map(|index| {
            let step = low + (index as u64 * (high - low)) / (count as u64 - 1);
            arc.state_at(step).position_m
        })
        .collect::<Vec<_>>();
    if reverse {
        points.reverse();
    }
    points
}

fn display_bridge_points(bridge: &AnalyticalBridgeV2) -> Vec<Vec2> {
    let count = bridge.steps.clamp(2, MAX_DISPLAY_POINTS) as usize;
    (0..count)
        .map(|index| {
            let step = (index as u64 * bridge.steps) / (count as u64 - 1);
            bridge.state_at(step).position_m
        })
        .collect()
}

fn polyline_for_points(
    points: &[Vec2],
    map: impl Fn(Vec2) -> (f64, f64),
    color: &str,
    width: f64,
    opacity: &str,
    dash: &str,
) -> String {
    polyline_for_points_with_identity(points, map, color, width, opacity, dash, "", "")
}

#[allow(clippy::too_many_arguments)]
fn polyline_for_points_with_identity(
    points: &[Vec2],
    map: impl Fn(Vec2) -> (f64, f64),
    color: &str,
    width: f64,
    opacity: &str,
    dash: &str,
    identity_attribute: &str,
    identity: &str,
) -> String {
    if points.is_empty() {
        return String::new();
    }
    let points = points
        .iter()
        .map(|point| {
            let (x, y) = map(*point);
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let dash_attr = if dash.is_empty() {
        String::new()
    } else {
        format!(" stroke-dasharray=\"{dash}\"")
    };
    let identity_attr = if identity_attribute.is_empty() {
        String::new()
    } else {
        format!(" {identity_attribute}=\"{}\"", escape_html(identity))
    };
    format!(
        r#"<polyline points="{points}" fill="none" stroke="{color}" stroke-width="{width:.2}" opacity="{opacity}" stroke-linecap="round" stroke-linejoin="round"{dash_attr}{identity_attr}/>"#,
        points = points,
        color = color,
        width = width,
        opacity = opacity,
        dash_attr = dash_attr,
        identity_attr = identity_attr,
    )
}

fn json_html<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("direct-bridge setup report should serialize")
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

const SETUP_CSS: &str = r#"
:root{color-scheme:light;--bg:#f1ede5;--paper:#fffdf8;--ink:#20211e;--muted:#6d665c;--line:#d9cdbc;--rust:#b95024;--green:#176b5c;--blue:#315f86;--bad:#a43a2c;--display:"Iowan Old Style","Palatino Linotype",Georgia,serif;--sans:"Avenir Next","IBM Plex Sans","Trebuchet MS",sans-serif;--mono:"Iosevka Term","SFMono-Regular",Consolas,monospace}
*{box-sizing:border-box}body{margin:0;min-height:100vh;color:var(--ink);font-family:var(--sans);background:radial-gradient(circle at 7% -8%,rgba(185,80,36,.15),transparent 31rem),linear-gradient(rgba(55,43,31,.018) 1px,transparent 1px),linear-gradient(90deg,rgba(55,43,31,.018) 1px,transparent 1px),linear-gradient(180deg,#fbf8f2,var(--bg));background-size:auto,32px 32px,32px 32px,auto}
main{width:min(1380px,100%);margin:auto;padding:28px 22px 64px}.hero,.panel{border:1px solid var(--line);border-radius:20px;background:rgba(255,253,248,.94);box-shadow:0 16px 40px rgba(54,39,25,.08)}.hero{position:relative;overflow:hidden;padding:25px 27px 26px;margin-bottom:16px}.hero:before{content:"";position:absolute;inset:0 0 auto;height:5px;background:linear-gradient(90deg,var(--rust) 0 35%,var(--green) 35% 68%,var(--blue) 68%)}h1,h2,h3{font-family:var(--display);font-weight:500;letter-spacing:-.02em}h1{font-size:clamp(2.1rem,4vw,3.3rem);line-height:1;margin:.4rem 0 .65rem}h2{font-size:1.65rem;margin:0}h3{font-size:1.25rem;margin:0 0 .6rem}.eyebrow{color:var(--rust);font-size:.68rem;font-weight:800;letter-spacing:.13em;text-transform:uppercase}.lede{max-width:110ch;color:var(--muted);line-height:1.52;margin:.35rem 0}.chips{display:flex;flex-wrap:wrap;gap:7px;margin-top:17px}.chips span,.badge{display:inline-flex;border:1px solid var(--line);border-radius:999px;padding:6px 10px;background:#f8f2e8;color:var(--muted);font-family:var(--mono);font-size:.76rem;overflow-wrap:anywhere}.panel{padding:20px 21px;margin:16px 0}.panel-head{display:flex;justify-content:space-between;align-items:flex-start;gap:15px;margin-bottom:13px}.table-wrap{overflow-x:auto;border:1px solid var(--line);border-radius:14px;background:var(--paper)}table{width:100%;border-collapse:collapse}th,td{padding:10px 12px;border-bottom:1px solid rgba(216,206,190,.72);text-align:left;vertical-align:top}th{color:var(--muted);font-size:.68rem;letter-spacing:.08em;text-transform:uppercase}tbody tr:hover{background:#fbf5eb}td a,td code{color:var(--green);font-weight:700;text-decoration:none}code{font-family:var(--mono);font-size:.82em;overflow-wrap:anywhere}.muted{color:var(--muted);line-height:1.48}.status{display:inline-flex;border-radius:999px;padding:4px 8px;font-size:.73rem;font-weight:800;white-space:nowrap}.status.accept{background:rgba(23,107,92,.11);color:var(--green)}.status.reject{background:rgba(164,58,44,.11);color:var(--bad)}.legend{display:flex;flex-wrap:wrap;gap:14px;margin:6px 0 12px;color:var(--muted);font-size:.87rem}.legend span{display:inline-flex;align-items:center;gap:6px}.swatch{display:inline-block;width:19px;height:4px;border-radius:3px;background:var(--ink)}.swatch.direct{background:#315f86;opacity:.55}.swatch.coast{background:#176b5c}.swatch.powered{height:7px;background:#b95024}.swatch.reference{height:10px;width:10px;border-radius:50%;background:#315f86}.swatch.initial{height:10px;width:10px;border-radius:50%;background:#7b4b94}.plot-wrap{overflow-x:auto;border:1px solid var(--line);border-radius:15px;background:#fffdf8;margin:0 0 17px}.plot-wrap svg{display:block;min-width:760px;width:100%;height:auto}.grid-two{display:grid;grid-template-columns:minmax(0,1.45fr) minmax(250px,.7fr);gap:18px}.callout{border-left:4px solid var(--green);padding:12px 14px;background:#f2f7f4;color:var(--ink);line-height:1.5;overflow-wrap:anywhere}.facts{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:9px;margin:0}.facts div{padding:9px 10px;border:1px solid var(--line);border-radius:10px;background:#faf6ef}.facts dt{color:var(--muted);font-size:.7rem;text-transform:uppercase;letter-spacing:.08em}.facts dd{margin:.28rem 0 0;font-family:var(--mono);font-size:.8rem;overflow-wrap:anywhere}.facts.compact{grid-template-columns:1fr}.provenance .facts{grid-template-columns:repeat(3,minmax(0,1fr))}.margin-details,.evidence-details{min-width:235px}.margin-details summary,.evidence-details summary{cursor:pointer;color:var(--green);font-weight:700}.margin-details table{margin-top:8px;font-size:.78rem}.margin-details th,.margin-details td{padding:5px 7px}.bridge-summary{line-height:1.55;white-space:nowrap}
@media(max-width:1100px){.grid-two{grid-template-columns:1fr}}@media(max-width:820px){main{padding:14px 11px 40px}.hero,.panel{padding:17px 14px}.panel-head{display:grid}.facts,.provenance .facts{grid-template-columns:1fr}th:nth-child(3),td:nth-child(3){display:none}}
"#;

#[cfg(test)]
mod tests {
    use super::{
        build_conservative_ballistic_setup_html, build_conservative_ballistic_setup_preview_svg,
    };
    use pd_plan::conservative_ballistic_bridge::{DirectBridgeReportV2, build_report_artifact_v2};
    use std::sync::OnceLock;

    fn report() -> DirectBridgeReportV2 {
        static REPORT: OnceLock<DirectBridgeReportV2> = OnceLock::new();
        REPORT.get_or_init(build_report_artifact_v2).clone()
    }

    #[test]
    fn setup_html_contains_four_v2_cases_geometry_evidence_and_neutral_terms() {
        let report = report();
        let html = build_conservative_ballistic_setup_html(&report);
        let lower = html.to_lowercase();
        for label in [
            "analytical setup only",
            "no controller",
            "no simulation",
            "direct virtual ballistic duration candidates",
            "not-certified means conservative rejection, not physical impossibility",
            "no waypoint search",
            "release reference",
            "source handoff",
            "terminal handoff",
            "touchdown reference",
            "component margins",
            "raw",
            "normalized",
            "attempts",
            "streamed ticks",
            "pad corridor",
            "attitude",
            "slew",
            "case result identity",
            "conservative_ballistic_direct_bridge_setup_report_v2",
        ] {
            assert!(lower.contains(&label.to_lowercase()), "missing {label}");
        }
        for id in [
            "clear_direct_probe",
            "long_span_probe",
            "ridge_probe",
            "long_range_probe",
        ] {
            assert!(html.contains(id), "missing {id}");
        }
        for forbidden in [
            "source gate",
            "terminal gate",
            "one-gate",
            "one gate",
            "authored",
            "fixed-gate",
            "gate geometry",
        ] {
            assert!(
                !lower.contains(forbidden),
                "unexpected old term {forbidden}"
            );
        }
        let candidate_count = report
            .evaluation
            .results
            .iter()
            .map(|result| result.candidates.len())
            .sum::<usize>();
        assert_eq!(
            html.matches("class=\"margin-details\"").count(),
            candidate_count
        );
        assert!(html.contains("failed"));
        assert!(!html.contains("17976931348623157"));
        assert!(html.contains("<code>"));
    }

    #[test]
    fn setup_renderer_escapes_case_identity_and_emits_v2_svg_geometry() {
        let mut escaped_report = report();
        escaped_report.fixture.cases[0].id = "<script>alert('x')</script>".to_owned();
        let html = build_conservative_ballistic_setup_html(&escaped_report);
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"));

        let report = report();
        let svg = build_conservative_ballistic_setup_preview_svg(&report);
        assert!(svg.starts_with("<svg "));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("<polyline"));
        assert!(svg.contains("data-case=\"clear_direct_probe\""));
        assert!(svg.contains("data-arc-identity="));
        assert!(svg.contains("#315f86"));
        assert!(svg.contains("#176b5c"));
        assert!(svg.contains("#b95024"));
        assert!(svg.contains("release reference"));
        assert!(svg.contains("source handoff"));
        assert!(svg.contains("terminal handoff"));
        assert!(svg.contains("touchdown reference"));
        assert!(svg.contains("source pad"));
        assert!(svg.contains("target pad"));
        assert!(svg.contains("initial state"));
        let lower_svg = svg.to_lowercase();
        assert!(!lower_svg.contains("source gate"));
        assert!(!lower_svg.contains("terminal gate"));
    }

    #[test]
    fn ridge_canary_html_contains_scoped_statuses_and_all_route_evidence() {
        let report = report();
        let html = build_conservative_ballistic_setup_html(&report);
        let lower = html.to_lowercase();
        for label in [
            "bounded ridge canary",
            "flat-twin control",
            "shortest robust nominal",
            "commitment boundary",
            "nominal source handoff",
            "optimistic local correction envelope",
            "time-shift drift",
            "exact crossing cut",
            "full derated thrust",
            "worst-case mass",
            "fuel/time assumptions",
            "derived blocking mesa",
            "blocking margin",
            "global-replan diagnostics",
            "global replan diagnostic",
            "deterministic one-waypoint witness",
            "source bridge",
            "intermediate bridge",
            "terminal bridge",
            "intermediate entry handoff",
            "intermediate exit handoff",
            "route-progress screen",
            "backtracking / allowance",
            "not a universal infeasibility result",
            "no claim about a controller or simulator",
        ] {
            assert!(lower.contains(&label.to_lowercase()), "missing {label}");
        }
        for multiplier in ["0.75x", "1.25x", "1.5x"] {
            assert!(html.contains(multiplier), "missing {multiplier}");
        }
        assert!(html.contains("RED · nominal-direct"));
        assert!(html.contains("one-waypoint witness: <span class=\"status accept\">GREEN"));
        assert!(!lower.contains("controller failure"));
        assert!(!lower.contains("simulation evidence"));
        assert!(html.contains("Higher arcs may remain GREEN"));
    }

    #[test]
    fn ridge_canary_preview_contains_tube_mesa_and_waypoint_layers() {
        let svg = build_conservative_ballistic_setup_preview_svg(&report());
        assert!(svg.contains("data-canary=\"ridge_probe\""));
        assert!(svg.contains("data-global-replan-identity"));
        assert!(svg.contains("data-nominal-identity"));
        assert!(svg.contains("waypoint"));
        assert!(svg.contains("commitment / source handoff"));
        assert!(svg.contains("#b95024"));
        assert!(svg.contains("#315f86"));
        assert!(svg.contains("#7b4b94"));
    }
}
