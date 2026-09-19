#!/usr/bin/env python3
"""Builds the Natural Earth 110m base layer for the map canvas
(web/map-canvas "Natural Earth is the only basemap") as one compact static
file, apps/web/public/basemap/ne-110m.json.

Source: Natural Earth 5.1.2 (public domain), from
https://github.com/nvkelso/natural-earth-vector/tree/v5.1.2/geojson:
  ne_110m_land.geojson                       sha256 9e0729ee253ca7d7a5c4ae9395fb1902264c5377c52e224d13dd85010e2835d9
  ne_110m_admin_0_boundary_lines_land.geojson sha256 d42479fd79552cca4eec7f85fcdca717a790d29ff06be7676f1af0568c6d3f7c
  ne_110m_lakes.geojson                      sha256 eb02ecc86c82004fccbf979058bfabbbd6c2d07968c7844d38eb1c9152d2ffc9

Format: each ring or line is a flat list of integers in hundredths of a
degree, the first pair absolute and the rest as differences from the previous
pair: [lon0, lat0, dlon1, dlat1, ...]. Usage: basemap.py <dir with the geojson>.
"""
import hashlib
import json
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "apps/web/public/basemap/ne-110m.json"
SOURCES = {
    "land": ("ne_110m_land.geojson", "9e0729ee253ca7d7a5c4ae9395fb1902264c5377c52e224d13dd85010e2835d9"),
    "borders": ("ne_110m_admin_0_boundary_lines_land.geojson", "d42479fd79552cca4eec7f85fcdca717a790d29ff06be7676f1af0568c6d3f7c"),
    "lakes": ("ne_110m_lakes.geojson", "eb02ecc86c82004fccbf979058bfabbbd6c2d07968c7844d38eb1c9152d2ffc9"),
}


def encode(coords):
    flat, prev = [], None
    for lon, lat in coords:
        q = (round(lon * 100), round(lat * 100))
        if prev is None:
            flat += [q[0], q[1]]
        elif q != prev:
            flat += [q[0] - prev[0], q[1] - prev[1]]
        else:
            continue
        prev = q
    return flat


def parts(geom):
    t, c = geom["type"], geom["coordinates"]
    if t == "Polygon":
        return c
    if t == "MultiPolygon":
        return [ring for poly in c for ring in poly]
    if t == "LineString":
        return [c]
    if t == "MultiLineString":
        return c
    raise ValueError(t)


def main():
    src = Path(sys.argv[1])
    out = {"source": "Natural Earth 5.1.2, 1:110m (public domain)", "units": "hundredths of a degree, delta-encoded after the first pair"}
    for key, (name, digest) in SOURCES.items():
        data = (src / name).read_bytes()
        assert hashlib.sha256(data).hexdigest() == digest, f"{name} digest mismatch"
        feats = json.loads(data)["features"]
        out[key] = [encode(ring) for f in feats for ring in parts(f["geometry"])]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(out, separators=(",", ":")))
    print(OUT, OUT.stat().st_size, "bytes")


if __name__ == "__main__":
    main()
