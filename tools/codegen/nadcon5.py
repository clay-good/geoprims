#!/usr/bin/env python3
"""Packs the first-step NADCON5 grids (each region's old datum to NAD 83) as
the geoprims asset `nadcon5`, version 20160901.

Source: NGS NADCON5 (20160901 release), as converted to GeoTIFF by PROJ
(https://cdn.proj.org/, public domain). Each output file has a one-line
ASCII header
  "NADCON5 <west> <south> <xstep> <ystep> <cols> <rows>\n"
followed by cols*rows pairs of little-endian float32 (latitude offset,
longitude offset east-positive), in arc-seconds, south row first. West may
exceed 180 for grids that cross the antimeridian (Alaska).
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
ID, VERSION = "nadcon5", "20160901"
GRIDS = ["nad27_nad83_1986_conus", "nad27_nad83_1986_alaska", "ohd_nad83_1986_hawaii", "pr40_nad83_1986_prvi",
         "sp1952_nad83_1986_stpaul", "as62_nad83_1993_as", "gu63_nad83_1993_guamcnmi"]


def pack(tif):
    page = tifffile.TiffFile(tif).pages[0]
    sx, sy, _ = page.tags["ModelPixelScaleTag"].value
    _, _, _, west, north, _ = page.tags["ModelTiepointTag"].value
    a = page.asarray()
    rows, cols, _ = a.shape
    south = north - sy * (rows - 1)
    out = bytearray(f"NADCON5 {west!r} {south!r} {sx!r} {sy!r} {cols} {rows}\n".encode())
    for r in range(rows - 1, -1, -1):
        for c in range(cols):
            out += struct.pack("<ff", float(a[r, c, 0]), float(a[r, c, 1]))
    return bytes(out)


def main():
    files = {}
    dest = ROOT / "assets/data" / ID / VERSION
    dest.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as d:
        for g in GRIDS:
            tif = Path(d) / f"{g}.tif"
            subprocess.run(["curl", "-sL", "-m", "120", "-o", str(tif), f"https://cdn.proj.org/us_noaa_nadcon5_{g}.tif"], check=True)
            data = pack(tif)
            (dest / f"{g}.grid").write_bytes(data)
            files[f"{g}.grid"] = {"sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)}
    reg_path = ROOT / "assets/registry.json"
    reg = json.loads(reg_path.read_text())
    reg["assets"] = [x for x in reg["assets"] if not x["id"].startswith("nadcon5")] + [{
        "id": ID,
        "version": VERSION,
        "title": "NADCON5 first-step grids: each region's old datum to NAD 83",
        "issuer": "National Geodetic Survey, NOAA; GeoTIFF packaging by the PROJ project",
        "license": "Public domain (US Government work)",
        "attribution": "NADCON5 by the National Geodetic Survey (NOAA); grids from PROJ-data.",
        "sourceUrl": "https://cdn.proj.org/",
        "retrievedAt": "2026-09-19",
        "files": files,
        "tiling": "one file per region",
        "loadPolicy": "on-demand",
        "interpolation": "NADCON5 biquadratic (NOAA TM NOS NGS 84), as PROJ implements it",
    }]
    reg_path.write_text(json.dumps(reg, indent=2, ensure_ascii=False) + "\n")
    print(len(files), "grids,", sum(f["bytes"] for f in files.values()), "bytes")


if __name__ == "__main__":
    main()
