#!/usr/bin/env python3
"""Golden vectors for the three sun tools, from pvlib's NREL SPA.

`time.sun.hotspot`, `time.sun.mapping-window` and `time.sun.night-currency`
carried five or six vectors each. This brings each past twenty.

The sun comes from pvlib, a separate implementation of the same NREL Solar
Position Algorithm the core uses; over 2,000 points the two agree within two
arcseconds (`core/crates/gp-time/tests/spa_parity.rs`). Everything built on top
of the sun is worked out here rather than taken from the core:

- hotspot: the antisolar direction is opposite the sun at minus its elevation,
  and the hotspot angle is the angle between that and the camera's look vector,
  from a dot product of two unit vectors. In frame when within half the
  diagonal field of view.
- mapping-window: the two instants the geometric elevation crosses the
  threshold, found by bisecting either side of the day's highest sun.
- night-currency: 14 CFR 61.57(b) applied by hand. An event counts when it
  falls from an hour after that evening's sunset to an hour before the next
  sunrise, sunset and sunrise being where the sun's centre is at -0.8333 deg;
  currency runs through the 90th day after the older of the third most recent
  qualifying takeoff and landing.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_sun_pvlib.py
"""
import json
import math
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pandas as pd
import pvlib

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
VER = f"pvlib {pvlib.__version__}"
SUN_SRC = "Sun position from pvlib's NREL SPA (sea level, 101325 Pa, 12 C)"
HOT_SRC = f"{SUN_SRC}; antisolar point and hotspot angle by vector geometry (tools/vectors/gen_sun_pvlib.py)"
WIN_SRC = f"{SUN_SRC}; threshold crossings by bisection (tools/vectors/gen_sun_pvlib.py)"
CUR_SRC = f"{SUN_SRC}; 14 CFR 61.57(b) applied by hand (tools/vectors/gen_sun_pvlib.py)"


def sun(lat, lon, when):
    """(apparent elevation, azimuth, geometric elevation) in degrees."""
    ts = pd.DatetimeIndex([pd.Timestamp(when)])
    r = pvlib.solarposition.spa_python(ts, lat, lon, altitude=0, pressure=101325, temperature=12)
    return (float(r["apparent_elevation"].iloc[0]), float(r["azimuth"].iloc[0]),
            float(r["elevation"].iloc[0]))


def unit(az, el):
    a, e = math.radians(az), math.radians(el)
    return (math.cos(e) * math.sin(a), math.cos(e) * math.cos(a), math.sin(e))


def angle_between(u, v):
    d = max(-1.0, min(1.0, sum(x * y for x, y in zip(u, v))))
    return math.degrees(math.acos(d))


# (lat, lon, ISO time with offset, camera heading deg or None, camera pitch deg)
HOTSPOT_CASES = [
    (39.74, -104.99, "2026-06-21T15:00:00-06:00", None, -90.0),
    (39.74, -104.99, "2026-06-21T07:30:00-06:00", None, -90.0),
    (39.74, -104.99, "2026-12-21T12:00:00-07:00", None, -90.0),
    (39.74, -104.99, "2026-06-21T12:00:00-06:00", 0.0, -60.0),
    (39.74, -104.99, "2026-06-21T12:00:00-06:00", 180.0, -60.0),
    (51.48, -0.12, "2026-05-15T13:00:00+01:00", None, -90.0),
    (51.48, -0.12, "2026-05-15T13:00:00+01:00", 90.0, -45.0),
    (-33.87, 151.21, "2026-01-15T13:00:00+11:00", None, -90.0),
    (-33.87, 151.21, "2026-07-15T12:00:00+10:00", None, -90.0),
    (1.35, 103.82, "2026-03-21T13:00:00+08:00", None, -90.0),
    (1.35, 103.82, "2026-03-21T08:00:00+08:00", 270.0, -30.0),
    (64.13, -21.90, "2026-06-21T12:00:00+00:00", None, -90.0),
    (64.13, -21.90, "2026-06-21T23:00:00+00:00", None, -90.0),
    (-22.91, -43.17, "2026-09-18T12:00:00-03:00", None, -90.0),
    (35.68, 139.69, "2026-04-10T10:00:00+09:00", 45.0, -70.0),
    (47.62, -122.35, "2026-08-05T17:00:00-07:00", None, -90.0),
    (25.20, 55.27, "2026-11-11T09:30:00+04:00", 315.0, -55.0),
]
DIAGONAL_FOV = 84.0  # the tool's default, in degrees

