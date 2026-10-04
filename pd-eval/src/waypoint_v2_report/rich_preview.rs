//! First presentation review slice: navigation plus one annotated original-style report.
//! No simulation, archive rewrites, stable-site publication or latest selection.

use super::*;
use pd_report::{
    flight_annotations::{AnnotationNavigation, NavigationLink},
    navigation_preview::{PreviewNavigation, render_preview_home, render_preview_suite},
};

pub const RENDERER_VERSION: &str = "waypoint-v2-rich-preview-1";
const ANNOTATED_CASE: &str = "v2_ridge_late";
const ANNOTATED_PAGE: &str = "waypoint-v2/cases/v2_ridge_late/index.html";

fn report_payload(html: &str) -> Result<Value> {
    let prefix = "const reportData = ";
    let start = html
        .find(prefix)
        .context("missing original rich report payload")?
        + prefix.len();
    let mut stream = serde_json::Deserializer::from_str(&html[start..]).into_iter::<Value>();
    let value = stream.next().context("empty report payload")??;
    ensure!(
        html[start + stream.byte_offset()..].starts_with(';'),
        "unterminated report payload"
    );
    ensure!(value.is_object(), "report payload is not an object");
    Ok(value)
}

/// No tolerance or rounding: annotations are the only new data key allowed.
fn check_payload_preserved(original: &str, annotated: &str) -> Result<()> {
    let original = report_payload(original)?;
    let mut annotated = report_payload(annotated)?;
    ensure!(
        annotated
            .as_object_mut()
            .unwrap()
            .remove("flightAnnotations")
            .is_some(),
        "missing flight annotations"
    );
    if !same_json(&original, &annotated) {
        let changed = original
            .as_object()
            .unwrap()
            .iter()
            .filter(|(key, value)| annotated.get(*key).is_none_or(|new| !same_json(value, new)))
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>();
        anyhow::bail!("rich report payload changed beyond annotations: {changed:?}");
    }
    Ok(())
}

fn optional_destination(repo: &Path, relative: &str) -> Option<String> {
    let path = repo.join(relative).canonicalize().ok()?;
    path.is_file().then(|| served_href(repo, &path)).flatten()
}

struct PreparedPreview {
    pages: Vec<(String, String)>,
    inputs: BTreeMap<String, String>,
    provenance: Value,
}

fn prepare(repo: &Path, root: &Path, output: &Path) -> Result<PreparedPreview> {
    prepare_edition(repo, root, output, false)
}

