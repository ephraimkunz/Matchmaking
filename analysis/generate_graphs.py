#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "pandas>=2.2",
#   "matplotlib>=3.9",
#   "numpy>=1.26",
# ]
# ///
"""Repeatable chart generator for the dating-compatibility questionnaire.

Reads the Google Forms schema (questionnaire_structure.json) plus a cleaned
response CSV, and renders one PNG per chart in the CHARTS list below, styled
and sized for dropping straight into a Google Slide.

Being a uv script with inline dependency metadata, it runs on any machine
with uv installed -- no venv setup, no requirements.txt.

Usage:
    Edit the CHARTS list below to say what you want, then run:

        uv run analysis/generate_graphs.py

    Every run deletes every PNG in analysis/output/ and regenerates exactly
    what CHARTS asks for, so the folder always matches this file -- nothing
    stale lingers between edits.

    To find the exact question text / section names to put in CHARTS:

        uv run analysis/generate_graphs.py --list
        uv run analysis/generate_graphs.py --sections

    A CHARTS folder can also point at a mining-output CSV (see
    analysis/insights.csv and friends) via the ("from_insights_csv", "<file>")
    entry -- see the "dealbreakers" folder below for an example. That renders
    one chart PER ROW of the CSV (auto-picking single/pie, scatter, or a
    grouped-by-category bar depending on the row's question_1/question_2/
    group_by columns) plus a matching presenter-notes .txt (same basename,
    holding n / stat_summary / data_summary / suggested_sentence / notes --
    terse, not prose) for each one.
"""
from __future__ import annotations

import argparse
import csv
import difflib
import json
import math
import re
import shutil
import sys
import textwrap
from dataclasses import dataclass, field
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.colors as mcolors
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

# ---------------------------------------------------------------------------
# Palette -- validated categorical / sequential / diverging tokens, per the
# dataviz skill (references/palette.md). Swap these hexes to re-theme.
# ---------------------------------------------------------------------------
SURFACE = "#fcfcfb"
INK_PRIMARY = "#0b0b0b"
INK_SECONDARY = "#52514e"
INK_MUTED = "#898781"
BASELINE = "#c3c2b7"
SERIES_1 = "#2a78d6"  # categorical slot 1 (blue)
SERIES_2 = "#eb6834"  # categorical slot 2 (orange)
CATEGORICAL = [SERIES_1, SERIES_2, "#1baf7a", "#eda100", "#e87ba4", "#008300", "#4a3aa7", "#e34948"]
SEQ_LIGHT = "#9ec5f4"  # sequential step 200
SEQ_DARK = "#104281"  # sequential step 650
DIV_NEUTRAL = "#f0efec"
DIV_LOW = "#2a78d6"  # diverging pole: low end of a scale
DIV_HIGH = "#e34948"  # diverging pole: high end of a scale

plt.rcParams.update(
    {
        "figure.facecolor": SURFACE,
        "axes.facecolor": SURFACE,
        "text.color": INK_PRIMARY,
        "axes.edgecolor": BASELINE,
        "font.family": "sans-serif",
        "font.sans-serif": ["Helvetica", "Arial", "DejaVu Sans"],
    }
)

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SCHEMA = REPO_ROOT / "questionnaire_structure.json"
DEFAULT_DATA = REPO_ROOT / "real_data/round1_emailed/real_round_1_emailed_cleaned.csv"
DEFAULT_OUT = Path(__file__).resolve().parent / "output"

IMPORTANCE_TITLE = (
    "From the question above, how important is it that your partner feels "
    "the same way about this as you do?"
)

# Columns whose schema type is "text" (so they're excluded from `plottable`) but whose
# actual content is numeric -- resolved specially by resolve_numeric_series() below.
SPECIAL_NUMERIC_TITLES = {
    "Age": "Age",
    "I'd like to have ___ child (children)": "Desired number of children",
}

# ---------------------------------------------------------------------------
# CHARTS -- the single source of truth for what gets generated. Every run
# wipes analysis/output/ and regenerates exactly this, so the folder always
# matches this file. Top-level keys become subfolders under analysis/output/.
#
# Entry shapes:
#   ("gender_counts",)                   bar chart of respondents by gender
#   ("age",)                             age histogram
#   ("pie", "<question substring>")      pie chart for that question
#   ("single", "<question substring>")   bar chart for that question
#   ("gender", "<question substring>")   same question, as a bar chart split by gender
#   ("overview", "<section name>")       diverging stacked bar for a whole section (see --sections)
#   ("overview", "<q1>", "<q2>", ...)    diverging stacked bar for a custom set of scale questions
#   ("instruction_followers",)           pie chart: exactly 3 free-response answers vs. not
#   ("from_insights_csv", "<file.csv>")  one chart + presenter-notes .txt per row of a mining-output
#                                        CSV (path relative to analysis/); see render_insights_csv()
#
# Substrings are matched case-insensitively against the exact question text
# from --list; ambiguous or unmatched substrings raise an error naming the
# candidates so you can copy the exact text back in.
# ---------------------------------------------------------------------------
CHARTS: dict[str, list[tuple[str, ...]]] = {
    "demographics": [
        ("age",),
        ("pie", "Gender"),
        ("instruction_followers",),
    ],
    "dealbreakers": [
        ("from_insights_csv", "insights_dealbreakers.csv"),
    ],
    "other_insights": [
        ("from_insights_csv", "insights_other.csv"),
    ],
}


@dataclass
class Question:
    title: str
    qtype: str  # "scale" | "choice" | "text"
    section: str
    low: int | None = None
    high: int | None = None
    low_label: str | None = None
    high_label: str | None = None
    choices: list[str] = field(default_factory=list)


@dataclass
class Chartable:
    display_title: str
    column: str
    question: Question


# ---------------------------------------------------------------------------
# Schema + data loading
# ---------------------------------------------------------------------------


def load_schema(path: Path) -> list[Question]:
    raw = path.read_text()
    raw = "\n".join(line for line in raw.splitlines() if not line.strip().startswith("//"))
    data = json.loads(raw)

    questions: list[Question] = []
    section = "General"
    for item in data["items"]:
        if "pageBreakItem" in item:
            section = item.get("title", section)
            continue
        q = item.get("questionItem", {}).get("question")
        if q is None:
            continue
        title = item["title"]
        if "scaleQuestion" in q:
            sq = q["scaleQuestion"]
            questions.append(
                Question(title, "scale", section, sq["low"], sq["high"], sq.get("lowLabel"), sq.get("highLabel"))
            )
        elif "choiceQuestion" in q:
            opts = [o["value"] for o in q["choiceQuestion"]["options"]]
            questions.append(Question(title, "choice", section, choices=opts))
        elif "textQuestion" in q:
            questions.append(Question(title, "text", section))
    return questions


