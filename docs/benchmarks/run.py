#!/usr/bin/env python3
"""Build and run the native audit, writing raw repetitions and provenance."""
import argparse
import csv
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import subprocess as sp
import sys
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
CORE = Path("crates/quantsim_core")
MPS_PATH = CORE / "examples/sycamore_mps"
WIDTHS = (8, 16, 24, 32, 50, 64, 128, 256)
STRUCTURED = (*WIDTHS, 512, 1024)
CAPS = (2, 4, 8, 16, 32)
COLUMNS = (
    "workload variant qubits backend stage repetition bond_cap status detail "
    "gates depth authored_columns input_prep_gates elapsed_ns peak_live_heap_bytes "
    "pre_run_live_heap_bytes peak_live_heap_above_baseline_bytes process_peak_live_heap_bytes peak_rss_bytes "
    "estimated_peak_state_bytes theoretical_dense_f32_bytes theoretical_dense_f64_bytes "
    "largest_dense_subsystem final_largest_factor peak_bond discarded_weight_sum "
    "validation_status dense_reference fidelity return_probability norm_error "
    "max_abs_amplitude_error l2_amplitude_error analytical_probe_count "
    "max_scaled_analytical_error validated_peak_factor_width"
).split()


def cases():
    for n in STRUCTURED:
        for family, backend, inverse in (
            ("product", "product", "reverse"),
            ("ghz", "tableau", "reverse"),
            ("groups", "grouped", "reverse"),
            ("qft", "product", "decode"),
        ):
            for variant in ("forward", inverse):
                yield family, n, backend, variant, 0, ""
        if n <= 16:
            for family in ("product", "ghz", "groups", "qft"):
                for variant in ("forward", "reverse"):
                    yield family, n, "dense", variant, 0, ""
            for variant in ("forward", "reverse"):
                yield "ghz", n, "grouped", variant, 0, ""
        if n in WIDTHS:
            limit = "" if n <= 24 else "dense-memory/runtime budget"
            for variant in ("forward", "reverse"):
                yield "non_clifford", n, "grouped", variant, 0, limit
                yield "non_clifford", n, "dense", variant, 0, limit
    for n in (*WIDTHS[:5], 53, *WIDTHS[5:]):
        for variant in ("phase_flip", "no_flip"):
            skip = "" if n <= 53 else "fixed 53-site template has no wider instance"
            yield "sycamore_echo", n, "tableau", variant, 0, skip
            if n <= 16:
                yield "sycamore_echo", n, "dense", variant, 0, ""
    for n in WIDTHS:
        if n <= 16:
            yield "mps_ladder", n, "dense", "forward", 0, ""
        extra_caps = (0, 64, 128) if n == 16 else (0,) if n == 8 else ()
        for cap in (*CAPS, *extra_caps):
            for variant in ("forward", "reverse"):
                yield "mps_ladder", n, "mps", variant, cap, ""


def command(*args):
    return sp.check_output(args, cwd=ROOT, text=True).strip()


def snapshot(out):
    harness = [
        CORE / "examples/audit_benchmark.rs",
        *(path.relative_to(ROOT) for path in sorted((ROOT / CORE / "examples/audit_benchmark").glob("*.rs"))),
        *(MPS_PATH / name for name in ("chain.rs", "layers.rs", "budget.rs", "planning.rs")),
        Path(__file__).resolve().relative_to(ROOT),
    ]
    files = [
        *sorted((ROOT / CORE / "src").rglob("*.rs")),
        ROOT / CORE / "circuits/sycamore_echo_53.json",
        ROOT / CORE / "build.rs",
        ROOT / CORE / "Cargo.lock",
        ROOT / CORE / "Cargo.toml",
        *(ROOT / path for path in harness),
    ]
    inputs = [p.relative_to(ROOT) for p in files]
    diff = sp.check_output(["git", "diff", "--binary", "HEAD", "--", *inputs], cwd=ROOT)
    untracked = sp.check_output(["git", "ls-files", "-z", "--others", "--", *inputs], cwd=ROOT)
    for path in filter(None, os.fsdecode(untracked).split("\0")):
        patch = sp.run(["git", "diff", "--no-index", "--binary", "--", "/dev/null", path],
                       cwd=ROOT, capture_output=True)
        if patch.returncode != 1:
            raise RuntimeError(patch.stderr.decode())
        diff += patch.stdout
    (out / "audited-source.patch").write_bytes(diff)
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}