# (lat, lon, date, UTC offset, threshold deg or None for the 30 deg default)
WINDOW_CASES = [
    (39.7392, -104.9903, "2026-03-21", "-06:00", None),
    (39.7392, -104.9903, "2026-12-21", "-07:00", None),
    (39.7392, -104.9903, "2026-06-21", "-06:00", 60.0),
    (51.5074, -0.1278, "2026-06-21", "+01:00", None),
    (51.5074, -0.1278, "2026-09-18", "+01:00", 20.0),
    (-33.8688, 151.2093, "2026-01-15", "+11:00", None),
    (-33.8688, 151.2093, "2026-06-21", "+10:00", 20.0),
    (1.3521, 103.8198, "2026-03-21", "+08:00", None),
    (1.3521, 103.8198, "2026-03-21", "+08:00", 70.0),
    (25.2048, 55.2708, "2026-11-11", "+04:00", None),
    (35.6762, 139.6503, "2026-04-10", "+09:00", None),
    (47.6062, -122.3321, "2026-08-05", "-07:00", None),
    (-22.9068, -43.1729, "2026-09-18", "-03:00", None),
    (19.4326, -99.1332, "2026-05-05", "-05:00", 45.0),
    (-1.2921, 36.8219, "2026-07-01", "+03:00", None),
    (30.0444, 31.2357, "2026-02-14", "+02:00", None),
]


def window(lat, lon, date, offset, threshold):
    """(start, end, max elevation) as UTC instants, by bisection on the SPA."""
    base = datetime.fromisoformat(f"{date}T00:00:00{offset}")
    thr = 30.0 if threshold is None else threshold

    def el(t):
        return sun(lat, lon, t.astimezone(timezone.utc).isoformat())[2]

    def apparent(t):
        return sun(lat, lon, t.astimezone(timezone.utc).isoformat())[0]

    # The day's highest sun, to the second, by ternary search over local noon.
    lo, hi = base, base + timedelta(days=1)
    for _ in range(80):
        a = lo + (hi - lo) / 3
        b = hi - (hi - lo) / 3
        if el(a) < el(b):
            lo = a
        else:
            hi = b
    peak = lo + (hi - lo) / 2
    # The crossings are solved on the geometric sun, which is what the
    # threshold means; the elevation reported is the apparent one.
    top, top_apparent = el(peak), apparent(peak)
    if top < thr:
        return None, None, top_apparent

    def cross(t0, t1):
        for _ in range(60):
            mid = t0 + (t1 - t0) / 2
            if (el(mid) >= thr) == (el(t0) >= thr):
                t0 = mid
            else:
                t1 = mid
        return t0 + (t1 - t0) / 2

    start = cross(base, peak)
    end = cross(base + timedelta(days=1), peak)
    return start, end, top_apparent


def sun_event(lat, lon, day, rising):
    """The instant the sun's centre is at -0.8333 deg, by bisection."""
    base = datetime.fromisoformat(f"{day}T00:00:00+00:00")

    def el(t):
        return sun(lat, lon, t.isoformat())[2]

    lo, hi = (base, base + timedelta(days=1))
    # Step over the day in 10-minute jumps to find the bracketing pair.
    prev = base
    step = timedelta(minutes=10)
    t = base + step
    want = -0.8333
    while t <= base + timedelta(days=1):
        a, b = el(prev), el(t)
        if (a < want <= b) if rising else (a > want >= b):
            lo, hi = prev, t
            break
        prev, t = t, t + step
    for _ in range(40):
        mid = lo + (hi - lo) / 2
        if (el(mid) < want) == (el(lo) < want):
            lo = mid
        else:
            hi = mid
    return lo + (hi - lo) / 2


