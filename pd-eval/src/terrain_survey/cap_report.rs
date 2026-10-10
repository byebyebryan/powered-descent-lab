//! Presentation of explicit paired cap-24 experiments, not production admission.
//! The study's read-only verifier owns its paired evidence contract. Detail
//! pages extend authenticated rich HTML with navigation and a saved planning
//! summary; existing rich plots, scripts and numeric payload are preserved.

use std::{collections::BTreeMap, process::Command};

use super::*;
use crate::evidence_io::write_json_create_only;
use crate::waypoint_v2_report::{execution_status, nominal_conflict_phase, with_planning_review};

mod planning;
mod review;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SweepKind {
    CapBudget,
    EarlyExit,
}

impl SweepKind {
    fn result_name(self) -> &'static str {
        match self {
            Self::CapBudget => "cap-sweep.json",
            Self::EarlyExit => "early-exit-sweep.json",
        }
    }

    fn result_schema(self) -> &'static str {
        match self {
            Self::CapBudget => "pd-lab.correction-cap-sweep-results.v1",
            Self::EarlyExit => "pd-lab.early-exit-sweep-results.v1",
        }
    }

    fn verifier(self) -> &'static str {
        match self {
            Self::CapBudget => "cap_sweep.py",
            Self::EarlyExit => "early_exit_sweep.py",
        }
    }

    fn baseline_label(self) -> &'static str {
        match self {
            Self::CapBudget => "Original · cap 6",
            Self::EarlyExit => "Previous · cap 24 without early exit",
        }
    }

    fn current_label(self) -> &'static str {
        match self {
            Self::CapBudget => "Experiment · cap 24",
            Self::EarlyExit => "Early exit · cap 24",
        }
    }

    fn baseline_json(self) -> &'static str {
        match self {
            Self::CapBudget => "survey.json",
            Self::EarlyExit => "cap-sweep.json",
        }
    }

    fn policy_id(self) -> &'static str {
        match self {
            Self::CapBudget => "piecewise_local_clearing_v2_policy_3_cap_probe_24",
            Self::EarlyExit => "piecewise_local_clearing_v2_policy_3_early_exit_probe_24",
        }
    }
}

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
    kind: SweepKind,
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
    let group = review::failure_group(&rows[index]);
    let peers = group.map(|g| review::peers(rows, g));
    let position = peers
        .as_ref()
        .and_then(|p| p.iter().position(|&i| i == index));
    let (previous, next) = if let (Some(peers), Some(position)) = (&peers, position) {
        (
            position
                .checked_sub(1)
                .map(|i| link("Previous stopped case", detail(peers[i]))),
            peers
                .get(position + 1)
                .map(|&i| link("Next stopped case", detail(i))),
        )
    } else {
        (
            index
                .checked_sub(1)
                .map(|i| link("Previous case", detail(i))),
            rows.get(index + 1)
                .map(|_| link("Next case", detail(index + 1))),
        )
    };
    AnnotationNavigation {
        home: Some(link("Report home", "/reports/".into())),
        collection: Some(match group {
            Some(group) => link(
                &format!("Stopped missions · {}", group.label()),
                format!("{site}index.html#tree-{}", group.id()),
            ),
            None => link("1,000-world batch", format!("{site}index.html")),
        }),
        previous,
        next,
        source_links: vec![
            link("All 1,000 worlds", format!("{site}index.html")),
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
                kind.baseline_label(),
                format!("{baseline}runs/{}/report.html", row.case_id),
            ),
            link("Paired sweep JSON", format!("{base}{}", kind.result_name())),
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
    for old in &original.source_links {
        let new = nav
            .source_links
            .iter()
            .find(|new| new.label == old.label)
            .context("missing rebased source link")?;
        copy = copy.replace(
            &format!("href=\"{}\"", old.href),
            &format!("href=\"{}\"", escape(&new.href)),
        );
    }
    Ok(copy)
}

