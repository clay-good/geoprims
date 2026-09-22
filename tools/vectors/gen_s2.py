#!/usr/bin/env python3
"""The S2 parity fixture (core/crates/gp-indexing/tests/data/s2_parity.jsonl).

Generated with s2sphere, the Python implementation of the same algorithms:
    python3 -m pip install --user s2sphere
400 cases spread over the sphere, at levels from 0 to 30, each carrying the
token, the cell's center and vertices, its edge neighbours, and its exact
area, so the Rust port is checked against another implementation rather than
against itself.
"""
import json, math, random, s2sphere
from pathlib import Path
random.seed(20260922)
rows=[]
for n in range(400):
    lat = math.degrees(math.asin(random.uniform(-1,1)))
    lon = random.uniform(-180,180)
    level = random.choice([0,1,2,5,8,10,13,15,16,20,25,30])
    ll = s2sphere.LatLng.from_degrees(lat, lon)
    cid = s2sphere.CellId.from_lat_lng(ll).parent(level)
    c = s2sphere.Cell(cid)
    center = s2sphere.LatLng.from_point(c.get_center())
    verts = []
    for k in range(4):
        v = s2sphere.LatLng.from_point(c.get_vertex(k))
        verts.append([v.lat().degrees, v.lng().degrees])
    neigh = [x.to_token() for x in cid.get_edge_neighbors()]
    rows.append({"lat":lat,"lon":lon,"level":level,"token":cid.to_token(),
                 "center":[center.lat().degrees, center.lng().degrees],
                 "vertices":verts,"neighbors":neigh,
                 "area":c.exact_area()})
open(Path(__file__).resolve().parents[2] / 'core/crates/gp-indexing/tests/data/s2_parity.jsonl','w').write("".join(json.dumps(r,separators=(",",":"))+"\n" for r in rows))
print("cases", len(rows))