def dedupe_respondents(df: pd.DataFrame, id_col: str = "Username", timestamp_col: str = "Timestamp") -> pd.DataFrame:
    """Keep only the most recent submission per respondent id (some people re-submitted)."""
    naive_ts = df[timestamp_col].astype(str).str.replace(r"\s+[A-Za-z]{2,5}$", "", regex=True)
    ts = pd.to_datetime(naive_ts, format="%Y/%m/%d %I:%M:%S %p", errors="coerce")
    ordered = df.assign(_ts=ts).sort_values("_ts", kind="stable")

    is_stale = ordered[id_col].duplicated(keep="last")
    if is_stale.any():
        name_col = "First and last name" if "First and last name" in df.columns else id_col
        names = ordered.loc[is_stale, name_col].fillna(ordered.loc[is_stale, id_col]).str.strip()
        print(
            f"Dropping {int(is_stale.sum())} duplicate submission(s), keeping the most recent per "
            f"{id_col} -- superseded: {', '.join(names)}",
            file=sys.stderr,
        )

    return ordered.loc[~is_stale].drop(columns="_ts").sort_index()


def load_chartables(schema_path: Path, csv_path: Path) -> tuple[pd.DataFrame, list[Chartable]]:
    questions = load_schema(schema_path)

    with csv_path.open(newline="", encoding="utf-8-sig") as f:
        raw_header = next(csv.reader(f))

    df = pd.read_csv(csv_path)
    if len(raw_header) != len(df.columns):
        raise RuntimeError("CSV header length changed between raw read and pandas read -- investigate the file.")
    df = dedupe_respondents(df)

    data_titles = raw_header[2:]  # drop Timestamp, Username
    data_cols = list(df.columns[2:])

    aligned: list[tuple[str, Question]] = []
    ci = ji = 0
    skipped = 0
    while ci < len(data_titles) and ji < len(questions):
        if data_titles[ci] == questions[ji].title:
            aligned.append((data_cols[ci], questions[ji]))
            ci += 1
            ji += 1
            skipped = 0
        else:
            ji += 1
            skipped += 1
            if skipped > 10:
                raise RuntimeError(
                    f"Could not align schema question {questions[ji].title!r} with CSV column "
                    f"{data_titles[ci]!r} after skipping {skipped} schema entries. The schema and "
                    "CSV appear to have drifted apart -- check --schema/--data point at matching versions."
                )
    if ci < len(data_titles):
        raise RuntimeError(f"{len(data_titles) - ci} trailing CSV column(s) had no matching schema question.")

    chartables: list[Chartable] = []
    last_real_title: str | None = None
    for col, q in aligned:
        if q.title == IMPORTANCE_TITLE:
            display = f"Importance: {last_real_title}"
        else:
            display = q.title
            last_real_title = q.title
        chartables.append(Chartable(display, col, q))
    return df, chartables


# ---------------------------------------------------------------------------
# Small helpers
# ---------------------------------------------------------------------------


def slugify(title: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", title.lower()).strip("_")[:80]


def wrap_title(text: str, width: int = 46) -> str:
    return "\n".join(textwrap.wrap(text, width=width)) if text else ""


def category_counts(series: pd.Series, categories: list) -> list[int]:
    vc = series.value_counts()
    counts = [int(vc.get(c, 0)) for c in categories]
    if sum(counts) != len(series):
        unexpected = sorted(set(series.unique()) - set(categories))
        print(f"warning: {len(series) - sum(counts)} response(s) not in {categories}: {unexpected}", file=sys.stderr)
    return counts


def sequential_colors(n: int) -> list[str]:
    cmap = mcolors.LinearSegmentedColormap.from_list("seq", [SEQ_LIGHT, SEQ_DARK])
    if n == 1:
        return [SEQ_DARK]
    return [mcolors.to_hex(cmap(i / (n - 1))) for i in range(n)]


def blend(c1: str, c2: str, t: float) -> str:
    rgb1, rgb2 = mcolors.to_rgb(c1), mcolors.to_rgb(c2)
    return mcolors.to_hex(tuple(a + (b - a) * t for a, b in zip(rgb1, rgb2)))


def diverging_colors_for_levels(levels: list[int]) -> dict[int, str]:
    mid = (levels[0] + levels[-1]) / 2
    lows = sorted((lvl for lvl in levels if lvl < mid), reverse=True)
    highs = sorted(lvl for lvl in levels if lvl > mid)
    colors: dict[int, str] = {}
    for i, lvl in enumerate(lows):
        colors[lvl] = blend(DIV_NEUTRAL, DIV_LOW, (i + 1) / len(lows))
    for i, lvl in enumerate(highs):
        colors[lvl] = blend(DIV_NEUTRAL, DIV_HIGH, (i + 1) / len(highs))
    if mid in levels:
        colors[int(mid)] = DIV_NEUTRAL
    return colors


def style_axes(ax) -> None:
    for spine in ("top", "right", "left"):
        ax.spines[spine].set_visible(False)
    ax.spines["bottom"].set_color(BASELINE)
    ax.spines["bottom"].set_linewidth(1)
    ax.tick_params(axis="y", left=False, labelleft=False)
    ax.tick_params(axis="x", colors=INK_MUTED, length=0)
    ax.set_yticks([])


def save_fig(fig, out_path: Path) -> Path:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=200, bbox_inches="tight", facecolor=SURFACE)
    plt.close(fig)
    print(f"wrote {out_path}")
    return out_path


def find_chartable(query: str, chartables: list[Chartable]) -> Chartable:
    q = query.lower().strip()
    exact = [c for c in chartables if c.display_title.lower() == q]
    if exact:
        return exact[0]
    matches = [c for c in chartables if q in c.display_title.lower()]
    if len(matches) == 1:
        return matches[0]
    if not matches:
        titles = [c.display_title for c in chartables]
        suggestions = difflib.get_close_matches(query, titles, n=5, cutoff=0.3)
        hint = "\n  ".join(suggestions) or "(no close matches -- try --list)"
        raise SystemExit(f"No question matches {query!r}. Did you mean:\n  {hint}")
    lines = "\n  ".join(c.display_title for c in matches)
    raise SystemExit(f"{query!r} matches {len(matches)} questions -- be more specific:\n  {lines}")


def resolve_section_questions(name: str, plottable: list[Chartable]) -> list[Chartable]:
    sections = list(dict.fromkeys(c.question.section for c in plottable))
    hits = [s for s in sections if name.lower() in s.lower()]
    if len(hits) != 1:
        return []
    return [c for c in plottable if c.question.qtype == "scale" and c.question.section == hits[0]]


