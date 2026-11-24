#!/usr/bin/env python3
"""Measure uniform bond caps on the 256-wire, eight-round, seed-53 MPS echo."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import csv
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import time

import run


TIMED_CAPS = (1, 16, 32, 64, 256)
VERIFY_CAPS = (255, 252, 240, 192, 128, 96, 224, 248)
TOLERANCE = 2e-10
FIELDS = ("bond_cap", "return_probability", "norm_error", "elapsed_ns",
          "peak_live_heap_above_baseline_bytes", "peak_rss_bytes", "estimated_peak_state_bytes")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_hashes(binary):
    core = run.ROOT / run.CORE
    files = {core / "Cargo.toml", core / "Cargo.lock", Path(__file__).resolve(),
             Path(run.__file__).resolve()}
    # Cargo's dep-info lists inputs to the default-feature build.
    for item in shlex.split(binary.with_suffix(".d").read_text().split(": ", 1)[1]):
        path = Path(item).resolve()
        if path.is_file() and path.is_relative_to(core):
            files.add(path)
    return {str(path.relative_to(run.ROOT)): digest(path) for path in sorted(files)}


def verify(binary, cap, timeout, env):
    args = [str(binary), "verify", "mps_ladder", "256", "mps-paired", "reverse", str(cap)]
    with tempfile.TemporaryFile() as output, subprocess.Popen(
        args, stdout=output, stderr=subprocess.STDOUT, env=env, cwd=run.ROOT,
    ) as proc:
        deadline = time.monotonic() + timeout
        timed_out = False
        while True:
            pid, status, usage = os.wait4(proc.pid, os.WNOHANG)
            if pid:
                break
            if time.monotonic() >= deadline:
                proc.kill()
                _, status, usage = os.wait4(proc.pid, 0)
                timed_out = True
                break
            time.sleep(0.025)
        proc.returncode = os.waitstatus_to_exitcode(status)
        stats = {"cpu_seconds": usage.ru_utime + usage.ru_stime,
                 "peak_rss_bytes": usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024)}
        if timed_out:
            return None, {**stats, "status": "timeout", "timeout_seconds": timeout}
        output.seek(0)
        if proc.returncode:
            raise RuntimeError(f"cap {cap} verify: {output.read().decode()}")
        result = json.load(output)
    if (result.get("validation_status") != "approximation_measured"
            or not 0 <= result["norm_error"] <= TOLERANCE
            or not 0 <= result["return_probability"] <= 1 + TOLERANCE):
        raise RuntimeError(f"cap {cap} verification failed: {result}")
    return {"bond_cap": cap, "return_probability": result["return_probability"],
            "norm_error": result["norm_error"], "peak_rss_bytes": stats["peak_rss_bytes"]}, {
                **stats, "status": "ok"}


def sweep(binary, timeout, workers, env):
    rows = []
    resources = {}
    # Only numerical checks run concurrently. CPU time and RSS belong to each child.
    with ThreadPoolExecutor(max_workers=workers) as pool:
        jobs = {pool.submit(verify, binary, cap, timeout, env): cap for cap in VERIFY_CAPS}
        for job in as_completed(jobs):
            cap = jobs[job]
            row, resources[cap] = job.result()
            if row:
                rows.append(row)
            print(json.dumps(row or {"bond_cap": cap, **resources[cap]}), file=sys.stderr, flush=True)

    # Isolate timed children from every other child in this sweep.
    for cap in TIMED_CAPS:
        flags = ["mps_ladder", "256", "mps-paired", "reverse", str(cap)]
        measure = run.child([str(binary), "measure", *flags], timeout, True, env)
        if measure.get("status") != "ok" or measure["gates"] != 5624 or measure["depth"] != 32:
            raise RuntimeError(f"cap {cap} measurement failed: {measure}")
        row, resources[cap] = verify(binary, cap, timeout, env)
        if row is None:
            raise RuntimeError(f"cap {cap} verification timed out")
        row.update({field: measure[field] for field in FIELDS[3:]})
        rows.append(row)
        print(json.dumps(row), file=sys.stderr, flush=True)
    return sorted(rows, key=lambda row: row["bond_cap"]), resources


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--out", type=Path, help="write CSV and metadata here instead of updating the UI data")
    parser.add_argument("--timeout", type=int, default=480, help="wall-time budget in seconds per child")
    parser.add_argument("--jobs", type=int, default=8, help="concurrent verification processes")
    args = parser.parse_args()
    if args.timeout <= 0 or args.jobs <= 0:
        parser.error("--timeout and --jobs must be positive")
    if sys.platform not in {"darwin", "linux"}:
        parser.error("RSS collection requires macOS or Linux")

    target = run.ROOT / "target/audit"
    env = {**os.environ, "RAYON_NUM_THREADS": "1", "CARGO_TARGET_DIR": str(target)}
    build = ["cargo", "build", "--locked", "--release", "--manifest-path",
             str(run.CORE / "Cargo.toml"), "--example", "audit_benchmark"]
    if args.offline:
        build.insert(2, "--offline")
    subprocess.run(build, cwd=run.ROOT, env=env, check=True)
    binary = target / "release/examples/audit_benchmark"
    hashes = source_hashes(binary)
    metadata = {
        "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "git_commit": run.command("git", "rev-parse", "HEAD"),
        "source_sha256": hashes, "binary_sha256": digest(binary),
        "hardware": run.hardware(), "rustc": run.command("rustc", "-vV"),
        "cargo": run.command("cargo", "-V"), "cargo_build": build,
        "environment": {k: env.get(k) for k in
                        ("RAYON_NUM_THREADS", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTUP_TOOLCHAIN")},
        "workload": "mps_ladder", "qubits": 256, "seed": 53, "rounds": 8,
        "backend": "mps-paired", "variant": "reverse", "gates": 5624, "depth": 32,
        "norm_tolerance": TOLERANCE, "timeout_seconds_per_child": args.timeout,
        "verification_workers": args.jobs, "isolated_timing_caps": TIMED_CAPS,
        "parallel_verification_caps": VERIFY_CAPS,
        "return_probability": "Raw squared all-zero amplitude after forward evolution and its adjoint.",
        "sampling": "One fresh process per verification. Timed caps also get one isolated timing child. "
                    "RSS is the timing child's peak when timed, otherwise the verification child's peak. "
                    "Blank CSV timing, allocator and tensor cells indicate verification only. "
                    "Timed-out checks are recorded here and omitted from the CSV.",
    }
    with tempfile.TemporaryDirectory(dir=target) as temp:
        pinned = Path(temp) / binary.name
        shutil.copy2(binary, pinned)
        if digest(pinned) != metadata["binary_sha256"]:
            raise RuntimeError("binary changed while preparing sweep")
        rows, resources = sweep(pinned, args.timeout, args.jobs, env)
    if source_hashes(binary) != hashes:
        raise RuntimeError("benchmark inputs changed during the sweep")

    csv_path = (args.out or run.ROOT / "crates/quantsim_ui/assets/benchmarks") / "mps-cap-curve.csv"
    metadata_path = (args.out or run.HERE) / "mps-cap-curve.json"
    csv_path.parent.mkdir(parents=True, exist_ok=True)
    with csv_path.open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)
    metadata["verification_resources_by_cap"] = dict(sorted(resources.items()))
    metadata["csv_sha256"] = digest(csv_path)
    metadata["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    metadata_path.write_text(json.dumps(metadata, indent=2) + "\n")


if __name__ == "__main__":
    main()
