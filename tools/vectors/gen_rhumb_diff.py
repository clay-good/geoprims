#!/usr/bin/env python3
"""Rhumb-line differential data and golden vectors from GeographicLib's
RhumbSolve (C++, the reference implementation of Karney's Rhumb): the data for
core/crates/gp-geo/tests/rhumb.rs, and core/vectors/navigation.rhumb.*.jsonl.

Writes core/crates/gp-geo/tests/data/rhumb_diff.csv: 2,000 seeded random pairs
(WGS 84), with extra near-east-west, near-meridian, and near-pole cases. Each
row: lat1,lon1,lat2,lon2,azi12,s12 from `RhumbSolve -i -p 12`. Requires
RhumbSolve on PATH (GeographicLib 2.x).
"""
import json
import random
import subprocess
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core/crates/gp-geo/tests/data/rhumb_diff.csv"


VECTORS = Path(__file__).resolve().parents[2] / "core/vectors"
SRC = "GeographicLib RhumbSolve (Karney's Rhumb), WGS 84"
VER = "GeographicLib 2.7"


def solve(args, lines):
    out = subprocess.run(["RhumbSolve", *args, "-p", "12"], input="".join(f"{l}\n" for l in lines), capture_output=True, text=True, check=True)
    return [list(map(float, l.split())) for l in out.stdout.splitlines()]


def geod(lines):
    out = subprocess.run(["GeodSolve", "-i", "-p", "12"], input="".join(f"{l}\n" for l in lines), capture_output=True, text=True, check=True)
    return [float(l.split()[2]) for l in out.stdout.splitlines()]


def vec(i, inp, exp, tol, src=SRC):
    e = dict(exp)
    e["ok"] = True
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": tol}