fn prepare_edition(
    repo: &Path,
    root: &Path,
    output: &Path,
    site_navigation: bool,
) -> Result<PreparedPreview> {
    let mut inputs = BTreeMap::new();
    let suite: SavedSuite =
        serde_json::from_slice(&read_hashed(root, "suite-summary.json", &mut inputs)?)?;
    ensure!(
        suite.schema_id == "waypoint_v2_practical_suite_run_v1"
            && suite.status == "completed"
            && suite.policy_version == 3
            && suite.case_count == 32
            && suite.cases.len() == 32,
        "rich preview requires the completed 32-case policy-3 capture"
    );
    let mut ids = BTreeSet::new();
    for case in &suite.cases {
        ensure!(
            !case.case_id.is_empty()
                && case
                    .case_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                && ids.insert(&case.case_id),
            "unsafe or duplicate case id"
        );
        ensure!(
            matches!(case.group.as_str(), "clear" | "ordinary" | "diagnostic"),
            "unknown case group"
        );
        ensure!(
            case.summary_path == format!("runs/{}/summary.json", case.case_id)
                && case.flight_path == format!("runs/{}/flight.json", case.case_id),
            "case paths do not match identity"
        );
    }
    let home_href = if site_navigation {
        "/reports/".into()
    } else {
        served_href(repo, &output.join("index.html"))
            .context("preview must be served from outputs")?
    };
    let suite_href = served_href(repo, &output.join("waypoint-v2/index.html"))
        .context("preview collection href")?;
    let annotated_href =
        served_href(repo, &output.join(ANNOTATED_PAGE)).context("preview report href")?;
    let receipt_href = served_href(repo, &output.join("render-provenance.json"))
        .context("preview provenance href")?;
    let mut cards = Vec::new();
    let mut original_pages = BTreeMap::new();
    let mut late = None;
    for case in &suite.cases {
        let scenario_path = format!("runs/{}/scenario.json", case.case_id);
        let scenario: ScenarioSpec =
            serde_json::from_slice(&read_hashed(root, &scenario_path, &mut inputs)?)?;
        let flight_bytes = read_hashed(root, &case.flight_path, &mut inputs)?;
        let flight_json: Value = serde_json::from_slice(&flight_bytes)?;
        let summary: Value =
            serde_json::from_slice(&read_hashed(root, &case.summary_path, &mut inputs)?)?;
        check_summary(&summary, &flight_json, case)
            .with_context(|| format!("case {}", case.case_id))?;
        let result: WaypointV2FlightResult = serde_json::from_slice(&flight_bytes)?;
        ensure!(
            scenario.id == case.case_id && policy_version(&result.policy)? == 3,
            "case scenario/policy disagreement"
        );
        let recorded_outcome = outcome(&result);
        let recorded_count = result.correction_count;
        let recorded_reason = result.reason.clone();
        let original_relative = format!("runs/{}/report.html", case.case_id);
        let href = if result.ordinary_flight.is_some() {
            let original = String::from_utf8(read_hashed(root, &original_relative, &mut inputs)?)?;
            let projected = project_flight(&scenario, &result)
                .with_context(|| format!("case {}", case.case_id))?;
            let original_href = served_href(repo, &inside_file(root, &original_relative)?)
                .context("original report href")?;
            if case.case_id == ANNOTATED_CASE {
                let mut annotation = executed_annotations(&result, &projected)?;
                ensure!(
                    annotation.corrections.len() == 1,
                    "Ridge late is not the one-correction preview case"
                );
                annotation.caption = format!(
                    "Annotated rich preview · {} · {}. Original telemetry, plots and statistics retained; only this case has new waypoint annotations in this preview.",
                    friendly_title(&case.case_id),
                    outcome(&result)
                );
                annotation.navigation = AnnotationNavigation {
                    home: Some(NavigationLink {
                        label: if site_navigation {
                            "Reports home"
                        } else {
                            "Preview reports home"
                        }
                        .into(),
                        href: home_href.clone(),
                    }),
                    collection: Some(NavigationLink {
                        label: "All V2 cases".into(),
                        href: format!("{suite_href}#case-{ANNOTATED_CASE}"),
                    }),
                    source_links: vec![NavigationLink {
                        label: "Original full report (without annotations)".into(),
                        href: original_href,
                    }],
                    ..Default::default()
                };
                for (label, relative) in [
                    ("Original scenario JSON", scenario_path.as_str()),
                    ("Original flight JSON", case.flight_path.as_str()),
                    ("Original summary JSON", case.summary_path.as_str()),
                ] {
                    annotation.navigation.source_links.push(NavigationLink {
                        label: label.into(),
                        href: served_href(repo, &inside_file(root, relative)?)
                            .context("source href")?,
                    });
                }
                annotation.navigation.source_links.push(NavigationLink {
                    label: "Render provenance (not the historical flight build)".into(),
                    href: receipt_href.clone(),
                });
                late = Some((scenario, result, original, annotation));
                Some(annotated_href.clone())
            } else {
                if site_navigation {
                    original_pages.insert(case.case_id.clone(), original);
                    Some(
                        served_href(
                            repo,
                            &output.join(format!("waypoint-v2/cases/{}/index.html", case.case_id)),
                        )
                        .context("navigation-only report href")?,
                    )
                } else {
                    Some(original_href)
                }
            }
        } else {
            ensure!(
                result.planning_stop == WaypointV2Stop::Unsupported
                    && result.manifest.is_none()
                    && result.segments.is_empty()
                    && result.cycles.is_empty()
                    && result.correction_count == 0
                    && result.physical_outcome.is_none()
                    && result.mission_outcome.is_none(),
                "non-simulated case has flight evidence"
            );
            ensure!(
                !root.join(original_relative).exists(),
                "non-simulated case has fabricated report"
            );
            None
        };
        cards.push(CaseCard {
            case_id: case.case_id.clone(),
            title: friendly_title(&case.case_id),
            group: case.group.clone(),
            outcome: recorded_outcome,
            correction_count: recorded_count,
            href,
            inspect: featured(&case.case_id).map(str::to_owned),
            reason: recorded_reason,
        });
    }
    ensure!(
        cards.iter().filter(|c| c.href.is_none()).count() == 2 && inputs.len() == 127,
        "unexpected capture support or file count"
    );
    let (scenario, result, original, mut annotation) =
        late.context("missing Ridge late preview case")?;
    let supported = cards
        .iter()
        .filter(|c| c.href.is_some())
        .collect::<Vec<_>>();
    let position = supported
        .iter()
        .position(|c| c.case_id == ANNOTATED_CASE)
        .unwrap();
    for (slot, neighbor, direction) in [
        (
            &mut annotation.navigation.previous,
            position.checked_sub(1).and_then(|i| supported.get(i)),
            "Previous",
        ),
        (
            &mut annotation.navigation.next,
            supported.get(position + 1),
            "Next",
        ),
    ] {
        if let Some(case) = neighbor {
            *slot = Some(NavigationLink {
                label: format!("{direction}: {} (original full report)", case.title),
                href: case.href.clone().unwrap(),
            });
        }
    }
    let ordinary = result.ordinary_flight.as_ref().unwrap();
    let rich_html = pd_report::render_run_report_with_flight_annotations(
        &scenario,
        None,
        result.manifest.as_ref().unwrap(),
        &ordinary.events,
        &ordinary.samples,
        &[],
        None,
        None,
        None,
        None,
        Some(&annotation),
    )?;
    check_payload_preserved(&original, &rich_html)?;
    let navigation = PreviewNavigation {
        suite: SuiteReport { title: "Waypoint planner V2".into(), capture_label: "Retained policy 3 flight capture · 2026-10-02".into(),
            policy_version: 3, cases: cards, diagnostics: vec![("Renderer".into(), RENDERER_VERSION.into()),
                ("Evidence scope".into(), "Retained flight consistency only; no flights or planner/controller tests rerun by rendering.".into())],
            source_links: vec![ReportLink { label: "Original suite summary".into(), href: served_href(repo, &inside_file(root, "suite-summary.json")?).context("suite source href")? },
                ReportLink { label: "Separate rendering provenance".into(), href: receipt_href }] },
        annotated_case_id: ANNOTATED_CASE.into(), home_href, suite_href: suite_href.clone(),
        guidance_href: optional_destination(repo, "outputs/reports/guidance/index.html"),
        batch_library_href: optional_destination(repo, "outputs/reports/eval/index.html"),
        setups_href: optional_destination(repo, "outputs/reports/setups/index.html"),
        replays_href: optional_destination(repo, "outputs/reports/replays/index.html"),
        history: optional_destination(repo, "outputs/reports/waypoint-v2/presentation_20261002_v1/index.html").map(|href| ReportLink {
            label: "Earlier standalone presentation (superseded; not the detailed report)".into(), href }).into_iter().collect(),
    };
    let mut pages = vec![
        ("index.html".into(), render_preview_home(&navigation)?),
        (
            "waypoint-v2/index.html".into(),
            render_preview_suite(&navigation)?,
        ),
        (ANNOTATED_PAGE.into(), rich_html),
    ];
    if site_navigation {
        let topic_link =
            "<a href=\"/reports/topics/waypoint-planning/index.html\">Waypoint planning</a>";
        let suite_page = &mut pages[1].1;
        *suite_page = suite_page.replacen(
            "<span aria-hidden=\"true\">›</span>",
            &format!(
                "<span aria-hidden=\"true\">›</span>{topic_link}<span aria-hidden=\"true\">›</span>"
            ),
            1,
        );
        pages[0].1 = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Waypoint planner V2 report edition</title></head><body><main><nav><a href=\"/reports/\">Reports home</a> · {topic_link}</nav><h1>Waypoint planner V2 report edition</h1><p>This is a navigation edition of the retained policy 3 capture, not a new flight or a separate report home. One case has waypoint annotations; the other full reports only add return navigation.</p><p><a href=\"{suite_href}\">Open V2 mission collection</a></p></main></body></html>"
        );
        for (case_id, original) in original_pages {
            let copy = navigation_copy(&original, &case_id, &navigation)?;
            ensure!(
                same_json(&report_payload(&original)?, &report_payload(&copy)?),
                "navigation copy changed rich report payload"
            );
            pages.push((format!("waypoint-v2/cases/{case_id}/index.html"), copy));
        }
        let annotated = &mut pages[2].1;
        *annotated = navigation_copy(annotated, ANNOTATED_CASE, &navigation)?;
        ensure!(
            pages.len() == 32,
            "navigation edition must contain 30 reports and two entrypoints"
        );
    }
    Ok(PreparedPreview {
        pages,
        inputs,
        provenance: suite.provenance,
    })
}

