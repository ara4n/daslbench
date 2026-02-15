#!/usr/bin/env python

"""Generate 10k synthetic Matrix events and store as canonical JSON and DAG-CBOR in SQLite."""

import hashlib
import random
import sqlite3
import string
import time

import cbrrr
import rfc8785

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


def main():
    rooms = [rand_room_id() for _ in range(50)]
    event_count = 10_000
    events = [make_event(random.choice(rooms)) for _ in range(event_count)]

    db = sqlite3.connect("events.db")
    db.execute("DROP TABLE IF EXISTS events_json")
    db.execute("DROP TABLE IF EXISTS events_cbor")
    db.execute("CREATE TABLE events_json (id INTEGER PRIMARY KEY, data BLOB)")
    db.execute("CREATE TABLE events_cbor (id INTEGER PRIMARY KEY, data BLOB)")

    json_total = 0
    cbor_total = 0
    for i, ev in enumerate(events):
        j = rfc8785.dumps(ev)
        c = cbrrr.encode_dag_cbor(ev)
        json_total += len(j)
        cbor_total += len(c)
        db.execute("INSERT INTO events_json VALUES (?, ?)", (i, j))
        db.execute("INSERT INTO events_cbor VALUES (?, ?)", (i, c))

    db.commit()
    db.close()
    print(f"Generated {event_count} events")
    print(f"  JSON total: {json_total:,} bytes ({json_total/event_count:.0f} avg)")
    print(f"  CBOR total: {cbor_total:,} bytes ({cbor_total/event_count:.0f} avg)")
    print(f"  CBOR/JSON ratio: {cbor_total/json_total:.2%}")


if __name__ == "__main__":
    main()