fn mission_preview_cell(attempt: &Attempt, preview: &str) -> String {
    let id = &attempt.attempt_id;
    let seed = attempt
        .seed
        .map_or_else(|| "control".into(), |s| s.to_string());
    format!(
        "<td class=\"tree-label\"><div class=\"preview-cell\"><a class=\"mission-link\" href=\"runs/{id}/index.html\">{id}</a><span class=\"row-note\">seed {seed}</span><a class=\"run-preview\" href=\"runs/{id}/index.html\">{preview}</a></div></td>"
    )
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
    let selector = mission_preview_cell(a, &preview);
    let old = &row.paired.baseline_result;
    let (_, execution) = execution_status(flight);
    let conflict = flight
        .cycles
        .last()
        .filter(|c| c.audit.as_ref().is_some_and(|a| !a.passed))
        .map(|c| format!(" · proposed conflict: {}", nominal_conflict_phase(c)))
        .unwrap_or_default();
    Ok(format!(
        "{selector}<td>{outcome}<span class=\"row-note\">{execution}{conflict}</span><span class=\"row-note\">Recorded integrity / source replay passed</span></td><td><a href=\"{baseline}runs/{case}/report.html\">{previous}</a><span class=\"row-note\">{old_h} H</span></td><td>{fuel:.3} kg</td><td>{time:.3} s</td><td>{offset}</td><td>— · no reference-controller comparison</td><td>{nominal}</td><td>{handoffs} H</td><td>{planning:.3} s</td>",
        case = a.case_id,
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
            "id=\"tree-{id}\" data-group=\"{id}\"{parent} data-kind=\"group\" data-depth=\"{depth}\" aria-expanded=\"false\" tabindex=\"0\""
        ),
        &format!(
            "<td class=\"tree-label\" style=\"--depth:{depth}\"><span class=\"expander\">+</span> {}</td><td colspan=\"9\">{count} cases · expand to inspect</td>",
            escape(label)
        ),
    )
}

