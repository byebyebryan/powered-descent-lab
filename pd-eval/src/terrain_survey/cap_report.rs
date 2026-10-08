//! Presentation of the explicit cap-24 experiment, not production admission.
//! The study's read-only verifier owns its paired evidence contract. Detail
//! pages extend authenticated rich HTML with navigation and a saved planning
//! summary; existing rich plots, scripts and numeric payload are preserved.

use std::{collections::BTreeMap, process::Command};

use super::*;
use crate::evidence_io::write_json_create_only;
use crate::waypoint_v2_report::{execution_status, nominal_conflict_phase, with_planning_review};

#[derive(Debug, Deserialize)]
struct CapSweep {
    schema: String,
    rows: Vec<CapAttempt>,
    summary: Value,
    paired_summary: Value,
    stopped_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CapAttempt {
    #[serde(flatten)]
    attempt: Attempt,
    paired: Paired,
}

#[derive(Debug, Deserialize)]
struct Paired {
    baseline_result: Projection,
}

#[derive(Default)]
struct ExecutionCounts {
    not_started: usize,
    in_progress: usize,
    crashed: usize,
}

fn category(p: &Projection) -> String {
    if p.verified_landing {
        if p.correction_count == 0 {
            "Direct landing".into()
        } else {
            "Corrected landing".into()
        }
    } else {
        format!("Finite planning stop · {}", enum_text(&p.planning_stop))
    }
}

fn outcome(p: &Projection) -> String {
    if p.verified_landing {
        "Verified target landing".into()
    } else {
        format!(
            "{} · physical {} · mission {}",
            enum_text(&p.planning_stop),
            p.physical_outcome
                .as_ref()
                .map_or_else(|| "none".into(), enum_text),
            p.mission_outcome
                .as_ref()
                .map_or_else(|| "none".into(), enum_text),
        )
    }
}

fn navigation(
    rows: &[CapAttempt],
    index: usize,
    site: &str,
    base: &str,
    baseline: &str,
) -> AnnotationNavigation {
    let row = &rows[index].attempt;
    let link = |label: &str, href: String| NavigationLink {
        label: label.into(),
        href,
    };
    let detail = |i: usize| format!("{site}runs/{}/index.html", rows[i].attempt.attempt_id);
    AnnotationNavigation {
        home: Some(link("Report home", "/reports/".into())),
        collection: Some(link("1,000-world batch", format!("{site}index.html"))),
        previous: index
            .checked_sub(1)
            .map(|i| link("Previous case", detail(i))),
        next: rows
            .get(index + 1)
            .map(|_| link("Next case", detail(index + 1))),
        source_links: vec![
            link(
                "Scenario JSON",
                format!("{base}runs/{}/scenario.json", row.attempt_id),
            ),
            link(
                "Flight JSON",
                format!("{base}runs/{}/flight.json", row.attempt_id),
            ),
            link(
                "Run summary JSON",
                format!("{base}runs/{}/summary.json", row.attempt_id),
            ),
            link(
                "Original rich report",
                format!("{base}runs/{}/report.html", row.attempt_id),
            ),
            link(
                "Baseline cap-6 report",
                format!("{baseline}runs/{}/report.html", row.case_id),
            ),
            link("Paired sweep JSON", format!("{base}cap-sweep.json")),
        ],
    }
}

/// Keep the entire numeric payload and all rich panels/scripts byte-for-byte;
/// only the native empty navigation slot and relative artifact links change.
fn navigable_copy(html: &str, nav: &AnnotationNavigation) -> Result<String> {
    let slot = "<nav aria-label=\"Flight report navigation\"></nav>";
    ensure!(
        html.matches(slot).count() == 1,
        "unexpected saved navigation slot"
    );
    let mut original = AnnotationNavigation::default();
    for (label, href) in [
        ("Scenario JSON", "scenario.json"),
        ("Flight JSON", "flight.json"),
        ("Run summary JSON", "summary.json"),
    ] {
        original.source_links.push(NavigationLink {
            label: label.into(),
            href: href.into(),
        });
    }
    let old = serde_json::to_string(&original)?;
    ensure!(
        html.matches(&old).count() == 1,
        "unexpected saved navigation payload"
    );
    let links = nav
        .home
        .iter()
        .chain(nav.collection.iter())
        .chain(nav.previous.iter())
        .chain(nav.next.iter())
        .map(|l| format!("<a href=\"{}\">{}</a>", escape(&l.href), escape(&l.label)))
        .collect::<String>();
    let mut copy = html.replacen(slot, &format!("<nav aria-label=\"Flight report navigation\">{links}<span>Experimental cap 24 · production remains cap 6</span></nav>"), 1)
        .replacen(&old, &serde_json::to_string(nav)?, 1);
    for (old, new) in original.source_links.iter().zip(&nav.source_links) {
        copy = copy.replace(
            &format!("href=\"{}\"", old.href),
            &format!("href=\"{}\"", escape(&new.href)),
        );
    }
    Ok(copy)
}

fn leaf(
    row: &CapAttempt,
    flight: &WaypointV2FlightResult,
    scenario: &ScenarioSpec,
    baseline: &str,
) -> Result<String> {
    let a = &row.attempt;
    let p = a.result.as_ref().context("missing recorded projection")?;
    let manifest = flight.manifest.as_ref().context("missing manifest")?;
    let ordinary = flight.ordinary_flight.as_ref().context("missing flight")?;
    let handoffs = flight
        .segments
        .iter()
        .filter(|s| s.kind == crate::WaypointV2SegmentKind::LocalCorrection)
        .map(|s| s.end_state.position_m)
        .collect::<Vec<_>>();
    let preview = pd_report::build_saved_flight_preview_svg(
        scenario,
        Some(manifest),
        &ordinary.samples,
        &handoffs,
    );
    let old = &row.paired.baseline_result;
    let (_, execution) = execution_status(flight);
    let conflict = flight
        .cycles
        .last()
        .filter(|c| c.audit.as_ref().is_some_and(|a| !a.passed))
        .map(|c| format!(" · proposed conflict: {}", nominal_conflict_phase(c)))
        .unwrap_or_default();
    Ok(format!(
        "<td class=\"tree-label\"><a class=\"mission-link\" href=\"runs/{id}/index.html\">{id}</a><span class=\"row-note\">seed {seed}</span></td><td>{outcome}<span class=\"row-note\">{execution}{conflict}</span><span class=\"row-note\">Recorded integrity / source replay passed</span></td><td><a href=\"{baseline}runs/{case}/report.html\">{previous}</a><span class=\"row-note\">{old_h} H</span></td><td>{fuel:.3} kg</td><td>{time:.3} s</td><td>{offset}</td><td>— · no reference-controller comparison</td><td><a class=\"run-preview\" href=\"runs/{id}/index.html\">{preview}</a></td><td>{nominal}</td><td>{handoffs} H</td><td>{planning:.3} s</td>",
        id = a.attempt_id,
        case = a.case_id,
        seed = a.seed.map_or_else(|| "control".into(), |s| s.to_string()),
        outcome = escape(&outcome(p)),
        previous = escape(&outcome(old)),
        old_h = old.correction_count,
        fuel = manifest.summary.fuel_used_kg,
        time = manifest.sim_time_s,
        offset = if p.verified_landing {
            metric(
                manifest
                    .summary
                    .landing
                    .as_ref()
                    .map(|l| l.touchdown_center_offset_m),
                "m",
            )
        } else {
            "— not landed".into()
        },
        nominal = escape(&p.nominal_class),
        handoffs = p.correction_count,
        planning = flight.timings.planning_s,
        conflict = escape(&conflict),
    ))
}

fn branch(id: &str, parent: Option<&str>, depth: usize, label: &str, count: usize) -> String {
    let parent = parent.map_or_else(String::new, |p| format!(" data-parent=\"{p}\" hidden"));
    batch::render_row(
        Some("summary-row current-row"),
        &format!(
            "data-group=\"{id}\"{parent} data-kind=\"group\" data-depth=\"{depth}\" aria-expanded=\"false\" tabindex=\"0\""
        ),
        &format!(
            "<td class=\"tree-label\" style=\"--depth:{depth}\"><span class=\"expander\">+</span> {}</td><td colspan=\"10\">{count} cases · expand to inspect</td>",
            escape(label)
        ),
    )
}

fn tree(rows: &[CapAttempt], cells: &[String]) -> String {
    let mut html = branch(
        "random",
        None,
        0,
        "Primary worlds · 1,000-case denominator",
        rows.iter().filter(|r| r.attempt.cohort == "random").count(),
    );
    let mut groups = BTreeMap::<String, BTreeMap<String, Vec<usize>>>::new();
    for (i, r) in rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.attempt.cohort == "random")
    {
        let recipe = r.attempt.geometry["recipe_id"]
            .as_str()
            .unwrap_or("Unknown recipe");
        groups
            .entry(recipe.into())
            .or_default()
            .entry(category(
                r.attempt.result.as_ref().expect("verified projection"),
            ))
            .or_default()
            .push(i);
    }
    for (i, (recipe, outcomes)) in groups.iter().enumerate() {
        let recipe_id = format!("recipe-{i}");
        html.push_str(&branch(
            &recipe_id,
            Some("random"),
            1,
            recipe,
            outcomes.values().map(Vec::len).sum(),
        ));
        for (j, (label, indices)) in outcomes.iter().enumerate() {
            let outcome_id = format!("outcome-{i}-{j}");
            html.push_str(&branch(
                &outcome_id,
                Some(&recipe_id),
                2,
                label,
                indices.len(),
            ));
            for &index in indices {
                html.push_str(&batch::render_row(Some("seed-row mission-row current-row"),
                    &format!("data-parent=\"{outcome_id}\" data-case-id=\"{}\" data-depth=\"3\" style=\"--depth:3\" hidden", rows[index].attempt.attempt_id), &cells[index]));
            }
        }
    }
    for (cohort, label) in [
        ("sentinel", "Preservation controls · separate denominator"),
        ("repeat", "Exact repeats · separate denominator"),
    ] {
        html.push_str(&branch(
            cohort,
            None,
            0,
            label,
            rows.iter().filter(|r| r.attempt.cohort == cohort).count(),
        ));
        for (i, row) in rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.attempt.cohort == cohort)
        {
            html.push_str(&batch::render_row(Some("seed-row mission-row current-row"),
                &format!("data-parent=\"{cohort}\" data-case-id=\"{}\" data-depth=\"1\" style=\"--depth:1\" hidden", row.attempt.attempt_id), &cells[i]));
        }
    }
    html
}

