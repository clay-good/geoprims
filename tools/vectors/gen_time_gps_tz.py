#!/usr/bin/env python3
"""Golden vectors for the GPS time scale and IANA zone lookup.

Three tools, three independent references, none of them the core:

- `time.scale.gps-week` and `time.scale.gps-to-utc` need TAI - UTC at an
  instant, which is a published table and not a formula. It is read here from
  `/usr/share/zoneinfo/leapseconds`, the IANA tzdb's own leap-second file, and
  the offset is accumulated from the 10 s that TAI - UTC stood at when the
  leap-second system began on 1972-01-01. GPS - UTC is that minus 19, because
  GPS time was set to TAI - 19 s so that it matched UTC at the GPS epoch.
- `time.scale.zone-info` comes from Python's `zoneinfo`, a separate reader of
  the same IANA release the core embeds. The next transition is found by
  stepping forward and bisecting, not by reading the core's answer.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_time_gps_tz.py
"""
import json
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from zoneinfo import ZoneInfo

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
LEAPFILE = Path("/usr/share/zoneinfo/leapseconds")
TZ_VERSION = Path("/usr/share/zoneinfo/+VERSION").read_text().strip()

GPS_EPOCH = datetime(1980, 1, 6, tzinfo=timezone.utc).timestamp()
WEEK = 604800
MON = {m: i + 1 for i, m in enumerate("Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec".split())}


def leap_table():
    """[(effective POSIX second, TAI - UTC from then on)], from the tzdb file."""
    out, tai = [], 10  # TAI - UTC when the leap-second system began, 1972-01-01
    for line in LEAPFILE.read_text().splitlines():
        if not line.startswith("Leap"):
            continue
        _, y, mon, d, _, sign, _ = line.split()
        # The second is inserted at the end of that day, so the new offset
        # applies from the following midnight UTC.
        t = datetime(int(y), MON[mon], int(d), tzinfo=timezone.utc).timestamp() + 86400
        tai += 1 if sign == "+" else -1
        out.append((t, tai))
    return out


LEAPS = leap_table()


def tai_minus_utc(t):
    v = 10
    for when, off in LEAPS:
        if t >= when:
            v = off
    return v


def gps_minus_utc(t):
    return tai_minus_utc(t) - 19


def gps_of(text):
    """(full week, seconds of week, 10-bit week, era, GPS - UTC) for a UTC time."""
    t = datetime.fromisoformat(text.replace("Z", "+00:00")).timestamp()
    s = t - GPS_EPOCH + gps_minus_utc(t)
    week, sow = divmod(s, WEEK)
    return int(week), sow, int(week) % 1024, int(week) // 1024, gps_minus_utc(t)


# Times chosen for the boundaries: the GPS epoch, both 1,024-week rollovers,
# the week a leap second landed in, the instants either side of the 2017 leap,
# and ordinary times spread across the eras.
GPS_UTC_CASES = [
    "1980-01-06T00:00:00Z", "1990-06-15T12:00:00Z", "1999-08-21T23:59:46Z",
    "1999-08-22T00:00:00Z", "2005-11-09T08:30:00Z", "2012-06-30T23:59:59Z",
    "2012-07-01T00:00:00Z", "2015-06-30T23:59:59Z", "2016-12-31T23:59:59Z",
    "2017-01-01T00:00:00Z", "2019-04-06T23:59:41Z", "2019-04-07T00:00:00Z",
    "2024-02-29T06:00:00Z", "2026-01-01T00:00:00Z", "2026-09-18T12:34:56Z",
    "2038-01-19T03:14:07Z",
]

# (zone, UTC instant) -- both hemispheres, zones with and without DST, a
# half-hour and a three-quarter-hour offset, and instants either side of a
# northern and a southern changeover.
TZ_CASES = [
    ("America/New_York", "2026-01-15T12:00:00Z"), ("America/New_York", "2026-07-15T12:00:00Z"),
    ("America/Denver", "2026-03-08T09:00:00Z"), ("America/Denver", "2026-03-08T10:00:00Z"),
    ("Europe/London", "2026-06-21T12:00:00Z"), ("Europe/Paris", "2026-01-01T00:00:00Z"),
    ("Europe/Paris", "2026-08-01T00:00:00Z"), ("Asia/Tokyo", "2026-05-05T00:00:00Z"),
    ("Asia/Kolkata", "2026-05-05T00:00:00Z"), ("Asia/Kathmandu", "2026-05-05T00:00:00Z"),
    ("Australia/Sydney", "2026-01-15T00:00:00Z"), ("Australia/Adelaide", "2026-01-15T00:00:00Z"),
    ("Pacific/Auckland", "2026-06-15T00:00:00Z"), ("Pacific/Auckland", "2026-12-15T00:00:00Z"),
    ("America/Phoenix", "2026-07-04T18:00:00Z"), ("America/Sao_Paulo", "2026-01-20T12:00:00Z"),
    ("Africa/Nairobi", "2026-03-01T06:00:00Z"), ("UTC", "2026-09-18T00:00:00Z"),
    ("America/Anchorage", "2026-11-01T09:00:00Z"), ("Asia/Shanghai", "2026-02-10T04:00:00Z"),
]


