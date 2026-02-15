"""Benchmark canonical JSON vs DAG-CBOR decode+encode in Python."""

import json
import sqlite3
import time

import canonicaljson
import cbrrr
import rfc8785

DB = "events.db"
ITERS = 10


def load_blobs(table):
    db = sqlite3.connect(DB)
    rows = db.execute(f"SELECT data FROM {table} ORDER BY id").fetchall()
    db.close()
    return [r[0] for r in rows]


def bench_json_rfc8785(blobs, iters):
    start = time.perf_counter()
    for _ in range(iters):
        for b in blobs:
            obj = json.loads(b)
            rfc8785.dumps(obj)
    return time.perf_counter() - start


def bench_json_canonicaljson(blobs, iters):
    start = time.perf_counter()
    for _ in range(iters):
        for b in blobs:
            obj = json.loads(b)
            canonicaljson.encode_canonical_json(obj)
    return time.perf_counter() - start


def bench_cbor(blobs, iters):
    start = time.perf_counter()
    for _ in range(iters):
        for b in blobs:
            obj = cbrrr.decode_dag_cbor(b)
            cbrrr.encode_dag_cbor(obj)
    return time.perf_counter() - start


def report(label, elapsed, n_events, total_bytes, iters):
    total = n_events * iters
    print(f"  {label:30s}  {elapsed:6.3f}s  {total/elapsed:10,.0f} ev/s  {total_bytes*iters/elapsed/1e6:6.1f} MB/s")


def main():
    json_blobs = load_blobs("events_json")
    cbor_blobs = load_blobs("events_cbor")
    n = len(json_blobs)
    json_bytes = sum(len(b) for b in json_blobs)
    cbor_bytes = sum(len(b) for b in cbor_blobs)

    print(f"Loaded {n} events (JSON: {json_bytes:,} bytes, CBOR: {cbor_bytes:,} bytes)")
    print(f"CBOR/JSON size ratio: {cbor_bytes/json_bytes:.2%}")
    print(f"\nDecode+encode x{ITERS} iterations:")

    t1 = bench_json_rfc8785(json_blobs, ITERS)
    report("JSON (rfc8785)", t1, n, json_bytes, ITERS)

    t2 = bench_json_canonicaljson(json_blobs, ITERS)
    report("JSON (canonicaljson)", t2, n, json_bytes, ITERS)

    t3 = bench_cbor(cbor_blobs, ITERS)
    report("DAG-CBOR (cbrrr)", t3, n, cbor_bytes, ITERS)


if __name__ == "__main__":
    main()
