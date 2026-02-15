package main

import (
	"database/sql"
	"encoding/json/jsontext"
	"encoding/json/v2"
	"fmt"
	"time"

	"github.com/hyphacoop/go-dasl/drisl"
	_ "github.com/mattn/go-sqlite3"
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

func report(label string, d time.Duration, nEvents, totalBytes, iters int) {
	total := nEvents * iters
	secs := d.Seconds()
	fmt.Printf("  %-30s  %6.3fs  %10.0f ev/s  %6.1f MB/s\n",
		label, secs, float64(total)/secs, float64(totalBytes*iters)/secs/1e6)
}

func main() {
	db, err := sql.Open("sqlite3", dbPath)
	if err != nil {
		panic(err)
	}
	defer db.Close()

	jsonBlobs := loadBlobs(db, "events_json")
	cborBlobs := loadBlobs(db, "events_cbor")
	jBytes := totalBytes(jsonBlobs)
	cBytes := totalBytes(cborBlobs)

	fmt.Printf("Loaded %d events (JSON: %d bytes, CBOR: %d bytes)\n", len(jsonBlobs), jBytes, cBytes)
	fmt.Printf("CBOR/JSON size ratio: %.2f%%\n", float64(cBytes)/float64(jBytes)*100)
	fmt.Printf("\nDecode+encode x%d iterations:\n", iters)

	t1 := benchJSON(jsonBlobs, iters)
	report("JSON (jsonv2 canonical)", t1, len(jsonBlobs), jBytes, iters)

	t2 := benchCBOR(cborBlobs, iters)
	report("DAG-CBOR (go-dasl)", t2, len(cborBlobs), cBytes, iters)
}
