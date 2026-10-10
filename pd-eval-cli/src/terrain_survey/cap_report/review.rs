//! Presentation-only review subsets. These classify saved stops by lifecycle,
//! not inferred physical root causes, and exclude controls/repeats from counts.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FailureGroup {
    NotLaunched,
    Clearing,
    Nominal,
    Other,
}

impl FailureGroup {
    const ALL: [Self; 4] = [
        Self::NotLaunched,
        Self::Clearing,
        Self::Nominal,
        Self::Other,
    ];

    pub(super) fn id(self) -> &'static str {
        match self {
            Self::NotLaunched => "stops-not-launched",
            Self::Clearing => "stops-clearing",
            Self::Nominal => "stops-nominal",
            Self::Other => "stops-other",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::NotLaunched => "Not launched",
            Self::Clearing => "Stopped during obstacle clearing",
            Self::Nominal => "No next target trajectory",
            Self::Other => "Other stops",
        }
    }
}

pub(super) fn failure_group(row: &CapAttempt) -> Option<FailureGroup> {
    let p = row.attempt.result.as_ref()?;
    if row.attempt.cohort != "random" || p.verified_landing {
        return None;
    }
    Some(match p.planning_stop {
        WaypointV2Stop::NoClearing if p.correction_count == 0 => FailureGroup::NotLaunched,
        WaypointV2Stop::NoClearing => FailureGroup::Clearing,
        WaypointV2Stop::NoNominal => FailureGroup::Nominal,
        _ => FailureGroup::Other,
    })
}

pub(super) fn peers(rows: &[CapAttempt], group: FailureGroup) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(i, row)| (failure_group(row) == Some(group)).then_some(i))
        .collect()
}

pub(super) fn intro(rows: &[CapAttempt]) -> String {
    let count = rows
        .iter()
        .filter(|row| failure_group(row).is_some())
        .count();
    let links = FailureGroup::ALL
        .iter()
        .filter_map(|&group| {
            let count = peers(rows, group).len();
            (count > 0).then(|| format!(
                "<a class=\"chip\" data-tree-focus=\"{id}\" href=\"#tree-{id}\">{label} · <strong>{count}</strong></a>",
                id = group.id(), label = group.label(),
            ))
        })
        .collect::<String>();
    let examples = [
        ("random-327", "327 · early departure obstruction"),
        ("random-791", "791 · early departure obstruction"),
        ("random-280", "280 · stopped after eight handoffs"),
        ("random-983", "983 · no next nominal after handoff"),
        ("random-516", "516 · no next nominal after handoff"),
        (
            "random-999",
            "999 · not launched, proposed approach blocked",
        ),
    ]
    .iter()
    .filter(|(id, _)| {
        rows.iter()
            .any(|row| row.attempt.attempt_id == *id && failure_group(row).is_some())
    })
    .map(|(id, label)| format!("<a href=\"runs/{id}/index.html\">{label}</a>"))
    .collect::<Vec<_>>()
    .join(" · ");
    format!(
        "<section class=\"panel\" id=\"failure-review\"><h2>Review remaining stops · {count} missions</h2><p>Choose a group to show its missions and terrain previews directly. Successful landings stay collapsed. Groups describe where planning stopped, not proven physical impossibility.</p><div class=\"chip-row\">{links}<a class=\"chip\" data-tree-focus=\"stops\" href=\"#tree-stops\">All stopped missions · <strong>{count}</strong></a></div><p>Suggested starting cases: {examples}</p><p>On a stopped mission, Previous / Next stopped case stays within that group. The group link returns here with the same subset open. Rich flight plots and actual handoffs are unchanged.</p></section>"
    )
}

fn recipe_leaves(
    html: &mut String,
    rows: &[CapAttempt],
    cells: &[String],
    parent: &str,
    indices: &[usize],
) {
    let mut recipes = BTreeMap::<&str, Vec<usize>>::new();
    for &i in indices {
        recipes
            .entry(
                rows[i].attempt.geometry["recipe_id"]
                    .as_str()
                    .unwrap_or("Unknown recipe"),
            )
            .or_default()
            .push(i);
    }
    for (j, (recipe, indices)) in recipes.iter().enumerate() {
        let group = format!("{parent}-recipe-{j}");
        html.push_str(&branch(&group, Some(parent), 2, recipe, indices.len()));
        for &i in indices {
            html.push_str(&batch::render_row(
                Some("seed-row mission-row current-row"),
                &format!("data-parent=\"{group}\" data-review-group=\"{parent}\" data-cohort=\"random\" data-case-id=\"{}\" data-depth=\"3\" style=\"--depth:3\" hidden", rows[i].attempt.attempt_id),
                &cells[i],
            ));
        }
    }
}

pub(super) fn tree(rows: &[CapAttempt], cells: &[String]) -> String {
    let count = rows
        .iter()
        .filter(|row| failure_group(row).is_some())
        .count();
    let mut html = branch(
        "stops",
        None,
        0,
        "Stopped primary missions · inspect first",
        count,
    );
    for group in FailureGroup::ALL {
        let indices = peers(rows, group);
        if indices.is_empty() {
            continue;
        }
        html.push_str(&branch(
            group.id(),
            Some("stops"),
            1,
            group.label(),
            indices.len(),
        ));
        recipe_leaves(&mut html, rows, cells, group.id(), &indices);
    }
    let landings = rows
        .iter()
        .filter(|row| {
            row.attempt.cohort == "random"
                && row
                    .attempt
                    .result
                    .as_ref()
                    .is_some_and(|p| p.verified_landing)
        })
        .count();
    html.push_str(&branch(
        "landings",
        None,
        0,
        "Successful primary missions · comparison",
        landings,
    ));
    for (id, corrected, label) in [
        ("landings-direct", false, "Direct landings"),
        ("landings-corrected", true, "Corrected landings"),
    ] {
        let indices = rows
            .iter()
            .enumerate()
            .filter_map(|(i, row)| {
                (row.attempt.cohort == "random"
                    && row.attempt.result.as_ref().is_some_and(|p| {
                        p.verified_landing && (p.correction_count > 0) == corrected
                    }))
                .then_some(i)
            })
            .collect::<Vec<_>>();
        if indices.is_empty() {
            continue;
        }
        html.push_str(&branch(id, Some("landings"), 1, label, indices.len()));
        recipe_leaves(&mut html, rows, cells, id, &indices);
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
                &format!("data-parent=\"{cohort}\" data-cohort=\"{cohort}\" data-case-id=\"{}\" data-depth=\"1\" style=\"--depth:1\" hidden", row.attempt.attempt_id), &cells[i]));
        }
    }
    html
}
