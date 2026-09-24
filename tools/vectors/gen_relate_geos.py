#!/usr/bin/env python3
"""The DE-9IM against GEOS, for geometry.predicate.relate.

Writes two files:

- core/crates/gp-geometry/tests/data/relate_geos.json: 1,500 random pairs of
  points, lines, and polygons (some with holes) on a small integer grid, with
  GEOS's matrix for each. Corners on a grid make the hard cases common:
  shared edges, a corner on an edge, collinear overlaps, lines ending on a
  boundary. Planar edges, so both sides decide on the same doubles with exact
  orientation tests and must agree character for character.
- core/vectors/geometry.predicate.relate.jsonl: golden vectors. Planar ones
  from GEOS directly. Geodesic ones either in general position, from GEOS on
  the ellipsoidal gnomonic plane at the shapes' center with edges cut into
  5 km geodesic pieces (computed here with GeographicLib's Python package,
  independently of the tool's geographiclib-rs), or degenerate by
  construction: a corner placed on a geodesic edge by GeodSolve touches it, and
  two polygons sharing an edge between the same corners touch along it.

Requires shapely (GEOS) and geographiclib; GeodSolve for the construction."""
import json
import math
import random
import subprocess
import sys
from pathlib import Path

import shapely
from geographiclib.geodesic import Geodesic
from shapely.geometry import LineString, MultiPoint, Polygon

G = Geodesic.WGS84
VER = f"shapely {shapely.__version__}, GEOS {'.'.join(map(str, shapely.geos_version))}"
SRC = "GEOS through shapely, via tools/vectors/gen_relate_geos.py"
SPEC = "add-navigation-and-geometry scenarios"
KIND = {"polygon": "polygon", "line": "line", "points": "points"}


def shape(kind, pts, holes=()):
    """GEOS geometry with x = longitude, y = latitude."""
    xy = [(lon, lat) for lat, lon in pts]
    if kind == "points":
        return MultiPoint(xy)
    if kind == "line":
        return LineString(xy)
    return Polygon(xy, [[(lon, lat) for lat, lon in h] for h in holes])


def rows(kind, pts, holes=()):
    out = [{"lat": a, "lon": b} for a, b in pts]
    for k, h in enumerate(holes, 1):
        out += [{"lat": a, "lon": b, "ring": k} for a, b in h]
    return out


# --- random grid shapes ---------------------------------------------------

def rand_polygon(rng):
    for _ in range(100):
        cx, cy = rng.randint(1, 7), rng.randint(1, 7)
        n = rng.randint(3, 6)
        angs = sorted(rng.uniform(0, 2 * math.pi) for _ in range(n))
        pts = []
        for a in angs:
            r = rng.randint(1, 4)
            p = (cy + round(r * math.sin(a)), cx + round(r * math.cos(a)))
            if not pts or pts[-1] != p:
                pts.append(p)
        if len(pts) > 1 and pts[0] == pts[-1]:
            pts.pop()
        if len(pts) < 3 or len(set(pts)) != len(pts):
            continue
        poly = shape("polygon", pts)
        if not poly.is_valid or poly.area == 0:
            continue
        holes = []
        if rng.random() < 0.5:
            for _ in range(30):
                y, x = rng.randint(0, 8), rng.randint(0, 8)
                h = [(y, x), (y, x + 1), (y + 1, x + 1), (y + 1, x)]
                if rng.random() < 0.5:
                    h.reverse()
                cand = shape("polygon", pts, [h])
                if cand.is_valid and poly.contains(shape("polygon", h)):
                    holes = [h]
                    break
        if rng.random() < 0.5:
            pts.reverse()
        return pts, holes
    raise RuntimeError("no polygon")


def rand_line(rng):
    for _ in range(100):
        n = rng.randint(2, 4)
        pts = [(rng.randint(0, 8), rng.randint(0, 8)) for _ in range(n)]
        if any(pts[i] == pts[i + 1] for i in range(n - 1)):
            continue
        if len(set(pts)) != len(pts):
            continue
        if n >= 3 and rng.random() < 0.25:
            pts.append(pts[0])  # a closed line, which has no boundary
        ls = shape("line", pts)
        if ls.is_simple and ls.length > 0:
            return pts, []
    raise RuntimeError("no line")


def rand_points(rng):
    n = rng.randint(1, 3)
    pts = list({(rng.randint(0, 8), rng.randint(0, 8)) for _ in range(n)})
    return pts, []


MAKE = {"polygon": rand_polygon, "line": rand_line, "points": rand_points}


