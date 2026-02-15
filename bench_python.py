"""Benchmark serialization formats: decode+encode in Python."""

import json
import sqlite3
import time

import bson
import canonicaljson
import cbrrr
import msgpack
import rfc8785
import ubjson
from amazon.ion import simpleion as ion

DB = "events.db"
ITERS = 10


def load_blobs(table):
    db = sqlite3.connect(DB)
    rows = db.execute(f"SELECT data FROM {table} ORDER BY id").fetchall()
    db.close()
    return [r[0] for r in rows]


def bench(decode_fn, encode_fn, blobs, iters):
    start = time.perf_counter()
    for _ in range(iters):
        for b in blobs:
            obj = decode_fn(b)
            encode_fn(obj)
    return time.perf_counter() - start


def report(label, elapsed, n_events, total_bytes, iters):
    total = n_events * iters
    print(f"  {label:30s}  {elapsed:6.3f}s  {total/elapsed:10,.0f} ev/s  {total_bytes*iters/elapsed/1e6:6.1f} MB/s")


BENCHMARKS = [
    ("JSON (rfc8785)",       "events_json",    json.loads,                    rfc8785.dumps),
    ("JSON (canonicaljson)", "events_json",    json.loads,                    canonicaljson.encode_canonical_json),
    ("DAG-CBOR (cbrrr)",     "events_cbor",    cbrrr.decode_dag_cbor,         cbrrr.encode_dag_cbor),
    ("MsgPack (msgpack)",    "events_msgpack", lambda b: msgpack.unpackb(b),  msgpack.packb),
    ("BSON (pymongo)",       "events_bson",    bson.decode,                   bson.encode),
    ("Ion (amazon.ion)",     "events_ion",     ion.loads,                     lambda o: ion.dumps(o, binary=True)),
    ("UBJSON (py-ubjson)",   "events_ubjson",  ubjson.loadb,                  ubjson.dumpb),
]


def main():
    # Load all format blobs and compute sizes
    all_blobs = {}
    for _, table, _, _ in BENCHMARKS:
        if table not in all_blobs:
            all_blobs[table] = load_blobs(table)

    n = len(all_blobs["events_json"])
    print(f"Loaded {n} events per format")
    for table, blobs in sorted(all_blobs.items()):
        total = sum(len(b) for b in blobs)
        print(f"  {table:20s}  {total:>10,} bytes")

    print(f"\nDecode+encode x{ITERS} iterations:")
    for label, table, decode_fn, encode_fn in BENCHMARKS:
        blobs = all_blobs[table]
        total_bytes = sum(len(b) for b in blobs)
        t = bench(decode_fn, encode_fn, blobs, ITERS)
        report(label, t, n, total_bytes, ITERS)


if __name__ == "__main__":
    main()