fn page(
    kind: SweepKind,
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
            kind.baseline_label(),
            summary["primary_count"].to_string(),
            format!("{} verified landings", paired["baseline_landings"]),
        ),
        (
            kind.current_label(),
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
            &format!(
                "<tr><th>Terrain recipe</th><th>Worlds</th><th>{} · landings</th><th>{} · landings</th><th>Clear · landed/total</th><th>Blocked · landed/total</th></tr>",
                kind.baseline_label(),
                kind.current_label()
            ),
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
                &format!("{} · explicitly isolated experiment; production policy remains cap 6", kind.policy_id()),
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
    let headers = format!(
        "<tr><th class=\"tree-label\">Selector / Preview</th><th>Success / Outcome</th><th>{} · baseline outcome</th><th>Fuel Used</th><th>Flight Time</th><th>Landing Offset</th><th>Reference deviation</th><th>Initial nominal</th><th>Handoffs</th><th>Planning</th></tr>",
        kind.baseline_label()
    );
    let table = batch::render_tree_table_section(
        "Stopped missions first · successful comparisons below",
        "1,000 primary worlds · 3 controls · 7 repeats",
        &batch::render_review_tree_table(
            "paired-terrain-sweep",
            &headers,
            &review::tree(&sweep.rows, cells),
        ),
    );
    let review = batch::render_review_tree_section(
        &pd_report::batch_tree::render_controls("Missions", false),
        "<p>Stopped missions → planning stage → terrain recipe → mission. The links at the top open every mission in the chosen stop group. Successful primary missions, preservation controls and repeats remain separate. H means completed executed handoff. Click a mission or preview for the full rich report.</p><p>Preview markers: <span style=\"color:#68717a\">○ Not launched</span> · <span style=\"color:#b26b00\">Ⅱ Saved airborne endpoint</span> · <span style=\"color:#c92a2a\">× Physical crash only</span> · <span style=\"color:#2f9e44\">● Mission success</span>. Hover a marker for its meaning. Proposed conflicts are planning queries, not executed positions.</p>",
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
    let preservation = match kind {
        SweepKind::CapBudget => format!("The 23 old cap stops become {}. Non-cap-bound flights match complete original records except explicit policy/input identity and wall timings; old cap-bound flights preserve the exact executed H6 prefix.", escape(&paired["old_cap_outcomes"].to_string())),
        SweepKind::EarlyExit => "Both runs use cap 24. The only new flight capability is one checked earlier direct exit per selected clearing maneuver; blocked or missing early nominals retain the old H fallback. All remaining stopped flights preserve the prior executed records. This does not promote the production cap or establish a safe continuation after a stopped endpoint.".into(),
    };
    let comparison = format!(
        "<section><h2>Comparison</h2><p>{}: {} landings; {}: {}. All {} earlier successes preserved. {preservation}</p><details><summary>Added landings · {} missions</summary><p>{added}</p></details><p>These are paired development worlds, not a fresh held-out reliability estimate. Saved verification does not perform a fresh flight or physical replay. <a href=\"{baseline}{}\">Previous 1k results JSON</a></p></section>",
        kind.baseline_label(),
        paired["baseline_landings"],
        kind.current_label(),
        summary["verified_landings"],
        paired["preserved_landings"],
        paired["new_landings"].as_array().map_or(0, Vec::len),
        kind.baseline_json(),
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
        subtitle: match kind {
            SweepKind::CapBudget => {
                "Paired cap 6 → cap 24 development comparison · not the accepted benchmark · production default unchanged"
            }
            SweepKind::EarlyExit => {
                "Latest early-exit comparison · same worlds, cap 24 in both runs · not the accepted benchmark · production default unchanged"
            }
        },
        chips_html: "<span class=\"chip\">1,000 primary worlds</span><span class=\"chip\">Experimental cap 24</span><span class=\"chip\">Separate controls / repeats</span>",
        actions_html: &format!(
            "<a href=\"{base}{}\">Paired sweep JSON</a><a href=\"{base}manifest.json\">Frozen inputs</a><a href=\"{base}receipt.json\">Capture receipt</a><a href=\"render.json\">Publication receipt</a>",
            kind.result_name()
        ),
        before_hero_html: "<nav><a href=\"/reports/\">Report home</a> · <a href=\"/reports/topics/waypoint-planning/index.html\">Waypoint planning</a> · <a href=\"/reports/eval/planner_v2_lab_suite/\">Accepted benchmark</a></nav>",
        after_hero_html: &format!(
            "{}<details class=\"panel\"><summary>Successful comparisons and additional examples</summary><p>{examples}</p></details>",
            review::intro(&sweep.rows)
        ),
        overview_html: &overview,
        planner_html: "",
        coverage_html: &coverage,
        context_html: &context,
        diagnostics_html: &diagnostics,
        review_tree_html: &review,
        comparison_html: &comparison,
        appendix_html: "",
        body_class: "planner-v2-common mission-preview-batch",
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
    render_saved_sweep(SweepKind::CapBudget, capture, output, base, false)
}

/// Explicit early-exit publication uses its own frozen verifier and baseline.
/// It cannot substitute early-exit evidence into the original cap comparison.
pub fn render_saved_early_exit_sweep(capture: &Path, output: &Path, base: &str) -> Result<()> {
    render_saved_sweep(SweepKind::EarlyExit, capture, output, base, false)
}

/// Opt-in, source-bound fixed-program diagnostics. No new nominal searches or
/// mission flights. Ordinary saved publication above remains simulation-free.
pub fn render_saved_early_exit_sweep_with_cycles(
    capture: &Path,
    output: &Path,
    base: &str,
) -> Result<()> {
    render_saved_sweep(SweepKind::EarlyExit, capture, output, base, true)
}

fn render_saved_sweep(
    kind: SweepKind,
    capture: &Path,
    output: &Path,
    base: &str,
    planning_cycles: bool,
) -> Result<()> {
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
        .arg(repo.join("studies/terrain_profiles").join(kind.verifier()))
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
    let complete = match kind {
        SweepKind::CapBudget => {
            verification["fresh_flights"] == 0 && verification["completed"] == true
        }
        SweepKind::EarlyExit => {
            verification["measured_attempts"] == 1010
                && verification["stopped_reason"].is_null()
                && verification["summary"]["recorded_count"] == 1000
        }
    };
    ensure!(complete, "paired sweep is not complete");
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
    let engine = if planning_cycles {
        Some(planning::engine_binding(&authenticated)?)
    } else {
        None
    };
    let sweep: CapSweep = serde_json::from_slice(&authenticated(kind.result_name())?)?;
    ensure!(
        sweep.schema == kind.result_schema()
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
    let mut cycle_count = 0;
    let mut nominal_count = 0;
    let mut query_count = 0;
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
            flight.policy.policy_id == kind.policy_id() && flight.policy.maximum_corrections == 24,
            "experimental policy identity differs for {}",
            a.attempt_id
        );
        ensure!(
            a.result.as_ref() == Some(&project(&flight)),
            "experimental flight projection differs"
        );
        cells.push(leaf(row, &flight, &scenario, &baseline)?);
        if a.cohort == "random" {
            if review::failure_group(row) == Some(review::FailureGroup::NotLaunched) {
                ensure!(
                    flight
                        .ordinary_flight
                        .as_ref()
                        .is_some_and(|ordinary| ordinary.final_state.physics_step == 0),
                    "not-launched group contains an executed flight"
                );
            }
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
            &navigation(kind, &sweep.rows, index, &site, base, &baseline),
        )?;
        let copy = with_planning_review(&copy, &flight)?;
        let copy = if planning_cycles {
            let data = planning::reconstruct(
                &scenario,
                &flight,
                planning::QUERY_CASES.contains(&a.attempt_id.as_str()),
            )
            .with_context(|| format!("planning reconstruction {}", a.attempt_id))?;
            cycle_count += data.cycles.len();
            nominal_count += data.cycles.iter().filter(|c| !c.nominal.is_empty()).count();
            query_count += data.cycles.iter().map(|c| c.queries.len()).sum::<usize>();
            let sidecar = format!("{prefix}/planning-cycles.json");
            fs::create_dir_all(output.join(&prefix))?;
            let bytes = serde_json::to_vec(
                &json!({"source_flight_sha256": receipt["files"][format!("{prefix}/flight.json")],
                "source_scenario_sha256": receipt["files"][format!("{prefix}/scenario.json")], "review": data}),
            )?;
            write_bytes_create_only_with_context(
                &output.join(&sidecar),
                &bytes,
                "create planning cycle diagnostics",
            )?;
            files.insert(sidecar, sha256_bytes(&bytes)?);
            pd_report::planning_cycles::attach(&copy, &data)?
        } else {
            copy
        };
        let relative = format!("{prefix}/index.html");
        fs::create_dir_all(output.join(&prefix))?;
        write_bytes_create_only_with_context(
            &output.join(&relative),
            copy.as_bytes(),
            "create cap-sweep detail",
        )?;
        files.insert(relative, sha256_bytes(copy.as_bytes())?);
    }
    let html = page(kind, &sweep, &cells, &execution, base, &baseline);
    let html = if planning_cycles {
        let examples = planning::QUERY_CASES
            .iter()
            .map(|id| format!("<a href=\"runs/{id}/index.html\">{id}</a>"))
            .collect::<Vec<_>>()
            .join(" · ");
        let missing = cycle_count - nominal_count;
        let intro = format!(
            "<section class=\"panel\"><h2>Inspect what each waypoint did</h2><p>Each detail keeps every original rich view and adds a planning-cycle selector above the trajectory: launch → actual H1 → H2 → final stop. Start with <a href=\"runs/random-715/index.html#planning-cycle-review\">715: cleared the first obstruction, stopped before a later approach</a>. Purple overlays show the recorded state-aware proposal; red × is its future terrain conflict, not a crash.</p><p>Bounded diagnostic examples: {examples}. Rejected-row examples are reconstructed only in these predeclared cases and plotted only when requested. Across 1010 saved attempts: {cycle_count} planning cycles, {nominal_count} recorded nominal paths and {missing} explicitly missing nominals (no invented trajectory).</p><p>This edition reenacts fixed programs against source-matched dynamics and saved state/reason checkpoints. No new nominal searches, mission flights, policy changes or changed results. <a href=\"render.json\">Reconstruction receipt</a>.</p></section>"
        );
        html.replacen("<section class=\"diagnostics-section\">", &format!("{intro}<section class=\"diagnostics-section\">"), 1)
            .replace("Saved verification does not perform a fresh flight or physical replay.", "Saved verification performs no new mission flight. This edition separately reenacts fixed recorded programs for diagnostic plotting, not acceptance replay.")
    } else {
        html
    };
    write_bytes_create_only_with_context(
        &output.join("index.html"),
        html.as_bytes(),
        "create cap-sweep batch",
    )?;
    files.insert("index.html".into(), sha256_bytes(html.as_bytes())?);
    let review_groups = ["stops-not-launched", "stops-clearing", "stops-nominal"]
        .iter()
        .map(|id| {
            (
                id,
                sweep
                    .rows
                    .iter()
                    .filter(|row| review::failure_group(row).is_some_and(|group| group.id() == *id))
                    .count(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut render_receipt = json!({
        "schema": match kind { SweepKind::CapBudget => "pd-lab.terrain-cap-sweep-publication.v1", SweepKind::EarlyExit => "pd-lab.terrain-early-exit-sweep-publication.v1" }, "source_capture": capture,
        "results_artifact": kind.result_name(), "baseline_capture": plan["baseline_capture"],
        "source_base_href": base, "source_receipt_sha256": sha256_bytes(&receipt_bytes)?,
        "verified_saved_evidence": verification, "fresh_flights": 0, "fresh_replays": 0,
        "policy_id": kind.policy_id(), "maximum_corrections": 24,
        "production_maximum_corrections": 6, "detail_change": "navigation, artifact links and saved planning summary; complete rich numeric payload and plots preserved", "files": files,
        "review_groups": review_groups,
    });
    if planning_cycles {
        render_receipt
            .as_object_mut()
            .unwrap()
            .remove("fresh_replays");
        render_receipt["mission_acceptance_replays"] = json!(0);
        render_receipt["new_nominal_searches"] = json!(0);
        render_receipt["diagnostic_reconstruction"] = json!({"source_programs": sweep.rows.len(),
            "planning_cycles": cycle_count, "recorded_nominal_programs": nominal_count,
            "missing_nominal_cycles": cycle_count - nominal_count, "recorded_rejected_rows": query_count,
            "query_case_ids": planning::QUERY_CASES, "engine": engine});
        render_receipt["detail_change"] = json!(
            "additive planning-cycle overlays and fixed-program reconstruction; all original rich panels/numeric payloads preserved"
        );
    }
    write_json_create_only(&output.join("render.json"), &render_receipt)?;
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
            SweepKind::CapBudget,
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
            "Success / Outcome",
            "Fuel Used",
            "Flight Time",
            "Landing Offset",
            "Reference deviation",
        ] {
            assert!(html.contains(&format!("<th>{heading}</th>")));
        }
        assert!(html.contains("<th class=\"tree-label\">Selector / Preview</th>"));
        assert!(html.contains("mission-preview-batch"));
        assert!(html.contains("colspan=\"9\""));
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
        for group in [
            "stops",
            "stops-clearing",
            "stops-clearing-recipe-0",
            "landings",
            "sentinel",
            "repeat",
        ] {
            assert!(html.contains(&format!("data-group=\"{group}\"")));
        }
        assert_eq!(html.matches("data-case-id=").count(), 4);
        assert!(
            html.contains("Corrected landings")
                && html.contains("Stopped during obstacle clearing")
        );
        assert!(html.contains("not a crash") && html.contains("production default unchanged"));
        assert!(html.contains("0 not launched · 1 stopped mid-flight · 0 physical crashes"));
        assert!(html.contains("× Physical crash only") && html.contains("○ Not launched"));
        assert!(html.contains("/reports/topics/waypoint-planning/index.html"));
    }

    fn review_attempt(
        id: &str,
        cohort: &str,
        stop: WaypointV2Stop,
        corrections: u32,
    ) -> CapAttempt {
        let landed = stop == WaypointV2Stop::Landed;
        serde_json::from_value(json!({
            "attempt_id":id, "case_id":id, "cohort":cohort,"seed":7,"scenario_path":"scenarios/x.json","status":"recorded","geometry":{"recipe_id":"mountains_4x"},
            "result":{"nominal_class":"blocked","verified_landing":landed,"planning_stop":stop,"physical_outcome":if landed {"landed_on_target"} else {"flying"},"mission_outcome":if landed {"success"} else {"in_progress"},"correction_count":corrections,"integrity_passed":true,"final_source_replay_passed":true},
            "paired":{"baseline_result":{"nominal_class":"blocked","verified_landing":false,"planning_stop":"no_nominal","physical_outcome":"flying","mission_outcome":"in_progress","correction_count":1,"integrity_passed":true,"final_source_replay_passed":true}}
        })).unwrap()
    }

    #[test]
    fn preview_shares_the_sticky_mission_cell_instead_of_a_distant_column() {
        let row = review_attempt("random-280", "random", WaypointV2Stop::NoClearing, 8);
        let svg = "<svg role=\"img\"><path d=\"M 0 0 L 1 1\"/></svg>";
        let cell = mission_preview_cell(&row.attempt, svg);
        assert!(cell.starts_with("<td class=\"tree-label\"><div class=\"preview-cell\">"));
        assert!(cell.ends_with("</div></td>"));
        assert_eq!(cell.matches("<td").count(), 1);
        assert_eq!(cell.matches(svg).count(), 1);
        assert_eq!(
            cell.matches("href=\"runs/random-280/index.html\"").count(),
            2
        );
        assert!(cell.contains("seed 7") && cell.contains("class=\"run-preview\""));
    }

    #[test]
    fn failure_groups_are_disjoint_and_detail_neighbors_skip_successes_and_other_groups() {
        use review::FailureGroup as G;
        let rows = vec![
            review_attempt("control", "sentinel", WaypointV2Stop::NoClearing, 0),
            review_attempt("random-000", "random", WaypointV2Stop::Landed, 0),
            review_attempt("random-001", "random", WaypointV2Stop::NoClearing, 0),
            review_attempt("random-002", "random", WaypointV2Stop::NoClearing, 2),
            review_attempt("random-003", "random", WaypointV2Stop::Landed, 2),
            review_attempt("random-004", "random", WaypointV2Stop::NoClearing, 0),
            review_attempt("random-005", "random", WaypointV2Stop::NoNominal, 2),
            review_attempt("random-006", "random", WaypointV2Stop::NoNominal, 1),
            review_attempt("repeat-002", "repeat", WaypointV2Stop::NoClearing, 2),
        ];
        assert_eq!(review::peers(&rows, G::NotLaunched), [2, 5]);
        assert_eq!(review::peers(&rows, G::Clearing), [3]);
        assert_eq!(review::peers(&rows, G::Nominal), [6, 7]);
        let html = review::tree(&rows, &vec!["<td>preview</td>".into(); rows.len()]);
        assert_eq!(html.matches("data-case-id=").count(), rows.len());
        for row in &rows {
            assert_eq!(
                html.matches(&format!("data-case-id=\"{}\"", row.attempt.attempt_id))
                    .count(),
                1
            );
        }
        assert!(
            html.find("data-group=\"stops\"").unwrap()
                < html.find("data-group=\"landings\"").unwrap()
        );
        let nav = navigation(
            SweepKind::EarlyExit,
            &rows,
            2,
            "/reports/site/",
            "/eval/current/",
            "/eval/previous/",
        );
        assert!(nav.previous.is_none());
        assert_eq!(
            nav.next.unwrap().href,
            "/reports/site/runs/random-004/index.html"
        );
        assert_eq!(
            nav.collection.unwrap().href,
            "/reports/site/index.html#tree-stops-not-launched"
        );
        assert!(
            nav.source_links
                .iter()
                .any(|l| l.href == "/eval/current/early-exit-sweep.json")
        );
        let nav = navigation(
            SweepKind::EarlyExit,
            &rows,
            5,
            "/reports/site/",
            "/eval/current/",
            "/eval/previous/",
        );
        assert_eq!(
            nav.previous.unwrap().href,
            "/reports/site/runs/random-001/index.html"
        );
        assert!(nav.next.is_none());
        let nav = navigation(
            SweepKind::EarlyExit,
            &rows,
            6,
            "/reports/site/",
            "/eval/current/",
            "/eval/previous/",
        );
        assert_eq!(
            nav.next.unwrap().href,
            "/reports/site/runs/random-006/index.html"
        );
        let nav = navigation(
            SweepKind::EarlyExit,
            &rows,
            3,
            "/reports/site/",
            "/eval/current/",
            "/eval/previous/",
        );
        assert!(nav.previous.is_none() && nav.next.is_none());
    }

    #[test]
    fn early_exit_page_keeps_same_cap_baseline_and_does_not_relabel_cap_sweep_outcomes() {
        let sweep = CapSweep {
            schema: SweepKind::EarlyExit.result_schema().into(),
            rows: vec![review_attempt(
                "random-983",
                "random",
                WaypointV2Stop::NoNominal,
                1,
            )],
            summary: json!({"primary_count":1,"verified_landings":0,"sentinels_recorded":0,"repeats_recorded":0}),
            paired_summary: json!({"baseline_landings":0,"preserved_landings":0,"new_landings":[],"per_recipe":{"mountains_4x":{"count":1,"baseline_landings":0,"landings":0}},"maximum_actual_corrections":1}),
            stopped_reason: None,
        };
        let html = page(
            SweepKind::EarlyExit,
            &sweep,
            &["<td>synthetic</td>".into()],
            &ExecutionCounts {
                in_progress: 1,
                ..Default::default()
            },
            "/eval/current/",
            "/eval/previous/",
        );
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(html.contains("Review remaining stops · 1 missions"));
        assert!(html.contains("data-tree-focus=\"stops-nominal\""));
        assert!(html.contains("same worlds, cap 24 in both runs"));
        assert!(html.contains("Previous · cap 24 without early exit"));
        assert!(html.contains("/eval/previous/cap-sweep.json"));
        assert!(html.contains("/eval/current/early-exit-sweep.json"));
        assert!(!html.contains("Original cap 6:") && !html.contains("23 old cap stops become"));
        assert!(html.contains("production default unchanged"));
        assert!(html.contains(SweepKind::EarlyExit.policy_id()));
        assert!(!html.contains(SweepKind::CapBudget.policy_id()));
    }

    #[test]
    fn paired_sweep_contracts_keep_distinct_schemas_policy_ids_and_verifiers() {
        assert_ne!(
            SweepKind::EarlyExit.result_schema(),
            SweepKind::CapBudget.result_schema()
        );
        assert_ne!(
            SweepKind::EarlyExit.policy_id(),
            SweepKind::CapBudget.policy_id()
        );
        assert_ne!(
            SweepKind::EarlyExit.verifier(),
            SweepKind::CapBudget.verifier()
        );
        assert_eq!(
            SweepKind::EarlyExit.policy_id(),
            "piecewise_local_clearing_v2_policy_3_early_exit_probe_24"
        );
    }
}
