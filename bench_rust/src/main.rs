use rusqlite::Connection;
use std::time::Instant;

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

fn main() {
    let conn = Connection::open(DB_PATH).unwrap();

    let json_blobs = load_blobs(&conn, "events_json");
    let cbor_blobs = load_blobs(&conn, "events_cbor");
    let j_bytes = total_bytes(&json_blobs);
    let c_bytes = total_bytes(&cbor_blobs);

    println!(
        "Loaded {} events (JSON: {} bytes, CBOR: {} bytes)",
        json_blobs.len(),
        j_bytes,
        c_bytes
    );
    println!("CBOR/JSON size ratio: {:.2}%", c_bytes as f64 / j_bytes as f64 * 100.0);
    println!("\nDecode+encode x{} iterations:", ITERS);

    let t1 = bench_json(&json_blobs, ITERS);
    report("JSON (canonical)", t1, json_blobs.len(), j_bytes, ITERS);

    let t2 = bench_cbor(&cbor_blobs, ITERS);
    report("DAG-CBOR (ipld)", t2, cbor_blobs.len(), c_bytes, ITERS);
}
