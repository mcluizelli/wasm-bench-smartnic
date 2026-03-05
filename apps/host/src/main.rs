// host/src/main.rs
//
// This version adds precise timing for:
//   - Engine creation
//   - Module load (wasm) or file read + deserialize (cwasm)
//   - WASI link time
//   - Instantiate time
//   - Per-request "invoke" time broken down into:
//       instantiate + alloc + write + handle + read + dealloc
//   - Startup "ready-to-accept" time
//
// Controls (env vars):
//   MEASURE_STARTUP=1        print startup timing once
//   MEASURE_PER_REQUEST=1    print per-request timing (WARNING: very verbose under wrk)
//   MEASURE_EVERY=N          print per-request timing every N requests (default: 1)
//   MEASURE_WARMUP=N         skip first N requests before printing (default: 0)
//
// Example:
//   MEASURE_STARTUP=1 MEASURE_PER_REQUEST=1 MEASURE_EVERY=1000 ./target/release/host ../guest.cwasm 8080
//
// Change in this file:
//   - Accept optional CLI port: host <guest.(wasm|cwasm)> [port]
//   - Include addr (port) in startup + request measurement lines

use anyhow::{Context, Result};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use wasmtime::{Config, Engine, Instance, Linker, Memory, Module, Store, TypedFunc};

// WASI Preview 1 support (wasi_snapshot_preview1)
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::WasiCtxBuilder;

type RespBody = Full<Bytes>;
type HttpResponse = Response<RespBody>;

#[derive(Clone, Default)]
struct StartupTimings {
    engine_create: Duration,
    file_read: Duration,   // only for .cwasm
    module_load: Duration, // from_file or deserialize
}

#[derive(Clone, Default)]
struct InstantiateTimings {
    wasi_link: Duration,
    instantiate: Duration,
}

#[derive(Clone, Default)]
struct InvokeTimings {
    instantiate_total: Duration, // includes wasi_link + instantiate + export lookups (roughly)
    alloc: Duration,
    write: Duration,
    handle: Duration,
    read: Duration,
    dealloc: Duration,
    total: Duration,
}

