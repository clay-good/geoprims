#!/usr/bin/env python3
"""Transverse Mercator with user-set parameters against GeographicLib's
TransverseMercatorProj -s, the same sixth-order Krüger series in its
author's implementation (geodesy/projections, "General projection methods
with custom parameters"; add-geodesy-suite task 4.1).

Random central meridians, latitudes of origin, scale factors, falsings, and
ellipsoids, with points within 35° of longitude of the central meridian.
TransverseMercatorProj gives x, y, the convergence, and the scale; the
latitude of origin enters as the northing of (lat0, lon0), subtracted.

Writes core/crates/gp-geodesy/tests/data/projections_tm.json (300 cases) and
appends vectors to both tools' files (existing lines are left byte for
byte). Requires TransverseMercatorProj (GeographicLib 2.x)."""
import json
import random
import subprocess
from pathlib import Path

FIX = Path("core/crates/gp-geodesy/tests/data/projections_tm.json")
TAG = "gen_tm_geographiclib.py"
AF = {"wgs84": (6378137.0, "1/298.257223563"), "grs80": (6378137.0, "1/298.257222101"),
      "clarke1866": (6378206.4, repr((6378206.4 - 6356583.8) / 6378206.4)), "intl1924": (6378388.0, "1/297"),
      "airy1830": (6377563.396, "1/299.3249646")}


def version():
    out = subprocess.run(["TransverseMercatorProj", "--version"], capture_output=True, text=True)
    return (out.stdout or out.stderr).split("\n")[0].strip()


def tm(lon0, k0, ell, pts):
    a, f = AF[ell]
    lines = "".join(f"{la!r} {lo!r}\n" for la, lo in pts)
    out = subprocess.run(["TransverseMercatorProj", "-s", "-l", repr(lon0), "-k", repr(k0), "-e", repr(a), f, "-p", "12"],
                         input=lines, capture_output=True, text=True, check=True).stdout
    return [tuple(float(v) for v in l.split()) for l in out.strip().split("\n")]


def main():
    rng = random.Random(5150)
    cases = []
    for _ in range(300):
        ell = rng.choice(list(AF))
        lon0 = round(rng.uniform(-180, 180), 4)
        lat0 = round(rng.uniform(-60, 60), 4)
        k0 = round(rng.uniform(0.9990, 1.0), 10)
        fe, fn_ = round(rng.uniform(0, 1e6), 3), round(rng.uniform(-1e7, 1e7), 3)
        lat = round(rng.uniform(-85, 85), 9)
        lon = round(((lon0 + rng.uniform(-35, 35)) + 180) % 360 - 180, 9)
        (x, y, gam, k), (_, y0, _, _) = tm(lon0, k0, ell, [(lat, lon), (lat0, lon0)])
        params = {"longitude_of_origin": lon0, "latitude_of_origin": lat0, "scale_factor": k0,
                  "false_easting": f"{fe} m", "false_northing": f"{fn_} m", "ellipsoid": ell}
        cases.append({"params": params, "lat": lat, "lon": lon, "e": fe + x, "n": fn_ + y - y0,
                      "convergence": gam, "h": k, "k": k})
    FIX.parent.mkdir(parents=True, exist_ok=True)
    FIX.write_text(json.dumps({"source": version(), "methods": {"tm": cases}}, separators=(",", ":")) + "\n")
    src = f"GeographicLib {version()} TransverseMercatorProj -s ({TAG})"
    fwd_tol = {"result.easting.value": {"abs": 1e-6}, "result.northing.value": {"abs": 1e-6},
               "result.convergence.value": {"abs": 1e-9}, "result.scale_meridian": {"abs": 1e-12}, "result.scale_parallel": {"abs": 1e-12}}
    inv_tol = {"result.lat.value": {"abs": 1e-10}, "result.lon.value": {"abs": 1e-10}}
    for direction in ("forward", "inverse"):
        path = Path(f"core/vectors/geodesy.projection.tm-{direction}.jsonl")
        lines = path.read_text().splitlines() if path.exists() else []
        kept = [l for l in lines if TAG not in l]
        n0, new = len(kept), []
        for c in cases[:20]:
            if direction == "forward":
                inp = {"lat": c["lat"], "lon": c["lon"], **c["params"]}
                exp = {"result.easting.value": c["e"], "result.northing.value": c["n"], "result.convergence.value": c["convergence"],
                       "result.scale_meridian": c["h"], "result.scale_parallel": c["k"], "ok": True}
                tol = fwd_tol
            else:
                inp = {"easting": f"{c['e']!r} m", "northing": f"{c['n']!r} m", **c["params"]}
                exp = {"result.lat.value": c["lat"], "result.lon.value": c["lon"], "ok": True}
                tol = inv_tol
            new.append({"id": f"v{n0 + len(new) + 1:03d}", "input": inp, "expect": exp, "source": src,
                        "sourceVersion": version(), "tolerance": tol})
        path.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
        print(path.name, n0, "->", n0 + len(new))


if __name__ == "__main__":
    main()
