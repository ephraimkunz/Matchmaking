#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "pandas>=2.2",
#   "numpy>=1.26",
#   "matplotlib>=3.9",
#   "scikit-learn>=1.5",
# ]
# ///
"""Cluster survey respondents into personas based on their questionnaire answers.

Encodes every self-description / interests / social-style / lifestyle / partner-
preference / dealbreaker answer from real_round_1_emailed_cleaned.csv as a numeric
feature (Likert scales as-is, the "importance to me that partner agrees" choice as
a 0-4 ordinal, and the handful of other ordinal choice questions as small integer
scales). Age, Gender, and the free-response questions are held out of the feature
matrix -- kept only as descriptive context for the clusters found.

Standardizes, reduces with PCA (enough components for 80% variance), then fits
KMeans, picking k by silhouette score over k=2..8. Writes:
  - cluster_assignments.csv   one row per respondent: name, gender, age, cluster
  - cluster_profile.csv       per cluster: size, gender/age mix, top distinguishing
                               features (z-score mean + plain-English direction)
  - pca_scatter.png           PC1 vs PC2, colored by cluster
  - k_selection.png           inertia + silhouette curves used to pick k
  - cluster_<n>_profile.png   diverging bar chart of that cluster's top features

Usage: uv run analysis/clustering/cluster.py
"""
from __future__ import annotations

import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.colors as mcolors
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
from sklearn.cluster import KMeans
from sklearn.decomposition import PCA
from sklearn.metrics import silhouette_score

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import generate_graphs as gg  # noqa: E402  (needs sys.path tweak above)

OUT_DIR = Path(__file__).resolve().parent
RANDOM_STATE = 42

# Choice questions (beyond the "Importance: ..." ones, handled generically) that
# need an explicit ordinal encoding, plus what the low (0) and high end mean in
# plain English for reporting.
ORDINAL_MAPS = {
    "I want to have children": {"No": 0, "Open to it": 1, "Yes": 2},
    "I'd like to be married within": {"0 - 2 years": 0, "2 - 5 years": 1, "5+ years": 2},
    "I intend to stay in Cache Valley long term": {"No": 0, "It depends": 1, "Yes": 2},
    "My partner's religious commitment level should be:": {
        "it doesn't matter": 0,
        "within one level of mine": 1,
        "the same as mine": 2,
    },
}
ORDINAL_LOW_HIGH = {
    "I want to have children": ("doesn't want kids", "wants kids"),
    "I'd like to be married within": ("fine with a longer timeline", "wants to marry very soon"),
    "I intend to stay in Cache Valley long term": ("wants to leave the area", "wants to stay long-term"),
    "My partner's religious commitment level should be:": (
        "partner's religiosity doesn't need to match",
        "wants partner's religiosity to closely match",
    ),
}

# Held out of the feature matrix entirely -- demographic identifiers, not attitudes.
SKIP_TITLES = {"First and last name", "Gender", "Age"}

FREE_TEXT_TITLES = [
    "The thing I find most attractive in a person:",
    "Unpopular opinion I stand by:",
    "Something I've changed my mind about recently:",
    "My weekend usually looks like:",
]


