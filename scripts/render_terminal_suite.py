#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///

from __future__ import annotations

import html
import math
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
OUTPUTS_HTML_PATH = REPO_ROOT / "outputs" / "terminal_suite" / "half_arc_terminal_v1.html"
DOCS_ASSET_DIR = REPO_ROOT / "docs" / "assets" / "terminal_suite"

RADIUS_NOMINAL_M = 800.0
PAD_WIDTH_M = 36.0
LOW_MULTIPLIER = 1.25
HIGH_MULTIPLIER = 0.75

SVG_WIDTH = 1120
SVG_HEIGHT = 760
PADDING_LEFT = 88
PADDING_RIGHT = 44
PADDING_TOP = 48
PADDING_BOTTOM = 84
WORLD_X_MIN = -40.0
WORLD_X_MAX = 860.0
WORLD_Y_MIN = 0.0
WORLD_Y_MAX = 860.0

BAND_STYLES = {
    "low": {"color": "#c17c11", "dash": "8 6", "label": "low"},
    "mid": {"color": "#1d70b8", "dash": None, "label": "mid"},
    "high": {"color": "#a73d2a", "dash": "3 5", "label": "high"},
}


@dataclass(frozen=True)
class ArcPoint:
    name: str
    angle_deg: float
    nominal_ttg_s: float


@dataclass(frozen=True)
class FamilySpec:
    slug: str
    title: str
    gravity_mps2: float
    subtitle: str
    output_svg_name: str
    arc_points: tuple[ArcPoint, ...]
    comparison_only: bool = False
    clamp_low_to_descending: bool = True


ARC_ANGLES: tuple[tuple[str, float], ...] = (
    ("a00", 0.0),
    ("a15", 15.0),
    ("a30", 30.0),
    ("a45", 45.0),
    ("a60", 60.0),
    ("a70", 70.0),
    ("a80", 80.0),
)


def build_arc_points(ttg_values: tuple[float, ...]) -> tuple[ArcPoint, ...]:
    return tuple(
        ArcPoint(name, angle_deg, nominal_ttg_s)
        for (name, angle_deg), nominal_ttg_s in zip(ARC_ANGLES, ttg_values, strict=True)
    )


FAMILY_SPECS: tuple[FamilySpec, ...] = (
    FamilySpec(
        slug="half_arc_terminal_v1",
        title="half_arc_terminal_v1",
        gravity_mps2=9.81,
        subtitle="One-sided terminal arrival geometry · radius = 800m · gravity = 9.81m/s² (earth baseline)",
        output_svg_name="half_arc_terminal_v1.svg",
        arc_points=build_arc_points((8.50, 8.50, 8.25, 8.00, 7.75, 7.50, 7.00)),
        clamp_low_to_descending=False,
    ),
    FamilySpec(
        slug="half_arc_terminal_mars_reference",
        title="half_arc_terminal_mars_reference",
        gravity_mps2=3.71,
        subtitle="Comparison reference · tuned for steadier practical flight time · gravity = 3.71m/s² (mars)",
        output_svg_name="half_arc_terminal_mars_reference.svg",
        arc_points=build_arc_points((9.50, 9.50, 9.25, 9.00, 8.75, 8.50, 8.00)),
        comparison_only=True,
        clamp_low_to_descending=False,
    ),
    FamilySpec(
        slug="half_arc_terminal_lunar_reference",
        title="half_arc_terminal_lunar_reference",
        gravity_mps2=1.62,
        subtitle="Comparison reference · long-margin low-gravity baseline · gravity = 1.62m/s² (lunar)",
        output_svg_name="half_arc_terminal_lunar_reference.svg",
        arc_points=build_arc_points((11.50, 11.50, 11.25, 11.00, 10.50, 10.00, 9.50)),
        comparison_only=True,
        clamp_low_to_descending=True,
    ),
)


@dataclass(frozen=True)
class BandSolution:
    band: str
    ttg_s: float
    vx_mps: float
    vy_mps: float
    speed_mps: float


