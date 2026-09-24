#!/usr/bin/env python3
"""Grid references in the coordinate reader (geodesy.parse.coordinates 1.3.0;
coordinate-parsing "Supported input notations" and the multi-grid case of
"Ambiguity is reported, never silently resolved").

Cell centers come from independent implementations: Google's
openlocationcode, pygeohash, and the maidenhead package. GARS (NGA) and
GEOREF (DMA) centers are computed here from their published letter schemes,
and UPS coordinates are inverted by PROJ (pyproj, EPSG:32661 and 32761).
Appends vectors to core/vectors/geodesy.parse.coordinates.jsonl; existing
lines are left byte for byte. Requires openlocationcode, pygeohash, and
maidenhead, and pyproj."""
import json
from pathlib import Path

import maidenhead
import pygeohash
from openlocationcode import openlocationcode as olc
from pyproj import Transformer, proj_version_str

PATH = Path("core/vectors/geodesy.parse.coordinates.jsonl")
TAG = "gen_parse_grids.py"
GARS_LAT = "ABCDEFGHJKLMNPQRSTUVWXYZ"  # I and O are skipped
GEOREF_15 = "ABCDEFGHJKLMNPQRSTUVWXYZ"
GEOREF_1 = "ABCDEFGHJKLMNPQ"


def gars(code):
    lon0 = -180 + (int(code[:3]) - 1) * 0.5
    lat0 = -90 + (GARS_LAT.index(code[3]) * 24 + GARS_LAT.index(code[4])) * 0.5
    q = int(code[5]) - 1  # 1 NW, 2 NE, 3 SW, 4 SE
    lon0 += (q % 2) * 0.25
    lat0 += (1 - q // 2) * 0.25
    k = int(code[6]) - 1  # keypad: 1 at the northwest, 9 at the southeast
    lon0 += (k % 3) * 5 / 60
    lat0 += (2 - k // 3) * 5 / 60
    return lat0 + 2.5 / 60, lon0 + 2.5 / 60


def georef(code):
    lon = -180 + 15 * GEOREF_15.index(code[0]) + GEOREF_1.index(code[2]) + int(code[4:6]) / 60
    lat = -90 + 15 * GEOREF_15.index(code[1]) + GEOREF_1.index(code[3]) + int(code[6:8]) / 60
    return lat + 1 / 120, lon + 1 / 120


def olc_center(code):
    a = olc.decode(code)
    return a.latitudeCenter, a.longitudeCenter


def gh(code):
    d = pygeohash.decode_exactly(code)
    return d[0], d[1]


CASES = [
    ("8FVC9G8F+6W", "Plus Code", olc_center("8FVC9G8F+6W"), "openlocationcode"),
    ("849VCWC8+R9", "Plus Code", olc_center("849VCWC8+R9"), "openlocationcode"),
    ("FN20xr", "Maidenhead", maidenhead.to_location("FN20xr", center=True), "maidenhead"),
    ("JO65ha", "Maidenhead", maidenhead.to_location("JO65ha", center=True), "maidenhead"),
    ("006AG39", "GARS", gars("006AG39"), "NGA GARS letter scheme"),
    ("361HN37", "GARS", gars("361HN37"), "NGA GARS letter scheme"),
    ("MKPG1204", "GEOREF", georef("MKPG1204"), "DMA GEOREF letter scheme"),
    ("dr5ru7", "geohash", gh("dr5ru7"), "pygeohash"),
    ("9q8yyk", "geohash", gh("9q8yyk"), "pygeohash"),
]


def main():
    lines = PATH.read_text().splitlines()
    kept = [l for l in lines if TAG not in l]
    n, new = len(kept), []

    def add(text, expect, src, tol):
        new.append({"id": f"v{n + len(new) + 1:03d}", "input": {"text": text}, "expect": {**expect, "ok": expect.get("ok", True)},
                    "source": f"{src} ({'tools/vectors/' + TAG})", "sourceVersion": "2026", "tolerance": tol})

    tol = {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}}
    for text, notation, (lat, lon), src in CASES:
        add(text, {"result.lat.value": lat, "result.lon.value": lon, "result.notation": notation,
                   "meta.warnings.*.code": "INPUT_NORMALIZED"}, src, tol)
    # A code valid in two grids: lower case reads as a geohash, upper case as
    # Maidenhead, and the other reading is offered.
    lat, lon = gh("fn20")
    mlat, mlon = maidenhead.to_location("FN20", center=True)
    alt = {"result.alternatives.0.lat.value": {"abs": 1e-9}, "result.alternatives.0.lon.value": {"abs": 1e-9}}
    add("fn20", {"result.lat.value": lat, "result.lon.value": lon, "result.notation": "geohash", "meta.warnings.*.code": "AMBIGUOUS_INPUT",
                 "result.alternatives.0.reading": "as Maidenhead", "result.alternatives.0.lat.value": mlat, "result.alternatives.0.lon.value": mlon},
        "pygeohash and maidenhead", {**tol, **alt})
    add("FN20", {"result.lat.value": mlat, "result.lon.value": mlon, "result.notation": "Maidenhead", "meta.warnings.*.code": "AMBIGUOUS_INPUT",
                 "result.alternatives.0.reading": "as geohash", "result.alternatives.0.lat.value": lat, "result.alternatives.0.lon.value": lon},
        "pygeohash and maidenhead", {**tol, **alt})
    # A short Plus Code needs a reference place to complete it.
    add("9G8F+6W", {"ok": False, "error.code": "INVALID_INPUT"}, "openlocationcode (a short code is not a full code)", {})
    # UPS, written as zone letter, easting, northing (A and B south, Y and Z
    # north as in MGRS, or N and S).
    for text, epsg, east, north in [("Z 2426773 1530125", 32661, 2426773, 1530125), ("B 1500000E 2500000N", 32761, 1500000, 2500000),
                             ("Y 1800000 2100000", 32661, 1800000, 2100000)]:
        lon, lat = Transformer.from_crs(epsg, 4326, always_xy=True).transform(east, north)
        add(text, {"result.lat.value": lat, "result.lon.value": lon, "result.notation": "UPS"}, f"PROJ {proj_version_str} EPSG:{epsg} inverse", tol)
    lon, lat = Transformer.from_crs(32661, 4326, always_xy=True).transform(2000000, 2000000)
    add("N 2000000 2000000", {"result.lat.value": lat, "result.notation": "UPS"}, f"PROJ {proj_version_str} EPSG:32661 inverse (the pole)", {"result.lat.value": {"abs": 1e-9}})
    # A million meters from the pole is south of 84 degrees: not a UPS point.
    add("Z 2000000 3000000", {"ok": False, "error.code": "INVALID_INPUT"}, "UPS covers north of 83.5 degrees (NGA.SIG.0012)", {})
    PATH.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
    print(n, "->", n + len(new))


if __name__ == "__main__":
    main()
