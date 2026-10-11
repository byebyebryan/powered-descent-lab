//! Composition only: saved native audits/inputs -> neutral shared report DTOs.
//! No noise generation, agent, flight, training or accepted-site publication.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use pd_missions::{
    CoverageAxisV1, DiagnosticAxisV1, InputAuditRowV1, InputCoverageAuditV1, PopulationCoverageV1,
    ResolvedMissionV2, ScalarSummaryV1, SelectionDispositionV2, load_collection_proposal_v2,
    morphology_group_key, read_collection_manifest_v2, sha256,
};
use pd_report::{
    batch::{escape_html, render_status_page},
    input_coverage::*,
};
use serde_json::{Value, json};

#[derive(Parser)]
struct Args {
    /// Completed, create-only input pilot. Saved audit identities are required.
    #[arg(long)]
    pilot: PathBuf,
    /// Fresh local report directory; existing paths are always refused.
    #[arg(long)]
    output: PathBuf,
}

type Result<T> = std::result::Result<T, String>;

fn read(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err("expected bounded regular report input".into());
    }
    let mut data = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(maximum + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > maximum {
        return Err("report input exceeded its read bound".into());
    }
    Ok(data)
}
fn create(path: &Path, data: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    f.write_all(data)
        .and_then(|()| f.sync_all())
        .map_err(|e| e.to_string())
}
fn json_bytes(value: &impl serde::Serialize) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|e| e.to_string())
}
fn safe_file(value: &str) -> bool {
    !value.is_empty() && !value.contains(['/', '\\', ':']) && value != "." && value != ".."
}
fn number(v: Option<f64>) -> String {
    v.map_or_else(|| "Missing".into(), |v| format!("{v:.1}"))
}
fn summary(s: &ScalarSummaryV1) -> String {
    format!(
        "min {} · p10 {} · median {} · p90 {} · max {} · missing {}",
        number(s.min),
        number(s.p10),
        number(s.p50),
        number(s.p90),
        number(s.max),
        s.missing
    )
}
fn axis_label(a: CoverageAxisV1) -> &'static str {
    match a {
        CoverageAxisV1::Distance => "Transfer distance (m)",
        CoverageAxisV1::SignedRise => "Signed target-pad rise (m)",
        CoverageAxisV1::SourceWidth => "Source-pad width (m)",
        CoverageAxisV1::TargetWidth => "Target-pad width (m)",
        CoverageAxisV1::ChordBarrier => "Maximum terrain above pad chord (m)",
        CoverageAxisV1::SustainedBarrierWidth => "Longest sustained barrier at 100 m (m)",
        CoverageAxisV1::DepartureDistance => "Departure-to-100 m barrier distance (m)",
        CoverageAxisV1::ApproachDistance => "Approach-to-100 m barrier distance (m)",
        CoverageAxisV1::CrestCount => "Significant crest count",
        CoverageAxisV1::MinCrestSpacing => "Minimum significant-crest spacing (m)",
    }
}
fn bins(a: &DiagnosticAxisV1) -> Vec<String> {
    let edges = &a.interior_boundaries;
    let mut b = vec![format!("<{}", number(Some(edges[0])))];
    b.extend(
        edges
            .windows(2)
            .map(|e| format!("{}–<{}", number(Some(e[0])), number(Some(e[1])))),
    );
    b.push(format!("≥{}", number(edges.last().copied())));
    b.push("Missing".into());
    b
}
fn value(row: &InputAuditRowV1, axis: CoverageAxisV1) -> Option<f64> {
    row.values
        .iter()
        .find(|v| v.axis == axis)
        .and_then(|v| v.value)
}
fn band(row: &InputAuditRowV1) -> (String, String) {
    // Explicit presentation bands, introduced after admission. They do not
    // alter the predeclared diagnostic axes, selection or V1 terrain strata.
    match value(row, CoverageAxisV1::ChordBarrier) {
        Some(v) if v < 100.0 => ("b0".into(), "Barrier <100 m".into()),
        Some(v) if v < 200.0 => ("b1".into(), "Barrier 100–<200 m".into()),
        Some(v) if v < 400.0 => ("b2".into(), "Barrier 200–<400 m".into()),
        Some(_) => ("b3".into(), "Barrier ≥400 m".into()),
        None => ("missing".into(), "Barrier missing".into()),
    }
}
fn qa_sets(a: &InputCoverageAuditV1) -> Vec<QaSetView> {
    [true, false]
        .into_iter()
        .map(|regular| {
            let qs: Vec<_> = a
                .reference_qa
                .iter()
                .filter(|q| a.spec.reference_qa.is_regular(q.index) == regular)
                .collect();
            QaSetView {
                label: if regular {
                    "Regular index sample"
                } else {
                    "Targeted admission boundaries"
                }
                .into(),
                count: qs.len(),
                rightward: qs.iter().filter(|q| q.index % 2 == 0).count(),
                leftward: qs.iter().filter(|q| q.index % 2 == 1).count(),
                raw_error_flags: qs.iter().filter(|q| q.flags.raw_interpolation).count(),
                barrier_flags: qs.iter().filter(|q| q.flags.chord_barrier).count(),
                width_flags: qs.iter().filter(|q| q.flags.sustained_width).count(),
                admission_flips: qs.iter().filter(|q| q.flags.admission_disagreement).count(),
                maximum_raw_error_m: qs
                    .iter()
                    .map(|q| q.maximum_raw_interpolation_error_m)
                    .fold(0.0, f64::max),
                flagged_inputs:qs.iter().filter(|q|q.flags.raw_interpolation || q.flags.chord_barrier || q.flags.sustained_width || q.flags.admission_disagreement).map(|q|format!("Mission {}: raw interpolation error {:.2} m; flags raw={} / barrier={} / width={} / admission={}",q.index,q.maximum_raw_interpolation_error_m,q.flags.raw_interpolation,q.flags.chord_barrier,q.flags.sustained_width,q.flags.admission_disagreement)).collect(),
            }
        })
        .collect()
}
fn collection_view(id: &str, a: &InputCoverageAuditV1, audit_sha: &str) -> InputCollectionView {
    let p = &a.proposed;
    let s = &a.admitted;
    let distributions = a
        .spec
        .coverage
        .axes
        .iter()
        .map(|d| {
            let proposed = p.axes.iter().find(|v| v.axis == d.axis).unwrap();
            let admitted = s.axes.iter().find(|v| v.axis == d.axis).unwrap();
            DistributionView {
                key: d.axis.key().into(),
                label: axis_label(d.axis).into(),
                bins: bins(d),
                proposed: proposed.counts.clone(),
                admitted: admitted.counts.clone(),
                admitted_right: admitted.right_counts.clone(),
                admitted_left: admitted.left_counts.clone(),
                summary: format!("Admitted: {}", summary(&admitted.summary)),
            }
        })
        .collect();
    let joints=s.joints.iter().zip(&p.joints).map(|(s,p)|JointView{
        label:format!("{} × {}",axis_label(s.axes[0]),axis_label(s.axes[1])),
        x_bins:bins(a.spec.coverage.axis(s.axes[0]).unwrap()),y_bins:bins(a.spec.coverage.axis(s.axes[1]).unwrap()),
        proposed:p.counts.clone(),admitted:s.counts.clone(),correlation_note:format!("Observed pairs: proposed {} / admitted {}. Pearson r: proposed {} / admitted {}. Missing values remain in the matrices, and are omitted from correlation.",p.observed_pairs,s.observed_pairs,number(p.pearson_r),number(s.pearson_r))}).collect();
    let mut dispositions = BTreeMap::new();
    for (k, v) in serde_json::to_value(&a.counts)
        .unwrap()
        .as_object()
        .unwrap()
    {
        if k != "scheduled" {
            dispositions.insert(k.clone(), v.as_u64().unwrap() as usize);
        }
    }
    let mut scalar_rows: Vec<_> = s
        .scalar_summaries
        .iter()
        .map(|(k, v)| [k.clone(), summary(v)])
        .collect();
    for (k, v) in &s.morphology_groups {
        scalar_rows.push([format!("Exclusive morphology: {k}"), v.to_string()]);
    }
    for (k, v) in &s.overlapping_labels {
        scalar_rows.push([format!("Overlapping label: {k}"), v.to_string()]);
    }
    for (k, v) in &s.challenge_paths {
        scalar_rows.push([format!("Overlapping admission path: {k}"), v.to_string()]);
    }
    for (group, counts) in &s.directions_by_group {
        scalar_rows.push([
            format!("Direction within {group}"),
            format!("{} right / {} left", counts[0], counts[1]),
        ]);
    }
    scalar_rows.push([
        "Physical direction".into(),
        format!("{} rightward / {} leftward", s.rightward, s.leftward),
    ]);
    let c = &s.concentration;
    InputCollectionView {
        id: id.into(),
        label: if id == "layered-broad" {
            "Layered Perlin"
        } else {
            "Value noise"
        }
        .into(),
        scheduled: a.counts.scheduled,
        proposed: p.count,
        admitted: s.count,
        dispositions,
        distributions,
        joints,
        qa_sets: qa_sets(a),
        scalar_rows,
        provenance_rows: vec![
            [
                "Seed · exposed development".into(),
                a.request.seed.to_string(),
            ],
            [
                "Recipe identity".into(),
                a.request.recipe.identity().unwrap(),
            ],
            ["Audit file SHA256".into(), audit_sha.into()],
            [
                "Manifest file SHA256".into(),
                a.immutable_file_sha256["manifest.json"].clone(),
            ],
            [
                "Metric version".into(),
                a.request.geometry_spec.metric_version.clone(),
            ],
            [
                "Admission rule".into(),
                serde_json::to_string(&a.request.challenge_rule).unwrap(),
            ],
            [
                "Morphology labels".into(),
                "Post-generation navigation; mixed and unclassified qualifying inputs retained"
                    .into(),
            ],
        ],
        concentration_note: format!(
            "Coarse shape buckets: {} unique, {} shared, {} inputs in shared buckets, largest {}. Method {} is a sparse quantized projection, not proof of distinct difficulty or a near-neighbor distance.",
            c.unique_buckets,
            c.shared_buckets,
            c.rows_in_shared_buckets,
            c.largest_bucket,
            c.method_version
        ),
    }
}
fn verify_files(root: &Path, files: &BTreeMap<String, String>) -> Result<()> {
    for (file, expected) in files {
        if !safe_file(file) || sha256(&read(&root.join(file), 64 * 1024 * 1024)?) != *expected {
            return Err(format!("immutable pilot input changed: {file}"));
        }
    }
    Ok(())
}
fn validate_projection(a: &InputCoverageAuditV1) -> Result<()> {
    a.spec.validate(&a.request)?;
    if a.schema_version != 1 || !a.complete_saved_input_verification {
        return Err("unverified or unsupported input audit".into());
    }
    let proposed: Vec<_> = a.rows.iter().collect();
    let admitted: Vec<_> = a
        .rows
        .iter()
        .filter(|r| r.disposition == SelectionDispositionV2::Accepted)
        .collect();
    for (rows, population) in [(&proposed, &a.proposed), (&admitted, &a.admitted)] {
        let measured: PopulationCoverageV1 =
            pd_missions::summarize_coverage(rows, &a.spec.coverage)?;
        // Round-tripped JSON floats can differ by an ULP. Counts/memberships are
        // compared exactly; scalar summaries are validated with relative tolerance.
        compare_json(
            &serde_json::to_value(measured).map_err(|e| e.to_string())?,
            &serde_json::to_value(population).map_err(|e| e.to_string())?,
        )?;
    }
    if a.preview_indices.len() > a.spec.preview_maximum
        || a.preview_indices.iter().collect::<BTreeSet<_>>().len() != a.preview_indices.len()
    {
        return Err("preview inventory exceeds its declared bound or order".into());
    }
    Ok(())
}
fn compare_json(a: &Value, b: &Value) -> Result<()> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) if a.is_f64() || b.is_f64() => {
            let x = a.as_f64().unwrap();
            let y = b.as_f64().unwrap();
            if (x - y).abs() > 1e-9 * x.abs().max(y.abs()).max(1.0) {
                return Err("audit scalar projection mismatch".into());
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (a, b) in a.iter().zip(b) {
                compare_json(a, b)?;
            }
        }
        (Value::Object(a), Value::Object(b)) if a.len() == b.len() => {
            for (key, value) in a {
                compare_json(value, b.get(key).ok_or("audit projection key missing")?)?;
            }
        }
        _ if a == b => {}
        _ => return Err("audit population projection mismatch".into()),
    }
    Ok(())
}

