//! Local, saved-evidence-only diagnostic batch. Does not publish or run flights.
use std::{collections::BTreeMap, fs, io::Write, path::Path};

use anyhow::{Context, Result, ensure};
use pd_core::{ScenarioSpec, Vec2};
use pd_eval::ballistic_feedback::FeedbackResult;
use pd_report::batch::{self, BatchPage, BatchTreeOptions};
use serde_json::Value;

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn text(v: &Value, key: &str) -> String {
    v[key]
        .as_str()
        .map_or_else(|| v[key].to_string(), str::to_owned)
}

fn branch(id: &str, parent: Option<&str>, depth: usize, label: &str, count: usize) -> String {
    let parent = parent.map_or_else(String::new, |p| format!(" data-parent=\"{p}\" hidden"));
    batch::render_row(
        Some("summary-row current-row"),
        &format!(
            "data-group=\"{id}\"{parent} data-kind=\"group\" data-depth=\"{depth}\" aria-expanded=\"false\" tabindex=\"0\""
        ),
        &format!(
            "<td class=\"tree-label\" style=\"--depth:{depth}\"><span class=\"expander\">+</span> {}</td><td colspan=\"6\">{count} cases</td>",
            escape(label)
        ),
    )
}

fn leaf(root: &Path, row: &Value, parent: &str) -> Result<String> {
    let id = text(row, "attempt_id");
    ensure!(
        !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "unsafe case ID"
    );
    let result = &row["result"];
    let detail = format!("runs/{id}/report.html");
    let preview = if row["status"] == "recorded" {
        let feedback: FeedbackResult =
            serde_json::from_slice(&fs::read(root.join(format!("runs/{id}/feedback.json")))?)?;
        let scenario: ScenarioSpec =
            serde_json::from_slice(&fs::read(root.join(format!("runs/{id}/scenario.json")))?)?;
        let handoffs: Vec<Vec2> = feedback.handoffs.iter().map(|s| s.position_m).collect();
        // No reconstructed manifest: actual saved samples and H only. Outcome
        // text below comes from the verified native physical/mission tuple.
        let svg = pd_report::build_saved_flight_preview_svg(
            &scenario,
            None,
            &feedback.ordinary_flight.samples,
            &handoffs,
        );
        format!("<a class=\"run-preview\" href=\"{detail}\">{svg}</a>")
    } else {
        "<span class=\"row-note\">No verified flight; inspect retained logs.</span>".into()
    };
    let label = if row["status"] == "recorded" {
        format!(
            "<a class=\"mission-link\" href=\"{detail}\">{}</a>",
            escape(&id)
        )
    } else {
        escape(&id)
    };
    let baseline = &row["baseline_result"];
    let baseline_text = if baseline.is_null() {
        "No paired baseline".into()
    } else {
        let cohort = baseline
            .get("nominal_class")
            .or_else(|| baseline.get("initial_candidate_arc"))
            .map_or_else(
                || "unclassified".into(),
                |v| escape(v.as_str().unwrap_or("unclassified")),
            );
        format!(
            "{} · baseline {}",
            cohort,
            text(baseline, "verified_landing")
        )
    };
    Ok(batch::render_row(
        Some("seed-row mission-row current-row"),
        &format!("data-parent=\"{parent}\" data-case-id=\"{id}\" data-depth=\"2\" hidden"),
        &format!(
            "<td class=\"tree-label\" style=\"--depth:2\"><div class=\"preview-cell\">{label}<span class=\"row-note\">seed {} · {}</span>{preview}</div></td><td>{}<span class=\"row-note\">{}</span></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            escape(&text(row, "seed")),
            escape(&text(&row["geometry"], "recipe_id")),
            escape(&text(result, "physical_outcome")),
            escape(&text(result, "mission_outcome")),
            escape(&text(result, "planning_stop")),
            text(result, "handoffs"),
            text(result, "sim_time_s"),
            baseline_text,
            escape(&text(result, "initial_candidate_arc")),
        ),
    ))
}

