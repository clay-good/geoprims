#!/usr/bin/env python3
"""Golden vectors for indexing.h3.cell-to-local-ij and
indexing.h3.local-ij-to-cell from H3 C through the `h3` Python bindings
(pip install h3==4.4.2).

    gen_h3_localij.py [core/vectors]

Local IJ coordinates are whole numbers, so H3 C and h3o agree exactly; the
pairs are taken around origins spread over the globe and at several
resolutions, each checked to round trip.
"""
import json
import sys
from pathlib import Path

import h3

SRC = f"H3 C {h3.versions()['c']} via h3-py {h3.__version__}"
VER = f"H3 {h3.versions()['c']}"
ORIGINS = [
    (40.446111, -79.982222, 9),
    (-33.8688, 151.2093, 7),
    (51.5074, -0.1278, 12),
    (64.8378, -147.7164, 5),
    (-0.5, 179.999, 3),
    (35.6762, 139.6503, 10),
    (-22.9068, -43.1729, 6),
    (55.7558, 37.6173, 11),
    (-33.9249, 18.4241, 8),
    (19.4326, -99.1332, 4),
    (1.3521, 103.8198, 13),
]


def vec(i, inp, exp):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 1e-11, "abs": 1e-9} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": SRC, "sourceVersion": VER, "tolerance": t}


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    to_ij, to_cell = [], []
    n = 0
    for lat, lon, res in ORIGINS:
        origin = h3.latlng_to_cell(lat, lon, res)
        # The origin itself, then a neighbor, so each vector is a distinct pair.
        for cell in (origin, sorted(h3.grid_disk(origin, 1))[-1]):
            i, j = h3.cell_to_local_ij(origin, cell)
            assert h3.local_ij_to_cell(origin, i, j) == cell, "round trip"
            n += 1
            to_ij.append(vec(n, {"origin": origin, "cell": cell},
                            {"result.i": float(i), "result.j": float(j), "result.anchor": origin, "result.cell": cell}))
            clat, clng = h3.cell_to_latlng(cell)
            to_cell.append(vec(n, {"origin": origin, "i": i, "j": j},
                             {"result.cell": cell, "result.lat.value": clat, "result.lon.value": clng}))
    for name, rows in (("cell-to-local-ij", to_ij), ("local-ij-to-cell", to_cell)):
        path = out / f"indexing.h3.{name}.jsonl"
        path.write_text("".join(json.dumps(r) + "\n" for r in rows))
        print(f"{path}: {len(rows)} vectors")


if __name__ == "__main__":
    main()
