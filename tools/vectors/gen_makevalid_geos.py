#!/usr/bin/env python3
"""Golden vectors for geometry.validity.make-valid from GEOS.

Validity has a definition -- OGC Simple Features 6.1.11.1 -- and GEOS is the
implementation the rest of the field checks against. So this asks GEOS three
things about each polygon: whether it is valid, how many parts repairing it
leaves, and what area those parts cover, the last measured back on the
ellipsoid by geographiclib rather than in the plane.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_makevalid_geos.py
"""
import json
from pathlib import Path

from geographiclib.geodesic import Geodesic
from geographiclib.polygonarea import PolygonArea
from pyproj import CRS, Transformer
from shapely import is_valid, make_valid
from shapely.geometry import Polygon

G = Geodesic.WGS84
OUT = Path(__file__).resolve().parents[2] / "core/vectors/geometry.validity.make-valid.jsonl"
SRC = (
    "GEOS 3.11.4 through shapely: is_valid and make_valid on PROJ's azimuthal "
    "equidistant plane, with the repaired area measured on the ellipsoid by "
    "geographiclib's PolygonArea"
)
VER = "GEOS 3.11.4 / PROJ 9.3.0 / geographiclib 2.1"
STEP_M = 5_000.0


def plane_for(pts):
    lat0 = sum(p[0] for p in pts) / len(pts)
    base = pts[0][1]
    lon0 = base + sum(((p[1] - base + 180.0) % 360.0) - 180.0 for p in pts) / len(pts)
    crs = CRS.from_proj4(
        f"+proj=aeqd +ellps=WGS84 +lat_0={lat0} +lon_0={lon0} +units=m +no_defs"
    )
    return (
        Transformer.from_crs("EPSG:4326", crs, always_xy=True),
        Transformer.from_crs(crs, "EPSG:4326", always_xy=True),
    )


def densified(pts):
    out = []
    for i, a in enumerate(pts):
        b = pts[(i + 1) % len(pts)]
        line = G.InverseLine(a[0], a[1], b[0], b[1])
        n = max(1, int(line.s13 / STEP_M) + 1)
        for k in range(n):
            p = line.Position(line.s13 * k / n)
            out.append((p["lat2"], p["lon2"]))
    return out


def verdict(pts):
    fwd, inv = plane_for(pts)
    poly = Polygon([fwd.transform(lo, la) for la, lo in densified(pts)])
    fixed = make_valid(poly)
    geoms = list(fixed.geoms) if fixed.geom_type.startswith("Multi") else [fixed]
    areas = [g for g in geoms if g.geom_type == "Polygon" and not g.is_empty]

    def ring(coords):
        pa = PolygonArea(G)
        for x, y in list(coords)[:-1]:
            lon, lat = inv.transform(x, y)
            pa.AddPoint(lat, lon)
        return abs(pa.Compute(False, True)[2])

    total = sum(ring(g.exterior.coords) - sum(ring(r.coords) for r in g.interiors) for g in areas)
    return {
        "valid": "yes" if is_valid(poly) else "no",
        "parts": len(areas),
        "area": total,
    }


SQUARE = [(40.0, -105.0), (40.0, -104.99), (40.01, -104.99), (40.01, -105.0)]
# The last field says whether the validity verdict and part count are pinned.
# Two degenerate inputs are marked False: a repeated corner, which GEOS calls
# valid and this tool calls a problem, and a zero-area spike, which GEOS
# repairs into two parts and this tool into one. Both are defensible readings
# of a degenerate ring, and the repaired AREA agrees in both cases, so that is
# what those two vectors pin. The divergence is written into the limitations
# rather than settled by fiat here.
CASES = [
    ("a valid square", SQUARE, True),
    ("a valid square, wound the other way", SQUARE[::-1], True),
    ("a bow tie", [(40.0, -105.0), (40.01, -104.99), (40.0, -104.99), (40.01, -105.0)], True),
    ("a valid L", [(40.0, -105.0), (40.0, -104.98), (40.01, -104.98),
                   (40.01, -104.99), (40.02, -104.99), (40.02, -105.0)], True),
    ("a ring pinched to a point", [(40.0, -105.0), (40.0, -104.98), (40.01, -104.99),
                                   (40.02, -104.98), (40.02, -105.0), (40.01, -104.99)], True),
    ("a square with a repeated corner", [(40.0, -105.0), (40.0, -104.99), (40.0, -104.99),
                                         (40.01, -104.99), (40.01, -105.0)], False),
    ("a square with a spike", [(40.0, -105.0), (40.0, -104.99), (40.005, -104.97),
                               (40.0, -104.99), (40.01, -104.99), (40.01, -105.0)], False),
    ("a valid triangle", [(-20.0, 30.0), (-20.04, 30.09), (-20.09, 30.02)], True),
    ("a crossed quadrilateral", [(-20.0, 30.0), (-20.09, 30.02), (-20.04, 30.09), (-20.02, 29.98)], True),
    ("a valid square at the equator", [(-0.01, -0.01), (-0.01, 0.01), (0.01, 0.01), (0.01, -0.01)], True),
    ("a bow tie at the equator", [(-0.01, -0.01), (0.01, 0.01), (-0.01, 0.01), (0.01, -0.01)], True),
    ("a valid square across the antimeridian", [(-17.0, 179.99), (-17.0, -179.99),
                                                (-16.99, -179.99), (-16.99, 179.99)], True),
    ("a bow tie across the antimeridian", [(-17.0, 179.99), (-16.99, -179.99),
                                           (-17.0, -179.99), (-16.99, 179.99)], True),
    ("a valid square at 70 north", [(70.0, 25.0), (70.0, 25.05), (70.02, 25.05), (70.02, 25.0)], True),
]



def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 8:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows = []
    for i, (name, pts, pin) in enumerate(CASES, start=start + 1):
        v = verdict(pts)
        # The repaired area is two measurements of one shape and agrees to
        # about 1e-10 relative everywhere, including on the two degenerate
        # inputs where the implementations describe the result differently.
        expect = {"ok": True, "result.area.value": v["area"]}
        tolerance = {"result.area.value": {"rel": 1e-8, "abs": 1e-6}}
        if pin:
            expect["result.valid"] = v["valid"]
            expect["result.parts"] = v["parts"]
            tolerance["result.valid"] = {"abs": 0}
            tolerance["result.parts"] = {"abs": 0}
        rows.append({
            "id": f"v{i:03d}",
            "input": {
                "polygon": [{"lat": la, "lon": lo} for la, lo in pts],
                "options": {"outputUnits": {"area": "m2"}},
            },
            "expect": expect,
            "source": f"{SRC}: {name}",
            "sourceVersion": VER,
            "tolerance": tolerance,
        })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