def fixture(n=1500, seed=20260924):
    rng = random.Random(seed)
    kinds = list(MAKE)
    out = []
    while len(out) < n:
        ka, kb = rng.choice(kinds), rng.choice(kinds)
        (pa, ha), (pb, hb) = MAKE[ka](rng), MAKE[kb](rng)
        if rng.random() < 0.3:
            # The second derived from the first, so they share edges: moved
            # one grid step, wound backwards, or with its hole dropped.
            kb, hb = ka, []
            dy, dx = rng.choice([(0, 0), (0, 1), (1, 0), (-1, 0), (0, -1), (1, 1)])
            pb = [(y + dy, x + dx) for y, x in pa]
            if rng.random() < 0.5:
                pb.reverse()
            if kb == "points":
                pb = pb[: max(1, len(pb) - 1)]
        m = shape(ka, pa, ha).relate(shape(kb, pb, hb))
        out.append({
            "kind_a": ka, "geometry_a": rows(ka, pa, ha),
            "kind_b": kb, "geometry_b": rows(kb, pb, hb),
            "matrix": m,
        })
    return out


# --- geodesic: GEOS on the gnomonic plane ---------------------------------

def center(pts):
    x = y = z = 0.0
    for la, lo in pts:
        a, o = math.radians(la), math.radians(lo)
        x += math.cos(a) * math.cos(o)
        y += math.cos(a) * math.sin(o)
        z += math.sin(a)
    return math.degrees(math.atan2(z, math.hypot(x, y))), math.degrees(math.atan2(y, x))


def gnomonic(c, p):
    r = G.Inverse(c[0], c[1], p[0], p[1], Geodesic.STANDARD | Geodesic.REDUCEDLENGTH | Geodesic.GEODESICSCALE)
    rho = r["m12"] / r["M12"]
    t = math.radians(r["azi1"])
    return rho * math.sin(t), rho * math.cos(t)


def cut(c, ring, closed):
    out = []
    n = len(ring)
    for i in range(n if closed else n - 1):
        a, b = ring[i], ring[(i + 1) % n]
        line = G.InverseLine(a[0], a[1], b[0], b[1])
        k = max(1, math.ceil(line.s13 / 5000.0))
        for j in range(k):
            q = a if j == 0 else (lambda d: (d["lat2"], d["lon2"]))(line.Position(line.s13 * j / k))
            out.append(gnomonic(c, q))
    if not closed:
        out.append(gnomonic(c, ring[-1]))
    return out


def geodesic_matrix(ka, pa, ha, kb, pb, hb):
    c = center(pa + [p for h in ha for p in h] + pb + [p for h in hb for p in h])

    def plane(kind, pts, holes):
        if kind == "points":
            return MultiPoint([gnomonic(c, p) for p in pts])
        if kind == "line":
            return LineString(cut(c, pts, False))
        return Polygon(cut(c, pts, True), [cut(c, h, True) for h in holes])

    return plane(ka, pa, ha).relate(plane(kb, pb, hb))


def geod(cmd, line):
    return subprocess.run(["GeodSolve", *cmd, "-p", "12"], input=line + "\n", capture_output=True, text=True, check=True).stdout.split()


def on_edge(a, b, f):
    """The point a fraction f along the geodesic a → b, by GeodSolve."""
    s12, azi1 = float(geod(["-i"], f"{a[0]} {a[1]} {b[0]} {b[1]}")[2]), float(geod(["-i"], f"{a[0]} {a[1]} {b[0]} {b[1]}")[0])
    lat, lon = geod([], f"{a[0]} {a[1]} {azi1:.15f} {s12 * f:.9f}")[:2]
    return float(lat), float(lon)


def vec(i, inp, exp, src=SRC):
    e = dict(exp)
    e.setdefault("ok", True)
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": {}}