def resolve_overview(terms: list[str], plottable: list[Chartable]) -> tuple[list[Chartable], str]:
    if len(terms) == 1:
        section_hits = resolve_section_questions(terms[0], plottable)
        if section_hits:
            return section_hits, terms[0]
    chs = []
    for term in terms:
        ch = find_chartable(term, plottable)
        if ch.question.qtype != "scale":
            raise SystemExit(f"{ch.display_title!r} is not a numeric-scale question; --overview needs scale questions.")
        chs.append(ch)
    return chs, " / ".join(terms)


# The "Importance: <statement>" companion questions are schema type="choice" (5 ordinal
# strings), not "scale" -- but every insights-CSV row that references one wants it treated
# as a numeric 1-5 ordinal, never as a grouping category. This is the encoding the
# correlation-mining pass used.
IMPORTANCE_ORDER = ["I don't care if we agree", "A little", "Somewhat", "Very", "We MUST agree on this"]


def resolve_numeric_series(title: str, df: pd.DataFrame, chartables: list[Chartable]) -> tuple[str, pd.Series]:
    """Numeric series for a title that's either a SPECIAL_NUMERIC_TITLES entry (schema
    type=text but numeric in practice, e.g. Age or desired-#-of-kids), an "Importance: ..."
    companion question (schema type=choice, ordinal-encoded 1-5 here), or a schema scale
    question (already numeric in the dataframe)."""
    if title in SPECIAL_NUMERIC_TITLES:
        return SPECIAL_NUMERIC_TITLES[title], pd.to_numeric(df[title], errors="coerce")
    if title.startswith("Importance: "):
        ch = find_chartable(title, [c for c in chartables if c.display_title.startswith("Importance: ")])
        ordinal = {label: i + 1 for i, label in enumerate(IMPORTANCE_ORDER)}
        return ch.display_title, df[ch.column].map(ordinal)
    ch = find_chartable(title, [c for c in chartables if c.question.qtype == "scale"])
    return ch.display_title, df[ch.column]


def resolve_category_series(title: str, df: pd.DataFrame, chartables: list[Chartable]) -> tuple[str, pd.Series, list[str]]:
    """Categorical series + schema-declared category order for a schema choice question
    (this also covers Gender, which is a choice question like any other)."""
    ch = find_chartable(title, [c for c in chartables if c.question.qtype == "choice"])
    return ch.display_title, df[ch.column], list(ch.question.choices)


def is_categorical_title(title: str, chartables: list[Chartable]) -> bool:
    # Importance companions and special-numeric columns are always numeric in this context
    # (see resolve_numeric_series) -- never treated as a grouping category here.
    if title.startswith("Importance: ") or title in SPECIAL_NUMERIC_TITLES:
        return False
    return any(c.display_title.lower() == title.strip().lower() and c.question.qtype == "choice" for c in chartables)


# ---------------------------------------------------------------------------
# Chart builders
# ---------------------------------------------------------------------------


def plot_single(ch: Chartable, df: pd.DataFrame, out_dir: Path) -> Path:
    q = ch.question
    series = df[ch.column].dropna()
    n = len(series)

    if q.qtype == "scale":
        categories = list(range(q.low, q.high + 1))
        counts = category_counts(series, categories)
        colors = sequential_colors(len(categories))
        xticklabels = [str(c) for c in categories]
        if q.low_label:
            xticklabels[0] = f"{categories[0]}\n{wrap_title(q.low_label, 16)}"
        if q.high_label:
            xticklabels[-1] = f"{categories[-1]}\n{wrap_title(q.high_label, 16)}"
    else:
        categories = q.choices
        counts = category_counts(series, categories)
        colors = [SERIES_1] * len(categories)
        xticklabels = [wrap_title(c, 14) for c in categories]

    fig, ax = plt.subplots(figsize=(8, 5))
    x = list(range(len(categories)))
    bars = ax.bar(x, counts, width=0.62, color=colors, zorder=3)
    for rect, c in zip(bars, counts):
        pct = c / n * 100 if n else 0
        ax.annotate(
            f"{c}\n({pct:.0f}%)",
            (rect.get_x() + rect.get_width() / 2, rect.get_height()),
            xytext=(0, 4),
            textcoords="offset points",
            ha="center",
            va="bottom",
            fontsize=10,
            color=INK_SECONDARY,
        )

    ax.set_xticks(x)
    ax.set_xticklabels(xticklabels, fontsize=10.5, color=INK_MUTED)
    ax.set_ylim(0, max(counts) * 1.24 if counts else 1)
    style_axes(ax)
    fig.suptitle(wrap_title(ch.display_title), fontsize=19, fontweight="bold", color=INK_PRIMARY, y=0.97)
    fig.tight_layout(rect=(0, 0, 1, 0.935))
    return save_fig(fig, out_dir / f"{slugify(ch.display_title)}.png")


def plot_by_gender(ch: Chartable, df: pd.DataFrame, out_dir: Path, gender_col: str = "Gender") -> None:
    q = ch.question
    present = list(df[gender_col].dropna().unique())
    genders = [g for g in ["Male", "Female"] if g in present] + [g for g in present if g not in ("Male", "Female")]
    if len(genders) < 2:
        raise SystemExit(f"Need at least 2 gender groups in the data to compare; found {genders}")

    if q.qtype == "scale":
        categories = list(range(q.low, q.high + 1))
        xticklabels = [str(c) for c in categories]
        if q.low_label:
            xticklabels[0] = f"{categories[0]}\n{wrap_title(q.low_label, 16)}"
        if q.high_label:
            xticklabels[-1] = f"{categories[-1]}\n{wrap_title(q.high_label, 16)}"
    else:
        categories = q.choices
        xticklabels = [wrap_title(c, 14) for c in categories]

    series_colors = [SERIES_1, SERIES_2, INK_MUTED, SEQ_DARK]
    n_g = len(genders)
    width = 0.8 / n_g
    x = list(range(len(categories)))

    fig, ax = plt.subplots(figsize=(8.5, 5.2))
    per_gender_counts = {}
    for i, g in enumerate(genders):
        sub = df.loc[df[gender_col] == g, ch.column].dropna()
        counts = category_counts(sub, categories)
        per_gender_counts[g] = counts
        offset = (i - (n_g - 1) / 2) * width
        bars = ax.bar(
            [xi + offset for xi in x], counts, width=width * 0.9, color=series_colors[i % len(series_colors)], label=g, zorder=3
        )
        for rect, c in zip(bars, counts):
            if c:
                ax.annotate(
                    str(c),
                    (rect.get_x() + rect.get_width() / 2, rect.get_height()),
                    xytext=(0, 3),
                    textcoords="offset points",
                    ha="center",
                    va="bottom",
                    fontsize=9,
                    color=INK_SECONDARY,
                )

    max_count = max((max(c) for c in per_gender_counts.values()), default=1)
    ax.set_ylim(0, max_count * 1.22)
    ax.set_xticks(x)
    ax.set_xticklabels(xticklabels, fontsize=10.5, color=INK_MUTED)
    style_axes(ax)

    handles, labels = ax.get_legend_handles_labels()
    fig.legend(
        handles,
        labels,
        frameon=False,
        loc="upper center",
        bbox_to_anchor=(0.5, 0.9),
        ncol=n_g,
        fontsize=10.5,
        labelcolor=INK_SECONDARY,
    )
    fig.suptitle(wrap_title(ch.display_title), fontsize=18, fontweight="bold", color=INK_PRIMARY, y=0.98)
    fig.tight_layout(rect=(0, 0, 1, 0.84))
    save_fig(fig, out_dir / f"{slugify(ch.display_title)}_by_gender.png")