def build_feature_frame(df: pd.DataFrame, chartables: list[gg.Chartable]) -> tuple[pd.DataFrame, dict, dict]:
    cols: dict[str, pd.Series] = {}
    low_labels: dict[str, str] = {}
    high_labels: dict[str, str] = {}

    for c in chartables:
        title = c.display_title
        q = c.question
        if title in SKIP_TITLES:
            continue

        if q.qtype == "text":
            if title == "I'd like to have ___ child (children)":
                cols[title] = pd.to_numeric(df[c.column], errors="coerce")
                low_labels[title], high_labels[title] = "wants 0 kids", "wants many kids"
            continue  # other text columns are free-response -- not clustering features

        if q.qtype == "scale":
            cols[title] = pd.to_numeric(df[c.column], errors="coerce")
            low_labels[title], high_labels[title] = q.low_label, q.high_label
            continue

        if q.qtype == "choice":
            if title == "Gender":
                continue
            if title.startswith("Importance: "):
                mapping = {v: i for i, v in enumerate(gg.IMPORTANCE_ORDER)}
                cols[title] = df[c.column].map(mapping)
                low_labels[title], high_labels[title] = "doesn't need partner to agree", "partner MUST agree"
                continue
            if title in ORDINAL_MAPS:
                cols[title] = df[c.column].map(ORDINAL_MAPS[title])
                low_labels[title], high_labels[title] = ORDINAL_LOW_HIGH[title]
                continue
            raise ValueError(f"Unhandled choice question: {title!r}")

    X = pd.DataFrame(cols)
    na_cols = X.columns[X.isna().any()].tolist()
    if na_cols:
        raise ValueError(f"Unexpected NaNs after encoding, investigate: {na_cols}")
    return X, low_labels, high_labels


def pick_k(Xp: np.ndarray, k_range: range) -> tuple[int, dict[int, float], dict[int, float]]:
    inertias, sils = {}, {}
    for k in k_range:
        km = KMeans(n_clusters=k, n_init=50, random_state=RANDOM_STATE).fit(Xp)
        inertias[k] = km.inertia_
        sils[k] = silhouette_score(Xp, km.labels_)
    best_k = max(sils, key=sils.get)
    return best_k, inertias, sils


def plot_k_selection(inertias: dict[int, float], sils: dict[int, float], best_k: int, out_path: Path) -> None:
    ks = sorted(inertias)
    fig, axes = plt.subplots(1, 2, figsize=(9, 3.4))
    for ax, data, title in ((axes[0], inertias, "Inertia (lower = tighter clusters)"), (axes[1], sils, "Silhouette score (higher = better separated)")):
        ax.plot(ks, [data[k] for k in ks], color=gg.SERIES_1, marker="o", linewidth=2)
        ax.axvline(best_k, color=gg.INK_MUTED, linestyle="--", linewidth=1)
        ax.set_title(title, fontsize=10, color=gg.INK_PRIMARY)
        ax.set_xlabel("k (number of clusters)", fontsize=9, color=gg.INK_SECONDARY)
        ax.set_xticks(ks)
        for spine in ("top", "right"):
            ax.spines[spine].set_visible(False)
        ax.spines["bottom"].set_color(gg.BASELINE)
        ax.spines["left"].set_color(gg.BASELINE)
        ax.tick_params(colors=gg.INK_MUTED)
    fig.suptitle(f"Chosen k = {best_k}", fontsize=11, color=gg.INK_PRIMARY)
    fig.tight_layout()
    gg.save_fig(fig, out_path)


# Respondents to call out by name on the PCA scatter (stripped of trailing
# whitespace some names have in the raw CSV before matching). Default label
# offset (in points) is (7, 6); override per-name here when two labelled
# points sit close enough together to collide.
HIGHLIGHT_NAMES = ["AJ Bixler", "Scott Draper", "Stephen Compton", "Tyler Barnard", "Keilani Merrell"]
LABEL_OFFSET_OVERRIDES = {}


