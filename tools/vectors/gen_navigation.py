#!/usr/bin/env python3
"""Golden vectors for navigation.geodesic.* (core/vectors/navigation.*.jsonl).

Sources, all independent of the Rust core:
- Published GeographicLib values quoted in the add-navigation-and-geometry
  spec scenarios (JFK-LHR, the nearly antipodal pair, the direct problem).
- Closed forms: along the equator a geodesic is the equator itself
  (s = a * dlon for |dlon| <= (1 - f) * 180 deg), and pole to pole is twice
  the WGS 84 meridian quadrant 10,001,965.729313 m (Karney 2011, GeodTest).
- Haversine by direct evaluation in Python.
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
    cases = [(40.6413, -73.7781, 51.47, -0.4543), (0, 0, 0, 90), (-33.9, 151.2, 51.5, -0.1), (10, 20, 10.001, 20.001), (89, 0, -89, 180)]
    out = []
    for i, c in enumerate(cases, 1):
        inp = dict(zip(["lat1", "lon1", "lat2", "lon2"], c))
        out.append(vec(i, inp, {"result.distance.value": km(hav(*c))}, {"result.distance.value": {"abs": 1e-9, "rel": 1e-13}},
                       "Haversine evaluated in Python with R1 = 6,371,008.771 m", "Moritz 2000"))
    out[0]["expect"]["result.difference.value"] = 14890.0
    out[0]["tolerance"]["result.difference.value"] = {"abs": 1.0}
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
    files = {"navigation.geodesic.inverse": inverse(), "navigation.geodesic.direct": direct(),
             "navigation.geodesic.haversine": haversine(), "navigation.geodesic.vincenty-inverse": vincenty_inverse(),
             "navigation.geodesic.vincenty-direct": vincenty_direct(), "navigation.geodesic.midpoint": midpoint()}
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