/// Adds only ordinary HTML navigation. The original payload, plot code and
/// report body remain verbatim, with no iframe, new samples or waypoint markers.
fn navigation_copy(
    original: &str,
    case_id: &str,
    navigation: &PreviewNavigation,
) -> Result<String> {
    ensure!(
        original.matches("<body>").count() == 1,
        "unexpected rich report body"
    );
    let cases = navigation
        .suite
        .cases
        .iter()
        .filter(|c| c.href.is_some())
        .collect::<Vec<_>>();
    let index = cases
        .iter()
        .position(|c| c.case_id == case_id)
        .context("missing navigation case")?;
    let mut neighbors = String::new();
    for (label, case) in [
        ("Previous", index.checked_sub(1).and_then(|i| cases.get(i))),
        ("Next", cases.get(index + 1)),
    ] {
        if let Some(case) = case {
            neighbors.push_str(&format!(
                "<a href=\"{}\">{label}: {}</a>",
                escape_navigation(case.href.as_ref().unwrap()),
                escape_navigation(&case.title)
            ));
        }
    }
    let banner = format!(
        "<nav aria-label=\"Report hierarchy\" style=\"max-width:1480px;margin:0 auto;padding:16px 22px;display:flex;gap:12px;flex-wrap:wrap;overflow-wrap:anywhere\"><a href=\"/reports/\">Reports home</a><span>›</span><a href=\"/reports/topics/waypoint-planning/index.html\">Waypoint planning</a><span>›</span><a href=\"{}#case-{}\">V2 mission collection</a><span style=\"flex-basis:100%\">{} · {}. Navigation edition of the retained capture; no new flight.</span>{neighbors}</nav>",
        escape_navigation(&navigation.suite_href),
        escape_navigation(case_id),
        escape_navigation(&cases[index].title),
        if case_id == ANNOTATED_CASE {
            "Annotated rich preview"
        } else {
            "Original full report; waypoint annotations not yet added"
        }
    );
    Ok(original.replacen("<body>", &format!("<body>{banner}"), 1))
}

