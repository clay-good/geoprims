#!/usr/bin/env python3
"""Build one compact Natural Earth base-map file.

Natural Earth 5.1.2 source files come from the project's official repository:
https://github.com/nvkelso/natural-earth-vector/tree/v5.1.2/geojson

The input digests below pin the upstream files. Output paths are fixed so the
110m map remains bundled while the larger 50m map is served on demand.

Format: each ring or line is a flat list of integers in hundredths of a
degree, the first pair absolute and the rest as differences from the previous
pair. Places are [name, lon, lat, minZoom], largest population first.

Usage: basemap.py <110m|50m> <directory containing the source GeoJSON>
"""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONFIG = {
    "110m": {
        "out": ROOT / "apps/web/public/basemap/ne-110m.json",
        "digests": {
            "land": "9e0729ee253ca7d7a5c4ae9395fb1902264c5377c52e224d13dd85010e2835d9",
            "borders": "d42479fd79552cca4eec7f85fcdca717a790d29ff06be7676f1af0568c6d3f7c",
            "lakes": "eb02ecc86c82004fccbf979058bfabbbd6c2d07968c7844d38eb1c9152d2ffc9",
            "states": "f204e94d5c4d16c6ce4b59ecb50e264bd95c22b9138d8a994c010c222e186aad",
            "places": "0dbd25c9ad8bd797ddf164b067f563be5c16be2c002254eb594862377963f9dc",
        },
    },
    "50m": {
        "out": ROOT / "assets/data/ne-50m/5.1.2/ne-50m.json",
        "digests": {
            "land": "e874b27a51d146452be360cafb3cc50c86001074a67d534113e6534682f9826b",
            "borders": "2faac4f6b34386f3d21b6e018cf151f241f00e5c936d44dd17d7d9bfb147fa48",
            "lakes": "d350b75978b26fe839b797c2c529b2fb8f47fb3983c03f4964e36d5df9378a52",
            "states": "72cca93c850d412628a5da4bc5ebfe21ba4d376eb34611bde6b623ee73f0fdcf",
            "places": "8e70756b39fae9bcdc1e332bfc510c024c5edd3a13203ffd20092ee37b61d978",
        },
    },
}
FILES = {
    "land": "ne_{scale}_land.geojson",
    "borders": "ne_{scale}_admin_0_boundary_lines_land.geojson",
    "lakes": "ne_{scale}_lakes.geojson",
    "states": "ne_{scale}_admin_1_states_provinces_lines.geojson",
    "places": "ne_{scale}_populated_places_simple.geojson",
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


def source(src, scale, key, digest):
    name = FILES[key].format(scale=scale)
    data = (src / name).read_bytes()
    assert hashlib.sha256(data).hexdigest() == digest, f"{name} digest mismatch"
    return json.loads(data)


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in CONFIG:
        raise SystemExit(f"usage: {Path(sys.argv[0]).name} <110m|50m> <source-directory>")
    scale, src = sys.argv[1], Path(sys.argv[2])
    cfg = CONFIG[scale]
    out = {
        "source": f"Natural Earth 5.1.2, 1:{scale} (public domain)",
        "units": "hundredths of a degree, delta-encoded after the first pair",
    }
    for key in ("land", "borders", "lakes", "states"):
        features = source(src, scale, key, cfg["digests"][key])["features"]
        out[key] = [encode(ring) for feature in features for ring in parts(feature["geometry"])]
    places = sorted(
        source(src, scale, "places", cfg["digests"]["places"])["features"],
        key=lambda feature: -(feature["properties"]["pop_max"] or 0),
    )
    out["places"] = [
        [feature["properties"]["name"], round(feature["properties"]["longitude"] * 100),
         round(feature["properties"]["latitude"] * 100), feature["properties"]["min_zoom"]]
        for feature in places
    ]
    target = cfg["out"]
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(out, separators=(",", ":")))
    print(target, target.stat().st_size, "bytes")


if __name__ == "__main__":
    main()
