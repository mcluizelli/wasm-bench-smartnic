#!/usr/bin/env python3
import os
import re
import glob
import csv

OUTPUT_FILE = "summary_host_startup.csv"

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

def parse_startup_line(line: str):
    """
    Example:
    startup,addr=0.0.0.0:8080,wasm_path=./guest_cpu_hash.wasm,engine_create_us=129,...
    Returns dict of key->value (as strings).
    """
    line = line.strip()
    if not line.startswith("startup,"):
        return None

    parts = line.split(",")
    out = {"record_type": parts[0]}  # "startup"
    for p in parts[1:]:
        if "=" not in p:
            continue
        k, v = p.split("=", 1)
        out[k.strip()] = v.strip()
    return out

def to_int_safe(x):
    try:
        return int(x)
    except Exception:
        return ""

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

        # Read only the first non-empty line; expect it to be the startup line.
        first_line = None
        with open(metrics_path, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                if line.strip():
                    first_line = line
                    break

        if not first_line:
            continue

        startup = parse_startup_line(first_line)
        if not startup:
            continue

        # Normalize: keep the addr, and also store port from folder.
        # Convert *_us fields to integers when possible.
        row = {
            **meta,
            "port": port,
            "addr": startup.get("addr", ""),
            "wasm_path": startup.get("wasm_path", ""),
            "engine_create_us": to_int_safe(startup.get("engine_create_us", "")),
            "file_read_us": to_int_safe(startup.get("file_read_us", "")),
            "module_load_us": to_int_safe(startup.get("module_load_us", "")),
            "ready_to_accept_us": to_int_safe(startup.get("ready_to_accept_us", "")),
            "first_wasi_link_us": to_int_safe(startup.get("first_wasi_link_us", "")),
            "first_instantiate_us": to_int_safe(startup.get("first_instantiate_us", "")),
            "metrics_file": os.path.relpath(metrics_path, results_dir),
        }
        rows.append(row)

fieldnames = [
    "app_name",
    "n_apps",
    "flows",
    "port",
    "addr",
    "wasm_path",
    "engine_create_us",
    "file_read_us",
    "module_load_us",
    "ready_to_accept_us",
    "first_wasi_link_us",
    "first_instantiate_us",
    "metrics_file",
]

with open(OUTPUT_FILE, "w", newline="", encoding="utf-8") as out:
    w = csv.DictWriter(out, fieldnames=fieldnames)
    w.writeheader()
    w.writerows(rows)

print(f"Generated {OUTPUT_FILE} with {len(rows)} rows.")
