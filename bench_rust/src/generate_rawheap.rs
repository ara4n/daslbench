use rusqlite::Connection;

mod rawheap;

const DB_PATH: &str = "../events.db";

fn main() {
    let conn = Connection::open(DB_PATH).unwrap();

    // Load all JSON events
    let mut stmt = conn
        .prepare("SELECT id, data FROM events_json ORDER BY id")
        .unwrap();
    let rows: Vec<(i64, Vec<u8>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    println!("Converting {} events to rawheap...", rows.len());

    // Create target table
    conn.execute("DROP TABLE IF EXISTS events_rawheap", [])
        .unwrap();
    conn.execute(
        "CREATE TABLE events_rawheap (id INTEGER PRIMARY KEY, data BLOB)",
        [],
    )
    .unwrap();

    let mut total_json_bytes: usize = 0;
    let mut total_rawheap_bytes: usize = 0;
    let mut min_ratio = f64::MAX;
    let mut max_ratio = 0.0f64;

    let tx = conn.unchecked_transaction().unwrap();
    {
        let mut insert = tx
            .prepare("INSERT INTO events_rawheap (id, data) VALUES (?, ?)")
            .unwrap();

        for (id, json_blob) in &rows {
            let val: serde_json::Value = serde_json::from_slice(json_blob).unwrap();
            let mut builder = rawheap::HeapBuilder::new();
            let root = builder.write_json_value(&val);
            let blob = builder.finish(root);

            total_json_bytes += json_blob.len();
            total_rawheap_bytes += blob.len();
            let ratio = blob.len() as f64 / json_blob.len() as f64;
            min_ratio = min_ratio.min(ratio);
            max_ratio = max_ratio.max(ratio);

            insert.execute(rusqlite::params![id, blob]).unwrap();
        }
    }
    tx.commit().unwrap();

    println!("Done!");
    println!(
        "  JSON total:    {:>10} bytes ({:.1} MB)",
        total_json_bytes,
        total_json_bytes as f64 / 1e6
    );
    println!(
        "  Rawheap total: {:>10} bytes ({:.1} MB)",
        total_rawheap_bytes,
        total_rawheap_bytes as f64 / 1e6
    );
    println!(
        "  Ratio:         {:.2}x (range {:.2}x - {:.2}x)",
        total_rawheap_bytes as f64 / total_json_bytes as f64,
        min_ratio,
        max_ratio
    );
    println!("  Events:        {}", rows.len());
}