def iso(t):
    return t.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def event_text(t, offset):
    """The tool's own wording: local time, then the same instant in Zulu.

    Rounded to the nearest minute, which is the accuracy the tool claims."""
    t = t + timedelta(seconds=30)
    t = t.replace(second=0, microsecond=0)
    local = t.astimezone(datetime.fromisoformat(f"2000-01-01T00:00:00{offset}").tzinfo)
    z = t.astimezone(timezone.utc)
    return f"{local:%Y-%m-%d %H:%M} local ({z:%Y-%m-%d %H%M}Z)"


def hotspot_rows(start):
    rows = []
    for i, (lat, lon, when, heading, pitch) in enumerate(HOTSPOT_CASES, start=start + 1):
        el, az, _ = sun(lat, lon, when)
        anti_az = (az + 180.0) % 360.0
        # The tool defaults the heading to 0; every case that leaves it out
        # points straight down, where the heading makes no difference anyway.
        look_az = 0.0 if heading is None else heading
        angle = angle_between(unit(look_az, pitch), unit(anti_az, -el))
        inp = {"lat": lat, "lon": lon, "time": when, "camera_pitch": f"{pitch:g} deg"}
        if heading is not None:
            inp["camera_heading"] = f"{heading:g} deg"
        rows.append((i, inp,
                     {"ok": True, "result.hotspot_angle.value": angle,
                      "result.sun_elevation.value": el, "result.sun_azimuth.value": az,
                      "result.antisolar_azimuth.value": anti_az,
                      "result.in_frame": "yes" if angle <= DIAGONAL_FOV / 2 else "no"},
                     {"result.hotspot_angle.value": {"abs": 3e-3},
                      "result.sun_elevation.value": {"abs": 3e-3},
                      "result.sun_azimuth.value": {"abs": 3e-3},
                      "result.antisolar_azimuth.value": {"abs": 3e-3},
                      "result.in_frame": {"abs": 0}},
                     HOT_SRC))
    return rows


def window_rows(start):
    rows = []
    i = start
    for lat, lon, date, offset, threshold in WINDOW_CASES:
        i += 1
        s, e, top = window(lat, lon, date, offset, threshold)
        inp = {"lat": lat, "lon": lon, "date": date, "offset": offset}
        if threshold is not None:
            inp["threshold"] = f"{threshold:g} deg"
        expect = {"ok": True, "result.max_elevation.value": top}
        tol = {"result.max_elevation.value": {"abs": 1e-3}}
        if s is not None:
            # The crossing is where an elevation curve meets a level, so a
            # thousandth of a degree of sun is a couple of seconds of clock.
            expect["result.window_start"] = event_text(s, offset)
            expect["result.window_end"] = event_text(e, offset)
            tol["result.window_start"] = {"abs": 0}
            tol["result.window_end"] = {"abs": 0}
        rows.append((i, inp, expect, tol, WIN_SRC))
    return rows


def main():
    plan = [("time.sun.hotspot", hotspot_rows), ("time.sun.mapping-window", window_rows),
            ("time.sun.night-currency", currency_rows)]
    for tool, build in plan:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 8:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol, src in rows:
                f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                    "source": src, "sourceVersion": VER, "tolerance": tol}) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()


# ------------------------------------------------------------ night currency

def crossing_after(lat, lon, t0, rising, limit_hours=30):
    """First instant after t0 where the sun's centre crosses -0.8333 deg."""
    want = -0.8333

    def el(t):
        return sun(lat, lon, t.astimezone(timezone.utc).isoformat())[2]

    step = timedelta(minutes=10)
    prev, t = t0, t0 + step
    end = t0 + timedelta(hours=limit_hours)
    while t <= end:
        a, b = el(prev), el(t)
        if (a < want <= b) if rising else (a > want >= b):
            lo, hi = prev, t
            for _ in range(40):
                mid = lo + (hi - lo) / 2
                if (el(mid) < want) == (el(lo) < want):
                    lo = mid
                else:
                    hi = mid
            return lo + (hi - lo) / 2
        prev, t = t, t + step
    return None