fn page(
    sweep: &CapSweep,
    cells: &[String],
    execution: &ExecutionCounts,
    base: &str,
    baseline: &str,
) -> String {
    let summary = &sweep.summary;
    let paired = &sweep.paired_summary;
    let mut overview = String::new();
    for (scope, count, result) in [
        (
            "Original · cap 6",
            summary["primary_count"].to_string(),
            format!("{} verified landings", paired["baseline_landings"]),
        ),
        (
            "Experiment · cap 24",
            summary["primary_count"].to_string(),
            format!(
                "{} verified landings · {} preserved · {} added",
                summary["verified_landings"],
                paired["preserved_landings"],
                paired["new_landings"].as_array().map_or(0, Vec::len)
            ),
        ),
        (
            "Preservation controls",
            summary["sentinels_recorded"].to_string(),
            "Verified · excluded from primary rate".into(),
        ),
        (
            "Exact repeats",
            summary["repeats_recorded"].to_string(),
            "Agree except wall timings · excluded from primary rate".into(),
        ),
    ] {
        overview.push_str(&batch::render_overview_row(
            "current-summary-row",
            &[
                scope,
                "Same saved worlds",
                &count,
                &result,
                "Recorded evidence",
                "See mission details",
                "No reference controller",
            ],
        ));
    }
    let overview = batch::render_overview_section(
        "",
        &batch::render_overview_table(
            "<tr><th>Cohort</th><th>Reference</th><th>Scope</th><th>Result</th><th>Timing</th><th>Efficiency</th><th>Reference / recovery</th></tr>",
            &overview,
        ),
    );
    let mut coverage_rows = String::new();
    for (recipe, value) in paired["per_recipe"]
        .as_object()
        .expect("verified per-recipe totals")
    {
        let rows = sweep
            .rows
            .iter()
            .filter(|r| {
                r.attempt.cohort == "random" && r.attempt.geometry["recipe_id"] == recipe.as_str()
            })
            .collect::<Vec<_>>();
        let count = |class: &str, landed: bool| {
            rows.iter()
                .filter(|r| {
                    r.attempt.result.as_ref().is_some_and(|p| {
                        p.nominal_class == class && (!landed || p.verified_landing)
                    })
                })
                .count()
        };
        coverage_rows.push_str(&batch::render_overview_row(
            "",
            &[
                &escape(recipe),
                &value["count"].to_string(),
                &value["baseline_landings"].to_string(),
                &value["landings"].to_string(),
                &format!("{}/{}", count("clear", true), count("clear", false)),
                &format!("{}/{}", count("blocked", true), count("blocked", false)),
            ],
        ));
    }
    let coverage = format!(
        "<section class=\"coverage-section\"><h2>Coverage</h2><p>Same saved worlds, unchanged fuel and deadline. Clear and blocked denominators come from the initial audit. Controls/repeats are not extra worlds.</p><div class=\"table-wrap\">{}</div></section>",
        batch::render_table(
            "coverage-table",
            "",
            "<tr><th>Terrain recipe</th><th>Worlds</th><th>Cap 6 · landings</th><th>Cap 24 · landings</th><th>Clear · landed/total</th><th>Blocked · landed/total</th></tr>",
            &coverage_rows
        )
    );
    let context = batch::render_context_section(
        false,
        false,
        "warn",
        "Experimental budget · production unchanged",
        &batch::render_context_table(
            "<tr><th>Contract</th><th>Value</th></tr>",
            &[
            batch::render_context_row(&[
                "Frozen setup",
                "Four Pylander recipes · 250 worlds each · 1200 m · tested vehicle · Earth · 120/60 Hz · 90 s · locally prepared pads",
            ]), batch::render_context_row(&[
                "Policy identity",
                "piecewise_local_clearing_v2_policy_3_cap_probe_24 · explicitly isolated experiment; production policy remains cap 6",
            ]),
            ].join(""),
        ),
    );
    let diagnostics = format!(
        "<section class=\"diagnostics-section\"><h2>Diagnostics</h2><p>Complete saved capture; all 1010 attempts pass recorded integrity/source replay. Planning stops: {}. Maximum actual corrections: {}. No relaxed-cap stop.</p><p>Primary worlds: {} not launched · {} stopped mid-flight · {} physical crashes. These counts describe execution, not the location of a proposed obstruction.</p><p>A finite NoClearing or NoNominal stop is not a crash: the simulator endpoint remains flying / in_progress, and planning has stopped. The reports end there; no subsequent landing is implied.</p></section>",
        escape(&summary["planning_stops"].to_string()),
        paired["maximum_actual_corrections"],
        execution.not_started,
        execution.in_progress,
        execution.crashed,
    );
    let headers = "<tr><th>Selector</th><th>Success / Outcome</th><th>Cap 6 · baseline outcome</th><th>Fuel Used</th><th>Flight Time</th><th>Landing Offset</th><th>Reference deviation</th><th>Preview</th><th>Initial nominal</th><th>Handoffs</th><th>Planning</th></tr>";
    let table = batch::render_tree_table_section(
        "Terrain recipes and outcomes",
        "1,000 primary worlds · 3 controls · 7 repeats",
        &batch::render_review_tree_table("cap-sweep", headers, &tree(&sweep.rows, cells)),
    );
    let review = batch::render_review_tree_section(
        &pd_report::batch_tree::render_controls("Missions", false),
        "<p>Expand Primary worlds → terrain recipe → outcome → mission. H means completed executed handoff. Click a mission or preview for the full rich report. Baseline outcome links open the original cap-6 detail.</p><p>Preview markers: <span style=\"color:#68717a\">○ Not launched</span> · <span style=\"color:#b26b00\">Ⅱ Saved airborne endpoint</span> · <span style=\"color:#c92a2a\">× Physical crash only</span> · <span style=\"color:#2f9e44\">● Mission success</span>. Hover a marker for its meaning. Proposed conflicts are planning queries, not executed positions.</p>",
        &table,
    );
    let added = paired["new_landings"]
        .as_array()
        .expect("verified added landings")
        .iter()
        .map(|v| {
            let id = v.as_str().expect("case id");
            format!("<a href=\"runs/{id}/index.html\">{id}</a>")
        })
        .collect::<Vec<_>>()
        .join(" · ");
    let comparison = format!(
        "<section><h2>Comparison</h2><p>Original cap 6: {} landings; experimental cap 24: {}. All {} earlier successes preserved. The 23 old cap stops become {}. Non-cap-bound flights match complete original records except explicit policy/input identity and wall timings; old cap-bound flights preserve the exact executed H6 prefix.</p><p>Added landings: {added}</p><p>These are paired development worlds, not a fresh held-out reliability estimate. Saved verification does not perform a fresh flight or physical replay. <a href=\"{baseline}survey.json\">Original 1k survey JSON</a></p></section>",
        paired["baseline_landings"],
        summary["verified_landings"],
        paired["preserved_landings"],
        escape(&paired["old_cap_outcomes"].to_string())
    );
    let examples = [
        ("random-000", "000 · clear direct landing, no waypoints"),
        ("random-030", "030 · formerly capped, now lands at H7"),
        ("random-565", "565 · lands after 13 handoffs"),
        ("random-280", "280 · later NoClearing, not a crash"),
        ("random-611", "611 · later NoNominal, not a crash"),
        (
            "random-988",
            "988 · not launched: proposed departure blocked",
        ),
        (
            "random-980",
            "980 · not launched: proposed departure blocked",
        ),
        (
            "random-999",
            "999 · not launched: proposed approach blocked",
        ),
    ]
    .iter()
    .filter(|(id, _)| sweep.rows.iter().any(|r| r.attempt.attempt_id == *id))
    .map(|(id, label)| format!("<a href=\"runs/{id}/index.html\">{label}</a>"))
    .collect::<Vec<_>>()
    .join(" · ");
    batch::render_batch_page(batch::BatchPage {
        title: "Planner V2 · 1,000-world terrain sweep",
        subtitle: "Paired cap 6 → cap 24 development comparison · not the accepted benchmark · production default unchanged",
        chips_html: "<span class=\"chip\">1,000 primary worlds</span><span class=\"chip\">Experimental cap 24</span><span class=\"chip\">Separate controls / repeats</span>",
        actions_html: &format!(
            "<a href=\"{base}cap-sweep.json\">Paired sweep JSON</a><a href=\"{base}manifest.json\">Frozen inputs</a><a href=\"{base}receipt.json\">Capture receipt</a><a href=\"render.json\">Publication receipt</a>"
        ),
        before_hero_html: "<nav><a href=\"/reports/\">Report home</a> · <a href=\"/reports/topics/waypoint-planning/index.html\">Waypoint planning</a> · <a href=\"/reports/eval/planner_v2_lab_suite/\">Accepted benchmark</a></nav>",
        after_hero_html: &format!(
            "<section class=\"panel\"><h2>Start here</h2><p>{examples}</p><p>For the full population, use the Review Tree below. All existing detailed plots and handoff annotations are preserved.</p></section>"
        ),
        overview_html: &overview,
        planner_html: "",
        coverage_html: &coverage,
        context_html: &context,
        diagnostics_html: &diagnostics,
        review_tree_html: &review,
        comparison_html: &comparison,
        appendix_html: "",
        body_class: "planner-v2-common",
        tree: batch::BatchTreeOptions {
            max_depth: 3,
            depth_by_kind: &[("group", 0)],
            leaf_selector: "tr.seed-row",
            default_expansion: 1,
        },
        coverage_tree_jump: false,
    })
}

