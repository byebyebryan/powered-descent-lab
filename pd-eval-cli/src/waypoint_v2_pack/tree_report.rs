//! A presentation adapter over verified native records, not a controller batch.
use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use pd_core::{MissionOutcome, PhysicalOutcome, Vec2};
use pd_plan::waypoint_v2::WaypointV2Stop;
use serde::Serialize;

use super::presentation::escape_html;
use super::{WaypointV2BatchCase, WaypointV2BatchReport, WaypointV2PackInput};
use crate::{WaypointV2FlightResult, waypoint_v2::WaypointV2SegmentKind};

#[derive(Debug, Serialize)]
struct TreeCase {
    case_id: String,
    group: String,
    family: String,
    label: String,
    detail_href: String,
    display_outcome: String,
    departed: bool,
    landed: bool,
    unsupported: bool,
    integrity_passed: bool,
    replay_passed: Option<bool>,
    correction_count: u32,
    fuel_used_pct: Option<f64>,
    flight_s: Option<f64>,
    landing_offset_m: Option<f64>,
    planning_s: Option<f64>,
    handoffs: Vec<Vec2>,
    #[serde(skip)]
    preview: String,
}

fn project(
    input: &WaypointV2PackInput,
    result: &WaypointV2FlightResult,
    case: &WaypointV2BatchCase,
) -> Result<TreeCase> {
    let departed = result
        .manifest
        .as_ref()
        .is_some_and(|m| m.physics_steps > 0)
        || result
            .ordinary_flight
            .as_ref()
            .is_some_and(|f| f.samples.iter().any(|s| s.physics_step > 0));
    let landed = result.planning_stop == WaypointV2Stop::Landed
        && result.physical_outcome == Some(PhysicalOutcome::LandedOnTarget)
        && result.mission_outcome == Some(MissionOutcome::Success)
        && result.integrity_passed
        && result.final_source_replay_passed;
    let unsupported = result.planning_stop == WaypointV2Stop::Unsupported;
    let handoffs = result
        .segments
        .iter()
        .filter(|s| s.kind == WaypointV2SegmentKind::LocalCorrection)
        .map(|s| s.end_state.position_m)
        .collect::<Vec<_>>();
    ensure!(
        handoffs.len() == result.correction_count as usize,
        "handoff count mismatch"
    );
    let display_outcome = if unsupported {
        "Unsupported — not simulated".to_owned()
    } else if !departed {
        format!(
            "Not departed — {}",
            case.planning_stop.as_deref().unwrap_or("planning stop")
        )
    } else if landed {
        "Landed on target".to_owned()
    } else {
        format!(
            "{} — {}",
            case.physical_outcome.as_deref().unwrap_or("unverified"),
            case.planning_stop.as_deref().unwrap_or("missing stop")
        )
    };
    let manifest = result.manifest.as_ref().filter(|_| departed);
    let samples = result
        .ordinary_flight
        .as_ref()
        .filter(|_| departed)
        .map_or(&[][..], |f| f.samples.as_slice());
    let fuel_used_pct = manifest.and_then(|m| {
        (input.scenario.vehicle.max_fuel_kg > 0.0)
            .then_some(100.0 * m.summary.fuel_used_kg / input.scenario.vehicle.max_fuel_kg)
    });
    let label = case
        .case_id
        .strip_prefix("v2_")
        .or_else(|| case.case_id.strip_prefix("fresh_"))
        .unwrap_or(&case.case_id)
        .replace('_', " ");
    Ok(TreeCase {
        case_id: case.case_id.clone(),
        group: case.group.as_str().into(),
        family: case.family.clone(),
        label,
        detail_href: case.annotated_report_path.clone(),
        display_outcome,
        departed,
        landed,
        unsupported,
        integrity_passed: result.integrity_passed,
        replay_passed: (!unsupported).then_some(result.final_source_replay_passed),
        correction_count: result.correction_count,
        fuel_used_pct,
        flight_s: manifest.map(|m| m.sim_time_s),
        landing_offset_m: manifest
            .filter(|_| landed)
            .and_then(|m| m.summary.landing.as_ref())
            .map(|l| l.touchdown_center_offset_m.abs()),
        planning_s: case.planning_s.filter(|_| !unsupported),
        handoffs: handoffs.clone(),
        preview: pd_report::build_saved_flight_preview_svg(
            &input.scenario,
            manifest,
            samples,
            &handoffs,
        ),
    })
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values = values.collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn aggregate_metric(values: impl Iterator<Item = f64>, unit: &str, precision: usize) -> String {
    let values = values.collect::<Vec<_>>();
    crate::comparison::metric_summary(&values).map_or_else(
        || "—".into(),
        |s| {
            format!(
                "{:.precision$} ± {:.precision$}{unit}",
                s.mean,
                s.stddev.unwrap_or(0.0)
            )
        },
    )
}

fn metric(value: Option<f64>, unit: &str, precision: usize) -> String {
    value.map_or_else(
        || "<span class=\"muted\">—</span>".into(),
        |v| format!("{v:.precision$}{unit}"),
    )
}

fn row_summary(
    rows: &[&TreeCase],
    label: &str,
    id: &str,
    parent: Option<&str>,
    depth: usize,
    kind: &str,
    diagnostic: bool,
) -> String {
    let landed = rows.iter().filter(|r| r.landed).count();
    let unsupported = rows.iter().filter(|r| r.unsupported).count();
    let direct = rows
        .iter()
        .filter(|r| r.landed && r.correction_count == 0)
        .count();
    let corrected = landed - direct;
    let outcome = if diagnostic {
        format!(
            "{landed} landed · {} stopped · {unsupported} unsupported",
            rows.len() - landed - unsupported
        )
    } else {
        format!("{landed}/{} target landings", rows.len())
    };
    let hs = rows
        .iter()
        .filter(|r| r.departed)
        .map(|r| r.correction_count)
        .collect::<Vec<_>>();
    let h_range = hs.iter().min().zip(hs.iter().max()).map_or_else(
        || "—".into(),
        |(lo, hi)| {
            if lo == hi {
                format!("{lo} H")
            } else {
                format!("{lo}–{hi} H")
            }
        },
    );
    let parent = parent.map_or_else(String::new, |p| {
        format!(" data-parent=\"{}\"", escape_html(p))
    });
    let attributes = format!(
        r#"data-group="{}"{} data-kind="{}" data-depth="{}" data-case-count="{}" data-landed="{landed}" data-direct="{direct}" data-corrected="{corrected}" aria-expanded="false" tabindex="0""#,
        escape_html(id),
        parent,
        kind,
        depth,
        rows.len(),
    );
    let cells = format!(
        r#"<td class="tree-label" style="--depth:{}"><span class="expander">+</span><span class="selector-inline">{}</span> <span class="selector-code">{}</span><span class="row-note muted">{} cases</span></td>
<td><div class="rate">{}<span class="row-note">{} direct · {} corrected</span></div></td><td>{}</td><td>{}</td><td>{}</td><td class="muted">—</td><td><span class="row-note muted">Expand to inspect missions</span></td><td>{}</td>"#,
        depth,
        kind,
        escape_html(label),
        rows.len(),
        outcome,
        direct,
        corrected,
        aggregate_metric(rows.iter().filter_map(|r| r.fuel_used_pct), "%", 1),
        aggregate_metric(rows.iter().filter_map(|r| r.flight_s), "s", 2),
        aggregate_metric(rows.iter().filter_map(|r| r.landing_offset_m), "m", 2),
        h_range,
    );
    pd_report::batch::render_row(
        Some("summary-row scenario-row current-row"),
        &attributes,
        &cells,
    )
}

fn mission_row(row: &TreeCase, parent: &str, depth: usize) -> String {
    let route = if !row.departed {
        "—".to_owned()
    } else if row.correction_count == 0 {
        "Direct · 0 H".into()
    } else {
        format!("{} H", row.correction_count)
    };
    let context = if row.unsupported {
        "Terrain context · not simulated"
    } else if !row.departed {
        "Terrain context · not departed"
    } else {
        "Recorded trajectory · H = executed handoff"
    };
    let evidence = format!(
        "{} / {}",
        if row.integrity_passed {
            "passed"
        } else {
            "FAILED"
        },
        row.replay_passed
            .map_or("N/A", |v| if v { "passed" } else { "FAILED" })
    );
    let attrs = format!(
        r#"data-parent="{}" data-case-id="{}" data-depth="{}" hidden"#,
        escape_html(parent),
        escape_html(&row.case_id),
        depth,
    );
    let cells = format!(
        r#"<td class="tree-label" style="--depth:{}"><a class="mission-link overview-stack" href="{}"><span class="selector-code">{}</span><span class="row-note mono">{}</span></a></td>
<td class="{}"><div class="overview-stack">{}<span class="row-note muted">Integrity / replay: {} · planning {}</span></div></td><td>{}</td><td>{}</td><td>{}</td><td class="muted">—</td>
<td><a class="run-preview" href="{}" aria-label="Open detailed report for {}">{}</a><span class="row-note muted">{}</span></td><td>{}</td>"#,
        depth,
        escape_html(&row.detail_href),
        escape_html(&row.label),
        escape_html(&row.case_id),
        if row.landed { "good" } else { "warn" },
        escape_html(&row.display_outcome),
        evidence,
        metric(row.planning_s, "s", 3),
        metric(row.fuel_used_pct, "%", 1),
        metric(row.flight_s, "s", 2),
        metric(row.landing_offset_m, "m", 3),
        escape_html(&row.detail_href),
        escape_html(&row.case_id),
        row.preview,
        context,
        route,
    );
    pd_report::batch::render_row(Some("seed-row mission-row current-row"), &attrs, &cells)
}

pub(super) fn render(
    report: &WaypointV2BatchReport,
    verified: &[(WaypointV2PackInput, WaypointV2FlightResult)],
    preview_base: Option<&str>,
) -> Result<String> {
    render_with_links(report, verified, preview_base, "")
}

pub(super) fn render_site(
    report: &WaypointV2BatchReport,
    verified: &[(WaypointV2PackInput, WaypointV2FlightResult)],
    source_base: &str,
) -> Result<String> {
    render_with_links(report, verified, None, source_base)
}

fn render_with_links(
    report: &WaypointV2BatchReport,
    verified: &[(WaypointV2PackInput, WaypointV2FlightResult)],
    preview_base: Option<&str>,
    source_base: &str,
) -> Result<String> {
    ensure!(
        report.cases.len() == verified.len(),
        "missing verified tree records"
    );
    let acceptance = crate::waypoint_v2_acceptance::assess_waypoint_v2_acceptance(report)?;
    let rows = report
        .cases
        .iter()
        .zip(verified)
        .map(|(c, (i, r))| project(i, r, c))
        .collect::<Result<Vec<_>>>()?;
    let mut tree = String::new();
    for (group, label) in [
        ("clear", "Clear controls"),
        ("ordinary", "Ordinary terrain"),
        ("additional_terrain", "Additional terrain"),
        ("diagnostic", "Diagnostics"),
    ] {
        let cases = rows.iter().filter(|r| r.group == group).collect::<Vec<_>>();
        let group_id = format!("v2-{group}");
        tree.push_str(&row_summary(
            &cases,
            label,
            &group_id,
            None,
            0,
            "group",
            group == "diagnostic",
        ));
        if matches!(group, "clear" | "diagnostic") {
            for row in cases {
                tree.push_str(&mission_row(row, &group_id, 1));
            }
        } else {
            // Stable first-seen ordering follows the frozen input pack; no new selector dimension.
            let mut seen = BTreeSet::new();
            for row in &cases {
                if !seen.insert(row.family.as_str()) {
                    continue;
                }
                let family_rows = cases
                    .iter()
                    .copied()
                    .filter(|r| r.family == row.family)
                    .collect::<Vec<_>>();
                let family_id = format!("{group_id}-{}", row.family);
                tree.push_str(&row_summary(
                    &family_rows,
                    &row.family.replace('_', " "),
                    &family_id,
                    Some(&group_id),
                    1,
                    "family",
                    false,
                ));
                for row in family_rows {
                    tree.push_str(&mission_row(row, &family_id, 2));
                }
            }
        }
    }
    use pd_report::batch;
    let s = &report.summary;
    let core = rows
        .iter()
        .filter(|r| r.group != "diagnostic")
        .collect::<Vec<_>>();
    let diagnostics = rows
        .iter()
        .filter(|r| r.group == "diagnostic")
        .collect::<Vec<_>>();
    let core_class = acceptance.status.css_class();
    let mut overview_rows = String::new();
    for (name, cases, result) in [
        (
            "Core evaluation",
            &core,
            format!(
                "<div class=\"overview-main {core_class}\">{}/36 target landings · {}</div><div class=\"overview-sub\">{} direct · {} corrected · {} non-landings</div>",
                s.valid_landing_count,
                acceptance.status.label(),
                s.direct_landing_count,
                s.corrected_landing_count,
                s.non_landing_count
            ),
        ),
        (
            "Diagnostics",
            &diagnostics,
            format!(
                "<div class=\"overview-main\">{} landed · {} supported non-landings · {} unsupported</div><div class=\"overview-sub\">{} crashes · separate denominator</div>",
                s.diagnostic_landing_count,
                s.diagnostic_non_landing_count,
                s.unsupported_count,
                s.crash_count
            ),
        ),
    ] {
        let pack = format!(
            "<div class=\"overview-main\">{name}</div><div class=\"overview-sub\"><code>{}</code></div>",
            escape_html(&report.pack_id)
        );
        let scope = format!("{} cases", cases.len());
        let timing = format!(
            "<div class=\"overview-main\">{} flight</div><div class=\"overview-sub\">{} planning mean</div>",
            aggregate_metric(cases.iter().filter_map(|r| r.flight_s), "s", 2),
            metric(mean(cases.iter().filter_map(|r| r.planning_s)), "s", 3)
        );
        let fuel = aggregate_metric(cases.iter().filter_map(|r| r.fuel_used_pct), "%", 1);
        overview_rows.push_str(&batch::render_overview_row(
            "current-summary-row",
            &[
                &pack,
                "Saved capture · no comparison baseline",
                &scope,
                &result,
                &timing,
                &fuel,
                "— · no recorded controller reference metric",
            ],
        ));
    }
    let overview_table = batch::render_overview_table(
        "<tr><th>Pack</th><th>Reference</th><th>Scope</th><th>Result</th><th>Timing</th><th>Efficiency</th><th>Reference / recovery</th></tr>",
        &overview_rows,
    );
    let overview = batch::render_overview_section("", &overview_table);

    let mut coverage_rows = String::new();
    for (group, label) in [
        ("clear", "Clear controls"),
        ("ordinary", "Ordinary terrain"),
        ("additional_terrain", "Additional terrain"),
        ("diagnostic", "Diagnostics"),
    ] {
        let group_cases = rows.iter().filter(|r| r.group == group).collect::<Vec<_>>();
        let mut seen = BTreeSet::new();
        for case in &group_cases {
            if !seen.insert(case.family.as_str()) {
                continue;
            }
            let cases = group_cases
                .iter()
                .copied()
                .filter(|r| r.family == case.family)
                .collect::<Vec<_>>();
            let landed = cases.iter().filter(|r| r.landed).count();
            let unsupported = cases.iter().filter(|r| r.unsupported).count();
            let direct = cases
                .iter()
                .filter(|r| r.landed && r.correction_count == 0)
                .count();
            let target = if matches!(group, "clear" | "diagnostic") {
                format!("v2-{group}")
            } else {
                format!("v2-{group}-{}", case.family)
            };
            let contents = if group == "diagnostic" {
                format!(
                    "<strong>{landed} landed</strong><span>{} stopped · {unsupported} unsupported · separate denominator</span>",
                    cases.len() - landed - unsupported
                )
            } else {
                format!(
                    "<strong>{landed}/{} target landings</strong><span>{direct} direct · {} corrected</span>",
                    cases.len(),
                    landed - direct
                )
            };
            let cell = batch::render_coverage_group_cell(
                if landed == cases.len() {
                    ""
                } else {
                    "has-failure"
                },
                &target,
                &contents,
            );
            let handoffs = cases.iter().map(|r| r.correction_count).sum::<u32>();
            let counts = format!(
                "{cell}<td><div class=\"overview-stack\">{handoffs} H<span class=\"row-note muted\">executed handoffs</span></div></td>"
            );
            let branch_label = if matches!(group, "clear" | "diagnostic") {
                label.to_owned()
            } else {
                format!("{label} / {}", case.family.replace('_', " "))
            };
            coverage_rows.push_str(&batch::render_coverage_row(&branch_label, &counts));
        }
    }
    let coverage_table = batch::render_coverage_table(
        "Terrain group / family",
        "<th>Recorded results</th><th data-optional=\"handoffs\">Executed handoffs</th>",
        &coverage_rows,
    );
    let coverage_pane = batch::render_coverage_pane(
        "planner_v2",
        "recorded",
        "current",
        "terrain_families",
        &coverage_table,
    );
    let coverage = batch::render_coverage_section(
        "Terrain-family coverage; diagnostics remain separate from core landing coverage",
        "",
        &coverage_pane,
    );

    let stops = s
        .planning_stops
        .iter()
        .map(|(k, n)| format!("{}: {n}", escape_html(k)))
        .collect::<Vec<_>>()
        .join(" · ");
    let outcomes = s
        .physical_outcomes
        .iter()
        .map(|(k, n)| format!("{}: {n}", escape_html(k)))
        .collect::<Vec<_>>()
        .join(" · ");
    let mut context_rows = String::new();
    for (label, value) in [
        ("Policy", format!("{} · native Planner V2", report.policy_version)),
        ("Supported source replay", format!("{}/42 supported source replays passed · unsupported cases are not replay claims", s.final_source_replay_passed_count)),
        ("Integrity", format!("{}/44 integrity passed", s.integrity_passed_count)),
        ("Planning stops", stops),
        ("Physical outcomes", outcomes),
        ("Metric basis", "Fuel and flight-time metrics use departed flights; landing offsets use verified target landings; planning uses supported cases. Aggregates use population mean ± standard deviation. Fuel is percent of vehicle maximum.".to_owned()),
        ("Waypoint meaning", "H markers are actual executed correction-segment endpoints, where planning restarted; they are not authored waypoint targets.".to_owned()),
        ("Reference / recovery", "Unavailable where no measured controller reference metric was recorded; no synthetic reference or recovery success is inferred.".to_owned()),
    ] { context_rows.push_str(&batch::render_context_row(&[label, &value])); }
    context_rows.push_str(&batch::render_context_row(&[
        "Acceptance verdict",
        &format!(
            "<strong id=\"planner-v2-acceptance-status\" class=\"{}\">{}</strong> · schema <code>{}</code>",
            acceptance.status.css_class(),
            acceptance.status.label(),
            escape_html(&acceptance.schema_id)
        ),
    ]));
    if acceptance.issues.is_empty() {
        context_rows.push_str(&batch::render_context_row(&[
            "Acceptance issues",
            "None; all declared gates passed.",
        ]));
    } else {
        context_rows.push_str(&batch::render_context_row(&[
            "Acceptance issues",
            "Resolve these capture or case findings before publication.",
        ]));
        for issue in &acceptance.issues {
            let case_link = issue.case_id.as_ref().and_then(|case_id| {
                report
                    .cases
                    .iter()
                    .find(|case| &case.case_id == case_id)
                    .map(|case| {
                        let href = preview_base.map_or_else(
                            || case.annotated_report_path.clone(),
                            |base| format!("{base}{}", case.annotated_report_path),
                        );
                        format!(
                            "<a href=\"{}\">{}</a>",
                            escape_html(&href),
                            escape_html(case_id)
                        )
                    })
            });
            let case_cell = case_link.unwrap_or_else(|| "Batch".into());
            let detail = format!(
                "<code>{}</code>: {}",
                escape_html(&issue.code),
                escape_html(&issue.message)
            );
            context_rows.push_str(&batch::render_context_row(&[&case_cell, &detail]));
        }
    }
    let context_table = batch::render_context_table(
        "<tr><th>Context / case</th><th>Recorded evidence / finding</th></tr>",
        &context_rows,
    );
    let attention = !acceptance.passed;
    let context = batch::render_context_section(
        attention,
        attention,
        acceptance.status.css_class(),
        &format!("Planner V2 acceptance · {}", acceptance.status.label()),
        &context_table,
    );
    let headers = "<tr><th>Selector</th><th>Success / Outcome</th><th>Fuel Used</th><th>Flight Time</th><th>Landing Offset</th><th>Reference deviation</th><th>Preview</th><th data-optional=\"handoffs\">Executed handoffs</th></tr>";
    let table = batch::render_review_tree_table("v2", headers, &tree);
    let tree_section = batch::render_tree_table_section(
        &format!("<code>{}</code>", escape_html(&report.pack_id)),
        &format!(
            "{}/36 core target landings · 8 separate diagnostics · {} executed handoffs · acceptance {}",
            s.valid_landing_count,
            rows.iter().map(|row| row.correction_count).sum::<u32>(),
            acceptance.status.label()
        ),
        &table,
    );
    let tree_stack = format!("<div class=\"tree-stack\">{tree_section}</div>");
    let controls = pd_report::batch_tree::render_controls("Missions", false);
    let review = batch::render_review_tree_section(
        &controls,
        "<p class=\"section-note\">Expand a group or family to inspect missions. H means executed handoff; unavailable measurements are shown as —.</p>",
        &tree_stack,
    );
    let title = format!("{} batch report", report.name);
    let subtitle = format!(
        "Native policy {} planner evaluation · saved {}-case capture · no new flights",
        report.policy_version, report.case_count
    );
    let chips = format!(
        "<span class=\"chip\"><strong>pack</strong><span class=\"mono\">{}</span></span><span class=\"chip\"><strong>runs</strong>{}</span><span class=\"chip\"><strong>policy</strong>{}</span><span class=\"chip\"><strong>spec</strong><span class=\"mono\">{}</span></span><span class=\"chip\"><strong>resolved</strong><span class=\"mono\">{}</span></span>",
        escape_html(&report.pack_id),
        report.case_count,
        report.policy_version,
        escape_html(&report.pack_snapshot_sha256[..8]),
        escape_html(&report.expanded_inputs_snapshot_sha256[..8])
    );
    let actions = format!(
        "<a href=\"{}summary.json\">summary.json</a><a href=\"{}expanded-inputs.json\">expanded-inputs.json</a><a href=\"{}pack.json\">pack.json</a>",
        escape_html(source_base),
        escape_html(source_base),
        escape_html(source_base)
    );
    let sources = report
        .input_identity
        .source_fixture_sha256
        .iter()
        .chain(&report.input_identity.source_manifest_sha256)
        .map(|(p, h)| {
            batch::render_context_row(&[
                &escape_html(p),
                &format!("<code>{}</code>", escape_html(h)),
            ])
        })
        .collect::<String>();
    let source_table =
        batch::render_context_table("<tr><th>Sealed source</th><th>SHA-256</th></tr>", &sources);
    let provenance = escape_html(&serde_json::to_string_pretty(&report.provenance)?);
    let data = serde_json::to_string(&rows)?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let acceptance_data = serde_json::to_string(&acceptance)?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let appendix = format!(
        "<details class=\"header-context\" id=\"provenance\"><summary>Input identity and provenance</summary><p>Typed expansion digest: <code>{}</code></p><p>Pack snapshot: <code>{}</code> · Expanded inputs: <code>{}</code></p><pre>{provenance}</pre><div class=\"table-wrap\">{source_table}</div></details><script id=\"planner-v2-acceptance\" type=\"application/json\">{acceptance_data}</script><script>const batchTreeData = {data};</script>",
        escape_html(&report.input_identity.rust_typed_expanded_inputs_sha256),
        escape_html(&report.pack_snapshot_sha256),
        escape_html(&report.expanded_inputs_snapshot_sha256)
    );
    let navigation = "<nav><a href=\"/reports/\">Report home</a> · <a href=\"/reports/topics/waypoint-planning/index.html\">Waypoint planning</a></nav>";
    let mut html = batch::render_batch_page(batch::BatchPage {
        title: &title,
        subtitle: &subtitle,
        chips_html: &chips,
        actions_html: &actions,
        before_hero_html: navigation,
        after_hero_html: "",
        overview_html: &overview,
        planner_html: "",
        coverage_html: &coverage,
        context_html: &context,
        diagnostics_html: "",
        review_tree_html: &review,
        comparison_html: "",
        appendix_html: &appendix,
        body_class: "",
        tree: batch::BatchTreeOptions {
            max_depth: 1,
            depth_by_kind: &[("group", 0), ("family", 1)],
            leaf_selector: "tr.mission-row",
            default_expansion: 1,
        },
        coverage_tree_jump: true,
    });
    if let Some(base) = preview_base {
        html = html.replacen(
            "<head>",
            &format!("<head><base href=\"{}\">", escape_html(base)),
            1,
        );
    }
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_metrics_are_not_successful_zero_values() {
        assert!(metric(None, "s", 1).contains('—'));
        assert_eq!(mean(std::iter::empty()), None);
        assert_eq!(mean([1.0, 3.0].into_iter()), Some(2.0));
    }
    #[test]
    fn mission_text_and_urls_are_escaped() {
        let row = TreeCase {
            case_id: "<script>".into(),
            group: "clear".into(),
            family: "clear".into(),
            label: "<unsafe>".into(),
            detail_href: "a\"b".into(),
            display_outcome: "Unsupported — not simulated".into(),
            departed: false,
            landed: false,
            unsupported: true,
            integrity_passed: true,
            replay_passed: None,
            correction_count: 0,
            fuel_used_pct: None,
            flight_s: None,
            landing_offset_m: None,
            planning_s: None,
            handoffs: vec![],
            preview: "<svg></svg>".into(),
        };
        let html = mission_row(&row, "parent", 1);
        assert!(!html.contains("<unsafe>"));
        assert!(html.contains("a&quot;b"));
        assert!(html.contains("not simulated"));
        assert!(!html.contains("Direct ·"));
    }
}
