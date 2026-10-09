#!/usr/bin/env python3
"""Golden vectors for geodesy.crs.search. The expected answers come from PROJ
itself, not from the tool's index: for a point, `projinfo --list-crs --bbox`
with PROJ's own area-of-use test (PROJ 9.9.0, EPSG v13.102), kept to the
families the index holds; for a code, `projinfo` naming it; and for SPCS2022,
the NGS bounds in the beta zone file. The order is the tool's documented one:
State Plane 1983, then 2022, then UTM, Web Mercator, and geographic, by code
within each.

Usage: python3 tools/vectors/gen_crs_search.py
"""
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/vectors/geodesy.crs.search.jsonl"
NGS = json.loads((ROOT / "assets/data/spcs2022-beta/2026-06-01/spcs2022-beta.json").read_text())["zones"]
# The SPCS83 zones' EPSG codes (the NAD83 metre CRS of each), for the family
# test: "NAD83 / <zone>" and its twins, with <zone> as EPSG names that CRS.
ZONE_CODES = re.findall(r'epsg: (\d+)', (ROOT / "core/crates/gp-geo/src/spcs83_zones.rs").read_text())
GEOGRAPHIC = {"4326", "4269", "6318", "4267"}
UTM = re.compile(r"^(WGS 84|NAD83|NAD83\(2011\)) / UTM zone \d+[NS]$")
SPCS = re.compile(r"^(NAD83|NAD83\(2011\)) / (.+?)( \(ftUS\)| \(ft\))?$")


def projinfo(*args):
    return subprocess.run(["projinfo", *args], check=True, capture_output=True, text=True).stdout


ZONE_NAMES = set()


def family(code, name):
    if code in GEOGRAPHIC:
        return 4, "Geographic"
    if code == "3857":
        return 3, "Web Mercator"
    if UTM.match(name):
        return 2, "UTM"
    m = SPCS.match(name)
    if m and m.group(2) in ZONE_NAMES:
        return 0, "State Plane 1983"
    return None


def epsg_at(lat, lon):
    out = []
    for kind in ("projected", "geographic"):
        listing = projinfo("--list-crs", kind, "--bbox", f"{lon},{lat},{lon},{lat}", "--spatial-test", "intersects")
        for code, name in re.findall(r'^EPSG:(\d+) "(.*)"$', listing, re.M):
            f = family(code, name)
            if f and (kind == "projected" or code in GEOGRAPHIC):
                out.append((f[0], f"{int(code):09}", f"EPSG:{code}", f[1]))
    return out


def wrap(x):
    return (x + 180) % 360 - 180


def ngs_at(lat, lon, words=None):
    out = []
    for z in NGS:
        w, s, e, n = z["bounds"]
        w, e, x = wrap(w), wrap(e), wrap(lon)
        inside = s <= lat <= n and ((w <= x <= e) if w <= e else (x >= w or x <= e))
        if inside:
            out.append((1, z["id"].split(":")[-1], z["id"], "State Plane 2022"))
    return out


def vector(inp, rows, note):
    rows = sorted(set(rows))
    exp = {"ok": True, "result.count": float(len(rows))}
    for i, r in enumerate(rows[:200]):
        exp[f"result.matches.{i}.code"] = r[2]
        exp[f"result.matches.{i}.kind"] = r[3]
    return {"input": inp, "expect": exp, "source": f"PROJ 9.9.0 (EPSG v13.102) and the NGS SPCS2022 beta bounds: {note}",
            "sourceVersion": "EPSG v13.102", "tolerance": {"result.count": {"rel": 1e-12, "abs": 1e-9}}}


def name_of(code):
    return json.loads(projinfo("-q", f"EPSG:{code}", "-o", "PROJJSON", "--single-line"))["name"]


def by_code(code, note):
    name = name_of(code)
    f = family(str(code), name)
    rows = [(f[0], f"{code:09}", f"EPSG:{code}", f[1])] if f else []
    v = vector({"query": str(code)}, rows, note)
    if rows:
        v["expect"]["result.matches.0.name"] = name
    return v


ZONE_NAMES.update(name_of(c).removeprefix("NAD83 / ") for c in ZONE_CODES)
assert len(ZONE_NAMES) == 124

POINTS = [
    (39.74, -104.99, "downtown Denver, the scenario point"),
    (21.31, -157.86, "Honolulu, where SPCS83 has no NAD83(2011) twin"),
    (52.0, 175.0, "the western Aleutians, across the antimeridian"),
    (64.84, -147.72, "Fairbanks"),
    (40.0, -40.0, "the open Atlantic: only the worldwide systems"),
    (48.85, 2.35, "Paris"),
    (-33.87, 151.21, "Sydney, in the south"),
    (18.47, -66.11, "San Juan, Puerto Rico"),
    (40.44, -79.99, "Pittsburgh, in Pennsylvania South"),
    (29.76, -95.37, "Houston"),
    (45.0, -108.0, "exactly on the UTM 12N and 13N line"),
]
CASES = []
for lat, lon, note in POINTS:
    CASES.append(vector({"lat": lat, "lon": lon}, epsg_at(lat, lon) + ngs_at(lat, lon), note))
for code, note in [(2232, "a feet code from a file's metadata"), (26913, "NAD83 UTM 13N"), (4326, "WGS 84"),
                   (6342, "NAD83(2011) UTM 13N"), (3857, "Web Mercator"), (32760, "WGS 84 UTM 60S")]:
    CASES.append(by_code(code, note))
CASES.append(vector({"query": "EPSG:2232"}, [(0, f"{2232:09}", "EPSG:2232", "State Plane 1983")], "the EPSG: prefix"))
CASES.append(vector({"query": "27700"}, [], "a code outside the index (British National Grid)"))
# Words: every word must be in the name or the kind's common names.
CASES.append(vector({"query": "UTM 13N"}, [(2, f"{c:09}", f"EPSG:{c}", "UTM") for c in (26913, 6342, 32613)], "three datums of one UTM zone"))
CASES.append(vector({"query": "Colorado Central ftUS"},
                    [(0, f"{c:09}", f"EPSG:{c}", "State Plane 1983") for c in (2232, 6428)], "a zone in US survey feet on both datums"))
CASES.append(vector({"query": "UTM", "lat": 39.74, "lon": -104.99},
                    [r for r in epsg_at(39.74, -104.99) if r[3] == "UTM"], "words and a point together"))
denver_ngs = [z for z in ngs_at(39.74, -104.99)]
CASES.append(vector({"query": "080001"}, [r for r in denver_ngs if r[1] == "080001"], "an SPCS2022 zone by its NGS code"))
CASES.append(vector({"query": "NGS:SPCS2022:081026"}, [r for r in denver_ngs if r[1] == "081026"], "an SPCS2022 zone by its full id"))

with OUT.open("w") as f:
    for i, row in enumerate(CASES, 1):
        f.write(json.dumps({"id": f"v{i:03d}", **row}) + "\n")
print(f"wrote {len(CASES)} vectors to {OUT}")