@dataclass(frozen=True)
class ArcProfile:
    family: FamilySpec
    arc: ArcPoint
    x_m: float
    y_m: float
    t_flat_s: float
    bands: tuple[BandSolution, ...]


def family_svg_desc(family: FamilySpec) -> str:
    if family.comparison_only:
        return (
            f"{family.title} comparison reference at 800 meter radius with low, mid, "
            "and high time-to-go bands."
        )
    return (
        f"{family.title} maintained baseline geometry at 800 meter radius with low, "
        "mid, and high time-to-go bands."
    )


def family_footer_note(family: FamilySpec) -> str:
    if family.clamp_low_to_descending:
        return (
            "Bands are solved from time-to-go. Low is clamped so shallow cells stay "
            "descending instead of starting upward."
        )
    return (
        "Bands are solved from time-to-go. Low may start upward at shallow cells "
        "when the tuned practical flight time calls for it."
    )


def world_to_svg_x(x_m: float) -> float:
    x_offset, _, scale = plot_transform()
    return x_offset + ((x_m - WORLD_X_MIN) * scale)


def world_to_svg_y(y_m: float) -> float:
    _, y_offset, scale = plot_transform()
    return y_offset + ((WORLD_Y_MAX - y_m) * scale)


def plot_transform() -> tuple[float, float, float]:
    usable_width = SVG_WIDTH - PADDING_LEFT - PADDING_RIGHT
    usable_height = SVG_HEIGHT - PADDING_TOP - PADDING_BOTTOM
    world_width = WORLD_X_MAX - WORLD_X_MIN
    world_height = WORLD_Y_MAX - WORLD_Y_MIN
    scale = min(usable_width / world_width, usable_height / world_height)
    x_offset = PADDING_LEFT + ((usable_width - (world_width * scale)) / 2.0)
    y_offset = PADDING_TOP + ((usable_height - (world_height * scale)) / 2.0)
    return x_offset, y_offset, scale


def solve_ballistic_velocity(x_m: float, y_m: float, ttg_s: float, gravity_mps2: float) -> tuple[float, float]:
    vx_mps = -x_m / ttg_s
    vy_mps = ((0.5 * gravity_mps2 * ttg_s * ttg_s) - y_m) / ttg_s
    return vx_mps, vy_mps


def sample_trajectory_points(
    x_m: float,
    y_m: float,
    vx_mps: float,
    vy_mps: float,
    ttg_s: float,
    gravity_mps2: float,
) -> list[tuple[float, float]]:
    point_count = 60
    points: list[tuple[float, float]] = []
    for index in range(point_count + 1):
        t = ttg_s * index / point_count
        x = x_m + (vx_mps * t)
        y = y_m + (vy_mps * t) - (0.5 * gravity_mps2 * t * t)
        points.append((x, y))
    return points


def derive_arc_profiles(family: FamilySpec) -> tuple[ArcProfile, ...]:
    profiles: list[ArcProfile] = []
    for arc in family.arc_points:
        angle_rad = math.radians(arc.angle_deg)
        x_m = RADIUS_NOMINAL_M * math.sin(angle_rad)
        y_m = RADIUS_NOMINAL_M * math.cos(angle_rad)
        t_flat_s = math.sqrt((2.0 * y_m) / family.gravity_mps2)
        low_ttg_s = arc.nominal_ttg_s * LOW_MULTIPLIER
        if family.clamp_low_to_descending:
            low_ttg_s = min(low_ttg_s, t_flat_s * 0.98)

        band_times = {
            "mid": arc.nominal_ttg_s,
            "low": low_ttg_s,
            "high": arc.nominal_ttg_s * HIGH_MULTIPLIER,
        }

        bands: list[BandSolution] = []
        for band_name in ("low", "mid", "high"):
            ttg_s = band_times[band_name]
            vx_mps, vy_mps = solve_ballistic_velocity(x_m, y_m, ttg_s, family.gravity_mps2)
            speed_mps = math.hypot(vx_mps, vy_mps)
            bands.append(
                BandSolution(
                    band=band_name,
                    ttg_s=ttg_s,
                    vx_mps=vx_mps,
                    vy_mps=vy_mps,
                    speed_mps=speed_mps,
                )
            )

        profiles.append(
            ArcProfile(
                family=family,
                arc=arc,
                x_m=x_m,
                y_m=y_m,
                t_flat_s=t_flat_s,
                bands=tuple(bands),
            )
        )
    return tuple(profiles)


