#!/usr/bin/env python3
import os
import glob
import csv

OUTPUT_FILE = "summary_perf.csv"

def parse_dir_metadata(dirname):
    # results-guest_cpu_hash.wasm-1-500
    base = os.path.basename(dirname)
    parts = base.split("-")
    if len(parts) < 4:
        return None
    return {
        "app_name": parts[1],
        "n_apps": int(parts[2]),
        "flows": int(parts[3]),
    }

def parse_perf_file(filepath):
    metrics = {}

    with open(filepath, "r") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue

            cols = line.split(",")
            if len(cols) < 3:
                continue

            value = cols[0].strip()
            metric = cols[2].strip()

            # Convert to integer safely
            try:
                metrics[metric] = int(value)
            except ValueError:
                continue

    return metrics

rows = []

for d in glob.glob("results-*"):
    perf_file = os.path.join(d, "perf_system.csv")
    if not os.path.exists(perf_file):
        continue

    meta = parse_dir_metadata(d)
    if not meta:
        continue

    perf = parse_perf_file(perf_file)

    cycles = perf.get("cycles", 0)
    instructions = perf.get("instructions", 0)
    cache_refs = perf.get("cache-references", 0)
    cache_misses = perf.get("cache-misses", 0)
    branches = perf.get("branches", 0)
    branch_misses = perf.get("branch-misses", 0)

    ipc = instructions / cycles if cycles > 0 else 0.0
    cache_miss_rate = cache_misses / cache_refs if cache_refs > 0 else 0.0
    branch_miss_rate = branch_misses / branches if branches > 0 else 0.0

    rows.append({
        **meta,
        "cycles": cycles,
        "instructions": instructions,
        "ipc": ipc,
        "cache_references": cache_refs,
        "cache_misses": cache_misses,
        "cache_miss_rate": cache_miss_rate,
        "branches": branches,
        "branch_misses": branch_misses,
        "branch_miss_rate": branch_miss_rate,
    })

# Write CSV
with open(OUTPUT_FILE, "w", newline="") as csvfile:
    fieldnames = [
        "app_name",
        "n_apps",
        "flows",
        "cycles",
        "instructions",
        "ipc",
        "cache_references",
        "cache_misses",
        "cache_miss_rate",
        "branches",
        "branch_misses",
        "branch_miss_rate",
    ]

    writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
    writer.writeheader()
    writer.writerows(rows)

print(f"Generated {OUTPUT_FILE} with {len(rows)} entries.")
