#!/usr/bin/env python3
import os
import re
import glob
import csv
from collections import defaultdict

OUTPUT_FILE = "summary_host_requests.csv"

# results-guest_cpu_hash.wasm-1-500
DIR_RE = re.compile(r"^results-(.+)-(\d+)-(\d+)$")
# hosts/port-8080
PORTDIR_RE = re.compile(r"^port-(\d+)$")

def parse_dir_metadata(results_dir: str):
    base = os.path.basename(results_dir.rstrip("/"))
    m = DIR_RE.match(base)
    if not m:
        return None
    return {
        "app_name": m.group(1),
        "n_apps": int(m.group(2)),
        "flows": int(m.group(3)),
    }

def parse_kv_line(line: str, prefix: str):
    """
    Parses lines like:
      request,addr=...,idx=...,instantiate_total_us=...,total_us=...
    Returns dict or None.
    """
    line = line.strip()
    if not line.startswith(prefix + ","):
        return None
    parts = line.split(",")
    out = {"record_type": parts[0]}
    for p in parts[1:]:
        if "=" not in p:
            continue
        k, v = p.split("=", 1)
        out[k.strip()] = v.strip()
    return out

def to_int(x):
    try:
        return int(x)
    except Exception:
        return None

def percentile(sorted_vals, q):
    # q in [0,100]
    if not sorted_vals:
        return None
    if len(sorted_vals) == 1:
        return sorted_vals[0]
    k = (len(sorted_vals) - 1) * (q / 100.0)
    f = int(k)
    c = min(f + 1, len(sorted_vals) - 1)
    if f == c:
        return sorted_vals[f]
    d0 = sorted_vals[f] * (c - k)
    d1 = sorted_vals[c] * (k - f)
    return d0 + d1

def mean(vals):
    return (sum(vals) / len(vals)) if vals else None

def fmt(x):
    if x is None:
        return ""
    if isinstance(x, float):
        return f"{x:.6f}"
    return str(x)

rows = []

for results_dir in sorted(glob.glob("results-*")):
    if not os.path.isdir(results_dir):
        continue

    meta = parse_dir_metadata(results_dir)
    if not meta:
        continue

    hosts_dir = os.path.join(results_dir, "hosts")
    if not os.path.isdir(hosts_dir):
        continue

    for port_dir in sorted(glob.glob(os.path.join(hosts_dir, "port-*"))):
        if not os.path.isdir(port_dir):
            continue

        port_base = os.path.basename(port_dir)
        pm = PORTDIR_RE.match(port_base)
        if not pm:
            continue
        port = int(pm.group(1))

        metrics_path = os.path.join(port_dir, "host_metrics.log")
        if not os.path.exists(metrics_path):
            continue

        # Collect per-request timings
        cols = {
            "instantiate_total_us": [],
            "alloc_us": [],
            "write_us": [],
            "handle_us": [],
            "read_us": [],
            "dealloc_us": [],
            "total_us": [],
        }

        addr = ""
        n_lines = 0

        with open(metrics_path, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                rec = parse_kv_line(line, "request")
                if not rec:
                    continue

                n_lines += 1
                if not addr:
                    addr = rec.get("addr", "")

                for k in cols.keys():
                    v = to_int(rec.get(k, ""))
                    if v is not None:
                        cols[k].append(v)

        if n_lines == 0:
            continue

        # Summaries: count, mean, p50, p95, p99 (all in microseconds)
        row = {
            **meta,
            "port": port,
            "addr": addr,
            "n_requests_measured": len(cols["total_us"]),
            "metrics_file": os.path.relpath(metrics_path, results_dir),
        }

        for k, vals in cols.items():
            vals_sorted = sorted(vals)
            row[f"{k}_mean"] = mean(vals)
            row[f"{k}_p50"] = percentile(vals_sorted, 50)
            row[f"{k}_p95"] = percentile(vals_sorted, 95)
            row[f"{k}_p99"] = percentile(vals_sorted, 99)

        rows.append(row)

fieldnames = [
    "app_name",
    "n_apps",
    "flows",
    "port",
    "addr",
    "n_requests_measured",

    "instantiate_total_us_mean", "instantiate_total_us_p50", "instantiate_total_us_p95", "instantiate_total_us_p99",
    "alloc_us_mean",             "alloc_us_p50",             "alloc_us_p95",             "alloc_us_p99",
    "write_us_mean",             "write_us_p50",             "write_us_p95",             "write_us_p99",
    "handle_us_mean",            "handle_us_p50",            "handle_us_p95",            "handle_us_p99",
    "read_us_mean",              "read_us_p50",              "read_us_p95",              "read_us_p99",
    "dealloc_us_mean",           "dealloc_us_p50",           "dealloc_us_p95",           "dealloc_us_p99",
    "total_us_mean",             "total_us_p50",             "total_us_p95",             "total_us_p99",

    "metrics_file",
]

with open(OUTPUT_FILE, "w", newline="", encoding="utf-8") as out:
    w = csv.DictWriter(out, fieldnames=fieldnames)
    w.writeheader()
    for r in rows:
        # format floats nicely
        out_row = {k: fmt(v) for k, v in r.items()}
        w.writerow(out_row)

print(f"Generated {OUTPUT_FILE} with {len(rows)} rows.")
