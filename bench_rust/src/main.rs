use rusqlite::Connection;
use std::time::Instant;

mod rawheap;

const DB_PATH: &str = "../events.db";
const ITERS: usize = 10;

fn load_blobs(conn: &Connection, table: &str) -> Vec<Vec<u8>> {
    let mut stmt = conn
        .prepare(&format!("SELECT data FROM {} ORDER BY id", table))
        .unwrap();
    stmt.query_map([], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
}

fn total_bytes(blobs: &[Vec<u8>]) -> usize {
    blobs.iter().map(|b| b.len()).sum()
}

fn bench_json(blobs: &[Vec<u8>], iters: usize) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..iters {
        for b in blobs {
            let v: serde_json::Value = serde_json::from_slice(b).unwrap();
            serde_json_canonicalizer::to_vec(&v).unwrap();
        }
    }
    start.elapsed()
}

fn bench_cbor(blobs: &[Vec<u8>], iters: usize) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..iters {
        for b in blobs {
            let v: serde_json::Value = serde_ipld_dagcbor::from_slice(b).unwrap();
            serde_ipld_dagcbor::to_vec(&v).unwrap();
        }
    }
    start.elapsed()
}

fn bench_msgpack(blobs: &[Vec<u8>], iters: usize) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..iters {
        for b in blobs {
            let v: serde_json::Value = rmp_serde::from_slice(b).unwrap();
            rmp_serde::to_vec(&v).unwrap();
        }
    }
    start.elapsed()
}

fn bench_bson(blobs: &[Vec<u8>], iters: usize) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..iters {
        for b in blobs {
            let v: bson::Document = bson::from_slice(b).unwrap();
            bson::to_vec(&v).unwrap();
        }
    }
    start.elapsed()
}

fn bench_cbor_dasl(blobs: &[Vec<u8>], iters: usize) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..iters {
        for b in blobs {
            let v: serde_json::Value = dasl::drisl::from_slice(b).unwrap();
            dasl::drisl::to_vec(&v).unwrap();
        }
    }
    start.elapsed()
}

fn bench_rawheap(blobs: &[Vec<u8>], iters: usize) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..iters {
        for b in blobs {
            // Decode: wrap as zero-copy view + walk all values
            let heap = rawheap::RawHeap::new(b);
            heap.root().visit();
            // Encode: no-op — data is already the persisted representation
        }
    }
    start.elapsed()
}

fn bench_ion(blobs: &[Vec<u8>], iters: usize) -> std::time::Duration {
    use ion_rs::v1_0::Binary;
    let start = Instant::now();
    for _ in 0..iters {
        for b in blobs {
            let element = ion_rs::Element::read_one(b.as_slice()).unwrap();
            element.encode_as(Binary).unwrap();
        }
    }
    start.elapsed()
}

fn report(label: &str, d: std::time::Duration, n_events: usize, total_bytes: usize, iters: usize) {
    let total = n_events * iters;
    let secs = d.as_secs_f64();
    println!(
        "  {:<30}  {:6.3}s  {:>10.0} ev/s  {:>6.1} MB/s",
        label,
        secs,
        total as f64 / secs,
        (total_bytes * iters) as f64 / secs / 1e6
    );
}

struct Bench {
    label: &'static str,
    table: &'static str,
    func: fn(&[Vec<u8>], usize) -> std::time::Duration,
}

fn main() {
    let conn = Connection::open(DB_PATH).unwrap();

    let benches = vec![
        Bench { label: "JSON (canonical)",    table: "events_json",    func: bench_json },
        Bench { label: "DAG-CBOR (ipld)",     table: "events_cbor",    func: bench_cbor },
        Bench { label: "DAG-CBOR (dasl)",     table: "events_cbor",    func: bench_cbor_dasl },
        Bench { label: "MsgPack (rmp-serde)", table: "events_msgpack", func: bench_msgpack },
        Bench { label: "BSON (bson)",         table: "events_bson",    func: bench_bson },
        Bench { label: "Ion (ion-rs)",        table: "events_ion",     func: bench_ion },
        Bench { label: "Rawheap (zero-copy)", table: "events_rawheap", func: bench_rawheap },
    ];

    let json_blobs = load_blobs(&conn, "events_json");
    let n = json_blobs.len();
    println!("Loaded {} events per format", n);
    println!("\nDecode+encode x{} iterations:", ITERS);

    // Cache loaded blobs to avoid reloading
    let mut cache: std::collections::HashMap<&str, Vec<Vec<u8>>> = std::collections::HashMap::new();
    cache.insert("events_json", json_blobs);

    for b in &benches {
        if !cache.contains_key(b.table) {
            cache.insert(b.table, load_blobs(&conn, b.table));
        }
        let blobs = &cache[b.table];
        let tb = total_bytes(blobs);
        let d = (b.func)(blobs, ITERS);
        report(b.label, d, n, tb, ITERS);
    }
}