def polyline_path(points: list[tuple[float, float]]) -> str:
    return " ".join(f"{world_to_svg_x(x):.1f},{world_to_svg_y(y):.1f}" for x, y in points)


def render_svg(family: FamilySpec, profiles: tuple[ArcProfile, ...]) -> str:
    terrain_y = world_to_svg_y(0.0)
    pad_half_width = PAD_WIDTH_M / 2.0
    pad_x0 = world_to_svg_x(-pad_half_width)
    pad_x1 = world_to_svg_x(pad_half_width)

    grid_lines: list[str] = []
    for y in range(0, 901, 100):
        if y > WORLD_Y_MAX:
            continue
        y_svg = world_to_svg_y(float(y))
        grid_lines.append(
            f'<line x1="{PADDING_LEFT}" y1="{y_svg:.1f}" x2="{SVG_WIDTH - PADDING_RIGHT}" y2="{y_svg:.1f}" '
            'stroke="#e8dfd1" stroke-width="1" />'
        )
        grid_lines.append(
            f'<text x="{PADDING_LEFT - 14}" y="{y_svg + 4:.1f}" text-anchor="end" class="axis-label">{y}m</text>'
        )
    for x in range(0, 901, 100):
        if x > WORLD_X_MAX:
            continue
        x_svg = world_to_svg_x(float(x))
        grid_lines.append(
            f'<line x1="{x_svg:.1f}" y1="{PADDING_TOP}" x2="{x_svg:.1f}" y2="{SVG_HEIGHT - PADDING_BOTTOM}" '
            'stroke="#efe7da" stroke-width="1" />'
        )
        grid_lines.append(
            f'<text x="{x_svg:.1f}" y="{SVG_HEIGHT - PADDING_BOTTOM + 22}" text-anchor="middle" class="axis-label">{x}m</text>'
        )

    trajectory_paths: list[str] = []
    labels: list[str] = []
    for profile in profiles:
        start_x = world_to_svg_x(profile.x_m)
        start_y = world_to_svg_y(profile.y_m)
        labels.append(
            f'<circle cx="{start_x:.1f}" cy="{start_y:.1f}" r="4.5" fill="#1f1a14" />'
        )
        labels.append(
            f'<text x="{start_x + 8:.1f}" y="{start_y - 10:.1f}" class="arc-label">{profile.arc.name}</text>'
        )
        for band in profile.bands:
            style = BAND_STYLES[band.band]
            points = sample_trajectory_points(
                profile.x_m,
                profile.y_m,
                band.vx_mps,
                band.vy_mps,
                band.ttg_s,
                family.gravity_mps2,
            )
            dash_attr = f' stroke-dasharray="{style["dash"]}"' if style["dash"] else ""
            trajectory_paths.append(
                f'<polyline points="{polyline_path(points)}" fill="none" stroke="{style["color"]}" stroke-width="3.2"'
                f'{dash_attr} stroke-linecap="round" stroke-linejoin="round" />'
            )

    legend_x = SVG_WIDTH - PADDING_RIGHT - 188
    legend_y = PADDING_TOP + 16
    legend_rows: list[str] = [
        f'<rect x="{legend_x}" y="{legend_y}" width="180" height="108" rx="12" fill="#fffaf1" stroke="#dcc9ab" stroke-width="1.2" />',
        f'<text x="{legend_x + 16}" y="{legend_y + 22}" class="legend-title">Band Legend</text>',
    ]
    for index, band_name in enumerate(("low", "mid", "high")):
        style = BAND_STYLES[band_name]
        row_y = legend_y + 42 + (index * 22)
        dash_attr = f' stroke-dasharray="{style["dash"]}"' if style["dash"] else ""
        legend_rows.append(
            f'<line x1="{legend_x + 16}" y1="{row_y}" x2="{legend_x + 52}" y2="{row_y}" stroke="{style["color"]}" '
            f'stroke-width="3.2"{dash_attr} stroke-linecap="round" />'
        )
        legend_rows.append(
            f'<text x="{legend_x + 62}" y="{row_y + 5}" class="legend-label">{html.escape(style["label"])}</text>'
        )

    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{SVG_WIDTH}" height="{SVG_HEIGHT}" viewBox="0 0 {SVG_WIDTH} {SVG_HEIGHT}" role="img" aria-labelledby="title desc">
  <title id="title">{family.title}</title>
  <desc id="desc">{family_svg_desc(family)}</desc>
  <style>
    .title {{ font: 700 28px ui-sans-serif, system-ui, sans-serif; fill: #1f1a14; }}
    .subtitle {{ font: 500 15px ui-sans-serif, system-ui, sans-serif; fill: #6d5844; }}
    .axis-label {{ font: 12px ui-monospace, SFMono-Regular, monospace; fill: #7b6a59; }}
    .legend-title {{ font: 700 14px ui-sans-serif, system-ui, sans-serif; fill: #2f271f; }}
    .legend-label {{ font: 13px ui-sans-serif, system-ui, sans-serif; fill: #3d3329; }}
    .arc-label {{ font: 600 12px ui-monospace, SFMono-Regular, monospace; fill: #2f271f; }}
    .note {{ font: 13px ui-sans-serif, system-ui, sans-serif; fill: #5e4d3d; }}
  </style>
  <rect width="{SVG_WIDTH}" height="{SVG_HEIGHT}" fill="#fbf8f1" />
  <text x="{PADDING_LEFT}" y="30" class="title">{family.title}</text>
  <text x="{PADDING_LEFT}" y="54" class="subtitle">{family.subtitle}</text>
  {''.join(grid_lines)}
  <line x1="{PADDING_LEFT}" y1="{terrain_y:.1f}" x2="{SVG_WIDTH - PADDING_RIGHT}" y2="{terrain_y:.1f}" stroke="#5d5448" stroke-width="2.2" />
  <rect x="{pad_x0:.1f}" y="{terrain_y - 7:.1f}" width="{pad_x1 - pad_x0:.1f}" height="7" fill="#cab488" />
  <text x="{world_to_svg_x(0.0) + 6:.1f}" y="{terrain_y - 12:.1f}" class="note">target pad</text>
  {''.join(trajectory_paths)}
  {''.join(labels)}
  {''.join(legend_rows)}
  <text x="{PADDING_LEFT}" y="{SVG_HEIGHT - 26}" class="note">{family_footer_note(family)}</text>
</svg>
"""


def render_html(families: tuple[tuple[FamilySpec, tuple[ArcProfile, ...], str], ...]) -> str:
    sections: list[str] = []
    for family, profiles, svg_markup in families:
        rows: list[str] = []
        for profile in profiles:
            for band in profile.bands:
                rows.append(
                    "<tr>"
                    f"<td>{profile.arc.name}</td>"
                    f"<td>{profile.arc.angle_deg:.0f}°</td>"
                    f"<td>{profile.x_m:.1f}</td>"
                    f"<td>{profile.y_m:.1f}</td>"
                    f"<td>{band.band}</td>"
                    f"<td>{band.ttg_s:.2f}s</td>"
                    f"<td>{band.vx_mps:.1f}</td>"
                    f"<td>{band.vy_mps:.1f}</td>"
                    f"<td>{band.speed_mps:.1f}</td>"
                    "</tr>"
                )
        comparison_note = (
            "<p>Comparison-only reference. This gravity family is not yet a maintained terminal suite definition. These reference charts lean toward steadier practical flight time, so shallow cases may start with upward vertical velocity instead of being clamped back to descending-only entries.</p>"
            if family.comparison_only
            else "<p>Maintained baseline family used by the design doc.</p>"
        )
        sections.append(
            f"""
      <section class="card">
        <h2>{family.title}</h2>
        <p>{family.subtitle}</p>
        {comparison_note}
      </section>
      <section class="card svg-wrap">
        {svg_markup}
      </section>
      <section class="card">
        <h2>Derived Cell Table</h2>
        <table>
          <thead>
            <tr>
              <th>Arc</th>
              <th>Angle</th>
              <th>x</th>
              <th>y</th>
              <th>Band</th>
              <th>TTG</th>
              <th>vx</th>
              <th>vy</th>
              <th>Speed</th>
            </tr>
          </thead>
          <tbody>
            {''.join(rows)}
          </tbody>
        </table>
      </section>
"""
        )
    return f"""<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>half_arc_terminal_v1</title>
    <style>
      :root {{
        color-scheme: light;
        --bg: #f7f2e8;
        --card: #fffaf1;
        --ink: #241d16;
        --muted: #6a5846;
        --line: #deccb1;
      }}
      * {{ box-sizing: border-box; }}
      body {{
        margin: 0;
        padding: 32px;
        font: 15px/1.5 ui-sans-serif, system-ui, sans-serif;
        color: var(--ink);
        background: radial-gradient(circle at top, #fcfaf5 0, var(--bg) 65%);
      }}
      main {{
        max-width: 1180px;
        margin: 0 auto;
        display: grid;
        gap: 20px;
      }}
      .card {{
        background: var(--card);
        border: 1px solid var(--line);
        border-radius: 16px;
        padding: 18px 20px;
        box-shadow: 0 10px 24px rgba(55, 40, 18, 0.06);
      }}
      h1, h2 {{ margin: 0 0 10px; }}
      p {{ margin: 0; color: var(--muted); }}
      .svg-wrap {{
        overflow-x: auto;
      }}
      table {{
        width: 100%;
        border-collapse: collapse;
        font-size: 14px;
      }}
      th, td {{
        padding: 8px 10px;
        border-top: 1px solid #eadbc4;
        text-align: right;
        white-space: nowrap;
      }}
      th:first-child, td:first-child,
      th:nth-child(2), td:nth-child(2),
      th:nth-child(5), td:nth-child(5) {{
        text-align: left;
      }}
      thead th {{
        border-top: 0;
        color: var(--muted);
        font-weight: 700;
      }}
      code {{
        background: #f2e7d3;
        border-radius: 6px;
        padding: 1px 6px;
      }}
    </style>
  </head>
  <body>
    <main>
      <section class="card">
        <h1>terminal suite gravity comparison</h1>
        <p>Generated by <code>scripts/render_terminal_suite.py</code>. The Earth chart is the maintained baseline family; Mars and Lunar are comparison references for design discussion.</p>
      </section>
      {''.join(sections)}
    </main>
  </body>
</html>
"""


def main() -> None:
    rendered_families: list[tuple[FamilySpec, tuple[ArcProfile, ...], str]] = []
    DOCS_ASSET_DIR.mkdir(parents=True, exist_ok=True)
    OUTPUTS_HTML_PATH.parent.mkdir(parents=True, exist_ok=True)

    for family in FAMILY_SPECS:
        profiles = derive_arc_profiles(family)
        svg_markup = render_svg(family, profiles)
        (DOCS_ASSET_DIR / family.output_svg_name).write_text(svg_markup, encoding="utf-8")
        rendered_families.append((family, profiles, svg_markup))

    html_markup = render_html(tuple(rendered_families))
    OUTPUTS_HTML_PATH.write_text(html_markup, encoding="utf-8")

    for family in FAMILY_SPECS:
        print(f"wrote {(DOCS_ASSET_DIR / family.output_svg_name).relative_to(REPO_ROOT)}")
    print(f"wrote {OUTPUTS_HTML_PATH.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
