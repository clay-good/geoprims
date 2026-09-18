#!/usr/bin/env python3
"""Golden vectors for geodesy.* (parse, format, UTM, UPS, MGRS).

Independent sources:
- The add-geodesy-suite spec scenarios (GeographicLib values).
- UTM on a central meridian: easting 500,000 m and northing k0 * M(lat),
  with the meridian arc M integrated numerically here (Simpson's rule).
- UPS by rotational symmetry from the spec's 85° N value.
- Exact fractions for DMS parsing and bearing arithmetic.
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

A, INVF = 6378137.0, 298.257223563
E2 = (2 - 1 / INVF) / INVF
SPEC = "add-geodesy-suite scenarios (GeographicLib values)"
ARC = "Meridian arc M(lat) by Simpson integration of a(1-e^2)/(1-e^2 sin^2)^1.5 (tools/vectors/gen_geodesy.py)"
EXACT = "Exact arithmetic"


def meridian_arc(lat):
    n = 200000
    b = math.radians(lat)
    h = b / n
    f = lambda p: A * (1 - E2) / (1 - E2 * math.sin(p) ** 2) ** 1.5
    s = f(0) + f(b) + sum((4 if i % 2 else 2) * f(i * h) for i in range(1, n))
    return s * h / 3


def vec(i, inp, exp, tol, src, ver="GeographicLib 2.x"):
    e = dict(exp)
    e.setdefault("ok", True)
    tol = dict(tol)
    for k, v in e.items():
        if isinstance(v, (int, float)) and not isinstance(v, bool):
            tol.setdefault(k, {"abs": 0})  # integers such as zone numbers match exactly
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


def dms(d, m, s):
    return float(F(d) + F(m, 60) + F(s, 3600))


def parse():
    lat, lon = dms(40, 26, 46), -dms(79, 58, 56)
    t = {"result.lat.value": {"abs": 1e-12}, "result.lon.value": {"abs": 1e-12}}
    return [
        vec(1, {"text": "40°26'46\"N 79°58'56\"W"}, {"result.lat.value": lat, "result.lon.value": lon, "result.notation": "DMS"}, t, SPEC),
        vec(2, {"text": "402646N0795856W"}, {"result.lat.value": lat, "result.lon.value": lon, "result.notation": "packed"}, t, SPEC),
        vec(3, {"text": "lon: -79.98, lat: 40.45"}, {"result.lat.value": 40.45, "result.lon.value": -79.98}, t, SPEC),
        vec(4, {"text": "-105.27, 40.01"}, {"result.lat.value": 40.01, "result.lon.value": -105.27, "meta.warnings.0.code": "AMBIGUOUS_INPUT"}, t, SPEC),
        vec(5, {"text": "40°26'60\"N 79°58'56\"W"}, {"ok": False, "error.code": "INVALID_INPUT"}, {}, SPEC),
        vec(6, {"text": "-40.5N, 79.9W"}, {"ok": False, "error.code": "INVALID_INPUT"}, {}, SPEC),
        vec(7, {"text": "40 26.767N 79 58.933W"}, {"result.lat.value": float(F(40) + F(26767, 60000)), "result.notation": "DDM"},
            {"result.lat.value": {"abs": 1e-12}}, EXACT, "definition"),
    ]


def fmt():
    return [
        vec(1, {"lat": 10.9999999, "lon": 0, "style": "dms", "decimals": 0}, {"result.formatted": "11°00'00\"N 0°00'00\"E"}, {}, SPEC),
        vec(2, {"lat": 40.446111, "lon": -79.982222, "style": "dms", "decimals": 1}, {"result.formatted": "40°26'46.0\"N 79°58'56.0\"W"}, {}, EXACT, "definition"),
        vec(3, {"lat": 40.446111, "lon": -79.982222, "style": "ddm", "decimals": 3}, {"result.formatted": "40°26.767'N 79°58.933'W"}, {}, EXACT, "definition"),
        vec(4, {"lat": -33.5, "lon": 151.25, "style": "dd", "decimals": 2, "signs": "signed"}, {"result.formatted": "-33.50° 151.25°"}, {}, EXACT, "definition"),
        vec(5, {"lat": 40, "lon": 0, "style": "dd", "decimals": 4}, {"result.latitude_resolution.value": 11.1}, {"result.latitude_resolution.value": {"abs": 0.05}}, SPEC),
    ]


def bearing():
    cases = [(350, 10, 20), (10, 350, -20), (90, 45, -45), (359, 1, 2), (0, 179, 179), (720, 30, 30)]
    return [vec(i, {"from": a, "to": b}, {"result.difference.value": float(d)}, {"result.difference.value": {"abs": 1e-12}}, EXACT, "definition")
            for i, (a, b, d) in enumerate(cases, 1)]


def utm_forward():
    out = [vec(1, {"lat": 40.446111, "lon": -79.982222},
               {"result.zone": 17, "result.hemisphere": "N", "result.easting.value": 586309.953, "result.northing.value": 4477770.428},
               {"result.easting.value": {"abs": 1e-3}, "result.northing.value": {"abs": 1e-3}}, SPEC)]
    for i, (lat, lon) in enumerate([(0, -81), (10, 3), (45, 9), (-33, 153), (70, -99), (83.9, -87)], 2):
        m = meridian_arc(abs(lat))
        n = 0.9996 * m if lat >= 0 else 10_000_000 - 0.9996 * m
        out.append(vec(i, {"lat": lat, "lon": lon}, {"result.easting.value": 500000.0, "result.northing.value": n},
                       {"result.easting.value": {"abs": 1e-9}, "result.northing.value": {"abs": 1e-6}}, ARC, "WGS 84"))
    out.append(vec(len(out) + 1, {"lat": 84.5, "lon": 0}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}, {}, SPEC))
    out.append(vec(len(out) + 1, {"lat": 40.446111, "lon": -79.982222, "zone": 18}, {"result.zone": 18, "meta.warnings.1.code": "NONSTANDARD_ZONE"}, {}, SPEC))
    return out


def utm_inverse():
    out = []
    for i, (lat, zone) in enumerate([(10, 31), (45, 32), (-33, 56), (70, 14), (0, 17)], 1):
        m = meridian_arc(abs(lat))
        n = 0.9996 * m if lat >= 0 else 10_000_000 - 0.9996 * m
        cm = 6 * zone - 183
        out.append(vec(i, {"zone": zone, "hemisphere": "N" if lat >= 0 else "S", "easting": 500000, "northing": n},
                       {"result.lat.value": float(lat), "result.lon.value": float(cm)},
                       {"result.lat.value": {"abs": 1e-10}, "result.lon.value": {"abs": 1e-10}}, ARC, "WGS 84"))
    out.append(vec(6, {"zone": 17, "hemisphere": "N", "easting": "586309.953 m", "northing": "4477770.428 m"},
                   {"result.lat.value": 40.446111, "result.lon.value": -79.982222},
                   {"result.lat.value": {"abs": 1e-8}, "result.lon.value": {"abs": 1e-8}}, SPEC))
    return out


def ups():
    rho = 2_000_000 - 1_444_542.609
    t = {"result.easting.value": {"abs": 1e-3}, "result.northing.value": {"abs": 1e-3}}
    return [
        vec(1, {"lat": 85, "lon": 0}, {"result.easting.value": 2_000_000.0, "result.northing.value": 1_444_542.609}, t, SPEC),
        vec(2, {"lat": 85, "lon": 90}, {"result.easting.value": 2_000_000 + rho, "result.northing.value": 2_000_000.0}, t, SPEC + " by rotational symmetry"),
        vec(3, {"lat": 85, "lon": -90}, {"result.easting.value": 2_000_000 - rho, "result.northing.value": 2_000_000.0}, t, SPEC + " by rotational symmetry"),
        vec(4, {"lat": -85, "lon": 0}, {"result.easting.value": 2_000_000.0, "result.northing.value": 2_000_000 + rho}, t, SPEC + " by polar symmetry"),
        vec(5, {"lat": 90, "lon": 0}, {"result.easting.value": 2_000_000.0, "result.northing.value": 2_000_000.0}, t, EXACT, "definition"),
        vec(6, {"lat": 45, "lon": 0}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}, {}, SPEC),
    ]


def ups_inverse():
    return [
        vec(1, {"hemisphere": "N", "easting": 2_000_000, "northing": 1_444_542.609}, {"result.lat.value": 85.0, "result.lon.value": 0.0},
            {"result.lat.value": {"abs": 1e-8}, "result.lon.value": {"abs": 1e-8}}, SPEC),
        vec(2, {"hemisphere": "N", "easting": 2_000_000, "northing": 2_000_000}, {"result.lat.value": 90.0}, {"result.lat.value": {"abs": 1e-9}}, EXACT, "definition"),
        vec(3, {"hemisphere": "S", "easting": 2_000_000, "northing": 2_555_457.391}, {"result.lat.value": -85.0, "result.lon.value": 0.0},
            {"result.lat.value": {"abs": 1e-8}, "result.lon.value": {"abs": 1e-8}}, SPEC + " by polar symmetry"),
        vec(4, {"hemisphere": "N", "easting": 2_555_457.391, "northing": 2_000_000}, {"result.lat.value": 85.0, "result.lon.value": 90.0},
            {"result.lat.value": {"abs": 1e-8}, "result.lon.value": {"abs": 1e-8}}, SPEC + " by rotational symmetry"),
        vec(5, {"hemisphere": "S", "easting": 2_000_000, "northing": 2_000_000}, {"result.lat.value": -90.0}, {"result.lat.value": {"abs": 1e-9}}, EXACT, "definition"),
    ]


def utm_zone():
    cases = [((60, 5), 32, "32V"), ((78, 10), 33, "33X"), ((40.446111, -79.982222), 17, "17T"), ((-80.5, 0), 0, "B"), ((85, -10), 0, "Y"), ((0, -78), 18, "18N")]
    return [vec(i, {"lat": a, "lon": b}, {"result.zone": z, "result.grid_zone": g}, {"result.zone": {"abs": 0}}, SPEC)
            for i, ((a, b), z, g) in enumerate(cases, 1)]


def mgrs_forward():
    p = {"lat": 40.446111, "lon": -79.982222}
    return [
        vec(1, p, {"result.mgrs": "17TNE8630977770"}, {}, SPEC),
        vec(2, dict(p, precision="10m"), {"result.mgrs": "17TNE86307777"}, {}, SPEC),
        vec(3, dict(p, precision="grid-zone"), {"result.mgrs": "17T"}, {}, SPEC),
        vec(4, {"lat": 78, "lon": 10, "precision": "grid-zone"}, {"result.mgrs": "33X"}, {}, SPEC),
        vec(5, {"lat": -80.5, "lon": 0, "precision": "grid-zone"}, {"result.mgrs": "B"}, {}, SPEC),
    ]


def mgrs_inverse():
    t = {"result.lat.value": {"abs": 1e-5}, "result.lon.value": {"abs": 1e-5}}
    return [
        vec(1, {"mgrs": "17TNE8630977770"}, {"result.lat.value": 40.446111, "result.lon.value": -79.982222, "result.square_size.value": 1.0},
            dict(t, **{"result.square_size.value": {"abs": 0}}), SPEC),
        vec(2, {"mgrs": "17T NE 86309 77770"}, {"result.lat.value": 40.446111, "result.lon.value": -79.982222}, t, SPEC),
        vec(3, {"mgrs": "17TNE86307777"}, {"result.square_size.value": 10.0}, {"result.square_size.value": {"abs": 0}}, SPEC),
        vec(4, {"mgrs": "17INE8630977770"}, {"ok": False, "error.code": "INVALID_INPUT"}, {}, SPEC),
        vec(5, {"mgrs": "17TNE863097777"}, {"ok": False, "error.code": "INVALID_INPUT"}, {}, SPEC),
    ]


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    files = {
        "geodesy.parse.coordinates": parse(), "geodesy.parse.format": fmt(), "geodesy.parse.bearing-difference": bearing(),
        "geodesy.utm.forward": utm_forward(), "geodesy.utm.inverse": utm_inverse(), "geodesy.utm.zone": utm_zone(),
        "geodesy.ups.forward": ups(), "geodesy.ups.inverse": ups_inverse(),
        "geodesy.grid-ref.mgrs-forward": mgrs_forward(), "geodesy.grid-ref.mgrs-inverse": mgrs_inverse(),
    }
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