def offset_text(td):
    total = int(td.total_seconds())
    sign = "-" if total < 0 else "+"
    total = abs(total)
    return f"{sign}{total // 3600:02d}:{total % 3600 // 60:02d}"


def next_transition(zone, t):
    """The first instant after t where the zone's offset changes, to the second."""
    z = ZoneInfo(zone)
    here = datetime.fromtimestamp(t, z).utcoffset()
    lo = t
    hi = t + 86400
    limit = t + 400 * 86400
    while hi < limit and datetime.fromtimestamp(hi, z).utcoffset() == here:
        lo, hi = hi, hi + 86400
    if hi >= limit:
        return None, None
    while hi - lo > 1:
        mid = (lo + hi) // 2
        if datetime.fromtimestamp(mid, z).utcoffset() == here:
            lo = mid
        else:
            hi = mid
    return hi, datetime.fromtimestamp(hi, z).utcoffset()


def iso(t):
    return datetime.fromtimestamp(t, timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def gps_week_rows(start):
    rows = []
    for i, text in enumerate(GPS_UTC_CASES, start=start + 1):
        week, sow, w10, era, off = gps_of(text)
        rows.append((i, {"utc": text},
                     {"ok": True, "result.gps_week": float(week), "result.seconds_of_week": float(sow),
                      "result.week_10bit": float(w10), "result.rollover_era": float(era),
                      "result.gps_minus_utc.value": float(off),
                      "result.tai_minus_utc.value": float(off + 19)},
                     {k: {"abs": 0} for k in
                      ("result.gps_week", "result.seconds_of_week", "result.week_10bit",
                       "result.rollover_era", "result.gps_minus_utc.value", "result.tai_minus_utc.value")},
                     GPS_SRC, TZ_VERSION))
    return rows


def gps_to_utc_rows(start):
    rows = []
    i = start
    for text in GPS_UTC_CASES:
        week, sow, w10, era, _ = gps_of(text)
        if sow != int(sow):
            continue
        i += 1
        # The full week, and the same instant as a 10-bit week plus its era,
        # alternately, so both ways of naming a week are covered. A full week
        # below 1,024 is itself ambiguous -- it could be a 10-bit week from any
        # era -- and the tool refuses it without one, so those always carry it.
        inp = {"week": week, "seconds_of_week": int(sow)} if i % 2 and week >= 1024 else \
              {"week": w10, "seconds_of_week": int(sow), "era": era}
        rows.append((i, inp,
                     {"ok": True, "result.utc": text, "result.gps_week": float(week)},
                     {"result.gps_week": {"abs": 0}}, GPS_SRC, TZ_VERSION))
    return rows


def tz_rows(start):
    rows = []
    for i, (zone, text) in enumerate(TZ_CASES, start=start + 1):
        t = int(datetime.fromisoformat(text.replace("Z", "+00:00")).timestamp())
        local = datetime.fromtimestamp(t, ZoneInfo(zone))
        when, off = next_transition(zone, t)
        expect = {"ok": True, "result.offset": offset_text(local.utcoffset()),
                  "result.abbr": local.tzname(),
                  "result.dst": "yes" if local.dst() != timedelta(0) else "no"}
        if when is not None:
            expect["result.next_change"] = iso(when)
            expect["result.next_offset"] = offset_text(off)
        rows.append((i, {"zone": zone, "time": text}, expect,
                     {k: {"abs": 0} for k in expect if k != "ok"}, TZ_SRC, TZ_VERSION))
    return rows


GPS_SRC = ("TAI - UTC accumulated from the IANA tzdb leapseconds file (10 s at 1972-01-01); "
           "GPS - UTC = TAI - UTC - 19 s; week and seconds of week from the GPS epoch 1980-01-06")
TZ_SRC = "Python zoneinfo on the system IANA release: offset, abbreviation, DST state, and the next transition by bisection"


def main():
    plan = [
        ("time.scale.gps-week", gps_week_rows),
        ("time.scale.gps-to-utc", gps_to_utc_rows),
        ("time.scale.zone-info", tz_rows),
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
                    "source": src, "sourceVersion": f"IANA tzdb {ver}", "tolerance": tol,
                }) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