fn run(args: Args) -> Result<()> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let executable_sha = sha256(&read(&executable, 128 * 1024 * 1024)?);
    let result_bytes = read(&args.pilot.join("run-result.json"), 1024 * 1024)?;
    let result: Value = serde_json::from_slice(&result_bytes).map_err(|e| e.to_string())?;
    if result["schema_version"] != 1
        || result["completed"] != true
        || result["source_unchanged"] != true
        || result["flight_training_budget"] != 0
        || result["benchmark_frozen"] != false
        || result["report_publication"] != false
    {
        return Err("requires a completed exposed input-only pilot".into());
    }
    let declarations = result["collections"]
        .as_array()
        .ok_or("pilot collection inventory missing")?;
    if declarations.len() != 2 {
        return Err("this pilot report requires both declared backend streams".into());
    }
    let mut source_files = BTreeMap::from([("run-result.json".to_string(), sha256(&result_bytes))]);
    let mut audits = Vec::new();
    let mut collection_views = Vec::new();
    let mut bands: BTreeMap<(String, String, String), InputBandView> = BTreeMap::new();
    let mut selected: Vec<(String, InputAuditRowV1, ResolvedMissionV2)> = Vec::new();
    for entry in declarations {
        let id = entry["id"].as_str().ok_or("collection id missing")?;
        let file = entry["audit_file"]
            .as_str()
            .ok_or("audit filename missing")?;
        if !["layered-broad", "value-broad"].contains(&id)
            || audits.iter().any(|(prior, _)| prior == id)
            || !safe_file(file)
        {
            return Err("unknown or duplicate pilot stream".into());
        }
        let data = read(&args.pilot.join(file), 128 * 1024 * 1024)?;
        let audit_sha = sha256(&data);
        if entry["audit_sha256"] != audit_sha {
            return Err("saved audit bytes differ from completed pilot".into());
        }
        source_files.insert(file.into(), audit_sha.clone());
        let audit: InputCoverageAuditV1 =
            serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        validate_projection(&audit)?;
        let root = args.pilot.join(id);
        verify_files(&root, &audit.immutable_file_sha256)?;
        let manifest = read_collection_manifest_v2(&root)?;
        if manifest.request != audit.request
            || manifest.counts != audit.counts
            || entry["counts"] != serde_json::to_value(&audit.counts).unwrap()
            || entry["manifest_sha256"] != audit.immutable_file_sha256["manifest.json"]
            || entry["preview_inputs"] != audit.preview_indices.len()
            || entry["reference_qa"] != audit.reference_qa.len()
        {
            return Err("pilot report inventory disagrees".into());
        }
        for row in &audit.rows {
            let receipt = manifest
                .receipts
                .iter()
                .find(|r| r.index == row.index)
                .ok_or("audit proposal absent from manifest")?;
            if receipt.proposal.as_ref() != Some(&row.proposal)
                || receipt.disposition != row.disposition
                || receipt.challenge.as_ref() != Some(&row.challenge)
                || receipt.labels != row.labels
            {
                return Err("audit row disagrees with saved selection receipt".into());
            }
            let (band_id, label) = band(row);
            let group = morphology_group_key(&row.labels).to_string();
            let b = bands
                .entry((id.into(), group.clone(), band_id.clone()))
                .or_insert_with(|| InputBandView {
                    collection: id.into(),
                    group,
                    band: band_id,
                    label,
                    proposed: 0,
                    admitted: 0,
                });
            b.proposed += 1;
            b.admitted += usize::from(row.disposition == SelectionDispositionV2::Accepted);
        }
        for index in &audit.preview_indices {
            let row = audit
                .rows
                .iter()
                .find(|r| r.index == *index)
                .ok_or("selected preview absent from audit")?;
            let receipt = manifest
                .receipts
                .iter()
                .find(|r| r.index == *index)
                .ok_or("selected preview absent from ledger")?;
            selected.push((
                id.into(),
                row.clone(),
                load_collection_proposal_v2(&root, &manifest, receipt)?,
            ));
        }
        collection_views.push(collection_view(id, &audit, &audit_sha));
        audits.push((id.to_string(), audit));
    }
    let mut bounds = InputPlotBounds {
        min_x_m: f64::INFINITY,
        max_x_m: f64::NEG_INFINITY,
        min_y_m: f64::INFINITY,
        max_y_m: f64::NEG_INFINITY,
    };
    for (_, _, m) in &selected {
        let source = &m.scenario.world.landing_pads[0];
        let target = &m.scenario.world.landing_pads[1];
        let sign = (target.center_x_m - source.center_x_m).signum();
        for p in m
            .raw_terrain
            .points()
            .iter()
            .chain(m.scenario.world.terrain.points())
        {
            let x = (p.x - source.center_x_m) * sign;
            let y = p.y - source.surface_y_m;
            bounds.min_x_m = bounds.min_x_m.min(x);
            bounds.max_x_m = bounds.max_x_m.max(x);
            bounds.min_y_m = bounds.min_y_m.min(y);
            bounds.max_y_m = bounds.max_y_m.max(y);
        }
    }
    bounds.min_x_m = (bounds.min_x_m / 500.0).floor() * 500.0;
    bounds.max_x_m = (bounds.max_x_m / 500.0).ceil() * 500.0;
    bounds.min_y_m = (bounds.min_y_m / 500.0).floor() * 500.0;
    bounds.max_y_m = (bounds.max_y_m / 500.0).ceil() * 500.0;
    // Creation happens after input validation; existing directories and dangling
    // destination symlinks are refused. Partial new output remains on failure.
    fs::create_dir(&args.output).map_err(|e| format!("fresh output required: {e}"))?;
    fs::create_dir(args.output.join("previews")).map_err(|e| e.to_string())?;
    fs::create_dir(args.output.join("details")).map_err(|e| e.to_string())?;
    let mut previews = Vec::new();
    let mut output_files = BTreeMap::new();
    for (id, row, m) in &selected {
        let key = format!("{id}-{:05}", row.index);
        let (band_id, _) = band(row);
        let svg_file = format!("previews/{key}.svg");
        let detail_file = format!("details/{key}.html");
        let snapshot_file = format!("details/{key}.json");
        let metrics = format!(
            "Distance {} m · pad rise {} m · barrier {} m · sustained width {} m · crests {}",
            number(value(row, CoverageAxisV1::Distance)),
            number(value(row, CoverageAxisV1::SignedRise)),
            number(value(row, CoverageAxisV1::ChordBarrier)),
            number(value(row, CoverageAxisV1::SustainedBarrierWidth)),
            number(value(row, CoverageAxisV1::CrestCount))
        );
        let disposition = match row.disposition {
            SelectionDispositionV2::Accepted => "Admitted",
            SelectionDispositionV2::InsufficientChallenge => "Excluded: insufficient challenge",
            SelectionDispositionV2::PhysicalDuplicate { .. } => "Excluded: physical duplicate",
            SelectionDispositionV2::StressExcluded => "Excluded: stress membership/quota",
            SelectionDispositionV2::StructuralExhaustion => "Unresolved: structural exhaustion",
        };
        let edits: Vec<_> = m
            .geometry
            .pad_edit_intervals_x_m
            .iter()
            .map(|i| [i.start_m, i.end_m])
            .collect();
        let svg = render_input_terrain_svg(InputTerrainPlot {
            raw: m.raw_terrain.points(),
            prepared: m.scenario.world.terrain.points(),
            source: &m.scenario.world.landing_pads[0],
            target: &m.scenario.world.landing_pads[1],
            edit_intervals_x_m: &edits,
            bounds,
        })?;
        let snapshot = read(
            &args.pilot.join(id).join(&row.proposal.file),
            16 * 1024 * 1024,
        )?;
        if sha256(&snapshot) != row.proposal.sha256 {
            return Err("preview input changed during projection".into());
        }
        let title = format!("{id} · mission {} · input geometry", row.index);
        let mut admission_rows = String::new();
        for (label, raw, prepared) in [
            (
                "Sustained obstruction",
                row.challenge.raw.sustained_barrier,
                row.challenge.prepared.sustained_barrier,
            ),
            (
                "Substantial pad rise",
                row.challenge.raw.elevation_transition,
                row.challenge.prepared.elevation_transition,
            ),
            (
                "Repeated significant crests",
                row.challenge.raw.successive_crests,
                row.challenge.prepared.successive_crests,
            ),
        ] {
            admission_rows.push_str(&format!(
                "<tr><th>{label}</th><td>{}</td><td>{}</td><td>{}</td></tr>",
                if raw { "Pass" } else { "Below threshold" },
                if prepared { "Pass" } else { "Below threshold" },
                if raw && prepared {
                    "Qualifying path"
                } else {
                    "—"
                }
            ));
        }
        let measured_rows = row
            .scalar_metrics
            .iter()
            .map(|(k, v)| {
                format!(
                    "<tr><th>{}</th><td>{}</td></tr>",
                    escape_html(k),
                    number(*v)
                )
            })
            .collect::<String>();
        let band_key =
            serde_json::to_string(&json!([id, morphology_group_key(&row.labels), band_id]))
                .unwrap();
        let encoded_band = band_key
            .bytes()
            .map(|b| format!("%{b:02X}"))
            .collect::<String>();
        let navigation = format!(
            "<nav><a href=\"../index.html?band={encoded_band}#input-review\">← Input coverage and this mission band</a></nav>"
        );
        let body = format!(
            "<h2>{}</h2><p>{}</p><p>{} · physical travel {} · display source → target toward the right</p><img style=\"width:100%;max-width:1100px\" src=\"../{svg_file}\" alt=\"Raw and prepared terrain with shared metre axes\"><p>Gray: raw terrain. Teal: prepared terrain. Amber: bounded pad-edit intervals. Dashed line: pad chord used by the metrics. Shared axes span the complete selected preview inventory; horizontal and vertical pixels per metre differ.</p><h3>Admission paths</h3><div class=\"table-wrap\"><table class=\"context-table\"><thead><tr><th>Declared demand</th><th>Raw</th><th>Prepared</th><th>Both</th></tr></thead><tbody>{admission_rows}</tbody></table></div><p>A mission qualifies when the same declared path passes on both raw and prepared terrain. Morphology labels: {}.</p><details><summary>Vehicle and mission envelope</summary><p>Gravity {:.2} m/s² · hull {:.1} × {:.1} m · fuel {:.1} kg · maximum thrust {:.1} N · horizon {:.1} s. These describe the input; they do not prove reachability.</p></details><details><summary>Complete measured scalar inputs</summary><div class=\"table-wrap\"><table class=\"context-table\"><tbody>{measured_rows}</tbody></table></div></details><details><summary>Raw / prepared descriptors and pad edits</summary><pre style=\"white-space:pre-wrap\">{}</pre></details><p>Physical identity <code style=\"overflow-wrap:anywhere\">{}</code></p><p><a href=\"{key}.json\">Original saved resolved input (JSON)</a></p>",
            escape_html(disposition),
            escape_html(&metrics),
            escape_html(id),
            if row.direction > 0 { "right" } else { "left" },
            escape_html(&serde_json::to_string(&row.labels).unwrap()),
            m.scenario.world.gravity_mps2,
            m.scenario.vehicle.geometry.hull_width_m,
            m.scenario.vehicle.geometry.hull_height_m,
            m.scenario.vehicle.initial_fuel_kg,
            m.scenario.vehicle.max_thrust_n,
            m.scenario.sim.max_time_s,
            escape_html(
                &serde_json::to_string_pretty(&json!({"raw":m.raw_geometry,"prepared":m.geometry}))
                    .unwrap()
            ),
            escape_html(&m.physical_sha256)
        );
        let html = render_status_page(&title, &navigation, &body);
        for (file, data) in [
            (&svg_file, svg.as_bytes()),
            (&detail_file, html.as_bytes()),
            (&snapshot_file, snapshot.as_slice()),
        ] {
            create(&args.output.join(file), data)?;
            output_files.insert(file.clone(), sha256(data));
        }
        previews.push(InputPreviewView {
            collection: id.clone(),
            group: morphology_group_key(&row.labels).into(),
            band: band_id,
            index: row.index,
            direction: row.direction,
            physical_sha256: row.proposal.physical_sha256.clone(),
            disposition: disposition.into(),
            metrics,
            preview_href: svg_file,
            detail_href: detail_file,
        });
    }
    let mut findings = Vec::new();
    for c in &collection_views {
        let audit = &audits.iter().find(|(id, _)| id == &c.id).unwrap().1;
        findings.push(format!(
            "{}: {}/{} admitted; {} unclassified and {} mixed qualifying inputs retained.",
            c.label,
            audit.counts.accepted,
            audit.counts.scheduled,
            audit
                .admitted
                .morphology_groups
                .get("unclassified")
                .copied()
                .unwrap_or(0),
            audit
                .admitted
                .morphology_groups
                .get("mixed")
                .copied()
                .unwrap_or(0)
        ));
    }
    let qa_count: usize = collection_views
        .iter()
        .flat_map(|c| &c.qa_sets)
        .map(|q| q.count)
        .sum();
    let qa_flips: usize = collection_views
        .iter()
        .flat_map(|c| &c.qa_sets)
        .map(|q| q.admission_flips)
        .sum();
    let qa_raw_flags: usize = collection_views
        .iter()
        .flat_map(|c| &c.qa_sets)
        .map(|q| q.raw_error_flags)
        .sum();
    findings.push(format!("Finer-resolution inspections: {qa_count}; admission flips: {qa_flips}; raw interpolation flags: {qa_raw_flags}. Regular and targeted samples are reported separately below."));
    let mut physical_streams: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, audit) in &audits {
        for row in audit
            .rows
            .iter()
            .filter(|r| r.disposition == SelectionDispositionV2::Accepted)
        {
            *physical_streams
                .entry(&row.proposal.physical_sha256)
                .or_default() += 1;
        }
    }
    let shared = physical_streams.values().filter(|&&n| n > 1).count();
    if result["exact_shared_admitted_inputs"] != shared {
        return Err("cross-stream duplicate receipt disagrees".into());
    }
    findings.push(format!("Exact admitted identities shared across backend streams: {shared}. Marginal and joint distributions below describe coverage; broad sampling does not imply independent or exhaustive physical demands."));
    let page = InputCoveragePage {
        title: "Challenging procedural inputs · V2 calibration".into(),
        subtitle: "Two independently sampled backend streams; continuous proposals, geometry admission and post-generation morphology.".into(),
        collections: collection_views, bands: bands.into_values().collect(), previews, findings,
        limits: vec![
            "Exposed development pilot; no held-out benchmark freeze and no controller performance estimate.".into(),
            "One nominal Earth vehicle, fixed fuel/thrust/horizon, rest starts and two pads. No start-velocity/resource/plant variation was sampled.".into(),
            "Terrain is a sampled 1D heightfield: no overhangs, caves, 3D traversal or continuous-terrain error bound.".into(),
            "Four morphology labels, ten axes and five joint audits describe selected projections. Empty bins can reflect support, admission or finite sampling; they are not automatically missing feasible cases.".into(),
            "Long/high transfers may exceed the declared plant resources or time limit. Geometry admission does not establish physical reachability.".into(),
            "Finer reference QA is bounded, counterfactual and input-only. The quantized shape bucket is a concentration diagnostic, not a near-neighbor proof.".into(),
        ], receipt_href: "render.json".into(),
    };
    let projection = json_bytes(&page)?;
    create(&args.output.join("projection.json"), &projection)?;
    output_files.insert("projection.json".into(), sha256(&projection));
    let html = render_input_coverage_page(&page)?;
    create(&args.output.join("index.html"), html.as_bytes())?;
    output_files.insert("index.html".into(), sha256(html.as_bytes()));
    for (id, audit) in &audits {
        verify_files(&args.pilot.join(id), &audit.immutable_file_sha256)?;
    }
    for (file, expected) in &source_files {
        if sha256(&read(&args.pilot.join(file), 128 * 1024 * 1024)?) != *expected {
            return Err("pilot source changed during rendering".into());
        }
    }
    if sha256(&read(&executable, 128 * 1024 * 1024)?) != executable_sha {
        return Err("report executable changed during rendering".into());
    }
    let receipt = json!({"schema_version":1,"projection_version":"pd-report/input-coverage-v1","completed":true,"executable_sha256":executable_sha,"pilot_root":args.pilot,"pilot_source_sha256":source_files,"collection_immutable_sha256":audits.iter().map(|(id,a)| (id,&a.immutable_file_sha256)).collect::<BTreeMap<_,_>>(),"plot_bounds":bounds,"output_sha256":output_files,"preview_count":selected.len(),"maximum_visible_previews":12,"open_preview_bands":1,"flight_training_budget":0,"accepted_report_publication":false,"immutable_inputs_unchanged":true});
    create(&args.output.join("render.json"), &json_bytes(&receipt)?)?;
    println!(
        "Rendered {} selected inputs into {}; no flights or accepted-site publication",
        selected.len(),
        args.output.display()
    );
    Ok(())
}
fn main() {
    if let Err(error) = run(Args::parse()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
