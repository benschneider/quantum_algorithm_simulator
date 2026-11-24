#!/usr/bin/env python3
"""Generate publication plots from the benchmark CSV."""
import argparse
from collections import defaultdict
import csv
import math
from pathlib import Path
from statistics import median

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import FixedLocator, ScalarFormatter

HERE = Path(__file__).resolve().parent
WIDTHS = (8, 16, 24, 32, 50, 64, 128, 256, 512, 1024)
PRIMARY = {
    ("product", "product", "forward"): "Product",
    ("ghz", "tableau", "forward"): "GHZ · tableau",
    ("groups", "grouped", "forward"): "Independent groups",
    ("qft", "product", "forward"): "Basis-input QFT",
    ("sycamore_echo", "tableau", "phase_flip"): "Sycamore echo",
    ("non_clifford", "grouped", "forward"): "Non-Clifford · grouped",
    ("non_clifford", "dense", "forward"): "Non-Clifford · dense",
}
COLORS = ("#1b6a8e", "#bb4b26", "#327651", "#6c53a3", "#646b73", "#a62766", "#a47300")
MIB = 1024**2


def read(path):
    samples = defaultdict(list)
    validation = {}
    theoretical = {}
    with path.open(newline="") as file:
        for row in csv.DictReader(file):
            n = int(row["qubits"])
            theoretical[n] = tuple(int(row[f"theoretical_dense_f{x}_bytes"]) for x in (32, 64))
            key = row["workload"], row["backend"], row["variant"], int(row["bond_cap"] or 0), n
            if row["status"] != "ok":
                continue
            if row["stage"] == "measure":
                samples[key].append(row)
            elif row["stage"] == "verify":
                validation[key] = row
    return samples, validation, theoretical


def match(samples, family, backend, variant, cap=0):
    return sorted(
        (n, rows) for (f, b, v, c, n), rows in samples.items() if (f, b, v, c) == (
            family, backend, variant, cap
        )
    )


def draw(ax, series, metric, scale, label, color, *, style="-", ranges=False):
    if not series:
        return
    x = [n for n, _ in series]
    values = [[float(row[metric]) / scale for row in rows] for _, rows in series]
    y = [median(group) for group in values]
    ax.plot(x, y, style, color=color, marker="o", ms=4, lw=1.5, label=label)
    if ranges:
        ax.vlines(x, [min(group) for group in values], [max(group) for group in values],
                  color=color, lw=0.8, alpha=0.65)


def width_axis(ax, ticks=WIDTHS):
    ax.set_xscale("log", base=2)
    ax.xaxis.set_major_locator(FixedLocator(ticks))
    ax.xaxis.set_major_formatter(ScalarFormatter())
    ax.tick_params(axis="x", labelrotation=35)
    ax.set_xlabel("Circuit qubits")


def finish(fig, out, name, formats):
    for extension in formats:
        fig.savefig(out / f"{name}.{extension}", dpi=200, facecolor="white")
    plt.close(fig)


def plot_mps_policies(path, out, formats):
    with path.open(newline="") as file:
        rows = list(csv.DictReader(file))
    fig, axes = plt.subplots(1, 2, figsize=(11, 4.7), layout="constrained")
    fig.suptitle("MPS accuracy and memory by execution policy", fontsize=15, fontweight="bold")
    policies = [
        ("mps", 32, "Fixed cap 32", COLORS[0], "o"),
        ("mps-paired", 32, "Paired gates, cap 32", COLORS[1], "^"),
        ("mps-adaptive", 32, "Paired gates, adaptive caps", COLORS[2], "s"),
        ("mps-paired", 0, "Paired gates, uncapped", "#575c65", "D"),
    ]
    for ax, n, variant, metric in (
        (axes[0], 16, "forward", "fidelity"),
        (axes[1], 256, "reverse", "return_probability"),
    ):
        for backend, cap, label, color, marker in policies:
            if n == 256 and cap == 0:
                continue
            group = [r for r in rows if int(r["qubits"]) == n and r["backend"] == backend
                     and int(r["bond_cap"]) == cap]
            peaks = [int(r["peak_live_heap_above_baseline_bytes"]) for r in group
                     if r["stage"] == "measure" and r["variant"] == variant and r["status"] == "ok"]
            checks = [r for r in group if r["stage"] == "verify" and r["variant"] == variant
                      and r[metric] and r["validation_status"] in ("pass", "approximation_measured")]
            if not peaks or len(checks) != 1:
                raise ValueError(f"Missing measurement or validation for {label}, {n} qubits")
            x = median(peaks) / MIB
            y = float(checks[0][metric])
            ax.plot(x, y, marker=marker, ms=8, color=color, linestyle="none")
            # The two fixed-cap results nearly coincide. Put their labels on
            # opposite sides and keep high-memory point labels inside the axes.
            dx = -6 if cap == 0 or (backend == "mps-adaptive" and n == 256) else 6
            dy = -6 if backend == "mps" or cap == 0 else 6
            value = f"{y:.3f}" if n == 16 else f"{y:.2e}"
            ax.annotate(f"{label}\n{value}", (x, y), xytext=(dx, dy), textcoords="offset points",
                        va="bottom" if dy > 0 else "top", ha="left" if dx > 0 else "right",
                        color=color, fontsize=9)
        ax.set_xlim(left=0)
        operation = "forward" if variant == "forward" else "echo"
        ax.set_xlabel(f"Median {operation} peak heap above baseline (MiB)")
    axes[0].set_title("16 qubits · forward circuit")
    axes[0].set_ylabel("Whole-state fidelity vs. dense f64")
    axes[0].set_ylim(-0.03, 1.05)
    axes[1].set_title("256 qubits · forward and reverse circuit")
    axes[1].set_ylabel("Return probability (ideal = 1)")
    axes[1].set_yscale("log")
    # Leave room for direct labels on the logarithmic scale.
    low, high = axes[1].get_ylim()
    axes[1].set_ylim(low / 2.5, high * 2.5)
    finish(fig, out, "mps-policy", formats)


