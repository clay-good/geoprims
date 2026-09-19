#!/usr/bin/env python3
"""Golden vectors for the ellipsoid and frame tools (geodesy/reference-frames).

Independent sources: mpmath at 40 digits for ellipsoid parameters, radii of
curvature, degree lengths, and auxiliary latitudes (inverses by findroot);
GeographicLib's geodesic for meridian arcs; and GeographicLib's CartConvert
for ECEF and local east-north-up conversions (AER from its ENU). Writes
core/vectors/geodesy.{ellipsoid,frame}.*.jsonl. Requires mpmath,
geographiclib, and CartConvert on PATH.
"""
import json
import random
import subprocess
from pathlib import Path

from geographiclib.geodesic import Geodesic
from mpmath import asin, asinh, atan, atan2, atanh, cbrt, cos, degrees, findroot, mp, mpf, pi, quad, radians, sin, sinh, sqrt, tan

mp.dps = 40
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/vectors"
CATALOG = {
    "wgs84": (mpf(6378137), 1 / mpf("298.257223563")),
    "grs80": (mpf(6378137), 1 / mpf("298.257222101")),
    "clarke1866": (mpf("6378206.4"), (mpf("6378206.4") - mpf("6356583.8")) / mpf("6378206.4")),
    "intl1924": (mpf(6378388), 1 / mpf(297)),
    "airy1830": (mpf("6377563.396"), 1 / mpf("299.3249646")),
    "bessel1841": (mpf("6377397.155"), 1 / mpf("299.1528128")),
    "krassovsky1940": (mpf(6378245), 1 / mpf("298.3")),
}
MP40 = ("mpmath 40-digit evaluation of the defining relations (tools/vectors/gen_frames.py)", "mpmath 1.3")
CART = ("GeographicLib CartConvert", "GeographicLib 2.7")
GEOD = ("GeographicLib geodesic along the meridian (Karney 2013)", "geographiclib 2.x (Python)")


def f(x):
    return float(x)


class Ell:
    def __init__(self, a, fl):
        self.a, self.f = mpf(a), mpf(fl)
        self.e2 = self.f * (2 - self.f)
        self.e = sqrt(self.e2)
        self.b = self.a * (1 - self.f)

    def M(self, p):
        return self.a * (1 - self.e2) / (1 - self.e2 * sin(p) ** 2) ** mpf(1.5)

    def N(self, p):
        return self.a / sqrt(1 - self.e2 * sin(p) ** 2)

    def arc(self, p):
        return quad(self.M, [0, p])

    def q(self, s):
        if self.e == 0:
            return 2 * s
        return (1 - self.e2) * (s / (1 - self.e2 * s * s) + atanh(self.e * s) / self.e)

    def aux(self, p):
        s = sin(p)
        psi = asinh(tan(p)) - (self.e * atanh(self.e * s) if self.e else 0)
        return {
            "geocentric": atan2((1 - self.e2) * sin(p), cos(p)),
            "parametric": atan2((1 - self.f) * sin(p), cos(p)),
            "rectifying": pi / 2 * self.arc(p) / self.arc(pi / 2),
            "conformal": atan(sinh(psi)),
            "authalic": asin(self.q(s) / self.q(1)),
            "isometric": psi,
        }