def vectors():
    rnd = random.Random(7)
    pairs = [(40.6413, -73.7781, 51.47, -0.4543), (0, 0, 0, 90), (10, 20, 50, 20), (-33.9, 151.2, -33.9, -70.6)]
    pairs += [(rnd.uniform(-80, 80), rnd.uniform(-180, 180), rnd.uniform(-80, 80), rnd.uniform(-180, 180)) for _ in range(18)]
    inv = solve(["-i"], [f"{a!r} {b!r} {c!r} {d!r}" for a, b, c, d in pairs])
    g = geod([f"{a!r} {b!r} {c!r} {d!r}" for a, b, c, d in pairs])
    out_i = []
    for i, ((a, b, c, d), (azi, s12, _), gs) in enumerate(zip(pairs, inv, g), 1):
        out_i.append(vec(i, {"lat1": a, "lon1": b, "lat2": c, "lon2": d},
                         {"result.distance.value": s12 / 1000, "result.course.value": azi % 360,
                          "result.geodesic_distance.value": gs / 1000, "result.extra_distance.value": max(s12 - gs, 0) / 1000},
                         {"result.distance.value": {"abs": 1e-9}, "result.course.value": {"abs": 1e-9},
                          "result.geodesic_distance.value": {"abs": 1e-9}, "result.extra_distance.value": {"abs": 2e-9}}))
    # The spec's JFK-LHR figures, as printed there.
    out_i[0]["expect"]["result.extra_percent"] = 3.95
    out_i[0]["tolerance"]["result.extra_percent"] = {"abs": 0.01}
    starts = [(40.6413, -73.7781, 78, 1_000_000), (0, 0, 90, 5_000_000), (60, 10, 270, 800_000), (-45, 170, 135, 3_000_000)]
    starts += [(rnd.uniform(-80, 80), rnd.uniform(-180, 180), rnd.uniform(0, 360), rnd.uniform(1e3, 1e7)) for _ in range(22)]
    ends = solve([], [f"{a!r} {b!r} {c!r} {d!r}" for a, b, c, d in starts])
    out_d = []
    # Random starts whose rhumb would cross a pole have no end (RhumbSolve prints nan).
    kept = [(st, en) for st, en in zip(starts, ends) if en[0] == en[0] and en[1] == en[1]]
    for i, ((a, b, c, d), (lat2, lon2, _)) in enumerate(kept, 1):
        out_d.append(vec(i, {"lat1": a, "lon1": b, "course": c, "distance": f"{d!r} m"},
                         {"result.lat2.value": lat2, "result.lon2.value": lon2},
                         {"result.lat2.value": {"abs": 1e-10}, "result.lon2.value": {"abs": 1e-10}}))
    # A rhumb due north from 80° N for 2,000 km stops at the pole (RhumbSolve gives the pole distance).
    (_, to_pole, _), = solve(["-i"], ["80 0 90 0"])
    out_d.append(vec(len(out_d) + 1, {"lat1": 80, "lon1": 0, "course": 0, "distance": "2000 km"},
                     {"result.lat2.value": 90.0, "result.beyond_pole.value": (2_000_000 - to_pole) / 1000, "meta.warnings.*.code": "RHUMB_REACHES_POLE"},
                     {"result.lat2.value": {"abs": 0}, "result.beyond_pole.value": {"abs": 1e-9}}))
    # Bowditch (NGA Pub. 9, 2024 edition, Art. 910, Mercator sailing examples 1 and 2). Bowditch takes a minute of
    # latitude as 1 nm; on WGS 84 a minute is 1,849 m at 35° and 1,861 m at 73°, so its distance and arrival
    # latitude differ from the ellipsoidal rhumb by up to 0.5%: 1.4 nm in example 1 and 1.0' in example 2.
    bow = ("Bowditch, The American Practical Navigator (NGA Pub. 9), Art. 910, Mercator sailing", "2024 edition")
    out_i.append({"id": f"v{len(out_i) + 1:03d}", "input": {"lat1": 32 + 14.7 / 60, "lon1": -(66 + 28.9 / 60), "lat2": 36 + 58.7 / 60, "lon2": -(75 + 42.2 / 60)},
                  "expect": {"result.course.value": 301.8, "result.distance.value": 538.7 * 1.852, "ok": True}, "source": bow[0], "sourceVersion": bow[1],
                  "tolerance": {"result.course.value": {"abs": 0.1}, "result.distance.value": {"abs": 1.5 * 1.852}}})
    out_d.append({"id": f"v{len(out_d) + 1:03d}", "input": {"lat1": 75 + 31.7 / 60, "lon1": -(79 + 8.7 / 60), "course": 155, "distance": "263.5 nmi"},
                  "expect": {"result.lat2.value": 71 + 32.9 / 60, "result.lon2.value": -(72 + 34.1 / 60), "ok": True}, "source": bow[0], "sourceVersion": bow[1],
                  "tolerance": {"result.lat2.value": {"abs": 1.2 / 60}, "result.lon2.value": {"abs": 1.8 / 60}}})
    for tool, vs in (("navigation.rhumb.inverse", out_i), ("navigation.rhumb.direct", out_d)):
        (VECTORS / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
    print(len(out_i), "inverse and", len(out_d), "direct vectors")


def main():
    rnd = random.Random(20260919)
    pts = []
    for _ in range(1600):
        pts.append((rnd.uniform(-89.9, 89.9), rnd.uniform(-180, 180), rnd.uniform(-89.9, 89.9), rnd.uniform(-180, 180)))
    for _ in range(200):  # nearly east-west: the divided differences matter here
        lat = rnd.uniform(-85, 85)
        pts.append((lat, rnd.uniform(-180, 180), lat + rnd.uniform(-1e-7, 1e-7), rnd.uniform(-180, 180)))
    for _ in range(100):  # nearly meridional
        lon = rnd.uniform(-180, 180)
        pts.append((rnd.uniform(-89, 89), lon, rnd.uniform(-89, 89), lon + rnd.uniform(-1e-7, 1e-7)))
    for _ in range(100):  # close to a pole
        pts.append((rnd.uniform(89.99, 89.9999), rnd.uniform(-180, 180), rnd.uniform(-89.9, 89.9), rnd.uniform(-180, 180)))
    text = "".join(f"{a!r} {b!r} {c!r} {d!r}\n" for a, b, c, d in pts)
    out = subprocess.run(["RhumbSolve", "-i", "-p", "12"], input=text, capture_output=True, text=True, check=True).stdout
    rows = ["lat1,lon1,lat2,lon2,azi12,s12"]
    for (a, b, c, d), line in zip(pts, out.splitlines()):
        azi, s12 = line.split()[:2]
        rows.append(f"{a!r},{b!r},{c!r},{d!r},{azi},{s12}")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(rows) + "\n")
    print(len(pts), "rows ->", OUT)
    vectors()


if __name__ == "__main__":
    main()
