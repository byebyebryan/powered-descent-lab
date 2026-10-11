//! Create-only preview edition from a sealed saved campaign; no simulation/replay.
#[path = "support/saved_campaign.rs"]
mod saved_campaign;

use anyhow::{Result, ensure};
use pd_report::campaign::CampaignPageV1;
use saved_campaign::{
    capture_directory, digest, preview_projection, read_capture, read_regular, write_new,
};
use std::{fs, path::PathBuf, time::Instant};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3,
        "usage: refine_campaign SAVED_CAMPAIGN_DIRECTORY NEW_EDITION_DIRECTORY SOURCE_HREF_BASE"
    );
    let source = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    let base = &args[2];
    ensure!(
        base.starts_with('/')
            && !base.starts_with("//")
            && !base.contains(['\\', '?', '#', ':'])
            && base.ends_with('/'),
        "source href base must be a local absolute directory URL"
    );
    let input = read_regular(&source.join("report-input.json"), 16 * 1024 * 1024)?;
    let mut page: CampaignPageV1 = serde_json::from_slice(&input)?;
    pd_report::campaign::render_campaign_page(&page).map_err(anyhow::Error::msg)?;
    fs::create_dir(&output)?;
    let started = Instant::now();
    let mut bindings = Vec::new();
    for i in 0..page.attempts.len() {
        let row = &page.attempts[i];
        if row.disposition != "executed" {
            continue;
        }
        let capture = capture_directory(&source, row)?;
        let (value, identity) = read_capture(&capture, &page, row)?;
        let asset = format!("previews/attempt-{:06}.svg", row.attempt_index);
        let (svg, view) = preview_projection(&value, asset.clone())?;
        write_new(&output.join(&asset), svg.as_bytes())?;
        bindings.push(serde_json::json!({"attempt_index":row.attempt_index,"physical_sha256":row.physical_sha256,"evaluation_sha256":identity,"preview_sha256":digest(svg.as_bytes())}));
        let row = &mut page.attempts[i];
        row.presentation = Some(view);
        row.report_href = row.report_href.as_ref().map(|href| format!("{base}{href}"));
    }
    page.summary_href = format!("{base}summary.json");
    let html = pd_report::campaign::render_campaign_page(&page).map_err(anyhow::Error::msg)?;
    write_new(&output.join("report.html"), html.as_bytes())?;
    write_new(
        &output.join("report-input.json"),
        &serde_json::to_vec_pretty(&page)?,
    )?;
    let receipt = serde_json::json!({"schema_version":1,"source":source,"source_projection_sha256":digest(&input),"scheduled":page.attempts.len(),"previews":bindings,"rendered_html_sha256":digest(html.as_bytes()),"elapsed_s":started.elapsed().as_secs_f64(),"new_flights":0,"new_replays":0});
    write_new(
        &output.join("render.json"),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!(
        "Rendered {} attempts in {:.2}s: {}",
        page.attempts.len(),
        started.elapsed().as_secs_f64(),
        output.display()
    );
    Ok(())
}
