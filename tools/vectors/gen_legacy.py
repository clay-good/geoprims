#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.legacy from PROJ's implementation of the
same EPSG operations (EPSG dataset v10.094 in pyproj's proj.db).

Writes core/vectors/geodesy.datum.legacy.jsonl.
"""
import json
from pathlib import Path

import pyproj

OUT = Path(__file__).resolve().parents[2] / "core/vectors/geodesy.datum.legacy.jsonl"
SRC = ("PROJ applying the EPSG operation through pyproj", "PROJ 9.3, EPSG v10.094")
OPS = {"ED50": (1133, [(48.8566, 2.3522), (40.4168, -3.7038), (59.91, 10.75)]), "NAD27": (1173, [(38.5, -98.0), (40.446111, -79.982222), (34.05, -118.25)]),
       "OSGB36": (1314, [(51.5007, -0.1246), (55.95, -3.19)]), "Tokyo": (1305, [(37.5665, 126.978), (35.18, 129.08)]),
       "AGD66": (1108, [(-35.3, 149.1), (-31.95, 115.86)]), "Pulkovo1942": (1267, [(55.7558, 37.6173), (59.94, 30.31)]),
       "SAD69": (1864, [(-15.79, -47.88), (-23.55, -46.63)]), "Arc1960": (1122, [(-1.2921, 36.8219), (-6.79, 39.21)])}


def main():
    vs = []

    def vec(inp, exp, src, tol=None, ok=True):
        e = dict(exp)
        e["ok"] = ok
        v = {"id": f"v{len(vs) + 1:03d}", "input": inp, "expect": e, "source": src[0], "sourceVersion": src[1]}
        if tol:
            v["tolerance"] = tol
        vs.append(v)

    tol = {"result.lat.value": {"abs": 1e-8}, "result.lon.value": {"abs": 1e-8}}
    for datum, (code, pts) in OPS.items():
        t = pyproj.Transformer.from_pipeline(pyproj.crs.CoordinateOperation.from_epsg(code).to_proj4())
        for k, (lat, lon) in enumerate(pts):
            fwd = k % 2 == 0
            la, lo = t.transform(lat, lon) if fwd else t.transform(lat, lon, direction="INVERSE")
            inp = {"datum": datum, "lat": lat, "lon": lon}
            if not fwd:
                inp["direction"] = "from-wgs84"
            vec(inp, {"result.lat.value": la, "result.lon.value": lo, "meta.warnings.*.code": "LOW_ACCURACY_TRANSFORM"}, SRC, tol)
    # A NAD27 shift applied in Europe is outside its area of use.
    vec({"datum": "NAD27", "lat": 48.85, "lon": 2.35}, {"meta.warnings.*.code": "OUTSIDE_AREA_OF_USE"}, ("EPSG 1173 area of use", "EPSG v10.094"))
    vec({"datum": "ED50", "lat": 95, "lon": 0}, {"error.code": "INVALID_INPUT"}, ("add-geodesy-suite input rules", "2026-09"), ok=False)
    OUT.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
    print(OUT.name, len(vs))


if __name__ == "__main__":
    main()