def night_window(lat, lon, when):
    """(start, end) of the 61.57(b) period for the night `when` falls in.

    An event before local noon belongs to the night that began the evening
    before, so the search starts from noon on that evening's date."""
    local_noon = when.replace(hour=12, minute=0, second=0, microsecond=0)
    if when < local_noon:
        local_noon -= timedelta(days=1)
    sunset = crossing_after(lat, lon, local_noon, rising=False)
    if sunset is None:
        return None, None
    sunrise = crossing_after(lat, lon, sunset, rising=True)
    if sunrise is None:
        return None, None
    return sunset + timedelta(hours=1), sunrise - timedelta(hours=1)


def counts(lat, lon, when):
    start, end = night_window(lat, lon, when)
    return start is not None and start <= when <= end


DEN = (39.86, -104.67)  # Denver Front Range, the place the first vectors used
SEA = (47.53, -122.30)  # Boeing Field, a later sunset and a shorter night
ANC = (61.17, -150.00)  # Anchorage: in June the night never qualifies at all


def ev(place, when, takeoffs=1, landings=1, aircraft="ASEL"):
    lat, lon = place
    return {"when": when, "lat": lat, "lon": lon,
            "takeoffs": takeoffs, "landings": landings, "aircraft": aircraft}


def qualifying(events, aircraft, as_of=None):
    """Qualifying takeoffs and landings for one category and class.

    With `as_of`, only those in the preceding 90 days -- which is what the
    tool's count means, and why an event exactly 91 days back stops counting
    while the currency it started still runs to its own 90th day."""
    out = {"takeoffs": [], "landings": []}
    for e in events:
        if e["aircraft"] != aircraft:
            continue
        when = datetime.fromisoformat(e["when"])
        if not counts(e["lat"], e["lon"], when):
            continue
        if as_of is not None and not (as_of - timedelta(days=90) <= when.date() <= as_of):
            continue
        for kind in out:
            out[kind] += [when.date()] * e[kind]
    return out


def through_date(events, aircraft):
    """The 90th day after the older of the third most recent qualifying
    takeoff and the third most recent qualifying full-stop landing.

    Counted per category and class: a multi-engine night does nothing for
    single-engine currency, so the events are grouped before they are counted."""
    q = qualifying(events, aircraft)
    thirds = []
    for kind in ("takeoffs", "landings"):
        days = sorted(q[kind], reverse=True)
        if len(days) < 3:
            return None
        thirds.append(days[2])
    return min(thirds) + timedelta(days=90)


