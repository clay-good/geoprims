#!/usr/bin/env python3
"""Builds core/crates/gp-time/data/tzdb.bin from IANA tzdata.

    tzdb.py TZDATA_DIR ZIC [OUT]

TZDATA_DIR holds an unpacked tzdata release (tzdata-latest.tar.gz from
data.iana.org); ZIC is a zic built from the matching tzcode release. Zones are
compiled with `zic -b slim` (TZif v2+ with a POSIX TZ footer) and packed:

    "GPTZ" | u8 version length | version | u16 zone count | u16 blob count
    zones:  u8 name length | name | u16 blob index      (sorted by name)
    blobs:  u32 length | TZif bytes                      (deduplicated)

All integers are little-endian. The tz database is in the public domain.
"""
import hashlib
import os
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

SOURCES = ["africa", "antarctica", "asia", "australasia", "europe", "northamerica", "southamerica", "etcetera", "backward"]


def main():
    tzdata, zic = Path(sys.argv[1]), sys.argv[2]
    out = Path(sys.argv[3] if len(sys.argv) > 3 else "core/crates/gp-time/data/tzdb.bin")
    version = (tzdata / "version").read_text().strip()
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run([zic, "-b", "slim", "-d", tmp, *[str(tzdata / s) for s in SOURCES]], check=True)
        zones, blobs, index = [], [], {}
        for d, _, files in os.walk(tmp):
            for f in files:
                path = Path(d) / f
                name = str(path.relative_to(tmp))
                data = path.read_bytes()
                if not data.startswith(b"TZif"):
                    continue
                h = hashlib.sha256(data).digest()
                if h not in index:
                    index[h] = len(blobs)
                    blobs.append(data)
                zones.append((name, index[h]))
    zones.sort()
    buf = bytearray(b"GPTZ")
    buf += struct.pack("<B", len(version)) + version.encode()
    buf += struct.pack("<HH", len(zones), len(blobs))
    for name, i in zones:
        n = name.encode()
        buf += struct.pack("<B", len(n)) + n + struct.pack("<H", i)
    for b in blobs:
        buf += struct.pack("<I", len(b)) + b
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(bytes(buf))
    print(f"tzdata {version}: {len(zones)} zones, {len(blobs)} unique, {len(buf)} bytes -> {out}")


if __name__ == "__main__":
    main()
