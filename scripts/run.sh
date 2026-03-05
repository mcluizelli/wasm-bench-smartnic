#!/usr/bin/env bash
set -euo pipefail

HOST_BIN="./host"

for appName in guest_cpu_hash.wasm guest_cpu_multi.wasm guest_net_checksum.wasm guest_net_scan.wasm; do
  for appNumber in $(seq 1 10); do
    echo "nApp: $appNumber appName: $appName"
    for flows in 50 100 500 1000; do
      sudo ./profile_wrk.sh \
        --host-cmd "$HOST_BIN ./$appName" \
        --wrk-cmd  "wrk -t8 -c$flows -d30s -s post.lua http://127.0.0.1:__PORT__" \
        --n-hosts "$appNumber" \
        --base-port 8080 \
        --out-dir "results-${appName}-${appNumber}-${flows}"
    done
  done
done



#for appName in guest_cpu_hash.wasm #guest_cpu_multi.wasm guest_net_checksum.wasm guest_net_scan.wasm


#sudo ./profile_wrk.sh   --host-cmd "./host ./guest_cpu_hash.wasm"   --wrk-cmd  "wrk -t8 -c500 -d30s -s post.lua http://127.0.0.1:__PORT__"   --n-hosts 4   --base-port 8080   --out-dir results_multi
