#!/usr/bin/env python3
"""A second pass of vectors for three navigation tools, from the same sources.

`gen_nav_geodesic.py` brought midpoint, vertex and range-rings to 17, 15 and 17
live vectors; the stable bar asks for twenty. This adds the cases that were
left out, using that script's own helpers so the reference is unchanged:
GeodSolve for the geodesic and Planimeter for the ring areas.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_nav_geodesic2.py
"""
import json
import sys
from pathlib import Path

import gen_nav_geodesic as g

MORE_PAIRS = [
    (35.6762, 139.6503, 37.6213, -122.379),   # Tokyo to San Francisco
    (55.7558, 37.6173, -26.2041, 28.0473),    # Moscow to Johannesburg
    (19.4326, -99.1332, 40.4168, -3.7038),    # Mexico City to Madrid
    (-41.2865, 174.7762, -33.4489, -70.6693), # Wellington to Santiago
    (64.1466, -21.9426, 1.3521, 103.8198),    # Reykjavik to Singapore
    (-54.8019, -68.3030, 78.2232, 15.6267),   # Ushuaia to Svalbard
]
MORE_RINGS = [
    (19.4326, -99.1332, [10_000.0], 144),
    (78.2232, 15.6267, [300_000.0], 144),
    (-41.2865, 174.7762, [150_000.0, 400_000.0], 144),
    (0.0, 179.5, [80_000.0], 144),   # a ring that crosses the antimeridian
]


def main():
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else g.ROOT
    g.PAIRS[:] = MORE_PAIRS
    g.RINGS[:] = MORE_RINGS
    for tool, build in [
        ("navigation.geodesic.midpoint", g.midpoint_rows),
        ("navigation.geodesic.vertex", g.vertex_rows),
        ("navigation.route.range-rings", g.ring_rows),
    ]:
        path = root / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 20:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol in rows:
                f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                    "source": g.SRC, "sourceVersion": g.VER, "tolerance": tol}) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
