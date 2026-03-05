#!/usr/bin/env python3
import os
import re
import glob
import csv

OUTPUT_FILE = "summary_wrk.csv"

DIR_RE = re.compile(r"^results-(.+)-(\d+)-(\d+)$")
PORT_RE = re.compile(r"^wrk\.port-(\d+)\.out\.txt$")

LAT_RE = re.compile(r"^\s*Latency\s+(\S+)", re.IGNORECASE)
RPS_RE = re.compile(r"^\s*Requests/sec:\s*([0-9]+(?:\.[0-9]+)?)", re.IGNORECASE)

def parse_dir_metadata(dirname: str):
    base = os.path.basename(dirname.rstrip("/"))
    m = DIR_RE.match(base)
    if not m:
        return None
    return {
        "app_name": m.group(1),
        "n_apps": int(m.group(2)),
        "flows": int(m.group(3)),
    }

def latency_to_ms(token: str) -> float:
    """
    token examples: '12.83ms', '468.81us', '1.23s'
    """
    token = token.strip()
    m = re.match(r"^([0-9]+(?:\.[0-9]+)?)([a-zA-Z]+)$", token)
    if not m:
        raise ValueError(f"Unrecognized latency token: {token}")
    val = float(m.group(1))
    unit = m.group(2).lower()

    if unit == "ms":
        return val
    if unit in ("us", "µs"):
        return val / 1000.0
    if unit == "s":
        return val * 1000.0

    # Some wrk builds may show 'm' or other variants; fail loudly.
    raise ValueError(f"Unrecognized latency unit: {unit}")

def parse_wrk_out(path: str):
    latency_avg_ms = None
    req_per_sec = None

    with open(path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            if latency_avg_ms is None:
                m = LAT_RE.match(line)
                if m:
                    latency_avg_ms = latency_to_ms(m.group(1))
                    continue
            if req_per_sec is None:
                m = RPS_RE.match(line)
                if m:
                    req_per_sec = float(m.group(1))
                    continue

    return latency_avg_ms, req_per_sec

rows = []

for d in sorted(glob.glob("results-*")):
    if not os.path.isdir(d):
        continue

    meta = parse_dir_metadata(d)
    if not meta:
        continue

    # Find all wrk outputs for this directory (can be multiple ports)
    for fname in os.listdir(d):
        pm = PORT_RE.match(fname)
        if not pm:
            continue

        port = int(pm.group(1))
        wrk_path = os.path.join(d, fname)

        try:
            lat_ms, rps = parse_wrk_out(wrk_path)
        except Exception as e:
            # Keep a row with blanks but record error if desired
            lat_ms, rps = None, None

        rows.append({
            **meta,
            "port": port,
            "latency_avg_ms": "" if lat_ms is None else f"{lat_ms:.6f}",
            "req_per_sec": "" if rps is None else f"{rps:.6f}",
            "wrk_file": fname,
        })

# Write CSV
fieldnames = ["app_name", "n_apps", "flows", "port", "latency_avg_ms", "req_per_sec", "wrk_file"]
with open(OUTPUT_FILE, "w", newline="", encoding="utf-8") as out:
    w = csv.DictWriter(out, fieldnames=fieldnames)
    w.writeheader()
    w.writerows(rows)

print(f"Generated {OUTPUT_FILE} with {len(rows)} rows.")
