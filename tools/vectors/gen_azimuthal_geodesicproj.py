#!/usr/bin/env python3
"""Azimuthal equidistant and gnomonic against GeographicLib's GeodesicProj
(geodesy/projections, "General projection methods with custom parameters";
the gnomonic horizon scenario).

GeodesicProj -z (azimuthal equidistant) and -g (gnomonic) give x and y for
random centers and points on five ellipsoids. The convergence and the two
point scales are differences of GeodesicProj's own coordinates over
GeographicLib ground distances (about 100 m of ground each way),
Richardson-extrapolated so the curvature of the meridian and parallel images
cancels.

Writes core/crates/gp-geodesy/tests/data/projections_azimuthal.json (300 cases
per method) and appends vectors to the four tools' files (existing lines are
left byte for byte). Requires GeodesicProj (GeographicLib 2.x) and
geographiclib."""
import json
import math
import random
import subprocess
from pathlib import Path

from geographiclib.geodesic import Geodesic

FIX = Path("core/crates/gp-geodesy/tests/data/projections_azimuthal.json")
TAG = "gen_azimuthal_geodesicproj.py"
AF = {"wgs84": (6378137.0, "1/298.257223563"), "grs80": (6378137.0, "1/298.257222101"),
      "clarke1866": (6378206.4, repr((6378206.4 - 6356583.8) / 6378206.4)), "intl1924": (6378388.0, "1/297"),
      "bessel1841": (6377397.155, "1/299.1528128")}


def fval(s):
    n, _, d = s.partition("/")
    return float(n) / float(d) if d else float(n)


def version():
    out = subprocess.run(["GeodesicProj", "--version"], capture_output=True, text=True)
    return (out.stdout or out.stderr).split("\n")[0].strip()


def project_many(flag, lat0, lon0, ell, pts):
    a, f = AF[ell]
    lines = "".join(f"{la!r} {lo!r}\n" for la, lo in pts)
    out = subprocess.run(["GeodesicProj", flag, repr(lat0), repr(lon0), "-e", repr(a), f, "-p", "12"],
                         input=lines, capture_output=True, text=True, check=True).stdout
    return [tuple(float(v) for v in l.split()[:2]) for l in out.strip().split("\n")]


def factors(flag, lat0, lon0, ell, lat, lon):
    a, fs = AF[ell]
    f = fval(fs)
    geod = Geodesic(a, f)
    e2 = f * (2 - f)
    phi = math.radians(lat)
    r = a * math.cos(phi) / math.sqrt(1 - e2 * math.sin(phi) ** 2)
    dd, dl = 100 / 111_000, 100 / r
    pts = []
    for s in (dd, dd / 2):
        pts += [(lat - s, lon), (lat + s, lon)]
    for s in (dl, dl / 2):
        pts += [(lat, lon - math.degrees(s / 2)), (lat, lon + math.degrees(s / 2))]
    xy = project_many(flag, lat0, lon0, ell, pts)

    def chord(i):
        (e1, n1), (e2_, n2) = xy[i], xy[i + 1]
        return e2_ - e1, n2 - n1

    conv, h = [], []
    for i, s in ((0, dd), (2, dd / 2)):
        de, dn = chord(i)
        conv.append(-math.atan2(de, dn))
        h.append(math.hypot(de, dn) / geod.Inverse(lat - s, lon, lat + s, lon)["s12"])
    k = []
    for i, s in ((4, dl), (6, dl / 2)):
        de, dn = chord(i)
        k.append(math.hypot(de, dn) / (2 * r * math.sin(s / 2)))
    rich = lambda v: (4 * v[1] - v[0]) / 3  # noqa: E731
    return math.degrees(rich(conv)), rich(h), rich(k)


def main():
    rng = random.Random(8812)
    fixture, first = {}, {}
    for name, flag in (("azimuthal-equidistant", "-z"), ("gnomonic", "-g")):
        cases = []
        while len(cases) < 300:
            ell = rng.choice(list(AF))
            lat0 = round(rng.uniform(-89, 89), 4)
            lon0 = round(rng.uniform(-180, 180), 4)
            fe, fn_ = round(rng.uniform(0, 1e6), 3), round(rng.uniform(0, 1e6), 3)
            a, fs = AF[ell]
            geod = Geodesic(a, fval(fs))
            # Up to 150° of arc for the equidistant, 75° for the gnomonic;
            # points within a degree of either pole are skipped (their
            # east-west difference step is degenerate).
            reach = 150 if name == "azimuthal-equidistant" else 75
            d = geod.ArcDirect(lat0, lon0, rng.uniform(0, 360), rng.uniform(0, reach))
            lat, lon = round(d["lat2"], 9), round(d["lon2"], 9)
            if abs(lat) > 89:
                continue
            (x, y), = project_many(flag, lat0, lon0, ell, [(lat, lon)])
            conv, h, k = factors(flag, lat0, lon0, ell, lat, lon)
            params = {"latitude_of_origin": lat0, "longitude_of_origin": lon0, "false_easting": f"{fe} m",
                      "false_northing": f"{fn_} m", "ellipsoid": ell}
            cases.append({"params": params, "lat": lat, "lon": lon, "e": fe + x, "n": fn_ + y,
                          "convergence": conv, "h": h, "k": k})
        fixture[name] = cases
        first[name] = cases[:20]
    FIX.parent.mkdir(parents=True, exist_ok=True)
    FIX.write_text(json.dumps({"source": version(), "methods": fixture}, separators=(",", ":")) + "\n")

    src = f"GeographicLib {version()} ({TAG})"
    fwd_tol = {"result.easting.value": {"abs": 1e-6}, "result.northing.value": {"abs": 1e-6},
               "result.convergence.value": {"abs": 1e-7}, "result.scale_meridian": {"abs": 1e-9, "rel": 1e-8},
               "result.scale_parallel": {"abs": 1e-9, "rel": 1e-8}}
    inv_tol = {"result.lat.value": {"abs": 1e-10}, "result.lon.value": {"abs": 1e-10}}
    for name, cases in first.items():
        for direction in ("forward", "inverse"):
            path = Path(f"core/vectors/geodesy.projection.{name}-{direction}.jsonl")
            lines = path.read_text().splitlines() if path.exists() else []
            kept = [l for l in lines if TAG not in l]
            n0, new = len(kept), []
            for c in cases:
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
