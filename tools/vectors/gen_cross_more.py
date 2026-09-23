#!/usr/bin/env python3
"""More golden vectors for indexing.convert.cross-index.

`gen_cross.py` works out each indexing system's cell size from its own
published definition -- the geohash bit split, the Plus Code and Maidenhead
character steps, the Web Mercator width at a latitude, the MGRS digit count,
the S2 average cell area -- and picks the resolution closest to the target by
log distance, since sizes step by factors rather than evenly. This reuses those
helpers unchanged and adds the cases they did not reach: the equator to 80
degrees north and south, target sizes from a metre to a thousand kilometres,
and the latitudes where the systems measured in degrees and the ones measured
in Web Mercator pull apart.

The H3 resolution is pinned too, from the published average-area table. Its
*size* is not: the core carries a more precise table than the documentation
prints, so the two agree on which resolution to choose and differ in the fifth
digit of how big it is.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_cross_more.py
"""
import json
import math
import sys
from pathlib import Path

from gen_cross import OUT, SRC, VER, H3_AREA, pick, sizes

CASES = [
    (0.0, 0.0, 1.0), (0.0, 0.0, 10000.0), (0.0, 0.0, 1000000.0),
    (20.0, 30.0, 500.0), (35.6762, 139.6503, 50.0), (-22.9068, -43.1729, 300.0),
    (45.0, -75.0, 1.0), (45.0, -75.0, 100.0), (45.0, -75.0, 100000.0),
    (60.0, 10.0, 5.0), (70.0, 25.0, 2000.0), (80.0, -60.0, 100.0),
    (-45.0, 170.0, 20.0), (-60.0, -60.0, 750.0), (-80.0, 120.0, 400.0),
    (1.3521, 103.8198, 30.0),
]


def h3_pick(target):
    """The H3 resolution whose average cell is closest to the target."""
    return pick([(r, math.sqrt(a * 1e6)) for r, a in enumerate(H3_AREA)], target)


def build(start):
    rows = []
    for i, (lat, lon, target) in enumerate(CASES, start + 1):
        gh, olc, qth, tile, mg = sizes(lat, target)
        s2 = pick([(lvl, math.sqrt(4 * math.pi / (6 * 4 ** lvl) * 6371008.8 ** 2)) for lvl in range(0, 31)], target)
        e = {
            "result.cells.0.resolution": f"resolution {h3_pick(target)[0]}",
            "result.cells.1.resolution": f"precision {gh[0]}",
            "result.cells.2.resolution": f"{olc[0]} characters",
            "result.cells.3.resolution": f"zoom {tile[0]}",
            "result.cells.4.resolution": f"{qth[0]} characters",
            "result.cells.5.resolution": f"level {s2[0]}",
            "result.cells.6.resolution": f"{mg[0]} digits, {int(mg[1])} m squares",
            "result.cells.1.cell_size.value": gh[1],
            "result.cells.3.cell_size.value": tile[1],
            "result.cells.5.cell_size.value": s2[1],
            "ok": True,
        }
        tol = {k: {"rel": 1e-9, "abs": 1e-6} for k, v in e.items() if isinstance(v, float)}
        rows.append({"id": f"v{i:03d}", "input": {"lat": lat, "lon": lon, "target_size": f"{target:g} m"},
                     "expect": e, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    return rows


def main():
    path = (Path(sys.argv[1]) if len(sys.argv) > 1 else OUT) / "indexing.convert.cross-index.jsonl"
    existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start > 10:
        raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
    rows = build(start)
    with path.open("a") as f:
        for r in rows:
            f.write(json.dumps(r, separators=(",", ":")) + "\n")
    print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
