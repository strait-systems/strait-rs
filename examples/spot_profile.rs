//! Single-threaded offline CPU/allocation probe. Setup and reporting are excluded.
//! The allocator delegates unchanged layouts/pointers to System; it does not pool,
//! dereference, retain or alter allocations. Atomics measure calls, not application ordering.
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use strait::connector::binance::spot::sbe::{DecodeLimits, Decoder, Scales};
use strait::market_data::Scale;

struct CountingAllocator;
static ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static REALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static FREES: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static FREED_BYTES: AtomicU64 = AtomicU64::new(0);

// SAFETY: Every operation delegates the original valid arguments directly to System.
// Instrumentation uses allocation-free atomics and never unwinds. Results and ownership
// are unchanged, including null allocation/reallocation results.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Caller supplies a valid Layout; System implements GlobalAlloc.
        let ptr = unsafe { System.alloc(layout) };
        if ENABLED.load(Ordering::Relaxed) && !ptr.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ENABLED.load(Ordering::Relaxed) {
            FREES.fetch_add(1, Ordering::Relaxed);
            FREED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        // SAFETY: Caller provides a live System allocation with its original Layout.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: Caller satisfies System's reallocation contract; arguments unchanged.
        let result = unsafe { System.realloc(ptr, layout, size) };
        if ENABLED.load(Ordering::Relaxed) {
            REALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            if !result.is_null() {
                ALLOCATED_BYTES.fetch_add(size as u64, Ordering::Relaxed);
                FREED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            }
        }
        result
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn reset() {
    for counter in [
        &ALLOCATIONS,
        &REALLOCATIONS,
        &FREES,
        &ALLOCATED_BYTES,
        &FREED_BYTES,
    ] {
        counter.store(0, Ordering::Relaxed);
    }
}

fn report(label: &str, count: u64, elapsed: Duration) {
    ENABLED.store(false, Ordering::Relaxed);
    println!(
        "{label}: operations={count} elapsed_ns={} allocations={} reallocations={} frees={} allocated_bytes={} freed_bytes={}",
        elapsed.as_nanos(),
        ALLOCATIONS.load(Ordering::Relaxed),
        REALLOCATIONS.load(Ordering::Relaxed),
        FREES.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
        FREED_BYTES.load(Ordering::Relaxed)
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seconds: u64 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "2".to_owned())
        .parse()?;
    let duration = Duration::from_secs(seconds);
    let limits = DecodeLimits {
        max_message_bytes: 4096,
        max_levels: 16,
        max_symbol_bytes: 32,
        max_rate_limits: 8,
    };
    let scales = Scales {
        price: Scale::new(2)?,
        quantity: Scale::new(3)?,
    };
    println!(
        "layout: price_bytes={} quantity_bytes={} level_bytes={}",
        std::mem::size_of::<strait::market_data::Price>(),
        std::mem::size_of::<strait::market_data::Quantity>(),
        std::mem::size_of::<strait::market_data::Level>(),
    );
    reset();
    ENABLED.store(true, Ordering::Relaxed);
    let setup = Instant::now();
    let mut storage = Decoder::new(limits)?;
    report("storage_setup", 1, setup.elapsed());
    // Warm-up all paths before measuring.
    let depth_bytes = include_bytes!("../tests/fixtures/binance/spot/depth.bin");
    let snapshot_bytes = include_bytes!("../tests/fixtures/binance/spot/snapshot.bin");
    let error_bytes = include_bytes!("../tests/fixtures/binance/spot/rate-limit.bin");
    storage.decode_depth(depth_bytes, "BTCUSDT", scales)?;
    storage.decode_snapshot(snapshot_bytes, "snap1", scales)?;
    for mode in 0..3 {
        reset();
        ENABLED.store(true, Ordering::Relaxed);
        let start = Instant::now();
        let mut count = 0;
        while start.elapsed() < duration {
            for _ in 0..1000 {
                match mode {
                    0 => {
                        let d = storage.decode_depth(black_box(depth_bytes), "BTCUSDT", scales)?;
                        black_box((d.bids, d.asks, d.last_update_id));
                    }
                    1 => {
                        black_box(storage.decode_snapshot(
                            black_box(snapshot_bytes),
                            "snap1",
                            scales,
                        )?);
                    }
                    _ => {
                        black_box(storage.decode_snapshot(
                            black_box(error_bytes),
                            "snap1",
                            scales,
                        )?);
                    }
                }
                count += 1;
            }
        }
        report(
            ["depth_3_levels", "ws_snapshot_3_levels", "ws_error"][mode],
            count,
            start.elapsed(),
        );
    }
    reset();
    ENABLED.store(true, Ordering::Relaxed);
    let cleanup = Instant::now();
    drop(storage);
    report("storage_cleanup", 1, cleanup.elapsed());
    Ok(())
}