def plot_pca_scatter(Xp: np.ndarray, labels: np.ndarray, genders: pd.Series, names: pd.Series, out_path: Path) -> None:
    fig, ax = plt.subplots(figsize=(7, 6))
    marker_by_gender = {"Male": "o", "Female": "^"}
    for cluster in sorted(set(labels)):
        color = gg.CATEGORICAL[cluster % len(gg.CATEGORICAL)]
        for gender, marker in marker_by_gender.items():
            mask = (labels == cluster) & (genders.to_numpy() == gender)
            if mask.any():
                ax.scatter(
                    Xp[mask, 0],
                    Xp[mask, 1],
                    color=color,
                    marker=marker,
                    s=55,
                    alpha=0.85,
                    edgecolor="white",
                    linewidth=0.5,
                    label=f"Cluster {cluster} ({gender})",
                )

    stripped_names = names.str.strip()
    for highlight in HIGHLIGHT_NAMES:
        matches = stripped_names[stripped_names == highlight].index
        if len(matches) == 0:
            print(f"warning: could not find {highlight!r} to label on scatter plot", file=sys.stderr)
            continue
        idx = matches[0]
        x, y = Xp[idx, 0], Xp[idx, 1]
        ax.scatter([x], [y], s=110, facecolor="none", edgecolor=gg.INK_PRIMARY, linewidth=1.3, zorder=5)
        ax.annotate(
            highlight,
            (x, y),
            textcoords="offset points",
            xytext=LABEL_OFFSET_OVERRIDES.get(highlight, (7, 6)),
            fontsize=8,
            color=gg.INK_PRIMARY,
            fontweight="bold",
            zorder=6,
        )

    ax.set_xlabel("PC1", color=gg.INK_SECONDARY)
    ax.set_ylabel("PC2", color=gg.INK_SECONDARY)
    ax.set_title("Respondents in principal-component space", color=gg.INK_PRIMARY, fontsize=12)
    for spine in ("top", "right"):
        ax.spines[spine].set_visible(False)
    ax.spines["bottom"].set_color(gg.BASELINE)
    ax.spines["left"].set_color(gg.BASELINE)
    ax.tick_params(colors=gg.INK_MUTED)
    ax.legend(*ax.get_legend_handles_labels(), fontsize=7, loc="best", frameon=False, ncol=2)
    gg.save_fig(fig, out_path)


def plot_cluster_profile(cluster_id: int, top_features: pd.Series, low_labels: dict, high_labels: dict, n: int, out_path: Path) -> None:
    ordered = top_features.sort_values()
    labels = [f"{name}  [{high_labels[name] if z > 0 else low_labels[name]}]" for name, z in ordered.items()]
    colors = [gg.DIV_HIGH if z > 0 else gg.DIV_LOW for z in ordered]

    fig, ax = plt.subplots(figsize=(9, 0.4 * len(ordered) + 1.2))
    ax.barh(range(len(ordered)), ordered.to_numpy(), color=colors)
    ax.set_yticks(range(len(ordered)))
    ax.set_yticklabels([gg.wrap_title(l, width=70) for l in labels], fontsize=8, color=gg.INK_PRIMARY)
    ax.axvline(0, color=gg.INK_MUTED, linewidth=1)
    ax.set_xlabel("Standardized mean (z-score) vs. all respondents", color=gg.INK_SECONDARY, fontsize=9)
    ax.set_title(f"Cluster {cluster_id} (n={n}) -- what sets this group apart", color=gg.INK_PRIMARY, fontsize=12)
    for spine in ("top", "right", "left"):
        ax.spines[spine].set_visible(False)
    ax.spines["bottom"].set_color(gg.BASELINE)
    ax.tick_params(axis="x", colors=gg.INK_MUTED)
    ax.tick_params(axis="y", length=0)
    fig.tight_layout()
    gg.save_fig(fig, out_path)