def plot_age(df: pd.DataFrame, out_dir: Path, age_col: str = "Age") -> None:
    ages = pd.to_numeric(df[age_col], errors="coerce").dropna().astype(int)
    by_age = ages.value_counts().sort_index()
    categories = list(by_age.index)
    counts = list(by_age.values)

    fig, ax = plt.subplots(figsize=(9, 5))
    x = list(range(len(categories)))
    bars = ax.bar(x, counts, width=0.65, color=SERIES_1, zorder=3)
    for rect, c in zip(bars, counts):
        ax.annotate(
            str(c),
            (rect.get_x() + rect.get_width() / 2, rect.get_height()),
            xytext=(0, 4),
            textcoords="offset points",
            ha="center",
            va="bottom",
            fontsize=10,
            color=INK_SECONDARY,
        )

    ax.set_xticks(x)
    ax.set_xticklabels([str(a) for a in categories], fontsize=10, color=INK_MUTED)
    ax.set_ylim(0, max(counts) * 1.24 if counts else 1)
    style_axes(ax)
    fig.suptitle("Age distribution", fontsize=19, fontweight="bold", color=INK_PRIMARY, y=0.97)
    fig.tight_layout(rect=(0, 0, 1, 0.935))
    save_fig(fig, out_dir / "age_distribution.png")


def plot_gender_counts(df: pd.DataFrame, out_dir: Path, gender_col: str = "Gender") -> None:
    counts = df[gender_col].value_counts()
    categories = [g for g in ["Male", "Female"] if g in counts.index] + [g for g in counts.index if g not in ("Male", "Female")]
    values = [int(counts[g]) for g in categories]
    colors = ([SERIES_1, SERIES_2] + [INK_MUTED] * len(categories))[: len(categories)]
    n = sum(values)

    fig, ax = plt.subplots(figsize=(6, 5))
    bars = ax.bar(categories, values, width=0.5, color=colors, zorder=3)
    for rect, v in zip(bars, values):
        ax.annotate(
            f"{v} ({v / n * 100:.0f}%)",
            (rect.get_x() + rect.get_width() / 2, rect.get_height()),
            xytext=(0, 4),
            textcoords="offset points",
            ha="center",
            va="bottom",
            fontsize=11,
            color=INK_SECONDARY,
        )

    ax.set_ylim(0, max(values) * 1.25)
    style_axes(ax)
    ax.tick_params(axis="x", labelsize=12, colors=INK_MUTED)
    fig.suptitle("Respondents by gender", fontsize=19, fontweight="bold", color=INK_PRIMARY, y=0.97)
    fig.tight_layout(rect=(0, 0, 1, 0.935))
    save_fig(fig, out_dir / "gender_counts.png")


def render_pie(
    title: str, labels: list[str], counts: list[int], out_path: Path, small_wedge_degrees: float = 22.0
) -> Path:
    n = sum(counts)
    colors = [CATEGORICAL[i % len(CATEGORICAL)] for i in range(len(labels))]

    fig, ax = plt.subplots(figsize=(7.5, 6.5))
    wedges, _ = ax.pie(
        counts,
        colors=colors,
        startangle=90,
        counterclock=False,
        wedgeprops={"linewidth": 2, "edgecolor": SURFACE},
    )

    for wedge, label, c in zip(wedges, labels, counts):
        pct = c / n * 100 if n else 0
        angle = (wedge.theta1 + wedge.theta2) / 2
        span = wedge.theta2 - wedge.theta1
        rad = math.radians(angle)
        if span >= small_wedge_degrees:
            text = f"{wrap_title(label, 12)}\n{c} ({pct:.0f}%)"
            x, y = 0.68 * math.cos(rad), 0.68 * math.sin(rad)
            ax.text(x, y, text, ha="center", va="center", fontsize=11, color="white", fontweight="bold", linespacing=1.3)
        else:
            text = f"{wrap_title(label, 16)}\n{c} ({pct:.0f}%)"
            x_edge, y_edge = math.cos(rad), math.sin(rad)
            x_label, y_label = 1.25 * math.cos(rad), 1.25 * math.sin(rad)
            ax.annotate(
                text,
                xy=(x_edge, y_edge),
                xytext=(x_label, y_label),
                ha="left" if x_label >= 0 else "right",
                va="center",
                fontsize=10,
                color=INK_SECONDARY,
                arrowprops={"arrowstyle": "-", "color": BASELINE, "linewidth": 1},
            )

    ax.set_aspect("equal")
    wrapped_title = wrap_title(title)
    title_lines = wrapped_title.count("\n") + 1
    suptitle_y = 0.97
    top = suptitle_y - 0.042 * title_lines
    fig.suptitle(wrapped_title, fontsize=19, fontweight="bold", color=INK_PRIMARY, y=suptitle_y)
    fig.tight_layout(rect=(0.06, 0.02, 0.94, top))
    return save_fig(fig, out_path)


def plot_pie(ch: Chartable, df: pd.DataFrame, out_dir: Path) -> Path:
    q = ch.question
    series = df[ch.column].dropna()

    if q.qtype == "scale":
        categories = list(range(q.low, q.high + 1))
        labels = [str(c) for c in categories]
    else:
        categories = list(q.choices)
        labels = list(categories)
    counts = category_counts(series, categories)

    kept = [(lab, c) for lab, c in zip(labels, counts) if c > 0]
    labels, counts = zip(*kept)
    return render_pie(ch.display_title, list(labels), list(counts), out_dir / f"{slugify(ch.display_title)}.png")


def plot_instruction_followers(
    df: pd.DataFrame, chartables: list[Chartable], out_dir: Path, required: int = 3
) -> None:
    """Free-response section asks respondents to pick exactly `required` of the prompts."""
    free_response_cols = [c.column for c in chartables if c.question.qtype == "text" and c.question.section == "Free-response"]

    def is_filled(v: object) -> bool:
        return isinstance(v, str) and v.strip() != ""

    filled_count = df[free_response_cols].map(is_filled).sum(axis=1)
    is_follower = filled_count == required

    labels = ["Instruction followers", "Did not follow instructions"]
    counts = [int(is_follower.sum()), int((~is_follower).sum())]
    render_pie("Instruction followers", labels, counts, out_dir / "instruction_followers.png")


