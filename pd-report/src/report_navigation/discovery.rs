//! Read-only, bounded discovery of saved report entrypoints, never mission data.
use std::io::Read;

use super::*;

// Scan collections/editions, not thousands of mission bundles or source clones.
const SCOPES: &[&str] = &[
    "reports/eval",
    "reports/setups",
    "reports/runs",
    "reports/replays",
    "eval",
    "research",
];
const SKIP: &[&str] = &[
    "latest",
    "cache",
    "runs",
    "cases",
    "details",
    "previews",
    "candidate",
    "source",
    "inputs",
    "baseline",
    "bin",
];

pub(super) fn extend_rows(
    rows: &mut Vec<Row>,
    outputs: &Path,
    topics: &BTreeMap<&str, &str>,
) -> Result<()> {
    let mut linked = rows
        .iter()
        .filter_map(|row| row.href.clone())
        .collect::<BTreeSet<_>>();
    for scope in SCOPES {
        let mut pages = Vec::new();
        scan(outputs, &outputs.join(scope), 3, &mut pages)?;
        for relative in pages {
            let href = format!("/{relative}");
            if !linked.insert(href.clone()) {
                continue;
            }
            let capture = !relative.starts_with("reports/");
            let topic_id = if relative.contains("planner") || relative.contains("waypoint") {
                "waypoint-planning"
            } else if relative.contains("terminal") || relative.contains("transfer") {
                "flight-control"
            } else {
                "unclassified"
            };
            let topic_id = if topics.contains_key(topic_id) {
                topic_id
            } else {
                "unclassified"
            };
            let path = Path::new(&relative);
            let parent = path.parent().expect("discovered page parent");
            let name = parent
                .file_name()
                .expect("discovered page collection")
                .to_string_lossy();
            let title = slug_title(name.strip_prefix("capture-").unwrap_or(&name));
            let (page_title, chips) = page_copy(&outputs.join(&relative))?;
            let mut description = page_title.unwrap_or_else(|| "Saved report page.".to_owned());
            if !chips.is_empty() {
                description.push_str(" · Reported: ");
                description.push_str(&chips.join(" · "));
            }
            let report_type = if capture {
                "Experimental / archived report"
            } else {
                "Saved report"
            };
            rows.push(Row {
                id: format!("saved-{}", path_id(&relative)),
                title,
                description,
                topic_id: topic_id.to_owned(),
                topic: topics
                    .get(topic_id)
                    .copied()
                    .unwrap_or("Unclassified")
                    .to_owned(),
                type_id: if capture {
                    "experimental-report"
                } else {
                    "saved-report"
                }
                .to_owned(),
                report_type: report_type.to_owned(),
                status: if capture {
                    "Saved capture · not acceptance"
                } else {
                    "Saved report · availability only"
                }
                .to_owned(),
                availability: "Available".to_owned(),
                href: Some(href),
                updated: None,
            });
        }
    }
    Ok(())
}

fn scan(outputs: &Path, directory: &Path, depth: usize, pages: &mut Vec<String>) -> Result<()> {
    // No followed directory/file symlinks, including a scope-root symlink.
    let mut ancestor = directory;
    while ancestor != outputs {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {}
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error).context("inspect report discovery directory"),
        }
        ancestor = ancestor
            .parent()
            .context("discovery path escaped outputs")?;
    }
    for child in fs::read_dir(directory)? {
        let child = child?;
        let kind = child.file_type()?;
        let name = child.file_name().to_string_lossy().into_owned();
        if name.starts_with('.')
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        {
            continue;
        }
        if kind.is_dir() && depth > 0 && !SKIP.contains(&name.as_str()) {
            scan(outputs, &child.path(), depth - 1, pages)?;
        } else if kind.is_file() && matches!(name.as_str(), "index.html" | "report.html") {
            let relative = child
                .path()
                .strip_prefix(outputs)?
                .to_string_lossy()
                .into_owned();
            // Exclude the generated scope indexes themselves.
            if SCOPES
                .iter()
                .any(|scope| relative == format!("{scope}/index.html"))
            {
                continue;
            }
            validate_html_path(&relative)?;
            pages.push(relative);
        }
    }
    pages.sort();
    Ok(())
}

/// Read only the small common HTML header, not embedded plots or native results.
fn page_copy(path: &Path) -> Result<(Option<String>, Vec<String>)> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(65_536).read_to_end(&mut bytes)?;
    let header = String::from_utf8_lossy(&bytes);
    let title = text_between(&header, "<title>", "</title>");
    let mut chips = Vec::new();
    let mut rest = header.as_ref();
    while let Some((_, after)) = rest.split_once("<span class=\"chip\">") {
        let Some((text, after)) = after.split_once("</span>") else {
            break;
        };
        if let Some(text) = plain_text(text) {
            chips.push(text);
        }
        rest = after;
        if chips.len() == 4 {
            break;
        }
    }
    Ok((title, chips))
}

fn text_between(text: &str, start: &str, end: &str) -> Option<String> {
    plain_text(text.split_once(start)?.1.split_once(end)?.0)
}

fn plain_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty()
        || text.len() > 400
        || text.contains(['<', '>'])
        || text.chars().any(char::is_control)
    {
        return None;
    }
    Some(
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
    )
}

pub(super) fn sort_rows(rows: &mut [Row], outputs: &Path) -> Result<()> {
    for row in rows.iter_mut() {
        row.updated = row
            .href
            .as_ref()
            .map(|href| -> Result<Option<SystemTime>> {
                let relative = href.strip_prefix('/').context("report href is not local")?;
                Ok(fs::metadata(outputs.join(relative))?.modified().ok())
            })
            .transpose()?
            .flatten();
    }
    rows.sort_by(|left, right| {
        right
            .href
            .is_some()
            .cmp(&left.href.is_some())
            .then(right.updated.cmp(&left.updated))
            .then(left.title.to_lowercase().cmp(&right.title.to_lowercase()))
            .then(left.id.cmp(&right.id))
    });
    Ok(())
}

pub(super) fn timestamp(updated: Option<SystemTime>) -> u128 {
    updated
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |time| time.as_millis())
}

pub(super) fn date_label(updated: Option<SystemTime>) -> String {
    let Some(time) = updated.and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
    else {
        return "Timestamp unavailable".to_owned();
    };
    let seconds = time.as_secs();
    let mut days = seconds / 86_400;
    let mut year = 1970;
    let leap = |y: u32| y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
    while days >= if leap(year) { 366 } else { 365 } {
        days -= if leap(year) { 366 } else { 365 };
        year += 1;
        if year > 9999 {
            return "Timestamp unavailable".to_owned();
        }
    }
    let lengths = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while days >= lengths[month] {
        days -= lengths[month];
        month += 1;
    }
    format!(
        "{year:04}-{:02}-{:02} {:02}:{:02} UTC",
        month + 1,
        days + 1,
        seconds % 86_400 / 3600,
        seconds % 3600 / 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates_and_header_text_are_bounded_and_escaped_by_the_renderer() {
        assert_eq!(
            date_label(Some(SystemTime::UNIX_EPOCH)),
            "1970-01-01 00:00 UTC"
        );
        assert_eq!(
            date_label(Some(
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(951_782_400)
            )),
            "2000-02-29 00:00 UTC"
        );
        assert_eq!(
            text_between("<title>A &amp; B</title>", "<title>", "</title>"),
            Some("A & B".into())
        );
        assert!(plain_text("<script>bad</script>").is_none());
    }
}