fn env_bool(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn dur_us(d: Duration) -> u128 {
    d.as_micros()
}

struct WasmHandler {
    engine: Engine,
    module: Module,
    startup: StartupTimings,
    measure_startup: bool,
    measure_per_request: bool,
    measure_every: u64,
    warmup: u64,
    req_counter: AtomicU64,
}

impl WasmHandler {
    fn new(path: &str) -> Result<Self> {
        let measure_startup = env_bool("MEASURE_STARTUP");
        let measure_per_request = env_bool("MEASURE_PER_REQUEST");
        let measure_every = env_u64("MEASURE_EVERY", 1).max(1);
        let warmup = env_u64("MEASURE_WARMUP", 0);

        let mut startup = StartupTimings::default();

        let t0 = Instant::now();
        let mut config = Config::new();
        // Required because your .cwasm was compiled with component-model support enabled.
        config.wasm_component_model(true);

        let engine = Engine::new(&config)
            .map_err(|e| anyhow::anyhow!("failed to create Wasmtime Engine: {e}"))?;
        startup.engine_create = t0.elapsed();

        let p = Path::new(path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");

        let module = match ext {
            "wasm" => {
                let t = Instant::now();
                let m = Module::from_file(&engine, path)
                    .map_err(|e| anyhow::anyhow!("failed to load wasm module from {path}: {e}"))?;
                startup.module_load = t.elapsed();
                m
            }

            "cwasm" => {
                let t_read = Instant::now();
                let bytes = fs::read(path).with_context(|| format!("failed to read {path}"))?;
                startup.file_read = t_read.elapsed();

                let t_deser = Instant::now();
                let m = unsafe { Module::deserialize(&engine, &bytes) }.map_err(|e| {
                    anyhow::anyhow!("failed to deserialize precompiled module {path}: {e}")
                })?;
                startup.module_load = t_deser.elapsed();
                m
            }

            _ => anyhow::bail!("unknown module extension '{ext}', expected .wasm or .cwasm"),
        };

        Ok(Self {
            engine,
            module,
            startup,
            measure_startup,
            measure_per_request,
            measure_every,
            warmup,
            req_counter: AtomicU64::new(0),
        })
    }

    fn instantiate_with_timing(
        &self,
    ) -> Result<(
        (
            Store<WasiP1Ctx>,
            Instance,
            Memory,
            TypedFunc<i32, i32>,        // alloc(len) -> ptr
            TypedFunc<(i32, i32), ()>,  // dealloc(ptr, len)
            TypedFunc<(i32, i32), i32>, // handle(ptr, len) -> rc
        ),
        InstantiateTimings,
    )> {
        let mut it = InstantiateTimings::default();

        let wasi: WasiP1Ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_env()
            .build_p1();

        let mut store: Store<WasiP1Ctx> = Store::new(&self.engine, wasi);
        let mut linker: Linker<WasiP1Ctx> = Linker::new(&self.engine);

        let t_link = Instant::now();
        p1::add_to_linker_sync(&mut linker, |cx: &mut WasiP1Ctx| cx)
            .map_err(|e| anyhow::anyhow!("failed to add WASIp1 to linker: {e}"))?;
        it.wasi_link = t_link.elapsed();

        let t_inst = Instant::now();
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| anyhow::anyhow!("failed to instantiate module: {e}"))?;
        it.instantiate = t_inst.elapsed();

        let memory = instance
            .get_memory(&mut store, "memory")
            .context("guest module must export memory")?;

        let alloc = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|e| anyhow::anyhow!("guest must export alloc(len)->ptr: {e}"))?;

        let dealloc = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")
            .map_err(|e| anyhow::anyhow!("guest must export dealloc(ptr,len): {e}"))?;

        let handle = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "handle")
            .map_err(|e| anyhow::anyhow!("guest must export handle(ptr,len)->i32: {e}"))?;

        Ok(((store, instance, memory, alloc, dealloc, handle), it))
    }

    fn call_with_timing(&self, input: &[u8]) -> Result<(Vec<u8>, InvokeTimings)> {
        let mut t = InvokeTimings::default();
        let t_total = Instant::now();

        let t_inst_total = Instant::now();
        let ((mut store, _inst, memory, alloc, dealloc, handle), _it) =
            self.instantiate_with_timing()?;
        t.instantiate_total = t_inst_total.elapsed();

        let len_i32 = i32::try_from(input.len()).context("request too large")?;

        let t_alloc = Instant::now();
        let ptr_i32 = alloc
            .call(&mut store, len_i32)
            .map_err(|e| anyhow::anyhow!("alloc failed: {e}"))?;
        t.alloc = t_alloc.elapsed();

        let ptr = ptr_i32 as usize;

        let t_write = Instant::now();
        memory
            .write(&mut store, ptr, input)
            .map_err(|e| anyhow::anyhow!("memory.write failed: {e}"))?;
        t.write = t_write.elapsed();

        let t_handle = Instant::now();
        let rc = handle
            .call(&mut store, (ptr_i32, len_i32))
            .map_err(|e| anyhow::anyhow!("handle call failed: {e}"))?;
        t.handle = t_handle.elapsed();

        if rc != 0 {
            let _ = dealloc.call(&mut store, (ptr_i32, len_i32));
            anyhow::bail!("guest returned error code {rc}");
        }

        let mut out = vec![0u8; input.len()];
        let t_read = Instant::now();
        memory
            .read(&mut store, ptr, &mut out)
            .map_err(|e| anyhow::anyhow!("memory.read failed: {e}"))?;
        t.read = t_read.elapsed();

        let t_dealloc = Instant::now();
        dealloc
            .call(&mut store, (ptr_i32, len_i32))
            .map_err(|e| anyhow::anyhow!("dealloc failed: {e}"))?;
        t.dealloc = t_dealloc.elapsed();

        t.total = t_total.elapsed();
        Ok((out, t))
    }

    fn maybe_print_startup(
        &self,
        addr: &str,
        wasm_path: &str,
        ready: Duration,
        first_inst: &InstantiateTimings,
    ) {
        if !self.measure_startup {
            return;
        }

        eprintln!(
            "startup,addr={},wasm_path={},engine_create_us={},file_read_us={},module_load_us={},ready_to_accept_us={},first_wasi_link_us={},first_instantiate_us={}",
            addr,
            wasm_path,
            dur_us(self.startup.engine_create),
            dur_us(self.startup.file_read),
            dur_us(self.startup.module_load),
            dur_us(ready),
            dur_us(first_inst.wasi_link),
            dur_us(first_inst.instantiate),
        );
    }

    fn maybe_print_per_request(&self, addr: &str, inv: &InvokeTimings) {
        if !self.measure_per_request {
            return;
        }

        let n = self.req_counter.fetch_add(1, Ordering::Relaxed) + 1;
        if n <= self.warmup {
            return;
        }
        if (n - self.warmup) % self.measure_every != 0 {
            return;
        }

        eprintln!(
            "request,addr={},idx={},instantiate_total_us={},alloc_us={},write_us={},handle_us={},read_us={},dealloc_us={},total_us={}",
            addr,
            n,
            dur_us(inv.instantiate_total),
            dur_us(inv.alloc),
            dur_us(inv.write),
            dur_us(inv.handle),
            dur_us(inv.read),
            dur_us(inv.dealloc),
            dur_us(inv.total),
        );
    }
}