def plot_overview(chs: list[Chartable], title: str, df: pd.DataFrame, out_dir: Path) -> None:
    n_rows = len(chs)
    rows = []
    max_extent = 0.01
    for ch in chs:
        q = ch.question
        series = df[ch.column].dropna()
        n = len(series)
        levels = list(range(q.low, q.high + 1))
        counts = category_counts(series, levels)
        shares = [c / n if n else 0 for c in counts]
        colors = diverging_colors_for_levels(levels)
        mid = (levels[0] + levels[-1]) / 2
        left = sorted(((lvl, sh) for lvl, sh in zip(levels, shares) if lvl < mid), key=lambda t: -t[0])
        right = sorted(((lvl, sh) for lvl, sh in zip(levels, shares) if lvl > mid), key=lambda t: t[0])
        center = sum(sh for lvl, sh in zip(levels, shares) if lvl == mid)
        rows.append((ch, left, center, right, colors))
        extent = center / 2 + max(sum(sh for _, sh in left), sum(sh for _, sh in right))
        max_extent = max(max_extent, extent)

    xlim = max_extent * 1.08
    pad = xlim * 0.03
    row_step = 1.0
    bar_height = 0.42
    title_dy = 0.34  # how far above each bar its question title sits
    fig_h = max(2.8, row_step * n_rows + 1.8)
    fig, ax = plt.subplots(figsize=(11, fig_h))

    for i, (ch, left, center, right, colors) in enumerate(rows):
        y = i * row_step
        cl = -center / 2
        for lvl, sh in left:
            cl -= sh
            ax.barh(y, sh, left=cl, height=bar_height, color=colors[lvl], zorder=3)
        cr = center / 2
        for lvl, sh in right:
            ax.barh(y, sh, left=cr, height=bar_height, color=colors[lvl], zorder=3)
            cr += sh
        if center:
            ax.barh(y, center, left=-center / 2, height=bar_height, color=DIV_NEUTRAL, zorder=3)
        ax.text(
            -xlim - pad, y, wrap_title(ch.question.low_label or "", 20), ha="right", va="center", fontsize=8, color=INK_MUTED
        )
        ax.text(
            xlim + pad, y, wrap_title(ch.question.high_label or "", 20), ha="left", va="center", fontsize=8, color=INK_MUTED
        )
        ax.text(
            -xlim,
            y - title_dy,
            wrap_title(ch.question.title, 78),
            ha="left",
            va="bottom",
            fontsize=10.5,
            color=INK_PRIMARY,
        )

    ax.axvline(0, color=BASELINE, linewidth=1, zorder=2)
    ax.set_xlim(-xlim, xlim)
    ax.set_ylim(-title_dy - 0.3, (n_rows - 1) * row_step + bar_height / 2 + 0.15)
    ax.set_yticks([])
    ax.invert_yaxis()
    ax.set_xticks([])
    for spine in ax.spines.values():
        spine.set_visible(False)

    fig.suptitle(wrap_title(title, 60), fontsize=20, fontweight="bold", color=INK_PRIMARY, y=0.995)
    fig.text(
        0.5,
        0.955,
        "Share of responses toward each item's low end (blue) vs. high end (red); darker = stronger.",
        ha="center",
        fontsize=9,
        color=INK_SECONDARY,
    )
    fig.tight_layout(rect=(0.02, 0.02, 0.98, 0.935))
    save_fig(fig, out_dir / f"overview_{slugify(title)}.png")


def plot_numeric_distribution(title: str, df: pd.DataFrame, out_dir: Path) -> Path:
    """Integer-value histogram for a SPECIAL_NUMERIC_TITLES column (mirrors plot_age's style)."""
    display = SPECIAL_NUMERIC_TITLES[title]
    vals = pd.to_numeric(df[title], errors="coerce").dropna().astype(int)
    by_val = vals.value_counts().sort_index()
    categories = list(by_val.index)
    counts = list(by_val.values)
    n = len(vals)

    fig, ax = plt.subplots(figsize=(9, 5))
    x = list(range(len(categories)))
    bars = ax.bar(x, counts, width=0.65, color=SERIES_1, zorder=3)
    for rect, c in zip(bars, counts):
        pct = c / n * 100 if n else 0
        ax.annotate(
            f"{c}\n({pct:.0f}%)",
            (rect.get_x() + rect.get_width() / 2, rect.get_height()),
            xytext=(0, 4),
            textcoords="offset points",
            ha="center",
            va="bottom",
            fontsize=10,
            color=INK_SECONDARY,
        )
    ax.set_xticks(x)
    ax.set_xticklabels([str(c) for c in categories], fontsize=10, color=INK_MUTED)
    ax.set_ylim(0, max(counts) * 1.24 if counts else 1)
    style_axes(ax)
    fig.suptitle(display, fontsize=19, fontweight="bold", color=INK_PRIMARY, y=0.97)
    fig.tight_layout(rect=(0, 0, 1, 0.935))
    return save_fig(fig, out_dir / f"{slugify(display)}.png")


