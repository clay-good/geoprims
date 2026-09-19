#!/usr/bin/env python3
"""Golden vectors for navigation.geodesic.* (core/vectors/navigation.*.jsonl).

Sources, all independent of the Rust core:
- Published GeographicLib values quoted in the add-navigation-and-geometry
  spec scenarios (JFK-LHR, the nearly antipodal pair, the direct problem).
- Closed forms: along the equator a geodesic is the equator itself
  (s = a * dlon for |dlon| <= (1 - f) * 180 deg), and pole to pole is twice
  the WGS 84 meridian quadrant 10,001,965.729313 m (Karney 2011, GeodTest).
- Haversine by direct evaluation in Python.
- Karney's GeodTest-short.dat (GeographicLib test data, 2010): geodesics
  computed with high-precision arithmetic, accurate to 0.1 nm. A 1,000-line
  sample (every 10th line) is committed at
  core/crates/gp-navigation/tests/data/GeodTest-sample.dat; vectors v007+
  (inverse) and v006+ (direct) are drawn from it. Inverse vectors skip nearly
  antipodal lines, whose azimuths are ill-conditioned in the 12-decimal
  endpoints.
"""
import json
import math
import sys
from pathlib import Path

A = 6378137.0
Q = 10001965.729313  # WGS 84 meridian quadrant (m)
SPEC = "add-navigation-and-geometry geodesic scenarios (GeographicLib GeodSolve values)"
CLOSED = "Closed form on WGS 84: equatorial arc s = a * dlon; meridian quadrant 10,001,965.729313 m"


def vec(i, inp, exp, tol, src, ver="GeographicLib 2.x"):
    e = dict(exp)
    e.setdefault("ok", True)
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver,
            "tolerance": {k: t for k, t in tol.items()}}


GEODTEST = Path(__file__).resolve().parents[2] / "core/crates/gp-navigation/tests/data/GeodTest-sample.dat"
GEODTEST_SRC = "GeographicLib GeodTest-short.dat, line sample (Karney, high-precision reference)"


