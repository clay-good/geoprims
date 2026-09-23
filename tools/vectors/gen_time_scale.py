#!/usr/bin/env python3
"""Golden vectors for three time-scale tools, from Python's own calendar.

`time.scale.julian-date`, `time.scale.decimal-hours` and `time.scale.block-time`
carried five to seven vectors each -- too few to promote. This brings each past
twenty.

The reference is Python's `datetime`, which implements the proleptic Gregorian
calendar independently of the core's own day-number arithmetic. The Julian date
itself is a definition and needs no library: JD 2440587.5 is 1970-01-01T00:00Z,
so JD is that plus the POSIX timestamp over 86,400. Day of year and the leap
year rule come from `datetime`. Decimal hours and block time are exact
arithmetic on minutes, done here in integers so a float rounding in the core
cannot be reproduced by accident.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_time_scale.py
"""
import json
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")

JD_AT_EPOCH = 2440587.5  # JD of 1970-01-01T00:00:00Z, by definition
JD_SRC = ("JD = 2440587.5 + (POSIX seconds)/86400, with the calendar and day of year from Python's "
          "datetime (proleptic Gregorian); MJD = JD - 2400000.5")
JD_VER = "CPython 3.13 datetime"
HM_SRC = "Exact integer arithmetic on minutes: minutes = round(hours x 60); hours = h + mm/60"
BLK_SRC = "Exact integer arithmetic on minutes, adding 24 h when a clock-only in time precedes out"

# Dates chosen for what they can break: both century rules, a leap day, the
# day after one, the last day of a leap year and of a common year, the epoch
# itself, the J2000 epoch, a GPS-era date, and times through the day so the
# half-day offset in JD cannot hide.
UTC_CASES = [
    "1970-01-01T00:00:00Z", "1980-01-06T00:00:00Z", "1999-12-31T23:59:00Z",
    "2000-02-29T00:00:00Z", "2000-03-01T00:00:00Z", "2000-12-31T12:00:00Z",
    "1900-03-01T00:00:00Z", "2100-02-28T00:00:00Z", "2024-02-29T06:00:00Z",
    "2023-12-31T23:00:00Z", "2026-01-01T00:00:00Z", "2026-06-15T18:30:00Z",
    "2026-07-01T06:45:00Z", "2033-08-22T00:00:00Z", "2012-06-30T23:59:59Z",
    "1969-07-20T20:17:00Z", "2038-01-19T03:14:07Z",
]
# (decimal hours as written, the minutes it must come to)
HOURS_CASES = [
    ("0", 0), ("0.25", 15), ("0.5", 30), ("0.75", 45), ("1", 60), ("1.5", 90),
    ("2.7", 162), ("3.33", 200), ("4.05", 243), ("5.9", 354), ("8", 480),
    ("10.1", 606), ("12.25", 735), ("0.016666666666666666", 1), ("23.99", 1439),
    ("100", 6000), ("1:18", 78), ("2:45", 165), ("0:06", 6), ("12:00", 720),
]
# (out, in, minutes) -- the same span written several ways, and every way a
# clock-only pair can wrap midnight.
BLOCK_CASES = [
    ("0000", "0000", 0), ("0000", "2359", 1439), ("2359", "0000", 1),
    ("1200", "1200", 0), ("0915", "1045", 90), ("2330", "0030", 60),
    ("0600", "1800", 720), ("1800", "0600", 720), ("0005", "0004", 1439),
    ("07:15", "09:50", 155), ("22:00", "02:30", 270), ("1415Z", "1630Z", 135),
    ("0030Z", "2330Z", 1380), ("1159", "1201", 2), ("0000", "1200", 720),
    ("2045", "2115", 30), ("0730", "0730", 0), ("1630", "0115", 525),
]


def jd_rows(start):
    rows = []
    for i, text in enumerate(UTC_CASES, start=start + 1):
        t = datetime.fromisoformat(text.replace("Z", "+00:00")).astimezone(timezone.utc)
        jd = JD_AT_EPOCH + t.timestamp() / 86400.0
        doy = (t.date() - t.date().replace(month=1, day=1)).days + 1
        rows.append((i, {"utc": text},
                     {"ok": True, "result.jd": jd, "result.mjd": jd - 2400000.5,
                      "result.day_of_year": float(doy), "result.year": float(t.year)},
                     {"result.jd": {"abs": 1e-9}, "result.mjd": {"abs": 1e-9},
                      "result.day_of_year": {"abs": 0}, "result.year": {"abs": 0}},
                     JD_SRC, JD_VER))
    return rows


def hm(minutes):
    return f"{minutes // 60}:{minutes % 60:02d}"


def hours_rows(start):
    rows = []
    for i, (text, minutes) in enumerate(HOURS_CASES, start=start + 1):
        rows.append((i, {"time": text},
                     {"ok": True, "result.minutes": float(minutes),
                      "result.hm": hm(minutes), "result.hours": minutes / 60.0},
                     {"result.minutes": {"abs": 0}, "result.hours": {"rel": 5e-16, "abs": 1e-12}},
                     HM_SRC, "definition"))
    return rows


def block_rows(start):
    rows = []
    for i, (out, inn, minutes) in enumerate(BLOCK_CASES, start=start + 1):
        # The core is asked the same question Python would be: the difference
        # in minutes, wrapped into a day when the in time precedes the out.
        a, b = [int(s[:2]) * 60 + int(s[2:4]) for s in
                (out.replace(":", "").rstrip("Z"), inn.replace(":", "").rstrip("Z"))]
        assert (b - a) % 1440 == minutes, f"{out} {inn}: {(b - a) % 1440} not {minutes}"
        assert timedelta(minutes=minutes) >= timedelta(0)
        rows.append((i, {"out_time": out, "in_time": inn},
                     {"ok": True, "result.minutes": float(minutes), "result.hm": hm(minutes)},
                     {"result.minutes": {"abs": 0}}, BLK_SRC, "definition"))
    return rows


def main():
    plan = [
        ("time.scale.julian-date", jd_rows),
        ("time.scale.decimal-hours", hours_rows),
        ("time.scale.block-time", block_rows),
    ]
    for tool, build in plan:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 8:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol, src, ver in rows:
                f.write(json.dumps({
                    "id": f"v{i:03d}", "input": inp, "expect": expect,
                    "source": src, "sourceVersion": ver, "tolerance": tol,
                }) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