def plot_scatter(x_title: str, y_title: str, df: pd.DataFrame, chartables: list[Chartable], out_dir: Path) -> Path:
    # "Opinion strength vs. demand for partner agreement" rows pair a statement with its own
    # importance-companion; the mined stat there is about opinion STRENGTH (how far from neutral,
    # regardless of direction), which a plain scatter of the raw 1-4 value can't show faithfully --
    # it can even come out the opposite sign (e.g. "I go to great lengths to minimize harm to the
    # planet" is raw_r=+0.50 but the real, symmetric-around-the-midpoint relationship is -0.24).
    # A folded/derived x-axis is hard to explain on a slide, so render these as a plain two-bucket
    # bar instead -- stays on the real 1-4 answer scale, no transform to explain.
    if y_title.strip() == f"Importance: {x_title.strip()}":
        return plot_extreme_vs_middle_bar(x_title, y_title, df, chartables, out_dir)

    xt, xs = resolve_numeric_series(x_title, df, chartables)
    yt, ys = resolve_numeric_series(y_title, df, chartables)
    x_axis_label = xt
    sub = pd.DataFrame({"x": xs, "y": ys}).dropna()

    rng = np.random.default_rng(0)  # fixed seed: jitter only breaks up overplotting, doesn't need to vary
    jx = sub["x"] + rng.uniform(-0.08, 0.08, len(sub))
    jy = sub["y"] + rng.uniform(-0.08, 0.08, len(sub))

    fig, ax = plt.subplots(figsize=(7.5, 6))
    ax.scatter(jx, jy, s=30, color=SERIES_1, alpha=0.55, edgecolors="none", zorder=3)
    if len(sub) >= 2 and sub["x"].std() > 0:
        coeffs = np.polyfit(sub["x"], sub["y"], 1)
        line_x = np.linspace(sub["x"].min(), sub["x"].max(), 50)
        ax.plot(line_x, np.polyval(coeffs, line_x), color=SERIES_2, linewidth=2.2, zorder=4)
    r = sub["x"].corr(sub["y"])
    label = f"r = {r:+.2f}" if len(sub) == len(df) else f"r = {r:+.2f}   n = {len(sub)}"
    ax.text(
        0.03,
        0.96,
        label,
        transform=ax.transAxes,
        fontsize=11.5,
        color=INK_SECONDARY,
        va="top",
        fontweight="bold",
    )
    ax.set_xlabel(wrap_title(x_axis_label, 44), fontsize=10.5, color=INK_MUTED)
    ax.set_ylabel(wrap_title(yt, 44), fontsize=10.5, color=INK_MUTED)
    for spine in ("top", "right"):
        ax.spines[spine].set_visible(False)
    ax.spines["left"].set_color(BASELINE)
    ax.spines["bottom"].set_color(BASELINE)
    ax.tick_params(colors=INK_MUTED, length=0)

    title = f"{xt} vs. {yt}"
    fig.suptitle(wrap_title(title, 64), fontsize=16, fontweight="bold", color=INK_PRIMARY, y=0.985)
    fig.tight_layout(rect=(0, 0, 1, 0.91))
    return save_fig(fig, out_dir / f"{slugify(title)}.png")


def render_grouped_bar(
    present: list[str],
    means: list[float],
    safe_stds: list[float],
    counts: list[int],
    full_n: int,
    title: str,
    value_label: str,
    out_dir: Path,
) -> Path:
    """Bar chart of mean(value) per category, with std-dev error bars and n annotated
    (n omitted when a bar's count equals the full sample -- see the n-label rule elsewhere
    in this file). Shared by plot_grouped_by_category and plot_extreme_vs_middle_bar.

    value_label is a y-axis label naming what the bar heights actually are (e.g. "Importance:
    ..." or "Desired number of children") -- without it, a bar chart is just numbers with no
    indication of what's being averaged or what scale it's on."""
    fig, ax = plt.subplots(figsize=(8, 5.2))
    x = list(range(len(present)))
    colors = [CATEGORICAL[i % len(CATEGORICAL)] for i in range(len(present))]
    ax.bar(
        x,
        means,
        width=0.55,
        color=colors,
        zorder=3,
        yerr=safe_stds,
        capsize=4,
        error_kw={"ecolor": INK_MUTED, "linewidth": 1},
    )
    for xi, m, s, n in zip(x, means, safe_stds, counts):
        label = f"{m:.2f}" if n == full_n else f"{m:.2f}\n(n={n})"
        ax.annotate(
            label,
            (xi, m + s),
            xytext=(0, 6),
            textcoords="offset points",
            ha="center",
            va="bottom",
            fontsize=10,
            color=INK_SECONDARY,
        )
    ax.set_xticks(x)
    ax.set_xticklabels([wrap_title(str(c), 16) for c in present], fontsize=10.5, color=INK_MUTED)
    top = max((m + s for m, s in zip(means, safe_stds)), default=1)
    ax.set_ylim(0, top * 1.4 if top else 1)
    style_axes(ax)
    ax.set_ylabel(wrap_title(value_label, 30), fontsize=10.5, color=INK_MUTED)
    ax.yaxis.set_label_coords(-0.09, 0.5)

    fig.suptitle(wrap_title(title, 60), fontsize=17, fontweight="bold", color=INK_PRIMARY, y=0.97)
    fig.tight_layout(rect=(0, 0, 1, 0.9))
    return save_fig(fig, out_dir / f"{slugify(title)}.png")


def plot_grouped_by_category(
    value_title: str, category_title: str, df: pd.DataFrame, chartables: list[Chartable], out_dir: Path
) -> Path:
    """The generic version of what plot_by_gender does specifically for Gender."""
    vt, vs = resolve_numeric_series(value_title, df, chartables)
    ct, cs, order = resolve_category_series(category_title, df, chartables)

    sub = pd.DataFrame({"v": vs, "c": cs}).dropna()
    present_set = set(sub["c"])
    present = [c for c in order if c in present_set] or list(dict.fromkeys(sub["c"]))
    means = [sub.loc[sub["c"] == c, "v"].mean() for c in present]
    stds = [sub.loc[sub["c"] == c, "v"].std() for c in present]
    counts = [int((sub["c"] == c).sum()) for c in present]
    safe_stds = [s if s == s else 0 for s in stds]  # NaN (n=1 groups) -> no error bar

    return render_grouped_bar(present, means, safe_stds, counts, len(df), f"{vt} by {ct}", vt, out_dir)


def plot_extreme_vs_middle_bar(
    x_title: str, y_title: str, df: pd.DataFrame, chartables: list[Chartable], out_dir: Path
) -> Path:
    """For a statement/own-importance-companion pair: bucket respondents by whether they
    answered at either extreme end of the scale or in the middle, then show mean importance
    per bucket. Stays on the real answer scale (no folded/derived variable to explain) while
    still showing the same relationship as the opinion-strength stat in the presenter notes."""
    scale_ch = find_chartable(x_title, [c for c in chartables if c.question.qtype == "scale"])
    low, high = scale_ch.question.low, scale_ch.question.high
    xt, xs = resolve_numeric_series(x_title, df, chartables)
    yt, ys = resolve_numeric_series(y_title, df, chartables)

    middle_values = " or ".join(str(v) for v in range(low + 1, high))
    extreme_label = f"Answered {low} or {high} (extreme)"
    middle_label = f"Answered {middle_values} (middle)"
    cs = xs.map(lambda v: extreme_label if v in (low, high) else middle_label)

    sub = pd.DataFrame({"v": ys, "c": cs}).dropna()
    present_set = set(sub["c"])
    present = [c for c in (extreme_label, middle_label) if c in present_set]
    means = [sub.loc[sub["c"] == c, "v"].mean() for c in present]
    stds = [sub.loc[sub["c"] == c, "v"].std() for c in present]
    counts = [int((sub["c"] == c).sum()) for c in present]
    safe_stds = [s if s == s else 0 for s in stds]

    title = f"{xt} vs. {yt}"
    return render_grouped_bar(present, means, safe_stds, counts, len(df), title, yt, out_dir)


