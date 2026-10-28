#!/usr/bin/env python3
"""Golden vectors for the three moon tools of time/moon (add-celestial-and-moon
task 4), from USNO's Astronomical Applications API, read out of
core/crates/gp-time/tests/data/usno_moon.json:

- rstt/oneday with tz=0: moonrise, moonset, and upper transit of the UTC day,
  to the minute, at 500 places and dates from 1995 to 2045.
- moon/phases/date: the instants of 208 phases over the same span. At a phase
  instant the illuminated fraction is known without any further source: all
  but nothing at a new moon, all but everything at a full moon, and half at a
  quarter. Those are the vectors, so the fraction is checked against USNO's
  own instants rather than against this code.
- celnav: the moon's geocentric altitude and its own parallax in altitude at
  129 places and instants, whose difference is the topocentric altitude.

USNO prints its times to the minute and the tool is held to a minute of them
by core/crates/gp-time/tests/moon.rs over all 500 rows. A vector string
carries no tolerance, so the rise-and-set vectors below use only the rows
where the two agree on the minute exactly; the rows a minute apart are
checked by that test, not here.

    gen_moon.py time.moon.position
    gen_moon.py time.moon.rise-set
    gen_moon.py time.moon.phase

Writes nothing: new vectors are appended by hand so published files stay
frozen (see the repository's vector rules).
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
USNO = json.loads((ROOT / "core/crates/gp-time/tests/data/usno_moon.json").read_text())
TENTH = 0.1 / 60
SOURCE = "USNO Astronomical Applications API 4.0.1 (aa.usno.navy.mil/api), fetched 2026-10-28"
SPEC = "add-celestial-and-moon time/moon scenario"


def tol(pairs):
    return {k: {"abs": round(v, 6)} for k, v in pairs}


def position():
    rows = []
    for r in USNO["celnav"][::5]:
        utc = r["ut1"].replace(" ", "T") + "Z"
        expect = {
            "ok": True,
            "result.altitude_geometric.value": r["hc"] - r["pa"],
            "result.dec.value": r["dec"],
        }
        t = [("result.altitude_geometric.value", 0.05 / 60), ("result.dec.value", 0.02 / 60)]
        # USNO's azimuth is that of the geocentric place; the difference from
        # the topocentric azimuth grows as 1/cos(altitude) and is left alone
        # near the zenith, where azimuth itself is ill-conditioned.
        if r["hc"] - r["pa"] < 85.0:
            expect["result.azimuth.value"] = r["zn"]
            t.append(("result.azimuth.value", 0.4 / 60))
        rows.append({
            "input": {"lat": r["lat"], "lon": r["lon"], "time": utc},
            "expect": expect,
            "source": SOURCE + "; celnav geocentric altitude less its own parallax in altitude",
            "sourceVersion": "API 4.0.1",
            "tolerance": tol(t),
        })
    rows.append({
        "input": {"lat": 0.0, "lon": 0.0, "time": "1750-01-01T00:00:00Z"},
        "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
        "source": SPEC + ": the moon's place is computed from 1800 to 2100",
        "sourceVersion": "2026", "tolerance": {},
    })
    return rows


# The rows where USNO's printed minute and the tool's differ by the one minute
# core/crates/gp-time/tests/moon.rs allows over all 500. A vector string
# carries no tolerance, so they are left to that test rather than written here.
# They cluster at high latitudes, where the moon crosses the horizon at a
# shallow angle and a second of arc is many seconds of time: 15 of about 450.
A_MINUTE_APART = {
    ("2042-07-23", -37.5892, -90.6084),
    ("2013-07-19", 62.0372, -49.3164),
    ("2012-03-19", 66.3322, 154.7913),
    ("2016-11-22", -40.5329, 137.2842),
    ("2020-10-09", 48.6882, 139.9741),
    ("2042-12-12", -36.5349, 149.7298),
    ("2029-05-09", -65.4804, 5.1613),
    ("2026-07-06", 70.3743, -125.1524),
    ("2032-02-11", -61.2339, -97.837),
    ("1999-03-06", 43.741, -86.0012),
    ("2012-03-08", -41.944, -157.8987),
    ("2039-09-10", 51.0754, -101.0495),
    ("2034-02-14", -71.5719, -16.1444),
    ("1995-06-23", 75.1296, 135.7884),
    ("2033-01-16", -74.3157, -144.9201),
}


def event_string(date, hhmm):
    """`2026-06-21 18:59 local (2026-06-21 1859Z)` at zero offset."""
    return f"{date} {hhmm} local ({date} {hhmm.replace(':', '')}Z)"


def rise_set():
    rows = []
    for r in USNO["rstt"]:
        phen = {e["phen"]: e["time"] for e in r["events"]}
        payload = {"lat": r["lat"], "lon": r["lon"], "date": r["date"], "offset": "+00:00"}
        if "Object continuously above the Horizon" in phen:
            rows.append({
                "input": payload,
                "expect": {"ok": True, "result.state": "always-up",
                           "meta.warnings.*.code": "POLAR_MOON"},
                "source": SOURCE + "; rstt/oneday reports the moon continuously above the horizon",
                "sourceVersion": "API 4.0.1", "tolerance": {},
            })
            continue
        if "Object continuously below the Horizon" in phen:
            rows.append({
                "input": payload,
                "expect": {"ok": True, "result.state": "always-down",
                           "meta.warnings.*.code": "POLAR_MOON"},
                "source": SOURCE + "; rstt/oneday reports the moon continuously below the horizon",
                "sourceVersion": "API 4.0.1", "tolerance": {},
            })
            continue
        if "Rise" not in phen or "Set" not in phen:
            state = "no-moonrise" if "Rise" not in phen else "no-moonset"
            rows.append({
                "input": payload,
                "expect": {"ok": True, "result.state": state},
                "source": SOURCE + "; rstt/oneday lists no " + state[3:] + " on this UTC date",
                "sourceVersion": "API 4.0.1", "tolerance": {},
            })
            continue
        if (r["date"], r["lat"], r["lon"]) in A_MINUTE_APART:
            continue
        want_rise = event_string(r["date"], phen["Rise"])
        want_set = event_string(r["date"], phen["Set"])
        rows.append({
            "input": payload,
            "expect": {"ok": True, "result.moonrise": want_rise,
                       "result.moonset": want_set, "result.state": "normal"},
            "source": SOURCE + "; rstt/oneday rise and set of the UTC day",
            "sourceVersion": "API 4.0.1", "tolerance": {},
        })
        if len(rows) >= 26:
            break
    return rows


def phase():
    rows = []
    want = {"New Moon": 0.0, "First Quarter": 0.5, "Full Moon": 1.0, "Last Quarter": 0.5}
    names = {"New Moon": "new-moon", "First Quarter": "first-quarter",
             "Full Moon": "full-moon", "Last Quarter": "last-quarter"}
    for p in USNO["phases"][::7]:
        rows.append({
            "input": {"time": p["utc"]},
            "expect": {"ok": True, "result.illuminated": want[p["phase"]],
                       "result.phase_name": names[p["phase"]]},
            "source": (SOURCE + f"; moon/phases/date puts a {p['phase'].lower()} at this instant, "
                       "where the lit fraction of the disc is known from the phase itself"),
            "sourceVersion": "API 4.0.1",
            "tolerance": tol([("result.illuminated", 0.01)]),
        })
    rows.append({
        "input": {"time": "2200-01-01T00:00:00Z"},
        "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
        "source": SPEC + ": the moon's phase is computed from 1800 to 2100",
        "sourceVersion": "2026", "tolerance": {},
    })
    return rows


GENERATORS = {
    "time.moon.position": position,
    "time.moon.rise-set": rise_set,
    "time.moon.phase": phase,
}

if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in GENERATORS:
        sys.exit(f"usage: {sys.argv[0]} {{{'|'.join(GENERATORS)}}}")
    for i, row in enumerate(GENERATORS[sys.argv[1]](), start=1):
        print(json.dumps({"id": f"v{i:03d}", **row}, ensure_ascii=False))