def main():
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")
    fx = fixture()
    (root / "core/crates/gp-geometry/tests/data/relate_geos.json").write_text(json.dumps(fx, separators=(",", ":")) + "\n")

    out = []

    def add(ka, pa, ha, kb, pb, hb, edges, m, src=SRC):
        inp = {"geometry_a": rows(ka, pa, ha), "kind_a": ka, "geometry_b": rows(kb, pb, hb), "kind_b": kb}
        if edges == "planar":
            inp["edges"] = "planar"
        out.append(vec(len(out) + 1, inp, {"result.matrix": m}, src))

    # The primary example: a flight line through a zone.
    zone = [(40.0, -105.0), (40.0, -104.99), (40.006, -104.99), (40.006, -105.0)]
    flight = [(40.001, -105.004), (40.009, -104.991)]
    add("line", flight, [], "polygon", zone, [], "geodesic", geodesic_matrix("line", flight, [], "polygon", zone, []))

    # Geodesic, general position, from GEOS on the gnomonic plane.
    field = [(40.0, -105.0), (40.0, -104.9), (40.08, -104.9), (40.08, -105.0)]
    pond = [(40.02, -104.97), (40.04, -104.97), (40.04, -104.94), (40.02, -104.94)]
    overlapping = [(40.04, -104.95), (40.04, -104.85), (40.12, -104.85), (40.12, -104.95)]
    inner = [(40.05, -104.99), (40.05, -104.98), (40.06, -104.98), (40.06, -104.99)]
    apart = [(41.0, -105.0), (41.0, -104.9), (41.08, -104.9), (41.08, -105.0)]
    cases = [
        ("polygon", field, [], "polygon", overlapping, []),
        ("polygon", field, [], "polygon", inner, []),
        ("polygon", inner, [], "polygon", field, []),
        ("polygon", field, [], "polygon", apart, []),
        ("polygon", field, [pond], "polygon", [(40.025, -104.965), (40.035, -104.965), (40.035, -104.945), (40.025, -104.945)], []),
        ("line", [(40.03, -105.05), (40.05, -104.85)], [], "polygon", field, [pond]),
        ("line", [(40.01, -104.99), (40.07, -104.91)], [], "polygon", field, []),
        ("line", [(40.03, -105.05), (40.05, -104.85)], [], "line", [(39.95, -104.95), (40.1, -104.94)], []),
        ("points", [(40.01, -104.99), (40.5, -104.5)], [], "polygon", field, []),
        ("points", [(40.03, -104.96)], [], "polygon", field, [pond]),
        # Across the antimeridian: two boxes that overlap there.
        ("polygon", [(-1.0, 179.0), (-1.0, -179.5), (1.0, -179.5), (1.0, 179.0)], [], "polygon", [(-0.5, 179.5), (-0.5, -179.0), (0.5, -179.0), (0.5, 179.5)], []),
    ]
    for c in cases:
        add(*c[:3], *c[3:], "geodesic", geodesic_matrix(*c))
    # A long east-west edge: the geodesic from (45, -10) to (45, 10) bows north
    # to 45.44° N at 0°, so points at 45.1° and 45.3° N are outside the box on
    # the ground, and inside it with straight edges along 45° N.
    bow = [(45.0, -10.0), (45.0, 10.0), (46.0, 10.0), (46.0, -10.0)]
    for pt in [(45.1, 0.0), (45.3, 0.0)]:
        add("points", [pt], [], "polygon", bow, [], "geodesic", geodesic_matrix("points", [pt], [], "polygon", bow, []))
        add("points", [pt], [], "polygon", bow, [], "planar", shape("points", [pt]).relate(shape("polygon", bow)))

    # Degenerate by construction (geodesic).
    mid = on_edge((40.0, -105.0), (40.0, -104.9), 0.5)  # on the field's south edge
    add("points", [mid], [], "polygon", field, [], "geodesic", "F0FFFF212", "a point placed on the geodesic edge by GeodSolve is on the boundary")
    tri = [mid, (39.95, -104.9), (39.95, -105.0)]
    add("polygon", tri, [], "polygon", field, [], "geodesic", "FF2F01212", "a triangle whose corner GeodSolve placed on the field's edge touches it at that point")
    south = [(40.0, -105.0), (40.0, -104.9), (39.92, -104.9), (39.92, -105.0)]
    add("polygon", south, [], "polygon", field, [], "geodesic", "FF2F11212", "two polygons sharing an edge between the same corners touch along it")
    add("polygon", field, [], "polygon", list(reversed(field)), [], "geodesic", "2FFF1FFF2", "the same polygon, wound the other way, equals itself")
    add("line", [(40.0, -105.0), (40.0, -104.9)], [], "polygon", field, [], "geodesic", "F1FF0F212", "a line along the polygon's own edge lies in its boundary")

    # Planar, from GEOS, including the grid's degenerate cases.
    rng = random.Random(7)
    picked = [c for c in fixture(200, 99) if c["matrix"] not in {"FF2FF1212", "FF1FF0212", "FF0FFF212", "FF1FF0102", "FF0FFF102", "FF0FFF0F2"}]
    rng.shuffle(picked)
    for c in picked[:12]:
        ga, gb = c["geometry_a"], c["geometry_b"]
        def unrows(r):
            pts = [(v["lat"], v["lon"]) for v in r if not v.get("ring")]
            hole = [(v["lat"], v["lon"]) for v in r if v.get("ring")]
            return pts, ([hole] if hole else [])
        (pa, ha), (pb, hb) = unrows(ga), unrows(gb)
        add(c["kind_a"], pa, ha, c["kind_b"], pb, hb, "planar", c["matrix"])

    # Errors.
    out.append(vec(len(out) + 1, {"geometry_a": rows("polygon", field[:2]), "geometry_b": rows("polygon", field)}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC))
    out.append(vec(len(out) + 1, {"geometry_a": rows("polygon", field), "geometry_b": rows("polygon", [(p[0] + 20, p[1]) for p in field])}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}, SPEC))

    (root / "core/vectors/geometry.predicate.relate.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))
    print(len(fx), "fixture pairs,", len(out), "vectors")


if __name__ == "__main__":
    main()