def geodtest(n=18):
    """Evenly spaced GeodTest lines: (lat1, lon1, azi1, lat2, lon2, azi2, s12, a12, m12, S12)."""
    rows = [list(map(float, l.split())) for l in GEODTEST.read_text().splitlines()]
    well = [r for r in rows if r[6] < 1.9e7 and abs(r[3]) < 89]
    return [well[i * len(well) // n] for i in range(n)]


def inverse_geodtest(start):
    out = []
    for i, (lat1, lon1, azi1, lat2, lon2, azi2, s12, a12, m12, S12) in enumerate(geodtest(), start):
        out.append(vec(i, {"lat1": lat1, "lon1": lon1, "lat2": lat2, "lon2": lon2},
                       {"result.distance.value": km(s12), "result.azimuth1.value": azi1, "result.azimuth2.value": azi2,
                        "result.reduced_length.value": m12, "result.area.value": S12},
                       {"result.distance.value": {"abs": 2e-11}, "result.azimuth1.value": {"abs": 1e-8},
                        "result.azimuth2.value": {"abs": 1e-8}, "result.reduced_length.value": {"abs": 1e-7},
                        "result.area.value": {"abs": 0.5}}, GEODTEST_SRC, "GeodTest 2010"))
    return out


def direct_geodtest(start):
    out = []
    for i, (lat1, lon1, azi1, lat2, lon2, azi2, s12, a12, m12, S12) in enumerate(geodtest(), start):
        out.append(vec(i, {"lat1": lat1, "lon1": lon1, "azimuth": azi1, "distance": f"{s12!r} m"},
                       {"result.lat2.value": lat2, "result.lon2.value": lon2, "result.azimuth2.value": azi2},
                       {"result.lat2.value": {"abs": 1e-10}, "result.lon2.value": {"abs": 1e-10},
                        "result.azimuth2.value": {"abs": 1e-8}}, GEODTEST_SRC, "GeodTest 2010"))
    return out


def km(m):
    return m / 1000.0


def inverse():
    mm = {"abs": 1e-6}  # 1 mm, in km
    return [
        vec(1, {"lat1": 40.6413, "lon1": -73.7781, "lat2": 51.47, "lon2": -0.4543},
            {"result.distance.value": km(5554908.791), "result.azimuth1.value": 51.3816479, "result.azimuth2.value": 107.9828291},
            {"result.distance.value": mm, "result.azimuth1.value": {"abs": 1e-7}, "result.azimuth2.value": {"abs": 1e-7}}, SPEC),
        vec(2, {"lat1": 0, "lon1": 0, "lat2": 0.5, "lon2": 179.7},
            {"result.distance.value": km(19944127.421), "result.azimuth1.value": 15.556883},
            {"result.distance.value": mm, "result.azimuth1.value": {"abs": 1e-6}}, SPEC),
        vec(3, {"lat1": 12.5, "lon1": 45, "lat2": 12.5, "lon2": 45}, {"result.distance.value": 0.0, "meta.warnings.0.code": "AZIMUTH_UNDEFINED"},
            {"result.distance.value": {"abs": 1e-12}}, CLOSED),
        vec(4, {"lat1": 0, "lon1": 0, "lat2": 0, "lon2": 90}, {"result.distance.value": km(A * math.pi / 2), "result.azimuth1.value": 90.0},
            {"result.distance.value": {"abs": 1e-9}, "result.azimuth1.value": {"abs": 1e-9}}, CLOSED),
        vec(5, {"lat1": 90, "lon1": 0, "lat2": -90, "lon2": 0}, {"result.distance.value": km(2 * Q)},
            {"result.distance.value": mm}, CLOSED),
        vec(6, {"lat1": 0, "lon1": 0, "lat2": 0, "lon2": 180}, {"result.distance.value": km(2 * Q), "meta.warnings.0.code": "AZIMUTH_NOT_UNIQUE"},
            {"result.distance.value": mm}, CLOSED),
    ]


def direct():
    return [
        vec(1, {"lat1": 40.6413, "lon1": -73.7781, "azimuth": 51, "distance": "1000 km"},
            {"result.lat2.value": 45.8920808, "result.lon2.value": -63.7549563, "result.azimuth2.value": 57.886373},
            {"result.lat2.value": {"abs": 1e-7}, "result.lon2.value": {"abs": 1e-7}, "result.azimuth2.value": {"abs": 1e-6}}, SPEC),
        vec(2, {"lat1": 0, "lon1": 0, "azimuth": 90, "distance": "1000 km"},
            {"result.lat2.value": 0.0, "result.lon2.value": math.degrees(1e6 / A), "result.azimuth2.value": 90.0},
            {"result.lat2.value": {"abs": 1e-12}, "result.lon2.value": {"abs": 1e-12}, "result.azimuth2.value": {"abs": 1e-12}}, CLOSED),
        vec(3, {"lat1": 0, "lon1": 10, "azimuth": 270, "distance": "1000 km"},
            {"result.lat2.value": 0.0, "result.lon2.value": 10 - math.degrees(1e6 / A)},
            {"result.lat2.value": {"abs": 1e-12}, "result.lon2.value": {"abs": 1e-12}}, CLOSED),
        vec(4, {"lat1": 0, "lon1": 0, "azimuth": 0, "distance": f"{Q} m"},
            {"result.lat2.value": 90.0}, {"result.lat2.value": {"abs": 1e-8}}, CLOSED),
        vec(5, {"lat1": 0, "lon1": 0, "azimuth": 90, "distance": "-1000 km"},
            {"result.lon2.value": -math.degrees(1e6 / A)}, {"result.lon2.value": {"abs": 1e-12}}, CLOSED),
    ]


def hav(lat1, lon1, lat2, lon2, r=6371008.771):
    p1, p2 = math.radians(lat1), math.radians(lat2)
    h = math.sin((p2 - p1) / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(math.radians(lon2 - lon1) / 2) ** 2
    return 2 * r * math.asin(math.sqrt(h))


def haversine():
    cases = [(40.6413, -73.7781, 51.47, -0.4543), (0, 0, 0, 90), (-33.9, 151.2, 51.5, -0.1), (10, 20, 10.001, 20.001), (89, 0, -89, 180),
             (0, 0, 0, 180), (90, 0, -90, 0), (35.6762, 139.6503, 37.7749, -122.4194), (-22.9068, -43.1729, 38.7223, -9.1393),
             (1.3521, 103.8198, -33.8688, 151.2093), (64.1466, -21.9426, 55.7558, 37.6173), (0, 179.9, 0, -179.9),
             (-89.9, 45, -89.9, -135), (30.0444, 31.2357, 30.0444, 31.2358), (51.5, -0.1, 51.5, -0.1),
             (-45.0312, 168.6626, 45.0312, -11.3374), (19.4326, -99.1332, 19.4326, 80.8668), (70, 20, 70, 30), (-10, -60, 10, -50)]
    out = []
    for i, c in enumerate(cases, 1):
        inp = dict(zip(["lat1", "lon1", "lat2", "lon2"], c))
        out.append(vec(i, inp, {"result.distance.value": km(hav(*c))}, {"result.distance.value": {"abs": 1e-9, "rel": 1e-13}},
                       "Haversine evaluated in Python with R1 = 6,371,008.771 m", "Moritz 2000"))
    out[0]["expect"]["result.difference.value"] = 14890.0
    out[0]["tolerance"]["result.difference.value"] = {"abs": 1.0}
    # Rosetta Code "Haversine formula" task: BNA to LAX on a 6,372.8 km sphere, about 2,887.26 km
    out.append(vec(len(out) + 1, {"lat1": 36.12, "lon1": -86.67, "lat2": 33.94, "lon2": -118.40, "radius": "6372.8 km"},
                   {"result.distance.value": 2887.26}, {"result.distance.value": {"abs": 0.005}},
                   "Rosetta Code, Haversine formula (task description)", "retrieved 2026-09-19"))
    return out


def vincenty_inverse():
    base = inverse()
    out = []
    for v in [base[0], base[3], base[4]]:
        out.append(dict(v, id=f"v{len(out)+1:03d}", expect={"ok": True, "result.distance.value": v["expect"]["result.distance.value"]},
                        tolerance={"result.distance.value": {"abs": 1e-6}}))
    out.append(vec(4, {"lat1": 0, "lon1": 0, "lat2": 0.5, "lon2": 179.7}, {"ok": False, "error.code": "DID_NOT_CONVERGE"}, {}, SPEC))
    out.append(vec(5, {"lat1": 1, "lon1": 2, "lat2": 1, "lon2": 2}, {"result.distance.value": 0.0}, {"result.distance.value": {"abs": 1e-12}}, CLOSED))
    return out


def vincenty_direct():
    # Tolerances stay at the published values' printed precision (Vincenty agrees within 1 mm).
    return [dict(v, source=v["source"] + " (Vincenty agrees within 1 mm)") for v in direct()]


def midpoint():
    return [
        vec(1, {"lat1": 0, "lon1": 0, "lat2": 0, "lon2": 60}, {"result.lat.value": 0.0, "result.lon.value": 30.0},
            {"result.lat.value": {"abs": 1e-12}, "result.lon.value": {"abs": 1e-12}}, CLOSED),
        vec(2, {"lat1": 0, "lon1": -20, "lat2": 0, "lon2": 40}, {"result.lon.value": 10.0}, {"result.lon.value": {"abs": 1e-12}}, CLOSED),
        vec(3, {"lat1": 0, "lon1": 170, "lat2": 0, "lon2": -170}, {"result.lon.value": -180.0}, {"result.lon.value": {"abs": 1e-9}}, CLOSED),
        vec(4, {"lat1": 0, "lon1": 0, "lat2": 0, "lon2": 90}, {"result.half_distance.value": km(A * math.pi / 4)},
            {"result.half_distance.value": {"abs": 1e-9}}, CLOSED),
        vec(5, {"lat1": -90, "lon1": 0, "lat2": 90, "lon2": 0}, {"result.lat.value": 0.0}, {"result.lat.value": {"abs": 1e-9}}, CLOSED),
    ]


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    inv, dirs = inverse(), direct()
    files = {"navigation.geodesic.inverse": inv + inverse_geodtest(len(inv) + 1),
             "navigation.geodesic.direct": dirs + direct_geodtest(len(dirs) + 1),
             "navigation.geodesic.haversine": haversine(), "navigation.geodesic.vincenty-inverse": vincenty_inverse(),
             "navigation.geodesic.vincenty-direct": vincenty_direct(), "navigation.geodesic.midpoint": midpoint()}
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