/// Create-only publication, using the study's exhaustive saved-evidence check.
/// This command requires Python 3, never builds the probe or executes a flight,
/// and does not weaken ordinary policy-3 survey or benchmark acceptance.
pub fn render_saved_cap_sweep(capture: &Path, output: &Path, base: &str) -> Result<()> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("repo root")?;
    validate_report_target(repo, output)?;
    ensure!(
        base.starts_with("/eval/planner_v2_random_terrain/")
            && base.ends_with('/')
            && base.split('/').all(|p| p.is_empty()
                || p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')),
        "unsafe cap-sweep capture URL"
    );
    ensure!(
        !fs::symlink_metadata(capture)?.file_type().is_symlink(),
        "symlink capture root"
    );
    let capture = capture.canonicalize()?;
    ensure!(
        capture
            == repo
                .join("outputs")
                .join(base.trim_start_matches('/'))
                .canonicalize()?,
        "capture URL differs from source directory"
    );
    let receipt_bytes = fs::read(safe_file(&capture, "receipt.json")?)?;
    let verified = Command::new("python3")
        .arg("-B")
        .arg(repo.join("studies/terrain_profiles/cap_sweep.py"))
        .arg("verify")
        .arg(&capture)
        .current_dir(repo)
        .output()
        .context("read-only cap-sweep verifier requires Python 3")?;
    ensure!(
        verified.status.success(),
        "saved cap-sweep verification failed: {}",
        String::from_utf8_lossy(&verified.stderr)
    );
    let verification: Value = serde_json::from_slice(&verified.stdout)?;
    ensure!(
        verification["fresh_flights"] == 0 && verification["completed"] == true,
        "cap sweep is not complete"
    );
    ensure!(
        fs::read(safe_file(&capture, "receipt.json")?)? == receipt_bytes,
        "capture receipt changed during validation"
    );
    let receipt: Value = serde_json::from_slice(&receipt_bytes)?;
    let authenticated = |relative: &str| -> Result<Vec<u8>> {
        let bytes = fs::read(safe_file(&capture, relative)?)?;
        ensure!(
            receipt["files"][relative] == sha256_bytes(&bytes)?,
            "capture artifact changed: {relative}"
        );
        Ok(bytes)
    };
    let sweep: CapSweep = serde_json::from_slice(&authenticated("cap-sweep.json")?)?;
    ensure!(
        sweep.schema == "pd-lab.correction-cap-sweep-results.v1"
            && sweep.stopped_reason.is_none()
            && sweep.rows.len() == 1010,
        "unexpected completed cap sweep"
    );
    let plan: Value = serde_json::from_slice(&authenticated("plan.json")?)?;
    let baseline = format!(
        "/{}/",
        plan["baseline_capture"]
            .as_str()
            .context("baseline capture")?
            .strip_prefix("outputs/")
            .context("served baseline")?
    );
    let site = format!(
        "/{}/",
        output
            .strip_prefix(repo.join("outputs"))?
            .to_str()
            .context("site URL")?
    );
    reserve_output_root(output)?;
    let mut files = BTreeMap::new();
    let mut cells = Vec::new();
    let mut execution = ExecutionCounts::default();
    for (index, row) in sweep.rows.iter().enumerate() {
        let a = &row.attempt;
        ensure!(
            a.status == "recorded"
                && !a.attempt_id.is_empty()
                && a.attempt_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "unsafe or unrecorded cap attempt"
        );
        let prefix = format!("runs/{}", a.attempt_id);
        let flight: WaypointV2FlightResult =
            serde_json::from_slice(&authenticated(&format!("{prefix}/flight.json"))?)?;
        let scenario: ScenarioSpec =
            serde_json::from_slice(&authenticated(&format!("{prefix}/scenario.json"))?)?;
        ensure!(
            flight.policy.policy_id == "piecewise_local_clearing_v2_policy_3_cap_probe_24"
                && flight.policy.maximum_corrections == 24
                && a.result.as_ref() == Some(&project(&flight)),
            "experimental flight projection differs"
        );
        cells.push(leaf(row, &flight, &scenario, &baseline)?);
        if a.cohort == "random" {
            match execution_status(&flight).0 {
                "not_started" => execution.not_started += 1,
                "in_progress" => execution.in_progress += 1,
                "crashed" => execution.crashed += 1,
                _ => {}
            }
        }
        let html = String::from_utf8(authenticated(&format!("{prefix}/report.html"))?)?;
        let copy = navigable_copy(
            &html,
            &navigation(&sweep.rows, index, &site, base, &baseline),
        )?;
        let copy = with_planning_review(&copy, &flight)?;
        let relative = format!("{prefix}/index.html");
        fs::create_dir_all(output.join(&prefix))?;
        write_bytes_create_only_with_context(
            &output.join(&relative),
            copy.as_bytes(),
            "create cap-sweep detail",
        )?;
        files.insert(relative, sha256_bytes(copy.as_bytes())?);
    }
    let html = page(&sweep, &cells, &execution, base, &baseline);
    write_bytes_create_only_with_context(
        &output.join("index.html"),
        html.as_bytes(),
        "create cap-sweep batch",
    )?;
    files.insert("index.html".into(), sha256_bytes(html.as_bytes())?);
    write_json_create_only(
        &output.join("render.json"),
        &json!({
            "schema": "pd-lab.terrain-cap-sweep-publication.v1", "source_capture": capture,
            "source_base_href": base, "source_receipt_sha256": sha256_bytes(&receipt_bytes)?,
            "verified_saved_evidence": verification, "fresh_flights": 0, "fresh_replays": 0,
            "policy_id": "piecewise_local_clearing_v2_policy_3_cap_probe_24", "maximum_corrections": 24,
            "production_maximum_corrections": 6, "detail_change": "navigation, artifact links and saved planning summary; complete rich numeric payload and plots preserved", "files": files,
        }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rich_copy_changes_only_navigation_and_rejects_missing_slots() {
        let original = AnnotationNavigation {
            source_links: ["Scenario JSON", "Flight JSON", "Run summary JSON"]
                .iter()
                .zip(["scenario.json", "flight.json", "summary.json"])
                .map(|(label, href)| NavigationLink {
                    label: (*label).into(),
                    href: href.into(),
                })
                .collect(),
            ..Default::default()
        };
        let payload = format!(
            "{{\"numeric\":1.2345678901234567,\"navigation\":{}}}",
            serde_json::to_string(&original).unwrap()
        );
        let html = format!(
            "<nav aria-label=\"Flight report navigation\"></nav><a href=\"flight.json\">Flight</a><script>const reportData = {payload};\n</script><div>unchanged plots</div>"
        );
        let mut nav = original.clone();
        nav.collection = Some(NavigationLink {
            label: "Batch & home".into(),
            href: "/reports/batch/index.html".into(),
        });
        nav.source_links[1].href = "/eval/capture/runs/x/flight.json".into();
        let copy = navigable_copy(&html, &nav).unwrap();
        assert!(copy.contains("1.2345678901234567") && copy.contains("<div>unchanged plots</div>"));
        assert!(
            copy.contains("Batch &amp; home")
                && copy.contains("href=\"/eval/capture/runs/x/flight.json\"")
        );
        assert!(
            copy.contains("production remains cap 6") && !copy.contains("href=\"flight.json\"")
        );
        assert!(navigable_copy("missing slots", &nav).is_err());
        assert!(navigable_copy(&(html.clone() + &html), &nav).is_err());
    }

    #[test]
    fn page_keeps_common_tree_and_separate_cohorts_without_calling_finite_stop_a_crash() {
        let make = |id: &str, cohort: &str, landed: bool, corrections: u32| {
            serde_json::from_value::<CapAttempt>(json!({
            "attempt_id":id, "case_id":"random-000", "cohort":cohort,"seed":7,"scenario_path":"scenarios/x.json","status":"recorded","geometry":{"recipe_id":"mountains_4x"},
            "result":{"nominal_class":"blocked","verified_landing":landed,"planning_stop":if landed {"landed"} else {"no_clearing"},"physical_outcome":if landed {"landed_on_target"} else {"flying"},"mission_outcome":if landed {"success"} else {"in_progress"},"correction_count":corrections,"integrity_passed":true,"final_source_replay_passed":true},
            "paired":{"baseline_result":{"nominal_class":"blocked","verified_landing":false,"planning_stop":"correction_limit","physical_outcome":"flying","mission_outcome":"in_progress","correction_count":6,"integrity_passed":true,"final_source_replay_passed":true}}
        })).unwrap()
        };
        let sweep = CapSweep {
            schema: "synthetic only".into(),
            rows: vec![
                make("random-000", "random", true, 7),
                make("random-001", "random", false, 8),
                make("control", "sentinel", true, 1),
                make("repeat-000", "repeat", true, 7),
            ],
            summary: json!({"primary_count":2,"verified_landings":1,"sentinels_recorded":1,"repeats_recorded":1}),
            paired_summary: json!({"baseline_landings":0,"preserved_landings":0,"new_landings":["random-000"],"per_recipe":{"mountains_4x":{"count":2,"baseline_landings":0,"landings":1}},"old_cap_outcomes":{"landed":1,"no_clearing":1},"maximum_actual_corrections":8}),
            stopped_reason: None,
        };
        let html = page(
            &sweep,
            &vec!["<td>synthetic</td>".into(); 4],
            &ExecutionCounts {
                not_started: 0,
                in_progress: 1,
                crashed: 0,
            },
            "/eval/synthetic/",
            "/eval/baseline/",
        );
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(
            html.contains("class=\"coverage-table\"")
                && html.contains("<th>Reference / recovery</th>")
        );
        assert!(
            html.contains("class=\"tree-table-section\"")
                && html.contains("data-tree-action=\"expand-seeds\"")
        );
        for heading in [
            "Selector",
            "Success / Outcome",
            "Fuel Used",
            "Flight Time",
            "Landing Offset",
            "Reference deviation",
            "Preview",
        ] {
            assert!(html.contains(&format!("<th>{heading}</th>")));
        }
        for section in [
            "Overview",
            "Coverage",
            "Context",
            "Diagnostics",
            "Review Tree",
            "Comparison",
        ] {
            assert!(html.contains(section));
        }
        for group in ["random", "recipe-0", "outcome-0-0", "sentinel", "repeat"] {
            assert!(html.contains(&format!("data-group=\"{group}\"")));
        }
        assert_eq!(html.matches("data-case-id=").count(), 4);
        assert!(
            html.contains("Corrected landing")
                && html.contains("Finite planning stop · no_clearing")
        );
        assert!(html.contains("not a crash") && html.contains("production default unchanged"));
        assert!(html.contains("0 not launched · 1 stopped mid-flight · 0 physical crashes"));
        assert!(html.contains("× Physical crash only") && html.contains("○ Not launched"));
        assert!(html.contains("/reports/topics/waypoint-planning/index.html"));
    }
}
