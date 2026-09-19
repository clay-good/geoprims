#!/usr/bin/env python3
"""Packs the NADCON5 NAD27 -> NAD83(1986) CONUS grid as a geoprims asset.

Source: NGS NADCON5 (20160901 release), as converted to GeoTIFF by PROJ
(https://cdn.proj.org/us_noaa_nadcon5_nad27_nad83_1986_conus.tif, public
domain). Output: assets/data/nadcon5-nad27-nad83-1986-conus/20160901/
nad27_nad83_1986_conus.grid, a one-line ASCII header
  "NADCON5 <west> <south> <step> <cols> <rows>\n"
followed by cols*rows pairs of little-endian float32 (latitude offset,
longitude offset east-positive), in arc-seconds, south row first.
Requires tifffile and imagecodecs.
"""
import hashlib
import json
import struct
import subprocess
import tempfile
from pathlib import Path

import tifffile

ROOT = Path(__file__).resolve().parents[2]
URL = "https://cdn.proj.org/us_noaa_nadcon5_nad27_nad83_1986_conus.tif"
ID, VERSION, FILE = "nadcon5-nad27-nad83-1986-conus", "20160901", "nad27_nad83_1986_conus.grid"


def main():
    with tempfile.TemporaryDirectory() as d:
        tif = Path(d) / "g.tif"
        subprocess.run(["curl", "-sL", "-m", "120", "-o", str(tif), URL], check=True)
        t = tifffile.TiffFile(tif)
        page = t.pages[0]
        sx, sy, _ = page.tags["ModelPixelScaleTag"].value
        _, _, _, west, north, _ = page.tags["ModelTiepointTag"].value
        a = page.asarray()
    rows, cols, _ = a.shape
    assert sx == sy
    south = north - sy * (rows - 1)
    out = bytearray(f"NADCON5 {west} {south} {sx} {cols} {rows}\n".encode())
    for r in range(rows - 1, -1, -1):
        for c in range(cols):
            out += struct.pack("<ff", float(a[r, c, 0]), float(a[r, c, 1]))
    dest = ROOT / "assets/data" / ID / VERSION / FILE
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(bytes(out))
    reg_path = ROOT / "assets/registry.json"
    reg = json.loads(reg_path.read_text())
    reg["assets"] = [x for x in reg["assets"] if x["id"] != ID] + [{
        "id": ID,
        "version": VERSION,
        "title": "NADCON5 NAD 27 to NAD 83 (1986), conterminous United States",
        "issuer": "National Geodetic Survey, NOAA; GeoTIFF packaging by the PROJ project",
        "license": "Public domain (US Government work)",
        "attribution": "NADCON5 by the National Geodetic Survey (NOAA); grid from PROJ-data.",
        "sourceUrl": URL,
        "retrievedAt": "2026-09-19",
        "files": {FILE: {"sha256": hashlib.sha256(out).hexdigest(), "bytes": len(out)}},
        "tiling": "none",
        "loadPolicy": "on-demand",
        "interpolation": "NADCON5 biquadratic (NOAA TM NOS NGS 84), as PROJ implements it",
    }]
    reg_path.write_text(json.dumps(reg, indent=2, ensure_ascii=False) + "\n")
    print(dest, len(out), "bytes")


if __name__ == "__main__":
    main()
