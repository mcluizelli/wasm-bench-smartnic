# wasm-bench-smartnic

WebAssembly Application Benchmarking on SmartNICs

This repository contains a lightweight benchmarking framework to evaluate WebAssembly applications running on SmartNIC-attached servers. The system consists of two main components: (i) guest applications compiled to WebAssembly (WASI), which represent different workload profiles, and (ii) a host runtime that loads and executes the WebAssembly modules through Wasmtime while exposing an HTTP interface for benchmarking tools such as `wrk`.

---

## Repository Structure

```
apps/
 ├── guest_*/        # Guest applications (CPU, memory, network workloads)
 └── host/           # Rust HTTP host that loads and executes WASM modules
scripts/             # Benchmark automation and profiling scripts
results/             # Experiment output directories
```

The **guest applications** implement synthetic workloads designed to stress specific system resources such as CPU, memory, or networking. The **host application** runs a WebAssembly runtime and exposes the guest function through an HTTP endpoint, allowing load generators to invoke the guest code repeatedly.

---

## Compiling Guest Applications

Each guest application must be compiled to the **WASI WebAssembly target** and optionally ahead-of-time compiled to a `cwasm` module using Wasmtime.

1. Navigate to the guest application directory:

```bash
cd ./apps/guest_*/
```

2. Install the WASI target (only required once):

```bash
rustup target add wasm32-wasip1
```

3. Compile the guest application:

```bash
cargo build --release --target wasm32-wasip1
```

4. Optionally produce a precompiled Wasmtime module (`.cwasm`):

```bash
wasmtime compile target/wasm32-wasip1/release/guest.wasm -o guest.cwasm
```

The `.cwasm` artifact avoids runtime compilation overhead and reduces startup latency during experiments.

---

## Compiling the Host Application

The host application is responsible for loading the WebAssembly module and exposing it through an HTTP server.

1. Move to the host directory:

```bash
cd ./apps/host
```

2. Clean previous builds (optional but recommended):

```bash
cargo clean
```

3. Compile the host:

```bash
cargo build --release
```

The resulting binary will be generated at:

```
./target/release/host
```

---

## Running the Host

To start the host runtime and load a guest application:

```bash
./target/release/host ../guest_cpu_hash.wasm 8080
```

This command launches the HTTP server and loads the specified WebAssembly module on port `8080`.

Once running, the host can be benchmarked using tools such as `wrk` or other HTTP load generators.

---

## Benchmarking

The repository includes scripts to automate experiments and collect system metrics such as:

* CPU and memory utilization (`sar`, `pidstat`)
* Hardware counters (`perf`)
* Request latency and throughput (`wrk`)
* Wasmtime execution metrics (startup, instantiation, execution times)

These scripts generate structured result directories that can later be aggregated and analyzed.

---

## Requirements

* Rust toolchain (`cargo`, `rustup`)
* Wasmtime CLI
* Linux profiling tools (`perf`, `sar`, `pidstat`)
* `wrk` for HTTP load generation

---

## License

This repository is intended for research and experimental benchmarking of WebAssembly workloads on SmartNIC-enabled infrastructures.
