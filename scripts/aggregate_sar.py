#!/usr/bin/env python3
import os
import csv
import glob

OUTPUT_FILE = "summary_cpu.csv"

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

def parse_sar_file(filepath):
    metrics = {
        "%usr": [],
        "%sys": [],
        "%soft": [],
        "%idle": [],
    }

    with open(filepath, "r") as f:
        for line in f:
            line = line.strip()
            if not line or "CPU" in line or "Linux" in line:
                continue

            cols = line.split()
            if len(cols) < 11:
                continue

            if cols[1] != "all":
                continue

            # Replace comma decimal separator
            usr = float(cols[2].replace(",", "."))
            sys = float(cols[4].replace(",", "."))
            soft = float(cols[8].replace(",", "."))
            idle = float(cols[10].replace(",", "."))

            metrics["%usr"].append(usr)
            metrics["%sys"].append(sys)
            metrics["%soft"].append(soft)
            metrics["%idle"].append(idle)

    return {k: (sum(v)/len(v) if v else 0.0) for k, v in metrics.items()}

rows = []

for d in glob.glob("results-*"):
    sar_file = os.path.join(d, "sar_cpu.txt")
    if not os.path.exists(sar_file):
        continue

    meta = parse_dir_metadata(d)
    if not meta:
        continue

    cpu_avg = parse_sar_file(sar_file)

    rows.append({
        **meta,
        **cpu_avg
    })

# Write CSV
with open(OUTPUT_FILE, "w", newline="") as csvfile:
    fieldnames = ["app_name", "n_apps", "flows", "%usr", "%sys", "%soft", "%idle"]
    writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
    writer.writeheader()
    writer.writerows(rows)

print(f"Generated {OUTPUT_FILE} with {len(rows)} entries.")
