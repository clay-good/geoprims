import json, math, random, s2sphere
from pathlib import Path
OUT = Path("/Users/user/Documents/development/public/geoprims/.claude/worktrees/geospatial-aviation-spec-b79149/core/vectors")
SRC = "s2sphere, an independent Python implementation of the S2 algorithms (tools/vectors/gen_s2.py)"
VER = "s2sphere 0.2.5"
random.seed(4090)

def vec(i, inp, expect, tol=1e-9):
    e = dict(expect); e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": SRC, "sourceVersion": VER, "tolerance": t}

pts = [(40.446111, -79.982222, 15), (0, 0, 10), (-33.8688, 151.2093, 20), (89.5, 45, 8),
       (-89.5, -120, 12), (51.5074, -0.1278, 16), (35.6762, 139.6503, 22), (-22.9068, -43.1729, 5),
       (60, 10, 13), (1.3521, 103.8198, 18)]
rows = []
for i, (lat, lon, level) in enumerate(pts, 1):
    cid = s2sphere.CellId.from_lat_lng(s2sphere.LatLng.from_degrees(lat, lon)).parent(level)
    c = s2sphere.Cell(cid)
    ctr = s2sphere.LatLng.from_point(c.get_center())
    rows.append(vec(i, {"lat": lat, "lon": lon, "level": level},
                    {"result.cell": cid.to_token(), "result.cell_id": str(cid.id()),
                     "result.level": float(level), "result.face": float(cid.face()),
                     "result.lat.value": ctr.lat().degrees, "result.lon.value": ctr.lng().degrees}))
(OUT / "indexing.s2.lat-lng-to-cell.jsonl").write_text(
    "".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
print("lat-lng-to-cell", len(rows))

rows = []
for i, (lat, lon, level) in enumerate(pts, 1):
    cid = s2sphere.CellId.from_lat_lng(s2sphere.LatLng.from_degrees(lat, lon)).parent(level)
    c = s2sphere.Cell(cid)
    ctr = s2sphere.LatLng.from_point(c.get_center())
    e = {"result.cell": cid.to_token(), "result.level": float(level), "result.face": float(cid.face()),
         "result.lat.value": ctr.lat().degrees, "result.lon.value": ctr.lng().degrees}
    if level > 0:
        e["result.parent"] = cid.parent(level - 1).to_token()
    if level < 30:
        for k, ch in enumerate(cid.children()):
            e[f"result.children.{k}.cell"] = ch.to_token()
    for k in range(4):
        v = s2sphere.LatLng.from_point(c.get_vertex(k))
        e[f"result.boundary.{k}.lat.value"] = v.lat().degrees
        e[f"result.boundary.{k}.lon.value"] = v.lng().degrees
    rows.append(vec(i, {"cell": cid.to_token()}, e))
(OUT / "indexing.s2.cell-info.jsonl").write_text(
    "".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
print("cell-info", len(rows))

rows = []
for i, (lat, lon, level) in enumerate(pts, 1):
    cid = s2sphere.CellId.from_lat_lng(s2sphere.LatLng.from_degrees(lat, lon)).parent(level)
    e = {"result.cell": cid.to_token(), "result.level": float(level)}
    for k, n in enumerate(cid.get_edge_neighbors()):
        e[f"result.neighbors.{k}.cell"] = n.to_token()
    e["result.across_a_face_edge"] = float(sum(1 for n in cid.get_edge_neighbors() if n.face() != cid.face()))
    rows.append(vec(i, {"cell": cid.to_token()}, e))
(OUT / "indexing.s2.neighbors.jsonl").write_text(
    "".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
print("neighbors", len(rows))