FREE_RESPONSE_BUCKET_RE = re.compile(r"bucket=(.+),\s*count=(\d+)\s*\((\d+)%\)")
FREE_RESPONSE_GROUP_RE = re.compile(r"([^:;]+):(\d+)")
RESPONSE_RATE_RE = re.compile(r"([A-Za-z0-9][^,:]*?)\s+(\d+)%")


def plot_response_rate_overview(row: dict, out_dir: Path) -> Path:
    """The free-response completion-rate summary row uses a synthetic placeholder
    question_1 (no real schema column behind it) -- parse its data_summary text directly,
    e.g. 'Response rate by question: Ideal hangout 73%, ... Changed my mind recently 24%'.
    """
    text = row["data_summary"]
    if ":" in text:
        text = text.split(":", 1)[1]
    pairs = [(m.group(1).strip(), int(m.group(2))) for m in RESPONSE_RATE_RE.finditer(text)]
    pairs.sort(key=lambda p: -p[1])
    labels = [wrap_title(p[0], 14) for p in pairs]
    values = [p[1] for p in pairs]

    fig, ax = plt.subplots(figsize=(9.5, 5.5))
    x = list(range(len(pairs)))
    bars = ax.bar(x, values, width=0.6, color=SERIES_1, zorder=3)
    for rect, v in zip(bars, values):
        ax.annotate(
            f"{v}%",
            (rect.get_x() + rect.get_width() / 2, rect.get_height()),
            xytext=(0, 4),
            textcoords="offset points",
            ha="center",
            va="bottom",
            fontsize=10,
            color=INK_SECONDARY,
        )
    ax.set_xticks(x)
    ax.set_xticklabels(labels, fontsize=9.5, color=INK_MUTED)
    ax.set_ylim(0, max(values) * 1.24 if values else 1)
    style_axes(ax)
    fig.suptitle("Free-response completion rate by question", fontsize=16, fontweight="bold", color=INK_PRIMARY, y=0.97)
    fig.tight_layout(rect=(0, 0, 1, 0.92))
    return save_fig(fig, out_dir / "free_response_completion_rates.png")


def free_response_titles(chartables: list[Chartable]) -> set[str]:
    return {c.display_title for c in chartables if c.question.qtype == "text" and c.question.section == "Free-response"}


def plot_free_response_buckets(question_title: str, rows: list[dict], out_dir: Path) -> Path:
    """One grouped bar chart for a free-response question, bars = its coded buckets (from
    insights CSV rows shaped like stat_summary='bucket=<name>, count=<n> (<pct>%)'). If a
    row's group_by looks like 'Male:7;Female:3' (bucket-level sub-group counts, e.g. the
    gender split on "most attractive trait"), bars split by that sub-group instead of a
    single bucket-count bar.
    """
    buckets = []
    subgroup_names: list[str] = []
    for r in rows:
        m = FREE_RESPONSE_BUCKET_RE.match(r["stat_summary"].strip())
        name, count, pct = (m.group(1), int(m.group(2)), int(m.group(3))) if m else (r["stat_summary"] or "?", 0, 0)
        groups = {k: int(v) for k, v in FREE_RESPONSE_GROUP_RE.findall(r["group_by"])} if r["group_by"].strip() else {}
        for g in groups:
            if g not in subgroup_names:
                subgroup_names.append(g)
        buckets.append(
            {
                "name": name,
                "count": count,
                "pct": pct,
                "groups": groups,
                "quotes": r.get("data_summary", "").strip(),
                "sentence": r.get("suggested_sentence", "").strip(),
                "notes": r.get("notes", "").strip(),
            }
        )
    buckets.sort(key=lambda b: -b["count"])

    fig, ax = plt.subplots(figsize=(9.5, 5.8))
    x = list(range(len(buckets)))
    labels = [wrap_title(b["name"], 16) for b in buckets]
    has_subgroups = bool(subgroup_names)

    if has_subgroups:
        n_g = len(subgroup_names)
        width = 0.8 / n_g
        colors = [SERIES_1, SERIES_2, INK_MUTED, SEQ_DARK]
        for i, g in enumerate(subgroup_names):
            vals = [b["groups"].get(g, 0) for b in buckets]
            offset = (i - (n_g - 1) / 2) * width
            bars = ax.bar(
                [xi + offset for xi in x], vals, width=width * 0.9, color=colors[i % len(colors)], label=g, zorder=3
            )
            for rect, v in zip(bars, vals):
                if v:
                    ax.annotate(
                        str(v),
                        (rect.get_x() + rect.get_width() / 2, rect.get_height()),
                        xytext=(0, 3),
                        textcoords="offset points",
                        ha="center",
                        va="bottom",
                        fontsize=9,
                        color=INK_SECONDARY,
                    )
        handles, hlabels = ax.get_legend_handles_labels()
        fig.legend(
            handles,
            hlabels,
            frameon=False,
            loc="upper center",
            bbox_to_anchor=(0.5, 0.9),
            ncol=n_g,
            fontsize=10.5,
            labelcolor=INK_SECONDARY,
        )
        max_v = max((max(b["groups"].values(), default=0) for b in buckets), default=1)
        ax.set_ylim(0, max_v * 1.35 if max_v else 1)
    else:
        counts = [b["count"] for b in buckets]
        bars = ax.bar(x, counts, width=0.6, color=SERIES_1, zorder=3)
        for rect, b in zip(bars, buckets):
            ax.annotate(
                f"{b['count']} ({b['pct']}%)",
                (rect.get_x() + rect.get_width() / 2, rect.get_height()),
                xytext=(0, 4),
                textcoords="offset points",
                ha="center",
                va="bottom",
                fontsize=10,
                color=INK_SECONDARY,
            )
        ax.set_ylim(0, max(counts) * 1.24 if counts else 1)

    ax.set_xticks(x)
    ax.set_xticklabels(labels, fontsize=9.5, color=INK_MUTED)
    style_axes(ax)
    fig.suptitle(wrap_title(question_title, 56), fontsize=17, fontweight="bold", color=INK_PRIMARY, y=0.975)
    fig.tight_layout(rect=(0, 0, 1, 0.86 if has_subgroups else 0.92))
    return save_fig(fig, out_dir / f"{slugify(question_title)}.png")