def plot_mps_cap_curve(path, out, formats):
    with path.open(newline="") as file:
        points = sorted((int(row["bond_cap"]), float(row["return_probability"]))
                        for row in csv.DictReader(file))
    fig, ax = plt.subplots(figsize=(8.4, 4.5), layout="constrained")
    ax.set_title("256-qubit ladder echo · 8 rounds · seed 53", pad=12)
    ax.set_xlabel("Uniform MPS bond cap")
    ax.set_xticks((1, 16, 32, 64, 96, 128, 160, 192, 224, 256))
    ax.set_ylabel("All-zero return probability (log scale)")
    ax.set_yscale("log")
    ax.set_ylim(bottom=points[0][1] / 100, top=50)
    ax.set_yticks([10.0**k for k in range(-50, 1, 10)])
    ax.axhline(0.99, color="#686c73", linestyle=":", linewidth=1.15)
    ax.annotate("99% target", (1, 0.99), xytext=(0, -5), textcoords="offset points",
                va="top", color="#555960")
    for cap, p in points:
        if cap == 256:
            ax.plot(cap, p, marker="D", color=COLORS[2], ms=6)
            ax.annotate(f"Cap 256 · structurally sufficient\nMeasured return {p:.2%}",
                        (cap, p), xytext=(-6, -13), textcoords="offset points",
                        va="top", ha="right", color=COLORS[2])
        else:
            ax.plot(cap, p, marker="o", color=COLORS[0], ms=5)
            offset = 4 if cap == 1 else -4
            ax.annotate(f"{p:.2e}", (cap, p), xytext=(6, offset), textcoords="offset points",
                        va="bottom" if offset > 0 else "top", color=COLORS[0], fontsize=8)
    lower_caps = [cap for cap, _ in points if cap < 256]
    if lower_caps and points[-1][0] == 256 and lower_caps[-1] < 255:
        ax.text(0.78, 0.53, f"No measured returns\nbetween caps {lower_caps[-1]} and 256",
                transform=ax.transAxes, ha="center", va="center", color="#555960")
    finish(fig, out, "mps-cap-curve", formats)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    data = parser.add_mutually_exclusive_group()
    data.add_argument("--csv", type=Path, default=HERE / "benchmark.csv")
    data.add_argument("--mps-policy-csv", type=Path, help="plot only the MPS policy comparison")
    data.add_argument("--mps-cap-curve", type=Path, help="plot only the 256-qubit echo bond-cap sweep")
    parser.add_argument("--out", type=Path, default=HERE)
    parser.add_argument("--format", dest="formats", action="append", choices=["png", "pdf", "svg"])
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    formats = args.formats or ["png"]
    plt.rcParams.update({
        "font.size": 10, "font.family": "DejaVu Sans", "axes.spines.top": False,
        "axes.spines.right": False, "axes.grid": True, "grid.alpha": 0.25,
        "axes.titleweight": "bold", "legend.frameon": False,
    })
    if args.mps_policy_csv:
        plot_mps_policies(args.mps_policy_csv, args.out, formats)
        return
    if args.mps_cap_curve:
        plot_mps_cap_curve(args.mps_cap_curve, args.out, formats)
        return
    samples, validation, theoretical = read(args.csv)

    fig, (context, heap, rss) = plt.subplots(1, 3, figsize=(15.5, 5.2), layout="constrained")
    fig.suptitle("Memory scaling by simulation representation", fontsize=16, fontweight="bold")
    ns = sorted(theoretical)
    for col, bits in enumerate((32, 64)):
        context.plot(ns, [math.log10(theoretical[n][col]) for n in ns],
                     linestyle=("--", "-")[col], color=("0.55", "0.15")[col],
                     label=f"Dense complex f{bits} · theory")
    for key, label, color in zip(PRIMARY, PRIMARY.values(), COLORS):
        series = match(samples, *key)
        if series:
            context.plot([n for n, _ in series],
                         [math.log10(median(int(r["process_peak_live_heap_bytes"])
                                             for r in rows)) for _, rows in series],
                         color=color, lw=1)
        draw(heap, series, "process_peak_live_heap_bytes", MIB, label, color, ranges=True)
        draw(rss, series, "peak_rss_bytes", MIB, label, color, ranges=True)
    context.set_title("Dense storage comparison")
    context.set_ylabel("log₁₀(bytes)")
    context.legend(loc="upper left", fontsize=8)
    for ax, title in ((heap, "Process peak requested heap"), (rss, "Process peak RSS")):
        ax.set_title(title)
        ax.set_yscale("log")
        ax.set_ylabel("MiB")
    rss.legend(loc="best", fontsize=8)
    for ax in (context, heap, rss):
        width_axis(ax, (8, 16, 32, 64, 128, 256, 512, 1024))
    finish(fig, args.out, "memory-scaling", formats)

    fig, (exact, mps) = plt.subplots(1, 2, figsize=(12.5, 5.0), layout="constrained")
    fig.suptitle("Native release-mode execution time", fontsize=16, fontweight="bold")
    for key, label, color in zip(PRIMARY, PRIMARY.values(), COLORS):
        draw(exact, match(samples, *key), "elapsed_ns", 1e6, label, color, ranges=True)
    for i, cap in enumerate((2, 4, 8, 16, 32)):
        draw(mps, match(samples, "mps_ladder", "mps", "forward", cap),
             "elapsed_ns", 1e6, f"Cap {cap}", COLORS[i], ranges=True)
    for ax, title in ((exact, "Core backends and Clifford echo"), (mps, "Experimental MPS · forward")):
        ax.set_title(title)
        ax.set_yscale("log")
        ax.set_ylabel("Wall time (ms)")
        width_axis(ax, (8, 16, 32, 64, 128, 256, 512, 1024) if ax == exact else (8, 16, 32, 64, 128, 256))
        ax.legend(fontsize=8)
    finish(fig, args.out, "time-scaling", formats)

    fig, (forward, reverse) = plt.subplots(1, 2, figsize=(12.5, 5.1), layout="constrained")
    fig.suptitle("Experimental MPS memory and accuracy", fontsize=16, fontweight="bold")
    for ax, variant, metric, sizes in (
        (forward, "forward", "fidelity", (8, 16)),
        (reverse, "reverse", "return_probability", (8, 16, 24, 32, 50, 64, 128, 256)),
    ):
        for i, n in enumerate(sizes):
            points = []
            for key, v in validation.items():
                family, backend, mode, cap, width = key
                if (family, backend, mode, width) != ("mps_ladder", "mps", variant, n) or not v[metric]:
                    continue
                data = samples[key]
                x = median(int(r["peak_live_heap_above_baseline_bytes"]) for r in data) / 1024
                points.append((x, float(v[metric]), cap))
            if not points:
                continue
            points.sort(key=lambda p: p[2] or float("inf"))
            color = COLORS[i % len(COLORS)]
            ax.plot([x for x, _, _ in points], [y for _, y, _ in points],
                    marker="o", ms=4, color=color, linestyle="--" if i >= len(COLORS) else "-",
                    label=f"{n} qubits")
            if ax is forward and n == 16:
                for x, y, cap in points:
                    # Label the non-overlapping, informative part of the sweep.
                    if cap in (0, 2, 4, 8, 16, 32, 64, 128):
                        ax.annotate(str(cap) if cap else "uncapped", (x, y),
                                    xytext=(0, 6), ha="center",
                                    textcoords="offset points", fontsize=8, color=color)
        ax.set_xscale("log")
        ax.set_xlabel("Peak live heap above baseline (KiB)")
        ax.legend(fontsize=8)
    forward.set_title("Forward state vs. independent dense f64")
    forward.set_ylabel("Normalized state fidelity")
    forward.set_ylim(-0.025, 1.09)
    reverse.set_title("Approximate forward + adjoint")
    reverse.set_ylabel("Probability of returning to |0…0⟩")
    reverse.set_yscale("log")
    reverse.set_ylim(top=1.6)
    finish(fig, args.out, "mps-memory-fidelity", formats)


if __name__ == "__main__":
    main()