def hardware():
    data = {"machine": platform.machine(), "system": platform.system(), "kernel": platform.release()}
    if sys.platform == "darwin":
        # Capture only relevant hardware fields.
        source = command("system_profiler", "SPHardwareDataType").splitlines()
        keys = {"Model Name", "Model Identifier", "Chip", "Total Number of Cores", "Memory"}
        data.update(dict(line.strip().split(": ", 1) for line in source
                         if line.strip().split(": ", 1)[0] in keys))
        data["os_version"] = command("sw_vers", "-productVersion")
        data["os_build"] = command("sw_vers", "-buildVersion")
    elif sys.platform == "linux":
        data["cpu"] = next((line.split(":", 1)[1].strip() for line in
                            Path("/proc/cpuinfo").read_text().splitlines()
                            if line.startswith("model name")), platform.processor())
        data["memory"] = Path("/proc/meminfo").read_text().splitlines()[0]
        data["logical_cpus"] = os.cpu_count()
    return data


def child(args, timeout, measure, env):
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        with sp.Popen(args, cwd=ROOT, env=env, stdout=stdout, stderr=stderr,
                      start_new_session=True) as process:
            deadline = time.monotonic() + timeout
            while True:
                pid, status, usage = os.wait4(process.pid, os.WNOHANG)
                if pid:
                    process.returncode = os.waitstatus_to_exitcode(status)
                    break
                if time.monotonic() >= deadline:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                    return {"status": "timeout", "detail": f"child exceeded {timeout} seconds"}
                time.sleep(0.002)
            if process.returncode:
                stderr.seek(0)
                return {"status": "process_error", "detail": stderr.read().decode().strip()}
            stdout.seek(0)
            data = json.load(stdout)
    if measure and data["status"] == "ok":
        # Darwin reports bytes, Linux reports KiB.
        data["peak_rss_bytes"] = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024)
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=HERE)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--workload", action="append", help="restrict to these families")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    if args.repetitions < 1 or args.timeout < 1:
        parser.error("repetitions and timeout must be positive")
    if sys.platform not in {"linux", "darwin"}:
        parser.error("peak RSS collection supports macOS and Linux")
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "RAYON_NUM_THREADS": "1", "CARGO_TARGET_DIR": str(ROOT / "target/audit")}
    build = ["cargo", "build", "--locked", "--release", "--manifest-path",
             str(CORE / "Cargo.toml"), "--example", "audit_benchmark"]
    if args.offline:
        build.insert(2, "--offline")
    sp.run(build, cwd=ROOT, env=env, check=True)
    binary = str(Path(env["CARGO_TARGET_DIR"]) / "release/examples/audit_benchmark")
    metadata = {
        "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "git_commit": command("git", "rev-parse", "HEAD"),
        "hardware": hardware(),
        "rustc": command("rustc", "-vV"),
        "cargo": command("cargo", "-V"),
        "cargo_build": build,
        "environment": {k: env.get(k) for k in ("RAYON_NUM_THREADS", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTUP_TOOLCHAIN")},
        "repetitions": args.repetitions,
        "timeout_seconds_per_child": args.timeout,
        "workload_filter": args.workload,
        "ladder_seed": 53,
        "ladder_rounds": 8,
        "validation_l2_and_norm_tolerances": {"dense": 3e-5, "grouped": 2e-7, "compact_other_and_mps": 2e-10},
        "sha256": snapshot(out),
    }
    (out / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    issues = child([binary, "issues"], args.timeout, False, env)
    (out / "correctness-probes.json").write_text(json.dumps(issues, indent=2) + "\n")
    if issues.get("status"):
        raise RuntimeError(issues)
    with (out / "benchmark.csv").open("w", newline="") as file:
        writer = csv.DictWriter(file, COLUMNS)
        writer.writeheader()
        for family, n, backend, variant, cap, skip in cases():
            if args.workload and family not in args.workload:
                continue
            label = f"{family} {n} {backend} {variant} cap={cap}"
            print(label, file=sys.stderr, flush=True)
            row = dict(workload=family, qubits=n, backend=backend, variant=variant,
                       bond_cap=cap if backend == "mps" else "",
                       theoretical_dense_f32_bytes=8 << n, theoretical_dense_f64_bytes=16 << n)
            if skip:
                writer.writerow({**row, "stage": "measure", "status": "skipped", "detail": skip})
                continue
            flags = [family, str(n), backend, variant, str(cap)]
            success = True
            for rep in range(1, args.repetitions + 1):
                measurement = child([binary, "measure", *flags], args.timeout, True, env)
                writer.writerow({**row, "stage": "measure", "repetition": rep, **measurement})
                file.flush()
                if measurement["status"] != "ok":
                    success = False
                    print(measurement, file=sys.stderr)
                    break
            if success:
                result = child([binary, "verify", *flags], args.timeout, False, env)
                writer.writerow({**row, "stage": "verify", "status": "ok", **result})
                file.flush()
                if result.get("validation_status") in {None, "failed"}:
                    print(result, file=sys.stderr)


if __name__ == "__main__":
    main()
