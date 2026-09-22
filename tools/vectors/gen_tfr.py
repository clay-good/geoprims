#!/usr/bin/env python3
"""Golden vectors for TFR and NOTAM areas: packed coordinates decoded by hand,
the 72-point geodesic circle from GeographicLib's GeodSolve, and every area
from GeographicLib's Planimeter (C++), independent of the tool's
geographiclib-rs. Requires GeographicLib."""
import json
import subprocess
import sys
from pathlib import Path

VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"
SRC = "GeographicLib GeodSolve and Planimeter (tools/vectors/gen_tfr.py)"
SPEC = "add-practitioner-essentials TFR scenario"
NM = 1852.0


def fixed(*xs):
    return " ".join(f"{float(x):.15f}" for x in xs)


def direct(lat, lon, az, s):
    return [float(x) for x in subprocess.run(["GeodSolve", "-p", "12"], input=fixed(lat, lon, az, s) + "\n", capture_output=True, text=True, check=True).stdout.split()][:2]


def area(pts):
    o = subprocess.run(["Planimeter", "-p", "9"], input="".join(fixed(a, b) + "\n" for a, b in pts), capture_output=True, text=True, check=True).stdout.split()
    return abs(float(o[2]))


def vec(i, inp, exp, src=SRC, tol=1e-6):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": 1e-9 if k.split(".")[1] in ("south", "north", "west", "east", "center_lat", "center_lon") else tol}
         for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": t}


def circle(c, r_nm):
    return [direct(c[0], c[1], 360 - 5 * k, r_nm * NM) for k in range(72)]


def bounds(pts):
    return {"result.south.value": min(p[0] for p in pts), "result.north.value": max(p[0] for p in pts),
            "result.west.value": min(p[1] for p in pts), "result.east.value": max(p[1] for p in pts)}


def cases():
    out = []
    c = (39 + 34 / 60, -(122 + 43 / 60 + 30 / 3600))
    ring = circle(c, 3)
    out.append(vec(1, {"center": "393400N1224330W", "radius": "3 NM", "floor": "SFC", "ceiling": "3000 FT MSL"},
                   {"result.area.value": area(ring) / NM ** 2, "result.center_lat.value": c[0], "result.center_lon.value": c[1],
                    "result.floor": "SFC", "result.ceiling": "3000 ft MSL", **bounds(ring)}, SPEC + " (3 NM geodesic circle, SFC to 3,000 ft MSL)"))
    c2 = (-(33 + 56 / 60), 151 + 10 / 60)
    ring = circle(c2, 10)
    out.append(vec(2, {"center": "3356S15110E", "radius": "10 NM", "ceiling": "FL180"}, {"result.area.value": area(ring) / NM ** 2, "result.ceiling": "FL180", **bounds(ring)}))
    # Fix-radial-distance: radial 12, 98.7 NM from a navaid at 37.72, -122.22 with 14 deg E variation.
    fc = direct(37.72, -122.22, 26, 98.7 * NM)
    ring = circle(fc, 5)
    out.append(vec(3, {"center": "ABC012098.7", "radius": "5 NM", "navaid_lat": 37.72, "navaid_lon": -122.22, "navaid_variation": "14 deg"},
                   {"result.center_lat.value": fc[0], "result.center_lon.value": fc[1], "result.area.value": area(ring) / NM ** 2}))
    pts = [(39.5, -105.0), (39.5, -104.5), (40.0, -104.5), (40.0, -105.0)]
    packed = ["393000N1050000W", "393000N1043000W", "400000N1043000W", "400000N1050000W"]
    out.append(vec(4, {"points": [{"point": p} for p in packed], "floor": "500 AGL", "ceiling": "12,000 ft"},
                   {"result.area.value": area(pts) / NM ** 2, "result.floor": "500 ft AGL", "result.ceiling": "12000 ft MSL", **bounds(pts)}))
    for inp in [{"center": "396000N1224330W", "radius": "3 NM"}, {"center": "ABC012098.7", "radius": "5 NM"},
                {"center": "393400N1224330W", "radius": "3 NM", "ceiling": "very high"},
                {"points": [{"point": "393000N1050000W"}, {"point": "393000N1043000W"}, {"point": "393000N1050000W"}]}]:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "aviation.airspace.tfr-area.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in cases()))


if __name__ == "__main__":
    main()
