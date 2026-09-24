#!/usr/bin/env python3
"""MGRS "AL" lettering on legacy ellipsoids, from NGA GEOTRANS (C) through
the mgrs package's library (geodesy/grid-references, "MGRS polar,
exception, and lettering rules"; add-geodesy-suite task 5.1):

  pip install mgrs==1.5.4
  python tools/vectors/gen_mgrs_al.py

GEOTRANS picks the lettering from its ellipsoid code: Clarke 1866 (CC) and
Bessel 1841 (BR) take AL. Random points between 80° S and 84° N; each row
holds the point, the precision, the ellipsoid, GEOTRANS's reference, and the
south-west corner GEOTRANS decodes it to.

Writes core/crates/gp-geodesy/tests/data/mgrs_al.csv and appends vectors to
both MGRS tools' files (existing lines left byte for byte)."""
import ctypes
import json
import math
import random
from pathlib import Path

import mgrs.core as core

LIB = core.rt
LIB.Set_MGRS_Parameters.argtypes = [ctypes.c_double, ctypes.c_double, ctypes.c_char_p]
LIB.Convert_Geodetic_To_MGRS.argtypes = [ctypes.c_double, ctypes.c_double, ctypes.c_long, ctypes.c_char_p]
LIB.Convert_MGRS_To_Geodetic.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_double), ctypes.POINTER(ctypes.c_double)]
ELL = {"clarke1866": ("CC", 6378206.4, (6378206.4 - 6356583.8) / 6378206.4), "bessel1841": ("BR", 6377397.155, 1 / 299.1528128)}
FIX = Path("core/crates/gp-geodesy/tests/data/mgrs_al.csv")
TAG = "gen_mgrs_al.py"


def geotrans(lat, lon, ell, prec):
    code, a, f = ELL[ell]
    if LIB.Set_MGRS_Parameters(a, f, code.encode()):
        raise RuntimeError("parameters")
    buf = ctypes.create_string_buffer(80)
    if LIB.Convert_Geodetic_To_MGRS(math.radians(lat), math.radians(lon), prec, buf):
        raise RuntimeError("encode")
    ref = buf.value.decode()
    la, lo = ctypes.c_double(), ctypes.c_double()
    if LIB.Convert_MGRS_To_Geodetic(ref.encode(), ctypes.byref(la), ctypes.byref(lo)):
        raise RuntimeError("decode")
    return ref, math.degrees(la.value), math.degrees(lo.value)


def main():
    rng = random.Random(1866)
    rows = []
    while len(rows) < 600:
        lat = round(math.degrees(math.asin(rng.uniform(math.sin(math.radians(-79.5)), math.sin(math.radians(83.5))))), 7)
        lon = round(rng.uniform(-180, 180), 7)
        ell = rng.choice(sorted(ELL))
        prec = rng.randint(1, 5)
        try:
            ref, clat, clon = geotrans(lat, lon, ell, prec)
        except RuntimeError:
            continue
        rows.append((lat, lon, prec, ell, ref, clat, clon))
    FIX.write_text("# lat,lon,digits,ellipsoid,geotrans_reference,corner_lat,corner_lon (NGA GEOTRANS via mgrs 1.5.4)\n"
                   + "".join(f"{r[0]},{r[1]},{r[2]},{r[3]},{r[4]},{r[5]!r},{r[6]!r}\n" for r in rows))
    precision = {1: "10km", 2: "1km", 3: "100m", 4: "10m", 5: "1m"}
    for tool in ("forward", "inverse"):
        path = Path(f"core/vectors/geodesy.grid-ref.mgrs-{tool}.jsonl")
        lines = path.read_text().splitlines()
        kept = [l for l in lines if TAG not in l]
        n0, new = len(kept), []
        for lat, lon, prec, ell, ref, clat, clon in rows[:6]:
            if tool == "forward":
                inp = {"lat": lat, "lon": lon, "precision": precision[prec], "ellipsoid": ell}
                exp, tol = {"result.mgrs": ref, "ok": True}, {}
            else:
                inp = {"mgrs": ref, "ellipsoid": ell}
                exp = {"result.corner_lat.value": clat, "result.corner_lon.value": clon, "ok": True}
                tol = {"result.corner_lat.value": {"abs": 2e-7}, "result.corner_lon.value": {"abs": 2e-7}}
            new.append({"id": f"v{n0 + len(new) + 1:03d}", "input": inp, "expect": exp,
                        "source": f"NGA GEOTRANS (mgrs 1.5.4) with the {ELL[ell][0]} ellipsoid code, AL lettering ({TAG})",
                        "sourceVersion": "GEOTRANS via mgrs 1.5.4", "tolerance": tol})
        path.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
        print(path.name, n0, "->", n0 + len(new))


if __name__ == "__main__":
    main()