fn ok(bytes: Vec<u8>) -> HttpResponse {
    Response::new(Full::new(Bytes::from(bytes)))
}

fn err(status: hyper::StatusCode, msg: String) -> HttpResponse {
    let mut r = Response::new(Full::new(Bytes::from(msg)));
    *r.status_mut() = status;
    r
}

#[tokio::main]
async fn main() -> Result<()> {
    let program_start = Instant::now();

    // host <guest.(wasm|cwasm)> [port]
    let wasm_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "guest.cwasm".to_string());

    let port: u16 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    let addr = format!("0.0.0.0:{port}");

    let handler = Arc::new(WasmHandler::new(&wasm_path)?);

    // Do a single instantiation at startup to measure "cold instantiation" cost.
    let (_tmp, first_inst) = handler.instantiate_with_timing()?;

    let listener = TcpListener::bind(&addr).await?;
    let ready = program_start.elapsed();

    handler.maybe_print_startup(&addr, &wasm_path, ready, &first_inst);

    eprintln!("listening on http://{addr} using {wasm_path}");

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let h = handler.clone();
        let addr_tag = addr.clone();

        tokio::spawn(async move {
            let svc = service_fn(move |req: Request<Incoming>| {
                let h2 = h.clone();
                let addr2 = addr_tag.clone();

                async move {
                    let body_res = req.into_body().collect().await;
                    let body = match body_res {
                        Ok(collected) => collected.to_bytes(),
                        Err(e) => {
                            return Ok::<HttpResponse, hyper::http::Error>(err(
                                hyper::StatusCode::BAD_REQUEST,
                                format!("read error: {e:?}"),
                            ));
                        }
                    };

                    // Measure invoke timing (includes per-request instantiation in your current design).
                    let resp = match h2.call_with_timing(&body) {
                        Ok((out, inv)) => {
                            h2.maybe_print_per_request(&addr2, &inv);
                            ok(out)
                        }
                        Err(e) => err(
                            hyper::StatusCode::INTERNAL_SERVER_ERROR,
                            format!("wasm error: {e:#}"),
                        ),
                    };

                    Ok::<HttpResponse, hyper::http::Error>(resp)
                }
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                eprintln!("conn error on {{addr_tag}}: {{e:?}}");
            }
        });
    }
}
