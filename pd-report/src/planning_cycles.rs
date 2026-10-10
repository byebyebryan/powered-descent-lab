//! Additive, display-only planning diagnostics for the common rich report.
//! This module neither chooses programs nor constructs simulation states.
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::flight_annotations::{ExecutedCorrection, FlightBoundary};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathPoint {
    pub time_s: f64,
    pub x_m: f64,
    pub y_m: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryReview {
    pub row_id: String,
    pub path: Vec<PathPoint>,
    pub progress_x_m: f64,
    pub stop_reason: String,
    pub handoff: Option<FlightBoundary>,
    pub rejection_state: Option<FlightBoundary>,
    pub rejection_status: Option<String>,
    pub rejection_reason: Option<String>,
    pub failed_predicates: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleReview {
    pub index: usize,
    pub decision: String,
    pub current: FlightBoundary,
    pub nominal_identity: Option<String>,
    pub nominal: Vec<PathPoint>,
    pub nominal_extension: Vec<PathPoint>,
    pub ballistic: Vec<PathPoint>,
    pub conflict: Option<FlightBoundary>,
    pub conflict_phase: Option<String>,
    pub correction: Option<ExecutedCorrection>,
    pub executed: Vec<PathPoint>,
    pub summary: String,
    pub boundary_counts: BTreeMap<String, usize>,
    pub nominal_rejections: BTreeMap<String, usize>,
    pub queries: Vec<QueryReview>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_goal: Option<pd_core::Vec2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_goal: Option<pd_core::Vec2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clearing_crest: Option<pd_core::Vec2>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cutoff_coast: Vec<PathPoint>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningReview {
    pub schema: String,
    pub provenance: String,
    pub cycles: Vec<CycleReview>,
}

/// Preserve all original panels and scripts. Diagnostics have a separate JSON
/// payload and append traces to the original spatial plot after it is ready.
pub fn attach(html: &str, review: &PlanningReview) -> Result<String> {
    ensure!(!review.cycles.is_empty(), "planning review has no cycles");
    ensure!(
        review.schema == "pd-lab.planning-cycle-review.v1",
        "unknown planning review schema"
    );
    ensure!(
        html.matches("<div class=\"left-stack\">").count() == 1
            && html.matches("</body>").count() == 1
            && !html.contains("id=\"planning-cycle-review\""),
        "unexpected rich report diagnostics slot"
    );
    for (index, cycle) in review.cycles.iter().enumerate() {
        ensure!(cycle.index == index, "unordered planning cycles");
        ensure!(
            cycle
                .active_goal
                .is_none_or(|p| p.x.is_finite() && p.y.is_finite()),
            "nonfinite active goal"
        );
        ensure!(
            cycle
                .previous_goal
                .is_none_or(|p| p.x.is_finite() && p.y.is_finite()),
            "nonfinite previous goal"
        );
        ensure!(
            cycle
                .clearing_crest
                .is_none_or(|p| p.x.is_finite() && p.y.is_finite()),
            "nonfinite clearing crest"
        );
        for state in std::iter::once(&cycle.current)
            .chain(cycle.conflict.iter())
            .chain(cycle.correction.iter().flat_map(|c| [&c.entry, &c.handoff]))
            .chain(
                cycle
                    .queries
                    .iter()
                    .flat_map(|q| q.handoff.iter().chain(q.rejection_state.iter())),
            )
        {
            ensure!(
                [
                    state.sim_time_s,
                    state.position_m.x,
                    state.position_m.y,
                    state.velocity_mps.x,
                    state.velocity_mps.y,
                    state.attitude_rad,
                    state.fuel_kg
                ]
                .iter()
                .all(|v| v.is_finite())
                    && state.sim_time_s >= 0.0
                    && state.fuel_kg >= 0.0,
                "invalid diagnostic boundary"
            );
        }
        for point in cycle
            .nominal
            .iter()
            .chain(&cycle.cutoff_coast)
            .chain(&cycle.nominal_extension)
            .chain(&cycle.ballistic)
            .chain(&cycle.executed)
            .chain(cycle.queries.iter().flat_map(|q| &q.path))
        {
            ensure!(
                [point.time_s, point.x_m, point.y_m]
                    .iter()
                    .all(|v| v.is_finite()),
                "nonfinite diagnostic point"
            );
        }
    }
    let json = serde_json::to_string(review)?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e");
    let panel = r#"
<section class="panel" id="planning-cycle-review">
  <div class="eyebrow">Why this route?</div><h2>Planning cycles · launch → each actual H</h2>
  <label>Inspect cycle <select id="planning-cycle-select" aria-label="Planning cycle"></select></label>
  <label id="planning-decision-nav" hidden>Jump to decision <select id="planning-decision-select" aria-label="Waypoint decision"></select></label>
  <label style="margin-left:12px"><input type="checkbox" id="planning-cycle-visible" checked> Show planning overlays</label>
  <p id="planning-cycle-state"></p><p id="planning-cycle-summary"></p>
  <p class="muted">Dashed purple: recorded state-aware nominal, not the illustrative launch arc. Pale purple: terrain-blind extension beyond the audited contact, not a safe route. Dotted orange: unpowered ballistic projection from this cycle’s actual position and velocity (ignores attitude change and any held thrust). Green: executed correction E → H. Red ×: proposed future obstruction, not a flown crash.</p>
  <details><summary>Search reasons and rejected query examples · not executed waypoints</summary><label><input type="checkbox" id="planning-cycle-query-visible"> Show rejected queries on the plot (not flown)</label><pre id="planning-cycle-reasons" style="white-space:pre-wrap"></pre><div id="planning-cycle-queries"></div></details>
  <details><summary>Diagnostic reconstruction provenance</summary><p id="planning-cycle-provenance"></p><a href="planning-cycles.json">Planning-cycle data</a></details>
  <p id="planning-cycle-plot-status" class="muted" role="status"></p>
</section>"#;
    let script = format!(
        "<script type=\"application/json\" id=\"planning-cycle-data\">{json}</script><script>{}</script>",
        include_str!("planning_cycles.js")
    );
    Ok(html
        .replacen(
            "<div class=\"left-stack\">",
            &format!("<div class=\"left-stack\">{panel}"),
            1,
        )
        .replacen("</body>", &format!("{script}</body>"), 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::Vec2;

    fn review() -> PlanningReview {
        PlanningReview {
            schema: "pd-lab.planning-cycle-review.v1".into(),
            provenance: "</script>&".into(),
            cycles: vec![CycleReview {
                index: 0,
                decision: "no_nominal".into(),
                current: FlightBoundary {
                    physics_step: 0,
                    sim_time_s: 0.0,
                    position_m: Vec2::new(0.0, 0.0),
                    velocity_mps: Vec2::new(0.0, 0.0),
                    attitude_rad: 0.0,
                    fuel_kg: 1.0,
                },
                nominal_identity: None,
                nominal: vec![],
                nominal_extension: vec![],
                ballistic: vec![],
                conflict: None,
                conflict_phase: None,
                correction: None,
                executed: vec![],
                summary: "none".into(),
                boundary_counts: BTreeMap::new(),
                nominal_rejections: BTreeMap::new(),
                queries: vec![],
                active_goal: None,
                goal_label: None,
                previous_goal: None,
                clearing_crest: None,
                cutoff_coast: vec![],
            }],
        }
    }

    #[test]
    fn extension_preserves_rich_payload_and_escapes_script_end() {
        let original = "<div class=\"left-stack\"><div>rich plots</div><script>const reportData = {\"x\":1.2345678901234567};</script></body>";
        let copy = attach(original, &review()).unwrap();
        assert!(copy.contains(
            "<div>rich plots</div><script>const reportData = {\"x\":1.2345678901234567};</script>"
        ));
        assert!(copy.contains("\\u003c/script\\u003e\\u0026"));
        assert!(attach(&copy, &review()).is_err());
        assert!(attach("missing", &review()).is_err());
    }

    #[test]
    fn rejects_nonfinite_or_unordered_diagnostic_data() {
        let mut r = review();
        let html = "<div class=\"left-stack\"></body>";
        r.cycles[0].nominal.push(PathPoint {
            time_s: 0.0,
            x_m: f64::NAN,
            y_m: 0.0,
        });
        assert!(attach(html, &r).is_err());
        r.cycles[0].nominal.clear();
        r.cycles[0].active_goal = Some(Vec2::new(f64::NAN, 0.0));
        assert!(attach(html, &r).is_err());
        r.cycles[0].active_goal = None;
        r.cycles[0].index = 2;
        assert!(attach(html, &r).is_err());
    }
}
