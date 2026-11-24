#!/usr/bin/env python3
"""Compare fixed, paired, and calibrated MPS policies on the same seeded ladder."""
import argparse
import csv
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import subprocess as sp
import sys

import run

BASE_CAP = 32
BUDGET_KIB = {16: 585, 256: 12288}
TOLERANCE = 2e-10


def source_hashes():
    core = run.ROOT / run.CORE
    files = [
        *sorted((core / "src").rglob("*.rs")),
        *sorted((core / "circuits").rglob("*.json")),
        core / "Cargo.lock", core / "Cargo.toml", core / "build.rs",
        core / "examples/audit_benchmark.rs",
        *sorted((core / "examples/audit_benchmark").glob("*.rs")),
        *(core / "examples/sycamore_mps" / name for name in
          ("budget.rs", "chain.rs", "layers.rs", "planning.rs")),
        Path(run.__file__).resolve(), Path(__file__).resolve(),
    ]
    return {str(p.relative_to(run.ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in files}


def cases(width):
    policies = [("mps", BASE_CAP), ("mps-paired", BASE_CAP), ("mps-adaptive", BASE_CAP)]
    if width == 16:
        policies.append(("mps-paired", 0))
    for backend, cap in policies:
        for variant in ("forward", "reverse"):
            yield backend, cap, variant


def check_verification(result, width, cap, variant):
    expected_status = "pass" if cap == 0 else "approximation_measured"
    if width > 16 and variant == "forward":
        expected_status = "norm_only"
    if result.get("validation_status") != expected_status:
        raise RuntimeError(f"expected {expected_status}, got {result}")
    if not 0 <= result["norm_error"] <= TOLERANCE:
        raise RuntimeError(f"norm check failed: {result}")
    if width <= 16:
        if result.get("dense_reference") != "independent_f64":
            raise RuntimeError(f"missing dense reference: {result}")
        if not -TOLERANCE <= result["fidelity"] <= 1 + TOLERANCE:
            raise RuntimeError(f"invalid dense fidelity: {result}")
    elif "fidelity" in result:
        raise RuntimeError(f"unexpected wide-circuit fidelity: {result}")
    if variant == "reverse" and not 0 <= result["return_probability"] <= 1 + TOLERANCE:
        raise RuntimeError(f"invalid echo return probability: {result}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=run.HERE)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--timeout", type=int, default=120, help="seconds per child process")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    if args.repetitions < 1 or args.timeout < 1:
        parser.error("repetitions and timeout must be positive")
    if sys.platform not in {"darwin", "linux"}:
        parser.error("peak RSS measurement requires macOS or Linux")

    env = {**os.environ, "RAYON_NUM_THREADS": "1", "CARGO_TARGET_DIR": str(run.ROOT / "target/audit")}
    build = ["cargo", "build", "--locked", "--release", "--manifest-path",
             str(run.CORE / "Cargo.toml"), "--example", "audit_benchmark"]
    if args.offline:
        build.insert(2, "--offline")
    hashes = source_hashes()
    sp.run(build, cwd=run.ROOT, env=env, check=True)
    binary = Path(env["CARGO_TARGET_DIR"]) / "release/examples/audit_benchmark"
    metadata = {
        "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "git_commit": run.command("git", "rev-parse", "HEAD"),
        "sha256": hashes, "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "hardware": run.hardware(), "rustc": run.command("rustc", "-vV"),
        "cargo": run.command("cargo", "-V"), "cargo_build": build,
        "environment": {k: env.get(k) for k in
                        ("RAYON_NUM_THREADS", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTUP_TOOLCHAIN")},
        "base_cap": BASE_CAP, "adaptive_tensor_budget_kib_by_width": BUDGET_KIB,
        "ladder_seed": 53, "ladder_rounds": 8, "repetitions": args.repetitions,
        "timeout_seconds_per_child": args.timeout, "norm_and_uncapped_l2_tolerance": TOLERANCE,
        "measurements": "One fresh process per row. Verification uses separate processes. "
                        "Policy order rotates by one case each measurement repetition.",
    }
    rows = []
    for width, budget in BUDGET_KIB.items():
        batch = list(cases(width))
        for rep in range(args.repetitions + 1):
            measure = rep < args.repetitions
            stage = "measure" if measure else "verify"
            shift = rep % len(batch) if measure else 0
            for backend, cap, variant in batch[shift:] + batch[:shift]:
                flags = ["mps_ladder", str(width), backend, variant, str(cap)]
                if backend == "mps-adaptive":
                    flags.append(str(budget))
                print(stage, *flags, file=sys.stderr, flush=True)
                result = run.child([str(binary), stage, *flags], args.timeout, measure, env)
                if measure:
                    if result.get("status") != "ok":
                        raise RuntimeError(result)
                    if backend == "mps-adaptive" and not (
                        result["estimated_peak_state_bytes"] <= result["planned_tensor_bound_bytes"]
                        <= result["tensor_budget_bytes"] == budget * 1024
                    ):
                        raise RuntimeError(f"retained tensor budget exceeded: {result}")
                else:
                    check_verification(result, width, cap, variant)
                rows.append({
                    "workload": flags[0], "qubits": width, "backend": backend, "variant": variant,
                    "bond_cap": cap, "adaptive_budget_kib": budget if backend == "mps-adaptive" else "",
                    "stage": stage, "repetition": rep + 1 if measure else "",
                    "theoretical_dense_f32_bytes": 8 << width, "theoretical_dense_f64_bytes": 16 << width,
                    **{k: json.dumps(v, separators=(",", ":")) if isinstance(v, list) else v
                       for k, v in result.items()},
                })
    if source_hashes() != hashes:
        raise RuntimeError("Source files changed during the comparison. Rerun with the final sources.")
    args.out.mkdir(parents=True, exist_ok=True)
    with (args.out / "mps-policy.csv").open("w", newline="") as out:
        columns = dict.fromkeys(key for row in rows for key in row)
        writer = csv.DictWriter(out, fieldnames=columns)
        writer.writeheader()
        writer.writerows(rows)
    metadata["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    (args.out / "mps-policy-metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")


if __name__ == "__main__":
    main()