def render_insights_csv(csv_path: Path, df: pd.DataFrame, chartables: list[Chartable], out_dir: Path) -> None:
    """One chart + one matching presenter-notes .txt per row of a mining-output insights
    CSV (theme, insight_id, chart_type, question_1, question_2, group_by, n, stat_summary,
    data_summary, suggested_sentence, notes -- see analysis/insights.csv and friends).

    Chart choice per row (question_1/question_2/chart_type as authored in the CSV are a
    hint, but the actual schema type of each column is what decides the plot, so a
    ("scatter"-labeled) row against a categorical column still renders as a grouped bar):
      - group_by set                          -> grouped-by-category bar (value=question_1)
      - question_2 set, categorical            -> grouped-by-category bar (value=question_1)
      - question_2 set, numeric                -> scatter
      - neither set, question_1 is a special
        numeric (free-text but numeric) column -> integer-value histogram
      - neither set, otherwise                 -> pie or single bar, per the row's chart_type

    Free-response rows (question_1 is a schema type="text", section="Free-response" column
    -- e.g. "Unpopular opinion I stand by:") are handled separately: every row sharing the
    same question_1 is one coded bucket of that question's answers, so they're grouped into
    ONE chart per distinct free-response question (see plot_free_response_buckets) rather
    than one chart per row.
    """
    with csv_path.open(newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    fr_titles = free_response_titles(chartables)
    fr_groups: dict[str, list[dict]] = {}
    regular_rows: list[dict] = []
    for row in rows:
        title = row["question_1"].strip()
        if title.startswith("[") and "response rate" in row.get("data_summary", "").lower():
            png_path = plot_response_rate_overview(row, out_dir)
            notes_lines = [f"insight_id: {row['insight_id']}", f"n = {row['n']}"]
            for field_name in ("stat_summary", "data_summary", "suggested_sentence", "notes"):
                if row.get(field_name):
                    notes_lines.append(row[field_name])
            png_path.with_suffix(".txt").write_text("\n".join(notes_lines) + "\n")
            print(f"wrote {png_path.with_suffix('.txt')}")
        elif title in fr_titles:
            fr_groups.setdefault(title, []).append(row)
        else:
            regular_rows.append(row)

    for row in regular_rows:
        q1, q2, group_by = row["question_1"].strip(), row["question_2"].strip(), row["group_by"].strip()

        if group_by:
            png_path = plot_grouped_by_category(q1, group_by, df, chartables, out_dir)
        elif q2:
            if is_categorical_title(q2, chartables):
                png_path = plot_grouped_by_category(q1, q2, df, chartables, out_dir)
            else:
                png_path = plot_scatter(q1, q2, df, chartables, out_dir)
        elif q1 in SPECIAL_NUMERIC_TITLES:
            png_path = plot_numeric_distribution(q1, df, out_dir)
        elif row["chart_type"] == "pie":
            png_path = plot_pie(find_chartable(q1, chartables), df, out_dir)
        else:
            png_path = plot_single(find_chartable(q1, chartables), df, out_dir)

        # Presenter notes: terse info, not prose -- same basename as the image.
        notes_lines = [f"insight_id: {row['insight_id']}", f"n = {row['n']}"]
        for field_name in ("stat_summary", "data_summary", "suggested_sentence", "notes"):
            if row.get(field_name):
                notes_lines.append(row[field_name])
        notes_path = png_path.with_suffix(".txt")
        notes_path.write_text("\n".join(notes_lines) + "\n")
        print(f"wrote {notes_path}")

    for title, group_rows in fr_groups.items():
        png_path = plot_free_response_buckets(title, group_rows, out_dir)

        notes_lines = [f"question: {title}", f"n_nonblank: {group_rows[0].get('n', '')}"]
        parsed = []
        for r in group_rows:
            m = FREE_RESPONSE_BUCKET_RE.match(r["stat_summary"].strip())
            name, count, pct = (m.group(1), int(m.group(2)), int(m.group(3))) if m else (r["stat_summary"], 0, 0)
            parsed.append((name, count, pct, r))
        parsed.sort(key=lambda t: -t[1])
        for name, count, pct, r in parsed:
            line = f"{name}: {count} ({pct}%)"
            if r["group_by"].strip():
                line += f" [{r['group_by'].strip()}]"
            notes_lines.append(line)
            if r.get("data_summary", "").strip():
                notes_lines.append(f"  quotes: {r['data_summary'].strip()}")
        seen: set[str] = set()
        for _, _, _, r in parsed:
            for text in (r.get("suggested_sentence", "").strip(), r.get("notes", "").strip()):
                if text and text not in seen:
                    seen.add(text)
                    notes_lines.append(text)

        notes_path = png_path.with_suffix(".txt")
        notes_path.write_text("\n".join(notes_lines) + "\n")
        print(f"wrote {notes_path}")


# ---------------------------------------------------------------------------
# Dispatch + CLI
# ---------------------------------------------------------------------------


def render_charts(
    charts: dict[str, list[tuple[str, ...]]],
    df: pd.DataFrame,
    chartables: list[Chartable],
    plottable: list[Chartable],
    out_dir: Path,
) -> None:
    for folder, specs in charts.items():
        folder_dir = out_dir / folder
        for spec in specs:
            kind, *rest = spec
            if kind == "gender_counts":
                plot_gender_counts(df, folder_dir)
            elif kind == "age":
                plot_age(df, folder_dir)
            elif kind == "single":
                plot_single(find_chartable(rest[0], plottable), df, folder_dir)
            elif kind == "pie":
                plot_pie(find_chartable(rest[0], plottable), df, folder_dir)
            elif kind == "gender":
                plot_by_gender(find_chartable(rest[0], plottable), df, folder_dir)
            elif kind == "overview":
                chs, title = resolve_overview(rest, plottable)
                plot_overview(chs, title, df, folder_dir)
            elif kind == "all_single":
                for c in plottable:
                    plot_single(c, df, folder_dir)
            elif kind == "instruction_followers":
                plot_instruction_followers(df, chartables, folder_dir)
            elif kind == "from_insights_csv":
                render_insights_csv(Path(__file__).resolve().parent / rest[0], df, chartables, folder_dir)
            else:
                raise ValueError(f"Unknown chart kind {kind!r} in CHARTS[{folder!r}]")


def clear_output_dir(out_dir: Path) -> None:
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)


def build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--list", action="store_true", help="List every chartable question (for the CHARTS entries above) and exit")
    p.add_argument("--sections", action="store_true", help="List section names (for an ('overview', <section>) entry) and exit")
    p.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    p.add_argument("--data", type=Path, default=DEFAULT_DATA)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    return p


def main(argv: list[str] | None = None) -> None:
    args = build_arg_parser().parse_args(argv)
    df, chartables = load_chartables(args.schema, args.data)
    plottable = [c for c in chartables if c.question.qtype in ("scale", "choice")]

    if args.list:
        for c in plottable:
            print(f"[{c.question.qtype:6}] ({c.question.section}) {c.display_title}")
        return
    if args.sections:
        for s in dict.fromkeys(c.question.section for c in plottable):
            print(s)
        return

    clear_output_dir(args.out)
    render_charts(CHARTS, df, chartables, plottable, args.out)
    print(f"\n{len(list(args.out.glob('**/*.png')))} chart(s) in {args.out}")


if __name__ == "__main__":
    main()
