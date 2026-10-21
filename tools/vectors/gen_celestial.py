#!/usr/bin/env python3
"""Golden vectors for navigation.celestial.almanac (add-celestial-and-moon,
navigation/celestial). Two independent sources:

- Bowditch 2024 (NGA Pub. 9) §2010 and §2011, which print Nautical Almanac
  values for 2024-06-02 (GHA Aries and four stars) and 2024-05-31 (the Sun).
  Table 2011's Sun GHA for 15h15m03s (49°22.0') is a misprint; USNO's
  celnav API gives 49.315819 deg for that second, used instead.
- USNO's celnav API (aa.usno.navy.mil/api/celnav, version 4.0.1), read from
  the gp-astro fixtures usno_celnav.json and usno_stars.json (times taken
  as UT1, so DUT1 is 0).

Tolerances: 0.1' (0.0017 deg), the spec's almanac tolerance, except Venus
(0.5') and the Moon (0.25'), where USNO itself departs from JPL Horizons
(the gp-astro tests and the change's design note).

    gen_celestial.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import re
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "core" / "crates" / "gp-astro" / "tests" / "data"
BODIES = ["Sun", "Moon", "Venus", "Mars", "Jupiter", "Saturn"]
STAR_NUMBER = {"Spica": 33, "Regulus": 26, "Procyon": 20, "Canopus": 17}
TENTH = 0.1 / 60


def dm(d, m):
    return (abs(d) + m / 60) * (-1 if d < 0 else 1)


def tol(fields, t=TENTH):
    return {f: {"abs": round(t, 6)} for f in fields}


rows = []
bow = "Bowditch 2024 (NGA Pub. 9) §2010, Tables 2010a and 2010b"
ex = {"ok": True, "result.aries_gha.value": dm(17, 18.7)}
for name, sha, dec in [("Spica", dm(158, 22.6), dm(-11, 17.4)), ("Regulus", dm(207, 35.1), dm(11, 50.9)),
                       ("Procyon", dm(244, 51.6), dm(5, 9.8)), ("Canopus", dm(263, 53.1), dm(-52, 42.6))]:
    n = STAR_NUMBER[name]
    ex[f"result.stars.{n}.sha.value"] = sha
    ex[f"result.stars.{n}.dec.value"] = dec
rows.append({"input": {"utc": "2024-06-02T08:24:03Z", "dut1": "0 s"}, "expect": ex,
             "source": bow + ": GHA Aries 11°17.0' at 08h plus 6°01.7', and the SHA and declination of Spica, Regulus, Procyon (Table 2010a), and Canopus",
             "sourceVersion": "2024 edition", "tolerance": tol([k for k in ex if k != "ok"])})
ex = {"ok": True, "result.sun_gha.value": dm(51, 36.5), "result.sun_dec.value": dm(22, 2.6)}
rows.append({"input": {"utc": "2024-05-31T15:24:13Z", "dut1": "0 s"}, "expect": ex,
             "source": "Bowditch 2024 (NGA Pub. 9) §2011, Table 2011: the Sun at 15h24m13s", "sourceVersion": "2024 edition",
             "tolerance": tol(["result.sun_gha.value", "result.sun_dec.value"])})
ex = {"ok": True, "result.sun_gha.value": 49.315819, "result.sun_dec.value": 22.042732}
rows.append({"input": {"utc": "2024-05-31T15:15:03Z", "dut1": "0 s"}, "expect": ex,
             "source": "USNO celnav API 4.0.1 for Bowditch 2024 Table 2011's first Sun sight (its printed GHA, 49°22.0', is a misprint), asked 2026-10-21",
             "sourceVersion": "API 4.0.1", "tolerance": tol(["result.sun_gha.value", "result.sun_dec.value"])})
bodies = json.loads((DATA / "usno_celnav.json").read_text())["rows"]
stars = json.loads((DATA / "usno_stars.json").read_text())["rows"]
text = (DATA.parents[1] / "src" / "star_tables.rs").read_text()
hip_number = {int(h): int(n) for n, h in re.findall(r"number: (\d+),\s*name: \"[^\"]+\",\s*hip: (\d+)", text)}
assert len(hip_number) == 58, len(hip_number)
instants = sorted({r["ut1"] for r in bodies})[::4]
for ut1 in instants:
    ex, t = {"ok": True}, {}
    for r in bodies:
        if r["ut1"] != ut1:
            continue
        i = BODIES.index(r["body"])
        lim = 0.5 / 60 if r["body"] == "Venus" else 0.25 / 60 if r["body"] == "Moon" else TENTH
        for k in ("gha", "dec"):
            ex[f"result.bodies.{i}.{k}.value"] = r[k]
            t[f"result.bodies.{i}.{k}.value"] = {"abs": round(lim, 6)}
    picked = [r for r in stars if r["ut1"] == ut1]
    for r in picked:
        if r.get("body") == "Aries":
            ex["result.aries_gha.value"] = r["gha"]
            t["result.aries_gha.value"] = {"abs": round(TENTH, 6)}
    for r in [r for r in picked if "hip" in r][:5]:
        n = hip_number[r["hip"]]
        ex[f"result.stars.{n}.dec.value"] = r["dec"]
        t[f"result.stars.{n}.dec.value"] = {"abs": round(TENTH, 6)}
    rows.append({"input": {"utc": ut1.replace(" ", "T") + ":00Z", "dut1": "0 s"}, "expect": ex,
                 "source": "USNO celnav API 4.0.1 (aa.usno.navy.mil/api/celnav), times as UT1, fetched 2026-10-20",
                 "sourceVersion": "API 4.0.1", "tolerance": t})
rows.append({"input": {"utc": "1799-12-31T12:00:00Z"}, "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
             "source": "add-celestial-and-moon navigation/celestial scenario: before 1800 is out of domain", "sourceVersion": "2026", "tolerance": {}})
for i, r in enumerate(rows, 1):
    print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))
