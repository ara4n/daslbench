#!/usr/bin/env python

"""Generate 10k synthetic Matrix events and store in multiple formats in SQLite."""

import hashlib
import random
import sqlite3
import string

import bson
import cbrrr
import msgpack
import rfc8785
import ubjson
from amazon.ion import simpleion as ion

random.seed(42)

SERVERS = ["matrix.org", "example.com", "alice.net", "bob.chat", "synapse.dev"]
EVENT_TYPES = [
    ("m.room.message", None),
    ("m.room.member", "state"),
    ("m.room.name", "state"),
    ("m.room.topic", "state"),
    ("m.room.power_levels", "state"),
    ("m.room.create", "state"),
]


def rand_id(prefix, length=20):
    chars = string.ascii_letters + string.digits
    return prefix + "".join(random.choices(chars, k=length))


def rand_event_id():
    return "$" + hashlib.sha256(random.randbytes(32)).hexdigest()[:43]


def rand_user():
    name = "".join(random.choices(string.ascii_lowercase, k=random.randint(3, 12)))
    return f"@{name}:{random.choice(SERVERS)}"


def rand_room_id():
    return f"!{rand_id('', 18)}:{random.choice(SERVERS)}"


def make_content(event_type):
    if event_type == "m.room.message":
        body = " ".join(random.choices(
            ["hello", "world", "foo", "bar", "test", "matrix", "event",
             "benchmark", "data", "the", "is", "a", "of", "and"],
            k=random.randint(3, 30),
        ))
        return {"body": body, "msgtype": random.choice(["m.text", "m.notice", "m.emote"])}
    if event_type == "m.room.member":
        return {
            "membership": random.choice(["join", "leave", "invite", "ban"]),
            "displayname": "".join(random.choices(string.ascii_letters, k=8)),
        }
    if event_type == "m.room.name":
        return {"name": " ".join(random.choices(["General", "Random", "Dev", "Test", "Chat"], k=2))}
    if event_type == "m.room.topic":
        return {"topic": " ".join(random.choices(["Welcome", "Discussion", "about", "things", "stuff"], k=4))}
    if event_type == "m.room.power_levels":
        return {
            "users_default": 0, "events_default": 0, "state_default": 50, "ban": 50, "kick": 50,
            "users": {rand_user(): 100},
        }
    if event_type == "m.room.create":
        return {"creator": rand_user(), "room_version": random.choice(["10", "11"])}
    return {}


def make_event(room_id):
    event_type, kind = random.choice(EVENT_TYPES)
    sender = rand_user()
    origin = sender.split(":")[1]
    sig_bytes = random.randbytes(64)
    event = {
        "auth_events": [rand_event_id() for _ in range(random.randint(1, 4))],
        "content": make_content(event_type),
        "depth": random.randint(1, 100000),
        "hashes": {"sha256": hashlib.sha256(random.randbytes(32)).digest().hex()[:43]},
        "origin_server_ts": random.randint(1_600_000_000_000, 1_700_000_000_000),
        "prev_events": [rand_event_id() for _ in range(random.randint(1, 3))],
        "room_id": room_id,
        "sender": sender,
        "signatures": {origin: {"ed25519:1": sig_bytes.hex()[:86]}},
        "type": event_type,
    }
    if kind == "state":
        event["state_key"] = rand_user() if event_type == "m.room.member" else ""
    event["unsigned"] = {"age": random.randint(100, 999999)}
    return event


TABLES = ["events_json", "events_cbor", "events_msgpack", "events_bson", "events_ion", "events_ubjson"]


def main():
    rooms = [rand_room_id() for _ in range(50)]
    event_count = 10_000
    events = [make_event(random.choice(rooms)) for _ in range(event_count)]

    db = sqlite3.connect("events.db")
    for t in TABLES:
        db.execute(f"DROP TABLE IF EXISTS {t}")
        db.execute(f"CREATE TABLE {t} (id INTEGER PRIMARY KEY, data BLOB)")

    totals = {t: 0 for t in TABLES}
    for i, ev in enumerate(events):
        encoded = {
            "events_json": rfc8785.dumps(ev),
            "events_cbor": cbrrr.encode_dag_cbor(ev),
            "events_msgpack": msgpack.packb(ev),
            "events_bson": bson.encode(ev),
            "events_ion": ion.dumps(ev, binary=True),
            "events_ubjson": ubjson.dumpb(ev),
        }
        for t, data in encoded.items():
            totals[t] += len(data)
            db.execute(f"INSERT INTO {t} VALUES (?, ?)", (i, data))

    db.commit()
    db.close()

    json_total = totals["events_json"]
    print(f"Generated {event_count} events")
    for t in TABLES:
        label = t.replace("events_", "").upper()
        avg = totals[t] / event_count
        ratio = totals[t] / json_total
        print(f"  {label:10s}  {totals[t]:>10,} bytes  ({avg:5.0f} avg)  {ratio:5.2%} of JSON")


if __name__ == "__main__":
    main()
