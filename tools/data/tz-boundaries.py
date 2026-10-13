#!/usr/bin/env python3
"""Build the compact time-zone polygon index used by the Wasm core.

The input is timezone-boundary-builder's pinned comprehensive GeoJSON ZIP.
Coordinates are rounded to microdegrees (under 0.06 m per ordinate), then
delta- and varint-encoded without simplifying or dropping vertices.

Format (little endian):
  GPTB, version u8, decimal scale u8, zone count u16, polygon count u32
  zone names: length u8, UTF-8 bytes
  polygons: zone u16, bbox 4*i32, payload length u32, ring count u16
  rings: point count u32, then zigzag-varint lon/lat; first pair absolute,
         later pairs are deltas from the preceding point

Usage: python3 tools/data/tz-boundaries.py
"""

import hashlib
import json
import struct
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "assets/data/tz-boundaries/2026e/timezones.geojson.zip"
TARGET = ROOT / "assets/data/tz-boundaries/2026e/boundaries.bin"
SOURCE_SHA256 = "a75361c760d6de2fc95c63e97e0b1086d5dfe6fdb2bc6d96efeb3a94673aaa01"
SCALE = 1_000_000


def varint(value):
    value = (value << 1) ^ (value >> 63)
    out = bytearray()
    while value >= 0x80:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    out.append(value)
    return out


def quantize(value):
    return round(value * SCALE)


def polygons(geometry):
    if geometry["type"] == "Polygon":
        return [geometry["coordinates"]]
    if geometry["type"] == "MultiPolygon":
        return geometry["coordinates"]
    raise ValueError(f"unsupported geometry {geometry['type']}")


def encode_polygon(rings):
    payload = bytearray()
    xs, ys = [], []
    encoded_rings = []
    for ring in rings:
        points = []
        for lon, lat in ring:
            point = (quantize(lon), quantize(lat))
            if not points or point != points[-1]:
                points.append(point)
        if len(points) > 1 and points[0] == points[-1]:
            points.pop()
        if len(points) < 3:
            raise ValueError("a ring has fewer than three distinct points")
        xs.extend(point[0] for point in points)
        ys.extend(point[1] for point in points)
        encoded_rings.append(points)

    for points in encoded_rings:
        payload += struct.pack("<I", len(points))
        previous = (0, 0)
        for i, point in enumerate(points):
            pair = point if i == 0 else (point[0] - previous[0], point[1] - previous[1])
            payload += varint(pair[0])
            payload += varint(pair[1])
            previous = point
    return (min(xs), min(ys), max(xs), max(ys)), payload, len(encoded_rings)


def main():
    source = SOURCE.read_bytes()
    digest = hashlib.sha256(source).hexdigest()
    if digest != SOURCE_SHA256:
        raise SystemExit(f"source digest {digest}, expected {SOURCE_SHA256}")
    with zipfile.ZipFile(SOURCE) as archive:
        if archive.namelist() != ["combined.json"]:
            raise SystemExit(f"unexpected archive members: {archive.namelist()}")
        collection = json.load(archive.open("combined.json"))

    features = sorted(collection["features"], key=lambda feature: feature["properties"]["tzid"])
    names = sorted(feature["properties"]["tzid"] for feature in features)
    if len(names) != len(set(names)) or any(len(name.encode()) > 255 for name in names):
        raise SystemExit("zone identifiers must be unique and fit in one-byte lengths")
    name_index = {name: i for i, name in enumerate(names)}
    encoded = []
    for feature in features:
        zone = name_index[feature["properties"]["tzid"]]
        for polygon in polygons(feature["geometry"]):
            bbox, payload, ring_count = encode_polygon(polygon)
            if ring_count > 65_535:
                raise SystemExit("polygon has too many rings")
            encoded.append((zone, bbox, payload, ring_count))

    output = bytearray(b"GPTB")
    output += struct.pack("<BBHI", 1, 6, len(names), len(encoded))
    for name in names:
        raw = name.encode()
        output.append(len(raw))
        output += raw
    for zone, bbox, payload, ring_count in encoded:
        output += struct.pack("<HiiiiIH", zone, *bbox, len(payload), ring_count)
        output += payload
    TARGET.write_bytes(output)
    print(f"{TARGET}: {len(names)} zones, {len(encoded)} polygons, {len(output):,} bytes, sha256 {hashlib.sha256(output).hexdigest()}")


if __name__ == "__main__":
    main()