fn escape_navigation(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Narrow, create-only preview. It deliberately never refreshes ReportSite.
pub fn render_preview(
    repo_root: &Path,
    suite_root: &Path,
    output_dir: &Path,
) -> Result<RenderReceipt> {
    render_edition(repo_root, suite_root, output_dir, false)
}

pub fn render_navigation_edition(
    repo_root: &Path,
    suite_root: &Path,
    output_dir: &Path,
) -> Result<RenderReceipt> {
    render_edition(repo_root, suite_root, output_dir, true)
}

fn render_edition(
    repo_root: &Path,
    suite_root: &Path,
    output_dir: &Path,
    site_navigation: bool,
) -> Result<RenderReceipt> {
    let repo = repo_root.canonicalize()?;
    let root = suite_root.canonicalize()?;
    ensure!(
        root == repo.join(PINNED_CAPTURE).canonicalize()?,
        "rich preview is limited to the pinned capture"
    );
    let parent = output_dir
        .parent()
        .context("output parent")?
        .canonicalize()?;
    ensure!(
        parent == repo.join("outputs/reports/waypoint-v2").canonicalize()?,
        "preview must be a fresh direct child of outputs/reports/waypoint-v2"
    );
    let name = output_dir.file_name().context("output name")?;
    ensure!(
        name.to_str().is_some_and(|s| s.starts_with("rich_preview_")
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))),
        "preview output name must start rich_preview_"
    );
    let output = parent.join(name);
    ensure!(
        !output.try_exists()?,
        "create-only preview output already exists"
    );
    let prepared = if site_navigation {
        prepare_edition(&repo, &root, &output, true)?
    } else {
        prepare(&repo, &root, &output)?
    };
    let mut renderer_hashes = BTreeMap::new();
    for relative in [
        "pd-report/src/lib.rs",
        "pd-report/src/flight_annotations.rs",
        "pd-report/src/navigation_preview.rs",
        "pd-report/src/waypoint_v2/data.rs",
        "pd-eval/src/waypoint_v2_report.rs",
        "pd-eval/src/waypoint_v2_report/rich_preview.rs",
    ] {
        renderer_hashes.insert(
            relative.into(),
            sha256_bytes(&fs::read(repo.join(relative))?)?,
        );
    }
    let process_hash = sha256_bytes(&fs::read(std::env::current_exe()?)?)?;
    crate::nominal_direct_flight::reserve_output_root(&output)?;
    let mut outputs = BTreeMap::new();
    for (relative, html) in prepared.pages {
        let path = output.join(&relative);
        fs::create_dir_all(path.parent().context("page parent")?)?;
        write_bytes_create_only(&path, html.as_bytes())?;
        outputs.insert(relative, sha256_bytes(html.as_bytes())?);
    }
    let receipt = RenderReceipt {
        schema_id: if site_navigation {
            "waypoint_v2_navigation_edition_v1"
        } else {
            "waypoint_v2_rich_preview_v1"
        },
        renderer_version: if site_navigation {
            "waypoint-v2-navigation-1"
        } else {
            RENDERER_VERSION
        },
        source_root: root,
        output_root: output,
        selected_case_id: Some(ANNOTATED_CASE.into()),
        case_count: 32,
        flight_page_count: if site_navigation { 30 } else { 1 },
        input_sha256: prepared.inputs,
        output_sha256: outputs,
        historical_flight_provenance: prepared.provenance,
        renderer_source_sha256: renderer_hashes,
        rendering_process_sha256: process_hash,
    };
    crate::nominal_direct_flight::write_create_only(
        &receipt.output_root.join("render-provenance.json"),
        &receipt,
    )?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn navigation_copy_is_only_a_banner_with_escaped_names_and_neighbors() {
        let cards = ["first", "second"].map(|id| CaseCard {
            case_id: id.into(),
            title: "<unsafe & title>".into(),
            group: "clear".into(),
            outcome: "Landed directly".into(),
            correction_count: 0,
            href: Some(format!("/reports/cases/{id}/index.html")),
            inspect: None,
            reason: None,
        });
        let navigation = PreviewNavigation {
            suite: SuiteReport {
                title: "V2".into(),
                capture_label: "Retained".into(),
                policy_version: 3,
                cases: cards.into(),
                diagnostics: vec![],
                source_links: vec![],
            },
            annotated_case_id: ANNOTATED_CASE.into(),
            home_href: "/reports/".into(),
            suite_href: "/reports/cases/index.html".into(),
            guidance_href: None,
            batch_library_href: None,
            setups_href: None,
            replays_href: None,
            history: vec![],
        };
        let original = "<!doctype html><body><main>Unchanged report</main><script>const reportData = {\"samples\":[1],\"stats\":{\"x\":2}};</script></body>";
        let copy = navigation_copy(original, "first", &navigation).unwrap();
        let begin = copy.find("<nav aria-label=\"Report hierarchy\"").unwrap();
        let end = begin + copy[begin..].find("</nav>").unwrap() + "</nav>".len();
        assert_eq!(format!("{}{}", &copy[..begin], &copy[end..]), original);
        assert!(same_json(
            &report_payload(original).unwrap(),
            &report_payload(&copy).unwrap()
        ));
        assert!(copy.contains("&lt;unsafe &amp; title&gt;"));
        assert!(copy.contains("Next: "));
        assert!(!copy.contains("Previous: "));
        assert!(!copy.contains("flightAnnotations"));
        assert!(navigation_copy(original, "missing", &navigation).is_err());
        assert!(navigation_copy("<body><body>", "first", &navigation).is_err());
    }

    #[test]
    #[ignore = "requires pinned retained local V2 capture; navigation only, no missions"]
    fn pinned_navigation_edition_preserves_all_thirty_full_report_payloads() {
        let repo = repo();
        let root = repo.join(PINNED_CAPTURE).canonicalize().unwrap();
        let prepared = prepare_edition(
            &repo,
            &root,
            &repo.join("outputs/reports/waypoint-v2/rich_preview_navigation_test"),
            true,
        )
        .unwrap();
        assert_eq!(prepared.pages.len(), 32);
        assert_eq!(prepared.inputs.len(), 127);
        for (relative, html) in prepared
            .pages
            .iter()
            .filter(|(name, _)| name.contains("/cases/"))
        {
            let id = relative.split('/').nth(2).unwrap();
            let original = fs::read_to_string(root.join(format!("runs/{id}/report.html"))).unwrap();
            if id == ANNOTATED_CASE {
                check_payload_preserved(&original, html).unwrap();
                assert_eq!(
                    report_payload(html).unwrap()["flightAnnotations"]["corrections"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
            } else {
                assert!(same_json(
                    &report_payload(&original).unwrap(),
                    &report_payload(html).unwrap()
                ));
                assert!(
                    !report_payload(html)
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .contains_key("flightAnnotations")
                );
            }
            assert!(html.contains("aria-label=\"Report hierarchy\""));
            assert!(html.contains("/reports/topics/waypoint-planning/index.html"));
        }
    }

    #[test]
    #[ignore = "requires pinned retained local V2 capture; presentation only, no mission execution"]
    fn pinned_rich_preview_preserves_every_original_payload_field() {
        let repo = repo();
        let root = repo.join(PINNED_CAPTURE).canonicalize().unwrap();
        let prepared = prepare(
            &repo,
            &root,
            &repo.join("outputs/reports/waypoint-v2/rich_preview_test_only"),
        )
        .unwrap();
        assert_eq!(prepared.pages.len(), 3);
        assert_eq!(prepared.inputs.len(), 127);
        let data = report_payload(&prepared.pages[2].1).unwrap();
        let handoff = &data["flightAnnotations"]["corrections"][0]["handoff"];
        assert_eq!(handoff["physicsStep"], 2426);
        assert_eq!(handoff["simTimeS"].as_f64().unwrap(), 2426.0 / 120.0);
        assert_eq!(data["samples"].as_array().unwrap().len(), 341);
        assert!(prepared.pages[1].1.contains("case-v2_diag_other_vehicle"));
    }

    #[test]
    fn parity_gate_rejects_changed_stats_samples_and_missing_annotations() {
        let old = "const reportData = {\"samples\":[1],\"stats\":{\"x\":2}};";
        assert!(
            check_payload_preserved(
                old,
                "const reportData = {\"samples\":[1],\"stats\":{\"x\":2},\"flightAnnotations\":{}};"
            )
            .is_ok()
        );
        for new in [
            "const reportData = {\"samples\":[2],\"stats\":{\"x\":2},\"flightAnnotations\":{}};",
            "const reportData = {\"samples\":[1],\"stats\":{\"x\":3},\"flightAnnotations\":{}};",
            old,
            "broken",
        ] {
            assert!(check_payload_preserved(old, new).is_err());
        }
    }

    #[test]
    fn rich_preview_rejects_other_capture_and_existing_outputs_without_writes() {
        let repo = repo();
        let output = repo.join("outputs/reports/waypoint-v2/presentation_20261002_v1");
        assert!(render_preview(&repo, &repo.join(PINNED_CAPTURE), &output).is_err());
        assert!(render_preview(&repo, &repo.join("outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_b"),
            &repo.join("outputs/reports/waypoint-v2/rich_preview_test_only")).is_err());
    }
}
