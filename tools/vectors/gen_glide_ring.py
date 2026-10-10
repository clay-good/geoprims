#!/usr/bin/env python3
"""Golden vectors for the reachable ring of aviation.performance.glide, placed
by GeographicLib for Python (Karney), which the core does not use.

At each bearing the range is the still-air range scaled by the groundspeed
along that bearing over the airspeed, (V - W cos(bearing - wind from)) / V, and
Geodesic.WGS84.Direct carries the position that far. Every sixth point of the
tool's 72 is held: bearings 0, 30, ..., 330. Ranges, glide angle, and sink rate
are worked here from the tool's stated relations.

Positions from 60 S to 70 N, heights of 1,500 to 35,000 ft, glide ratios of 7
to 45, winds up to 60% of the airspeed from every quarter, and still air.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_glide_ring.py   (with geographiclib installed)
"""
import json
import math
from importlib.metadata import version
from pathlib import Path

from geographiclib.geodesic import Geodesic

VER = f"geographiclib {version('geographiclib')}"
SRC = f"Ring by GeographicLib for Python {version('geographiclib')} (Geodesic.WGS84.Direct); ranges from the stated glide relations (tools/vectors/gen_glide_ring.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.performance.glide.jsonl"
FT, NM, KT = 0.3048, 1852.0, 1852.0 / 3600.0

# Lines from other generators are kept byte for byte.
kept = [line for line in OUT.read_text().splitlines() if "gen_glide_ring.py" not in json.loads(line)["source"]]
rows = []
# lat, lon, height ft, glide ratio, TAS kt, headwind kt, wind from deg or None
CASES = [(39.86, -104.67, 5000, 9, 70, 20, 270), (51.47, -0.45, 3000, 10, 65, 0, None), (-33.95, 151.18, 8000, 12, 80, 15, 135),
         (64.13, -21.94, 10000, 25, 55, 10, 20), (-54.84, -68.3, 1500, 8, 68, 5, 250), (1.36, 103.99, 35000, 17, 220, 60, 90),
         (70.2, -148.46, 12000, 9.5, 90, 54, 330), (35.55, 139.78, 4500, 7, 75, 12, 180), (-17.55, -149.61, 6000, 11, 100, 30, 110),
         (19.44, -99.07, 2500, 8.5, 72, 8, 5), (-26.14, 28.25, 9000, 30, 60, 25, 300), (47.45, 8.56, 15000, 45, 58, 18, 225),
         (25.25, 55.36, 20000, 15, 180, 40, 65), (-60.0, -45.0, 7000, 10, 85, 0, None), (0.0, -179.9, 18000, 14, 160, 35, 270)]
for lat, lon, h_ft, ld, tas, hw, frm in CASES:
    still = h_ft * FT * ld
    inp = {"height": f"{h_ft} ft", "glide_ratio": ld, "tas": f"{tas} kt", "headwind": f"{hw} kt", "lat": lat, "lon": lon}
    if frm is not None:
        inp["wind_direction"] = f"{frm} deg"
    exp = {"ok": True, "result.still_air_range.value": still / NM, "result.wind_range.value": still * (tas - hw) / tas / NM,
           "result.glide_angle.value": math.degrees(math.atan(1 / ld)), "result.sink_rate.value": tas * KT / ld / FT * 60}
    tol = {k: {"rel": 1e-9, "abs": 1e-9} for k in exp if k != "ok"}
    for i in range(0, 72, 6):
        az = 5.0 * i
        rng = still * (tas - hw * math.cos(math.radians(az - (frm or 0)))) / tas
        p = Geodesic.WGS84.Direct(lat, lon, az, rng)
        exp[f"result.rings.{i}.lat.value"], exp[f"result.rings.{i}.lon.value"] = p["lat2"], (p["lon2"] + 540) % 360 - 180
        tol[f"result.rings.{i}.lat.value"] = tol[f"result.rings.{i}.lon.value"] = {"abs": 1e-9}
    rows.append({"id": f"v{len(kept) + len(rows) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
OUT.write_text("".join(line + "\n" for line in kept + [json.dumps(r, ensure_ascii=False, separators=(",", ":")) for r in rows]))
print(f"{OUT.name}: {len(kept) + len(rows)} vectors")
