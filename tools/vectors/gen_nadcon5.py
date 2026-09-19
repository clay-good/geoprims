#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.nadcon5 from PROJ's +proj=gridshift with
the same NADCON5 grid (https://cdn.proj.org/us_noaa_nadcon5_nad27_nad83_1986_conus.tif),
which applies NADCON5's biquadratic interpolation from the grid's metadata.

Writes core/vectors/geodesy.datum.nadcon5.jsonl. Requires pyproj and curl.
"""
import json
import random
import subprocess
import tempfile
from pathlib import Path

import pyproj
from pyproj import Transformer

OUT = Path(__file__).resolve().parents[2] / "core/vectors/geodesy.datum.nadcon5.jsonl"
NAME = "us_noaa_nadcon5_nad27_nad83_1986_conus.tif"
SRC = ("PROJ +proj=gridshift with the NADCON5 grid " + NAME + ", through pyproj", "PROJ 9.3; NADCON5 20160901")


def main():
    with tempfile.TemporaryDirectory() as d:
        subprocess.run(["curl", "-sL", "-m", "120", "-o", str(Path(d) / NAME), "https://cdn.proj.org/" + NAME], check=True)
        pyproj.datadir.append_data_dir(d)
        pipe = f"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=gridshift +grids={NAME} +step +proj=unitconvert +xy_in=rad +xy_out=deg"
        t = Transformer.from_pipeline(pipe)
        rnd = random.Random(20260919)
        pts = [(39.224, -98.542), (40.446111, -79.982222), (34.05, -118.25), (47.6, -122.33), (25.77, -80.19), (44.98, -93.27), (24.0, -125.0 + 0.25 * 3),
               (49.99, -66.1), (38.0, -100.5), (36.125, -115.125)]
        pts += [(round(rnd.uniform(24.5, 49.5), 6), round(rnd.uniform(-124.5, -67.5), 6)) for _ in range(12)]
        vs = []
        for i, (lat, lon) in enumerate(pts):
            fwd = i % 3 != 2
            lo, la = t.transform(lon, lat) if fwd else t.transform(lon, lat, direction="INVERSE")
            inp = {"lat": lat, "lon": lon}
            if not fwd:
                inp["direction"] = "nad83-to-nad27"
            vs.append({"id": f"v{len(vs) + 1:03d}", "input": inp, "expect": {"result.lat.value": la, "result.lon.value": lo, "ok": True},
                       "source": SRC[0], "sourceVersion": SRC[1], "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}}})
    # "Outside grid": a NAD 27 point in Mexico beyond NADCON5 CONUS coverage.
    vs.append({"id": f"v{len(vs) + 1:03d}", "input": {"lat": 19.43, "lon": -99.13}, "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
               "source": "add-geodesy-suite scenario: outside grid", "sourceVersion": "2026-09"})
    OUT.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
    print(OUT.name, len(vs))


if __name__ == "__main__":
    main()
