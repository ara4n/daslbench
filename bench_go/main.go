package main

import (
	"database/sql"
	"encoding/json/jsontext"
	"encoding/json/v2"
	"fmt"
	"time"

	"github.com/amazon-ion/ion-go/ion"
	"github.com/hyphacoop/go-dasl/drisl"
	_ "github.com/mattn/go-sqlite3"
	"github.com/toitware/ubjson"
	"github.com/vmihailenco/msgpack/v5"
	"go.mongodb.org/mongo-driver/bson"
)

const dbPath = "../events.db"
const iters = 10

func loadBlobs(db *sql.DB, table string) [][]byte {
	rows, err := db.Query("SELECT data FROM " + table + " ORDER BY id")
	if err != nil {
		panic(err)
	}
	defer rows.Close()
	var blobs [][]byte
	for rows.Next() {
		var data []byte
		if err := rows.Scan(&data); err != nil {
			panic(err)
		}
		blobs = append(blobs, data)
	}
	return blobs
}

func totalBytes(blobs [][]byte) int {
	n := 0
	for _, b := range blobs {
		n += len(b)
	}
	return n
}

func benchJSON(blobs [][]byte, iters int) time.Duration {
	start := time.Now()
	for i := 0; i < iters; i++ {
		for _, b := range blobs {
			var v any
			if err := json.Unmarshal(b, &v); err != nil {
				panic(err)
			}
			out, err := json.Marshal(v)
			if err != nil {
				panic(err)
			}
			jv := jsontext.Value(out)
			if err := jv.Canonicalize(); err != nil {
				panic(err)
			}
		}
	}
	return time.Since(start)
}

func benchCBOR(blobs [][]byte, iters int) time.Duration {
	start := time.Now()
	for i := 0; i < iters; i++ {
		for _, b := range blobs {
			var v any
			if err := drisl.Unmarshal(b, &v); err != nil {
				panic(err)
			}
			if _, err := drisl.Marshal(v); err != nil {
				panic(err)
			}
		}
	}
	return time.Since(start)
}

func benchMsgpack(blobs [][]byte, iters int) time.Duration {
	start := time.Now()
	for i := 0; i < iters; i++ {
		for _, b := range blobs {
			var v any
			if err := msgpack.Unmarshal(b, &v); err != nil {
				panic(err)
			}
			if _, err := msgpack.Marshal(v); err != nil {
				panic(err)
			}
		}
	}
	return time.Since(start)
}

func benchBSON(blobs [][]byte, iters int) time.Duration {
	start := time.Now()
	for i := 0; i < iters; i++ {
		for _, b := range blobs {
			var v bson.M
			if err := bson.Unmarshal(b, &v); err != nil {
				panic(err)
			}
			if _, err := bson.Marshal(v); err != nil {
				panic(err)
			}
		}
	}
	return time.Since(start)
}

func benchIon(blobs [][]byte, iters int) time.Duration {
	start := time.Now()
	for i := 0; i < iters; i++ {
		for _, b := range blobs {
			var v any
			if err := ion.Unmarshal(b, &v); err != nil {
				panic(err)
			}
			if _, err := ion.MarshalBinary(v); err != nil {
				panic(err)
			}
		}
	}
	return time.Since(start)
}

func benchUBJSON(blobs [][]byte, iters int) time.Duration {
	start := time.Now()
	for i := 0; i < iters; i++ {
		for _, b := range blobs {
			var v any
			if err := ubjson.Unmarshal(b, &v); err != nil {
				panic(err)
			}
			if _, err := ubjson.Marshal(v); err != nil {
				panic(err)
			}
		}
	}
	return time.Since(start)
}

type benchEntry struct {
	label string
	table string
	fn    func([][]byte, int) time.Duration
}

func report(label string, d time.Duration, nEvents, tBytes, iters int) {
	total := nEvents * iters
	secs := d.Seconds()
	fmt.Printf("  %-30s  %6.3fs  %10.0f ev/s  %6.1f MB/s\n",
		label, secs, float64(total)/secs, float64(tBytes*iters)/secs/1e6)
}

func main() {
	db, err := sql.Open("sqlite3", dbPath)
	if err != nil {
		panic(err)
	}
	defer db.Close()

	benches := []benchEntry{
		{"JSON (jsonv2 canonical)", "events_json", benchJSON},
		{"DAG-CBOR (go-dasl)", "events_cbor", benchCBOR},
		{"MsgPack (vmihailenco)", "events_msgpack", benchMsgpack},
		{"BSON (mongo-driver)", "events_bson", benchBSON},
		{"Ion (ion-go)", "events_ion", benchIon},
		{"UBJSON (toitware)", "events_ubjson", benchUBJSON},
	}

	// Load all blobs
	blobSets := make(map[string][][]byte)
	for _, b := range benches {
		if _, ok := blobSets[b.table]; !ok {
			blobSets[b.table] = loadBlobs(db, b.table)
		}
	}

	n := len(blobSets["events_json"])
	fmt.Printf("Loaded %d events per format\n", n)
	fmt.Printf("\nDecode+encode x%d iterations:\n", iters)

	for _, b := range benches {
		blobs := blobSets[b.table]
		tBytes := totalBytes(blobs)
		d := b.fn(blobs, iters)
		report(b.label, d, n, tBytes, iters)
	}
}