def write(name, vs):
    (OUT / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
    print(name, len(vs))


def vec(i, inp, exp, src, tol=None, ok=True):
    e = dict(exp)
    e["ok"] = ok
    v = {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src[0], "sourceVersion": src[1]}
    if tol:
        v["tolerance"] = tol
    return v


def rel(keys, r=1e-14, a=1e-9):
    return {k: {"rel": r, "abs": a} for k in keys}


def parameters():
    vs = []
    keys = ["result.b.value", "result.flattening", "result.e2", "result.ep2", "result.n", "result.mean_radius.value",
            "result.authalic_radius.value", "result.volumetric_radius.value", "result.quarter_meridian.value"]

    def expect(e):
        c2 = e.a ** 2 / 2 + e.b ** 2 / 2 * (atanh(e.e) / e.e if e.e else 1)
        out = {"result.b.value": f(e.b), "result.flattening": f(e.f), "result.e2": f(e.e2), "result.ep2": f(e.e2 / (1 - e.e2)),
               "result.n": f(e.f / (2 - e.f)), "result.mean_radius.value": f((2 * e.a + e.b) / 3),
               "result.authalic_radius.value": f(sqrt(c2)), "result.volumetric_radius.value": f(cbrt(e.a ** 2 * e.b)),
               "result.quarter_meridian.value": f(e.arc(pi / 2))}
        if e.f:
            out["result.inverse_flattening"] = f(1 / e.f)
        return out

    tol = rel(keys + ["result.inverse_flattening"])
    i = 1
    for eid, (a, fl) in CATALOG.items():
        vs.append(vec(i, {"ellipsoid": eid}, expect(Ell(a, fl)), MP40, tol)); i += 1
    rnd = random.Random(20260919)
    customs = [(3396190, mpf("169.894")), (6371000, 0), (1737400, 0), (71492000, mpf("15.4144"))]
    customs += [(rnd.randint(1_000_000, 70_000_000), mpf(rnd.randint(20, 1000)) + mpf(rnd.randint(0, 999)) / 1000) for _ in range(6)]
    for a, rf in customs:
        e = Ell(a, 1 / rf if rf else 0)
        vs.append(vec(i, {"a": a, "inverse_flattening": float(rf)}, expect(e), MP40, tol)); i += 1
    for a, b in [(6378137, "6356752.314245"), (3396190, "3376200"), (6371000, "6371000")]:
        e = Ell(a, (mpf(a) - mpf(b)) / mpf(a))
        vs.append(vec(i, {"a": a, "b": float(b)}, expect(e), MP40, tol)); i += 1
    for inp, code in [({"a": 6378137, "b": 6400000}, "OUT_OF_DOMAIN"), ({"b": 6356752}, "INVALID_INPUT"),
                      ({"ellipsoid": "wgs84", "a": 6378137, "b": 6356752}, "INVALID_INPUT")]:
        vs.append(vec(i, inp, {"error.code": code}, ("add-geodesy-suite input rules", "2026-09"), ok=False)); i += 1
    write("geodesy.ellipsoid.parameters", vs)


def radii():
    vs = []
    rnd = random.Random(7)
    geod = Geodesic.WGS84
    cases = [(45, None, None, "wgs84"), (0, None, None, "wgs84"), (90, None, None, "wgs84"), (-90, None, None, "wgs84"),
             (45, 45, 46, "wgs84"), (60, 90, -30, "grs80"), (-33.9, 0, None, "clarke1866"), (51.5, 30, None, "airy1830"),
             (55.75, None, 60, "krassovsky1940"), (35.7, 120, None, "bessel1841")]
    cases += [(round(rnd.uniform(-89.9, 89.9), 6), round(rnd.uniform(0, 360), 3) if rnd.random() < 0.5 else None,
               round(rnd.uniform(-89.9, 89.9), 6) if rnd.random() < 0.5 else None, rnd.choice(list(CATALOG))) for _ in range(14)]
    for i, (lat, az, lat2, eid) in enumerate(cases, 1):
        e = Ell(*CATALOG[eid])
        p = radians(mpf(lat))
        lo, hi = max(lat - 0.5, -90), min(lat + 0.5, 90)
        m_, n_ = e.M(p), e.N(p)
        exp = {"result.meridional.value": f(m_), "result.prime_vertical.value": f(n_), "result.gaussian.value": f(sqrt(m_ * n_)),
               "result.degree_lat.value": f((e.arc(radians(mpf(hi))) - e.arc(radians(mpf(lo)))) / (hi - lo)),
               "result.degree_lon.value": f(pi / 180 * n_ * cos(p)) if abs(lat) != 90 else 0.0}
        inp = {"lat": lat}
        if eid != "wgs84":
            inp["ellipsoid"] = eid
        src = MP40
        if eid == "wgs84":
            # Meridian arcs from GeographicLib's geodesic, independent of the elliptic integrals.
            exp["result.meridian_arc.value"] = geod.Inverse(0, 0, lat, 0)["s12"] * (1 if lat >= 0 else -1)
            src = (MP40[0] + "; meridian arcs from GeographicLib's geodesic", MP40[1] + ", geographiclib 2.x")
        else:
            exp["result.meridian_arc.value"] = f(e.arc(p))
        if az is not None:
            inp["azimuth"] = az
            al = radians(mpf(az))
            exp["result.in_azimuth.value"] = f(1 / (cos(al) ** 2 / m_ + sin(al) ** 2 / n_))
        if lat2 is not None:
            inp["lat2"] = lat2
            exp["result.arc_between.value"] = f(e.arc(radians(mpf(lat2))) - e.arc(p))
        tol = {k: {"abs": 1e-6, "rel": 1e-13} for k in exp}
        vs.append(vec(i, inp, exp, src, tol))
    i = len(vs) + 1
    vs.append(vec(i, {"lat": 91}, {"error.code": "INVALID_INPUT"}, ("add-geodesy-suite input rules", "2026-09"), ok=False))
    write("geodesy.ellipsoid.radii", vs)


KINDS = ["geocentric", "parametric", "rectifying", "conformal", "authalic", "isometric"]


def auxiliary():
    vs = []
    rnd = random.Random(11)
    lats = [45, 0, 1, 30, 60, 89.9, -45, 89.999, 90, -90] + [round(rnd.uniform(-89.99, 89.99), 7) for _ in range(6)]
    i = 1
    for lat in lats:
        eid = "wgs84" if i <= 12 else rnd.choice(list(CATALOG))
        e = Ell(*CATALOG[eid])
        p = radians(mpf(lat))
        exp = {"result.geodetic.value": float(lat)}
        tol = {}
        if abs(lat) == 90:
            for k in KINDS[:-1]:
                exp[f"result.{k}.value"] = float(lat)
                tol[f"result.{k}.value"] = {"abs": 1e-12}
        else:
            a = e.aux(p)
            for k in KINDS:
                exp[f"result.{k}.value"] = f(degrees(a[k]))
                # Isometric latitude grows without bound near the poles; its error follows tan φ.
                tol[f"result.{k}.value"] = {"abs": 1e-12, "rel": 1e-13} if k != "isometric" else {"abs": 1e-11, "rel": 1e-12}
        tol["result.geodetic.value"] = {"abs": 1e-12}
        inp = {"latitude": lat}
        if eid != "wgs84":
            inp["ellipsoid"] = eid
        vs.append(vec(i, inp, exp, MP40, tol)); i += 1
    # Inverses: from each kind back to geodetic, by findroot on the forward relation.
    for kind in KINDS:
        for lat in [rnd.uniform(-89, 89), rnd.uniform(-89, 89)]:
            e = Ell(*CATALOG["wgs84"])
            p = radians(mpf(lat))
            val = f(degrees(e.aux(p)[kind]))
            geo = findroot(lambda x: degrees(e.aux(radians(x))[kind]) - val, mpf(lat))
            vs.append(vec(i, {"latitude": val, "from": kind}, {"result.geodetic.value": f(geo)}, MP40,
                          {"result.geodetic.value": {"abs": 1e-12}})); i += 1
    vs.append(vec(i, {"latitude": 95}, {"error.code": "INVALID_INPUT"}, ("add-geodesy-suite input rules", "2026-09"), ok=False)); i += 1
    # Isometric latitude past 90°: 120° is about 76° geodetic.
    e = Ell(*CATALOG["wgs84"])
    geo = findroot(lambda x: degrees(e.aux(radians(x))["isometric"]) - 120, mpf(76))
    vs.append(vec(i, {"latitude": 120, "from": "isometric"}, {"result.geodetic.value": f(geo)}, MP40,
                  {"result.geodetic.value": {"abs": 1e-12}}))
    write("geodesy.ellipsoid.auxiliary-latitude", vs)


def cart(args, lines):
    out = subprocess.run(["CartConvert", "-p", "9", *args], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True).stdout
    return [list(map(float, l.split())) for l in out.splitlines() if l.strip()]


def points(rnd, n):
    return [(round(rnd.uniform(-89.9, 89.9), 7), round(rnd.uniform(-180, 179.999), 7), round(rnd.choice([0, rnd.uniform(-400, 9000), rnd.uniform(0, 4e7)]), 3)) for _ in range(n)]


def ecef():
    rnd = random.Random(13)
    pts = [(40.446111, -79.982222, 300), (0, 0, 0), (90, 0, 0), (-90, 45, 1000), (0, 180, 0), (45, 90, -10000), (30, -60, 35786000)]
    pts += points(rnd, 16)
    fwd = cart([], [f"{a} {b} {c}" for a, b, c in pts])
    vs = []
    for i, ((la, lo, h), (x, y, z)) in enumerate(zip(pts, fwd), 1):
        tol = {k: {"abs": 2e-6, "rel": 1e-15} for k in ["result.x.value", "result.y.value", "result.z.value"]}
        vs.append(vec(i, {"lat": la, "lon": lo, "height": h}, {"result.x.value": x, "result.y.value": y, "result.z.value": z}, CART, tol))
    vs.append(vec(len(vs) + 1, {"lat": 40, "lon": -105, "height": -20000}, {"error.code": "OUT_OF_DOMAIN"}, ("add-geodesy-suite input rules", "2026-09"), ok=False))
    write("geodesy.frame.geodetic-to-ecef", vs)

    # Inverse: CartConvert -r on the same XYZ (rounded to the mm the forward vectors print).
    xyz = [(round(x, 3), round(y, 3), round(z, 3)) for x, y, z in fwd]
    b = f(Ell(*CATALOG["wgs84"]).b)
    xyz += [(0.0, 0.0, b), (0.0, 0.0, -b - 1000), (6378137.0, 0.0, 0.0), (1000.0, 0.0, 0.0), (0.0, 50.0, 10.0)]
    inv = cart(["-r"], [f"{x!r} {y!r} {z!r}" for x, y, z in xyz])
    vs = []
    for i, ((x, y, z), (la, lo, h)) in enumerate(zip(xyz, inv), 1):
        exp = {"result.lat.value": la, "result.lon.value": lo if lo < 180 else lo - 360, "result.height.value": h}
        tol = {"result.lat.value": {"abs": 1e-11}, "result.lon.value": {"abs": 1e-11}, "result.height.value": {"abs": 2e-6, "rel": 1e-15}}
        if x == 0 and y == 0:
            exp["meta.warnings.*.code"] = "LONGITUDE_UNDEFINED"
            exp["result.lon.value"] = 0
            tol["result.lon.value"] = {"abs": 0}
        vs.append(vec(i, {"x": x, "y": y, "z": z}, exp, CART, tol))
    vs.append(vec(len(vs) + 1, {"x": 0, "y": 0, "z": 0}, {"error.code": "DEGENERATE_GEOMETRY"}, ("add-geodesy-suite scenario: the Earth's center", "2026-09"), ok=False))
    write("geodesy.frame.ecef-to-geodetic", vs)


def local():
    import math
    rnd = random.Random(17)
    vs_to, vs_from = [], []
    origin = (40.446111, -79.982222, 300)
    cases = [(origin, (40.446111, -79.982222, 1300)), (origin, (40.5, -80.0, 500)), ((0, 0, 0), (0, 1, 0)), ((89.9, 10, 0), (89.95, -170, 100))]
    for _ in range(16):
        o = (round(rnd.uniform(-85, 85), 6), round(rnd.uniform(-180, 179.9), 6), round(rnd.uniform(-100, 3000), 2))
        t = (round(o[0] + rnd.uniform(-5, 5), 6), round(o[1] + rnd.uniform(-5, 5), 6), round(rnd.uniform(-100, 20000), 2))
        cases.append((o, (max(-89.9, min(89.9, t[0])), t[1], t[2])))
    for i, (o, t) in enumerate(cases, 1):
        (e_, n_, u_), = cart(["-l", *map(str, o)], [f"{t[0]} {t[1]} {t[2]}"])
        horiz = math.hypot(e_, n_)
        rng = math.hypot(horiz, u_)
        exp = {"result.east.value": e_, "result.north.value": n_, "result.up.value": u_, "result.range.value": rng,
               "result.elevation.value": math.degrees(math.atan2(u_, horiz))}
        tol = {k: {"abs": 2e-6} for k in ["result.east.value", "result.north.value", "result.up.value", "result.range.value"]}
        tol["result.elevation.value"] = {"abs": 1e-9}
        if horiz < 1e-6:
            exp["result.azimuth.value"] = 0
            exp["result.elevation.value"] = 90 if u_ > 0 else -90
            exp["meta.warnings.*.code"] = "AZIMUTH_UNDEFINED"
            tol["result.azimuth.value"] = {"abs": 0}
            tol["result.elevation.value"] = {"abs": 0}
        else:
            exp["result.azimuth.value"] = math.degrees(math.atan2(e_, n_)) % 360
            tol["result.azimuth.value"] = {"abs": 1e-9}
        vs_to.append(vec(i, {"lat0": o[0], "lon0": o[1], "h0": o[2], "lat": t[0], "lon": t[1], "height": t[2]}, exp, CART, tol))
    write("geodesy.frame.to-local", vs_to)

    # From local: random offsets in each frame, geodetic answers from CartConvert -l -r.
    i = 1
    for k in range(21):
        o = (round(rnd.uniform(-85, 85), 6), round(rnd.uniform(-180, 179.9), 6), round(rnd.uniform(-100, 3000), 2))
        e_, n_, u_ = rnd.uniform(-2e5, 2e5), rnd.uniform(-2e5, 2e5), rnd.uniform(-5e3, 3e4)
        frame = ["enu", "ned", "aer"][k % 3]
        if frame == "enu":
            inp = {"east": e_, "north": n_, "up": u_}
        elif frame == "ned":
            inp = {"north": n_, "east": e_, "down": -u_}
        else:
            az, el, rg = round(rnd.uniform(0, 360), 4), round(rnd.uniform(-10, 60), 4), round(rnd.uniform(100, 3e5), 3)
            inp = {"azimuth": az, "elevation": el, "range": rg}
            horiz = rg * math.cos(math.radians(el))
            e_, n_, u_ = horiz * math.sin(math.radians(az)), horiz * math.cos(math.radians(az)), rg * math.sin(math.radians(el))
        (la, lo, h), = cart(["-l", *map(str, o), "-r"], [f"{e_!r} {n_!r} {u_!r}"])
        inp = {"lat0": o[0], "lon0": o[1], "h0": o[2], "frame": frame, **inp}
        tol = {"result.lat.value": {"abs": 1e-11}, "result.lon.value": {"abs": 1e-11}, "result.height.value": {"abs": 2e-6}}
        vs_from.append(vec(i, inp, {"result.lat.value": la, "result.lon.value": lo if lo < 180 else lo - 360, "result.height.value": h}, CART, tol)); i += 1
    vs_from.append(vec(i, {"lat0": 40, "lon0": -105, "frame": "aer", "azimuth": 10, "range": 100}, {"error.code": "INVALID_INPUT"},
                       ("add-geodesy-suite input rules", "2026-09"), ok=False))
    write("geodesy.frame.from-local", vs_from)


if __name__ == "__main__":
    parameters()
    radii()
    auxiliary()
    ecef()
    local()
