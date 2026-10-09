#!/usr/bin/env python3
"""Golden vectors for geodesy.crs.transform where PROJ is a fair oracle:
pairs of systems on the same frame, which PROJ 9.9.0 (cs2cs, EPSG v13.102)
joins by conversions alone, so its answer is the projections' and nothing
else. Between NAD83(2011) and WGS 84 PROJ applies a near-null "ballpark"
step instead of the frame chain, so those cases are checked in
core/crates/gp-geodesy/tests/crs.rs against the datum, State Plane, and UTM
tools they are built from, not here. Refusals are listed by rule.

Usage: python3 tools/vectors/gen_crs_transform.py
"""
import json
import subprocess
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core/vectors/geodesy.crs.transform.jsonl"
GEOGRAPHIC = {4326, 6318}


def cs2cs(src, dst, a, b):
    out = subprocess.run(["cs2cs", "-d", "10", f"EPSG:{src}", f"EPSG:{dst}"], input=f"{a} {b}\n",
                         check=True, capture_output=True, text=True).stdout.split()
    return float(out[0]), float(out[1])


def case(src, dst, a, b, note, tol=1e-6):
    inp = {"from": str(src), "to": str(dst)}
    if src in GEOGRAPHIC:
        inp.update(lat=a, lon=b)
    else:
        inp.update(easting=a, northing=b)
    x, y = cs2cs(src, dst, a, b)
    exp = {"ok": True, "result.accuracy.value": 0.0}
    if dst in GEOGRAPHIC:
        exp.update({"result.lat.value": x, "result.lon.value": y})
        t = {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}}
    else:
        exp.update({"result.easting.value": x, "result.northing.value": y})
        t = {"result.easting.value": {"abs": tol}, "result.northing.value": {"abs": tol}}
    t["result.accuracy.value"] = {"abs": 1e-12}
    return {"input": inp, "expect": exp, "source": f"PROJ 9.9.0 cs2cs EPSG:{src} EPSG:{dst} (EPSG v13.102): {note}",
            "sourceVersion": "PROJ 9.9.0", "tolerance": t}


def refused(inp, code, field, note):
    return {"input": inp, "expect": {"ok": False, "error.code": code, "error.field": field},
            "source": f"The tool's stated domain: {note}", "sourceVersion": "2026", "tolerance": {}}


CASES = [
    case(6427, 6342, 953000, 515000, "Colorado Central to UTM 13N, both NAD83(2011)"),
    case(6342, 6427, 495733.1057, 4397344.8706, "and back"),
    case(6428, 6318, 3126000, 1690000, "Colorado Central in US survey feet to latitude and longitude"),
    case(6318, 6428, 39.74, -104.99, "latitude and longitude to Colorado Central in US survey feet"),
    case(32613, 3857, 500000, 4400000, "UTM 13N to Web Mercator, both WGS 84"),
    case(3857, 4326, -11688546.5333, 4829665.7996, "Web Mercator to latitude and longitude"),
    case(4326, 32613, 39.74, -104.99, "latitude and longitude to UTM 13N"),
    case(32713, 4326, 500000, 6000000, "UTM 13S, in the south"),
    case(4326, 32760, -45.0, 177.5, "latitude and longitude to UTM 60S"),
    case(32613, 32612, 300000, 4400000, "into the next zone west, outside its area"),
    case(6405, 6318, 700000, 900000, "Arizona Central in international feet"),
    case(6318, 6342, 40.5, -106.2, "NAD83(2011) latitude and longitude to UTM 13N"),
    case(6342, 6318, 400000, 4500000, "UTM 13N to latitude and longitude on NAD83(2011)"),
    case(6427, 6429, 953000, 515000, "Colorado Central to Colorado North, the next zone"),
    case(32601, 4326, 500000, 7000000, "UTM 1N near the antimeridian"),
    refused({"from": "26913", "to": "32613", "easting": 500000, "northing": 4400000}, "OUT_OF_DOMAIN", "/from",
            "the original NAD83 moves by NADCON5, which is not chained"),
    refused({"from": "6342", "to": "4267", "easting": 500000, "northing": 4400000}, "OUT_OF_DOMAIN", "/to",
            "NAD27 moves by NADCON5, which is not chained"),
    refused({"from": "27700", "to": "4326", "easting": 500000, "northing": 200000}, "OUT_OF_DOMAIN", "/from",
            "British National Grid is not in the index"),
    refused({"from": "Colorado", "to": "4326", "easting": 1, "northing": 1}, "INVALID_INPUT", "/from",
            "a name is not a code"),
    refused({"from": "6342", "to": "32613", "easting": 500000, "northing": 4400000}, "INVALID_INPUT", "/epoch",
            "NAD83(2011) to WGS 84 changes frame, so it needs the epoch"),
]

with OUT.open("w") as f:
    for i, row in enumerate(CASES, 1):
        f.write(json.dumps({"id": f"v{i:03d}", **row}) + "\n")
print(f"wrote {len(CASES)} vectors to {OUT}")
