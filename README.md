# DASL (DAG-CBOR) vs Canonical JSON Benchmark

Benchmarks comparing DAG-CBOR and canonical JSON (RFC 8785) encode/decode performance across Python, Go and Rust, using 10k synthetic Matrix events stored in SQLite.

## Setup

### Python

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install rfc8785 cbrrr canonicaljson
```

### Go

Requires Go 1.26+ with jsonv2 experiment support, and a C compiler for cgo (sqlite).

```bash
cd bench_go
go mod tidy
```

### Rust

```bash
cd bench_rust
cargo build --release
```

## Running

```
% ./generate.py 
Generated 10000 events
  JSON total: 7,033,917 bytes (703 avg)
  CBOR total: 6,337,536 bytes (634 avg)
  CBOR/JSON ratio: 90.10%
```

```
% python ./bench_python.py
Loaded 10000 events (JSON: 7,033,917 bytes, CBOR: 6,337,536 bytes)
CBOR/JSON size ratio: 90.10%

Decode+encode x10 iterations:
  JSON (rfc8785)                   3.105s      32,204 ev/s    22.7 MB/s
  JSON (canonicaljson)             0.973s     102,806 ev/s    72.3 MB/s
  DAG-CBOR (cbrrr)                 0.302s     331,275 ev/s   209.9 MB/s

```

```
% cd bench_go
% GOEXPERIMENT=jsonv2 go run bench_go 
Loaded 10000 events (JSON: 7033917 bytes, CBOR: 6337536 bytes)
CBOR/JSON size ratio: 90.10%

Decode+encode x10 iterations:
  JSON (jsonv2 canonical)          0.959s      104287 ev/s    73.4 MB/s
  DAG-CBOR (go-dasl)               0.780s      128284 ev/s    81.3 MB/s
```

```
% cd bench_rust 
% target/release/bench_rust 
Loaded 10000 events (JSON: 7033917 bytes, CBOR: 6337536 bytes)
CBOR/JSON size ratio: 90.10%

Decode+encode x10 iterations:
  JSON (canonical)                 1.007s       99339 ev/s    69.9 MB/s
  DAG-CBOR (ipld)                  0.400s      250257 ev/s   158.6 MB/s
```

(Produced by Opus 4.6, but with manual checks that it looks to be doing the right thing)