fn render(root: &Path, report: &Value) -> Result<String> {
    let rows = report["rows"].as_array().context("missing rows")?;
    let mut groups: BTreeMap<(usize, String), Vec<&Value>> = BTreeMap::new();
    for row in rows {
        let result = &row["result"];
        let (category, mechanism) = if row["cohort"] == "repeat" {
            (3, "Exact repeat checks".into())
        } else if row["status"] != "recorded" {
            (0, text(row, "status"))
        } else if result["verified_landing"] == true {
            (
                2,
                if result["handoffs"] == 0 {
                    "Landed with no handoff".into()
                } else {
                    "Landed after waypoint handoff".into()
                },
            )
        } else {
            (1, text(result, "stop_group"))
        };
        groups.entry((category, mechanism)).or_default().push(row);
    }
    let labels = [
        "Collection / evidence issues",
        "Did not land — inspect first",
        "Verified landings",
        "Repeats (outside 1k denominator)",
    ];
    let mut tree = String::new();
    for (category, label) in labels.iter().enumerate() {
        let count = groups
            .iter()
            .filter(|((c, _), _)| *c == category)
            .map(|(_, r)| r.len())
            .sum();
        if count == 0 {
            continue;
        }
        let parent = format!("category-{category}");
        tree.push_str(&branch(&parent, None, 0, label, count));
        for (index, ((_, mechanism), cases)) in groups
            .iter()
            .filter(|((c, _), _)| *c == category)
            .enumerate()
        {
            let group = format!("mechanism-{category}-{index}");
            tree.push_str(&branch(&group, Some(&parent), 1, mechanism, cases.len()));
            for row in cases {
                tree.push_str(&leaf(root, row, &group)?);
            }
        }
    }
    let table = batch::render_review_tree_table(
        "ballistic-diagnostic",
        "<tr><th>Case / actual flight</th><th>Physical / mission</th><th>Planner stop</th><th>Actual H</th><th>Seconds</th><th>Original baseline cohort</th><th>Candidate initial arc</th></tr>",
        &tree,
    );
    let review = batch::render_review_tree_section(
        &pd_report::batch_tree::render_controls("Missions", false),
        "<p class=\"section-note\">Expand a stop mechanism to see its cases. Previews are actual saved trajectories and H markers. Open a case for the full rich report and refresh-by-refresh desired, ballistic, and short-command views.</p>",
        &table,
    );
    let stats = &report["summary"];
    let denominator = text(stats, "primary_count");
    let baseline_chip = if stats["baseline_landings"].is_null() {
        "No paired baseline".into()
    } else {
        format!(
            "Baseline {}/{denominator}",
            text(stats, "baseline_landings")
        )
    };
    let chips = format!(
        "<span class=\"chip\">Diagnostic candidate — NOT accepted</span><span class=\"chip\">{}/{} verified landings</span><span class=\"chip\">{baseline_chip}</span><span class=\"chip\">Cap 24 · unchanged inputs</span>",
        text(stats, "verified_landings"),
        text(stats, "primary_count"),
    );
    let candidate_comparison = if let Some(previous) = stats.get("previous_candidate_landings") {
        let older = if report.get("diagnostic_context").is_none() {
            format!(
                " This is separate from the older policy-3 baseline of {}/{denominator};",
                text(stats, "baseline_landings")
            )
        } else {
            String::new()
        };
        format!(
            "<section class=\"panel\"><h2>Compared with the previous ballistic candidate</h2><p>Previous: {}/{denominator} verified landings. This revision gains {} and loses {}.{older} inspect the results JSON for every paired case ID.</p></section>",
            escape(&previous.to_string()),
            stats["gained_previous_candidate_landings"]
                .as_array()
                .map_or(0, Vec::len),
            stats["lost_previous_candidate_landings"]
                .as_array()
                .map_or(0, Vec::len),
        )
    } else if report.get("diagnostic_context").is_some()
        && stats["baseline_landings"].is_number()
        && stats["gained_landings"].is_array()
    {
        format!(
            "<section class=\"panel\"><h2>Same-world paired comparison</h2><p>Acquisition-only: {}/{denominator} verified landings. Combined gains {} and loses {}. Inspect the results JSON for every paired case ID.</p></section>",
            text(stats, "baseline_landings"),
            stats["gained_landings"].as_array().map_or(0, Vec::len),
            stats["lost_landings"].as_array().map_or(0, Vec::len),
        )
    } else {
        String::new()
    };
    let context = if let Some(context) = report.get("diagnostic_context") {
        format!(
            "<section class=\"panel\"><h2>Population and comparison</h2><p>{}</p><p>{}</p></section>",
            escape(&text(context, "population")),
            escape(&text(context, "comparison")),
        )
    } else {
        String::new()
    };
    let summary = format!(
        "{candidate_comparison}<section class=\"panel\"><h2>Paired diagnostic result</h2><p>Recorded {}/{denominator} primary cases. {} zero-H landings; {} landings after H. Frozen opt-in candidate; original baseline clear/blocked labels are not the candidate's own arc classification.</p><p>Collection stop: {}. {} external same-world repeats are outside the primary denominator; every native attempt separately repeats decisions and replays commands. A selected mechanism panel is development evidence, not a fresh population estimate.</p><h3>Stop counts</h3><pre>{}</pre><h3>Terrain recipes</h3><pre>{}</pre></section>",
        text(stats, "recorded_count"),
        text(stats, "zero_h_landings"),
        text(stats, "waypoint_landings"),
        escape(&text(report, "stopped_reason")),
        text(stats, "repeats_recorded"),
        escape(&serde_json::to_string_pretty(&stats["planning_stops"])?),
        escape(&serde_json::to_string_pretty(&stats["per_recipe"])?)
    );
    let title = report["diagnostic_context"]["title"].as_str().map_or_else(
        || format!("Ballistic feedback — paired {denominator}-world diagnostic"),
        str::to_owned,
    );
    let subtitle = report["diagnostic_context"]["population"].as_str().unwrap_or(
        "Original procedural worlds; frozen diagnostic candidate; failures first. Maintained policy 3 and accepted reports remain unchanged."
    );
    Ok(batch::render_batch_page(BatchPage {
        title: &title,
        subtitle,
        chips_html: &chips,
        actions_html: "<a href=\"ballistic-feedback-sweep.json\">Results / all case IDs</a><a href=\"manifest.json\">Frozen manifest</a><a href=\"plan.json\">Collection plan</a><a href=\"receipt.json\">Receipt</a>",
        before_hero_html: "",
        after_hero_html: "",
        overview_html: &format!("{context}{summary}"),
        planner_html: "",
        coverage_html: "",
        context_html: "",
        diagnostics_html: "",
        review_tree_html: &review,
        comparison_html: "",
        appendix_html: "",
        body_class: "mission-preview-batch",
        tree: BatchTreeOptions {
            max_depth: 2,
            depth_by_kind: &[("group", 1)],
            leaf_selector: "tr.seed-row",
            default_expansion: 1,
        },
        coverage_tree_jump: false,
    }))
}

