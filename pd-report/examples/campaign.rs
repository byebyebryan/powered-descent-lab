//! Create-only projection of a common campaign onto the maintained report shell.
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.len() != 2 {
        return Err("usage: campaign REPORT_INPUT_JSON NEW_REPORT_HTML".into());
    }
    let page: pd_report::campaign::CampaignPageV1 =
        serde_json::from_slice(&std::fs::read(&arguments[0])?)?;
    let html = pd_report::campaign::render_campaign_page(&page).map_err(std::io::Error::other)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&arguments[1])?;
    file.write_all(html.as_bytes())?;
    Ok(())
}