# (as_of, events) -- each event sits at least 20 minutes inside or outside its
# boundary, so a yes/no never hangs on the solver's last minute.
CURRENCY_CASES = [
    # Three qualifying nights, checked inside and outside the 90 days.
    ("2026-06-10", [ev(DEN, "2026-05-01T22:30:00-06:00"), ev(DEN, "2026-05-10T22:30:00-06:00"),
                    ev(DEN, "2026-06-02T23:00:00-06:00")]),
    ("2026-07-31", [ev(DEN, "2026-05-01T22:30:00-06:00"), ev(DEN, "2026-05-10T22:30:00-06:00"),
                    ev(DEN, "2026-06-02T23:00:00-06:00")]),
    # Two is not three.
    ("2026-06-10", [ev(DEN, "2026-05-01T22:30:00-06:00"), ev(DEN, "2026-05-10T22:30:00-06:00")]),
    # An hour after sunset is the boundary: 20:30 is before it, 22:30 after.
    ("2026-06-10", [ev(DEN, "2026-05-01T20:30:00-06:00"), ev(DEN, "2026-05-10T22:30:00-06:00"),
                    ev(DEN, "2026-06-02T23:00:00-06:00")]),
    # A time before noon belongs to the night that began the evening before:
    # 04:00 still counts, 05:30 is inside the hour before sunrise and does not.
    ("2026-06-10", [ev(DEN, "2026-05-02T04:00:00-06:00"), ev(DEN, "2026-05-11T04:00:00-06:00"),
                    ev(DEN, "2026-06-03T04:00:00-06:00")]),
    ("2026-06-10", [ev(DEN, "2026-05-02T05:30:00-06:00"), ev(DEN, "2026-05-11T04:00:00-06:00"),
                    ev(DEN, "2026-06-03T04:00:00-06:00")]),
    # Takeoffs and landings are counted separately, so three of one and two of
    # the other is not currency.
    ("2026-06-10", [ev(DEN, "2026-05-01T22:30:00-06:00", landings=0),
                    ev(DEN, "2026-05-10T22:30:00-06:00"), ev(DEN, "2026-06-02T23:00:00-06:00")]),
    # Several in one night count for as many as were flown.
    ("2026-06-10", [ev(DEN, "2026-06-02T23:00:00-06:00", takeoffs=3, landings=3)]),
    # Category and class are counted apart.
    ("2026-06-10", [ev(DEN, "2026-05-01T22:30:00-06:00"), ev(DEN, "2026-05-10T22:30:00-06:00"),
                    ev(DEN, "2026-06-02T23:00:00-06:00"), ev(DEN, "2026-06-02T23:30:00-06:00", aircraft="AMEL")]),
    ("2026-06-10", [ev(DEN, "2026-05-01T22:30:00-06:00", aircraft="AMEL"),
                    ev(DEN, "2026-05-10T22:30:00-06:00", aircraft="AMEL"),
                    ev(DEN, "2026-06-02T23:00:00-06:00", aircraft="AMEL")]),
    # Somewhere else, with a later sunset.
    ("2026-06-10", [ev(SEA, "2026-05-01T23:30:00-07:00"), ev(SEA, "2026-05-10T23:30:00-07:00"),
                    ev(SEA, "2026-06-02T23:59:00-07:00")]),
    ("2026-06-10", [ev(SEA, "2026-05-01T21:30:00-07:00"), ev(SEA, "2026-05-10T23:30:00-07:00"),
                    ev(SEA, "2026-06-02T23:59:00-07:00")]),
    # Winter, when the window is wide open.
    ("2026-03-01", [ev(DEN, "2026-01-05T19:30:00-07:00"), ev(DEN, "2026-01-15T19:30:00-07:00"),
                    ev(DEN, "2026-02-01T19:30:00-07:00")]),
    ("2026-03-01", [ev(SEA, "2026-01-05T18:30:00-08:00"), ev(SEA, "2026-01-15T18:30:00-08:00"),
                    ev(SEA, "2026-02-01T18:30:00-08:00")]),
    # A mix of places on one logbook.
    ("2026-06-10", [ev(DEN, "2026-05-01T22:30:00-06:00"), ev(SEA, "2026-05-10T23:30:00-07:00"),
                    ev(DEN, "2026-06-02T23:00:00-06:00")]),
    # The day currency lapses, and the day after.
    ("2026-07-30", [ev(DEN, "2026-05-01T22:30:00-06:00"), ev(DEN, "2026-05-10T22:30:00-06:00"),
                    ev(DEN, "2026-06-02T23:00:00-06:00")]),
]


def currency_rows(start):
    rows = []
    for i, (as_of, events) in enumerate(CURRENCY_CASES, start=start + 1):
        # `through` is reported for the FIRST category and class in the log.
        first = events[0]["aircraft"]
        through = through_date(events, first)
        q = qualifying(events, first, datetime.fromisoformat(as_of).date())
        current = through is not None and datetime.fromisoformat(as_of).date() <= through
        expect = {"ok": True, "result.current": "yes" if current else "no",
                  "result.state": 1.0 if current else 2.0}
        tol = {"result.current": {"abs": 0}, "result.state": {"abs": 0}}
        expect["result.aircraft.0.takeoffs"] = float(len(q["takeoffs"]))
        expect["result.aircraft.0.landings"] = float(len(q["landings"]))
        tol["result.aircraft.0.takeoffs"] = {"abs": 0}
        tol["result.aircraft.0.landings"] = {"abs": 0}
        if through is not None:
            expect["result.through"] = through.isoformat()
            tol["result.through"] = {"abs": 0}
        for k, e in enumerate(events):
            expect[f"result.events.{k}.counts"] = "yes" if counts(e["lat"], e["lon"], datetime.fromisoformat(e["when"])) else "no"
            tol[f"result.events.{k}.counts"] = {"abs": 0}
        rows.append((i, {"as_of": as_of, "events": events}, expect, tol, CUR_SRC))
    return rows