fn main() -> Result<()> {
    let root = std::env::args_os()
        .nth(1)
        .context("usage: ballistic_feedback_batch_report CAPTURE")?;
    let root = Path::new(&root);
    let report: Value =
        serde_json::from_slice(&fs::read(root.join("ballistic-feedback-sweep.json"))?)?;
    let html = render(root, &report)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("index.html"))?;
    output.write_all(html.as_bytes())?;
    println!(
        "Rendered local common-template diagnostic batch; no publication or flight execution."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_shell_and_no_false_flight_for_unattempted_case() {
        let report = serde_json::json!({ "rows": [{"cohort":"random", "status":"not_attempted", "attempt_id":"random-000", "baseline_result":{}, "geometry":{}}], "summary":{} });
        let html = render(Path::new("unused"), &report).unwrap();
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(html.contains("No verified flight"));
        assert!(!html.contains("class=\"run-preview\""));
        assert!(!html.contains("href=\"runs/random-000/report.html\""));
        assert!(html.contains("NOT accepted"));
    }

    #[test]
    fn selected_panels_keep_their_denominator_and_safe_control_identifiers() {
        let report = serde_json::json!({ "rows": [{"cohort":"control", "status":"not_attempted", "attempt_id":"v2_clear_845", "baseline_result":{}, "geometry":{}}], "summary":{"primary_count":16} });
        let html = render(Path::new("unused"), &report).unwrap();
        assert!(html.contains("paired 16-world diagnostic"));
        assert!(!html.contains("/1000"));
        let mut unsafe_report = report;
        unsafe_report["rows"][0]["attempt_id"] = "../escape".into();
        assert!(render(Path::new("unused"), &unsafe_report).is_err());
    }

    #[test]
    fn fresh_unpaired_reports_do_not_invent_a_baseline() {
        let report = serde_json::json!({
            "rows": [], "summary": {"primary_count":1000, "baseline_landings":null},
            "diagnostic_context":{"population":"Fresh seed-disjoint worlds <test>",
                                  "comparison":"Acquisition-only; no previous same-world outcome"}
        });
        let html = render(Path::new("unused"), &report).unwrap();
        assert!(html.contains("No paired baseline"));
        assert!(!html.contains("Baseline null"));
        assert!(html.contains("Fresh seed-disjoint worlds &lt;test&gt;"));
        assert!(html.contains("data-batch-template=\"common-v1\""));
    }
}