def main() -> None:
    df, chartables = gg.load_chartables(gg.DEFAULT_SCHEMA, gg.DEFAULT_DATA)
    df = df.reset_index(drop=True)
    X, low_labels, high_labels = build_feature_frame(df, chartables)
    print(f"{len(df)} respondents (after dedup), {X.shape[1]} features")

    mean, std = X.mean(), X.std(ddof=0)
    Xz_df = (X - mean) / std
    Xz = Xz_df.to_numpy()

    pca_full = PCA(random_state=RANDOM_STATE).fit(Xz)
    cum = np.cumsum(pca_full.explained_variance_ratio_)
    n_components = int(np.searchsorted(cum, 0.80) + 1)
    n_components = max(2, min(n_components, 20, Xz.shape[0] - 1))
    pca = PCA(n_components=n_components, random_state=RANDOM_STATE)
    Xp = pca.fit_transform(Xz)
    print(f"PCA: {n_components} components explain {cum[n_components - 1]:.1%} of variance")

    best_k, inertias, sils = pick_k(Xp, range(2, 9))
    print("silhouette by k:", {k: round(v, 3) for k, v in sils.items()})
    print(f"chosen k = {best_k}")
    plot_k_selection(inertias, sils, best_k, OUT_DIR / "k_selection.png")

    km = KMeans(n_clusters=best_k, n_init=200, random_state=RANDOM_STATE).fit(Xp)
    labels = km.labels_

    genders = df["Gender"].reset_index(drop=True)
    ages = pd.to_numeric(df["Age"], errors="coerce").reset_index(drop=True)
    names = df["First and last name"].reset_index(drop=True)
    usernames = df["Username"].reset_index(drop=True)

    plot_pca_scatter(Xp, labels, genders, names, OUT_DIR / "pca_scatter.png")

    dist_to_own_centroid = np.linalg.norm(Xp - km.cluster_centers_[labels], axis=1)
    assignments = pd.DataFrame(
        {
            "name": names,
            "username": usernames,
            "gender": genders,
            "age": ages,
            "cluster": labels,
            "distance_to_centroid": dist_to_own_centroid,
        }
    ).sort_values(["cluster", "distance_to_centroid"])
    assignments.to_csv(OUT_DIR / "cluster_assignments.csv", index=False)

    profile_rows = []
    N_TOP = 12
    for cluster_id in sorted(set(labels)):
        mask = labels == cluster_id
        n = int(mask.sum())
        cluster_means = Xz_df.loc[mask].mean()
        top = cluster_means.reindex(cluster_means.abs().sort_values(ascending=False).index).head(N_TOP)
        plot_cluster_profile(cluster_id, top, low_labels, high_labels, n, OUT_DIR / f"cluster_{cluster_id}_profile.png")

        g_counts = genders[mask].value_counts()
        exemplars = assignments[assignments["cluster"] == cluster_id].head(3)

        for rank, (feat, z) in enumerate(top.items(), start=1):
            profile_rows.append(
                {
                    "cluster": cluster_id,
                    "n": n,
                    "male": int(g_counts.get("Male", 0)),
                    "female": int(g_counts.get("Female", 0)),
                    "mean_age": round(float(ages[mask].mean()), 1),
                    "rank": rank,
                    "feature": feat,
                    "z_score": round(float(z), 2),
                    "direction": high_labels[feat] if z > 0 else low_labels[feat],
                    "exemplar_names": "; ".join(exemplars["name"].tolist()),
                }
            )
    pd.DataFrame(profile_rows).to_csv(OUT_DIR / "cluster_profile.csv", index=False)

    print(f"\nCluster sizes: {dict(pd.Series(labels).value_counts().sort_index())}")
    for cluster_id in sorted(set(labels)):
        mask = labels == cluster_id
        print(f"\n=== Cluster {cluster_id} (n={int(mask.sum())}) ===")
        g_counts = genders[mask].value_counts()
        print(f"  gender: {dict(g_counts)}  mean age: {ages[mask].mean():.1f}")
        cluster_means = Xz_df.loc[mask].mean()
        top = cluster_means.reindex(cluster_means.abs().sort_values(ascending=False).index).head(6)
        for feat, z in top.items():
            direction = high_labels[feat] if z > 0 else low_labels[feat]
            print(f"    {feat}  ->  {direction}  (z={z:+.2f})")
        print("  free-text flavor:")
        for _, row in assignments[assignments["cluster"] == cluster_id].head(2).iterrows():
            person_idx = names[names == row["name"]].index[0]
            quote = df.loc[person_idx, "The thing I find most attractive in a person:"]
            if isinstance(quote, str) and quote.strip():
                print(f"    {row['name']} on what they find attractive: \"{quote.strip()}\"")

    print(f"\nWrote outputs to {OUT_DIR}")


if __name__ == "__main__":
    main()
