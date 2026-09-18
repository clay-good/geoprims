#!/usr/bin/env python3
"""H3 C differential fixtures via the `h3` Python bindings (H3 C inside).

    gen_h3_diff.py OUT.csv N SEED

Writes N random points x 16 resolutions: lat,lng,res,cell,center_lat,
center_lng,parent_res0 (the H3 C answers). The committed fixture
(core/crates/gp-indexing/tests/data/h3_c_diff.csv) uses N=250; the full
100,000-point run is local (H3_DIFF=path cargo test -- --ignored)."""
import math
import random
import sys

import h3

out, n, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
rng = random.Random(seed)
with open(out, "w") as f:
    f.write(f"# H3 C {h3.versions()['c']} via h3-py {h3.__version__}; seed {seed}\n")
    for _ in range(n):
        # Uniform on the sphere, so the poles get their share.
        lat = math.degrees(math.asin(rng.uniform(-1, 1)))
        lng = rng.uniform(-180, 180)
        for res in range(16):
            c = h3.latlng_to_cell(lat, lng, res)
            clat, clng = h3.cell_to_latlng(c)
            f.write(f"{lat!r},{lng!r},{res},{c},{clat!r},{clng!r},{h3.cell_to_parent(c, 0)}\n")
