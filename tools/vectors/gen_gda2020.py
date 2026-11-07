#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.gda2020 (add-international-grids,
geodesy/national-grids).

The oracle is PROJ 9.9.0 through cs2cs applying EPSG operation 8048 itself,
ICSM's conformal transformation, so the parameters come from the EPSG dataset
rather than from this repository.

    gen_gda2020.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import math
import subprocess

GDA94 = "EPSG:4283"
GDA2020 = "EPSG:7844"


def cs2cs(src, dst, lat, lon, h):
    """PROJ, holding it to the conformal operation rather than letting it pick
    the grid, so both sides apply the same published parameters."""
    helmert = ["+step"] + ([] if src == GDA94 else ["+inv"]) + [
        "+proj=helmert", "+x=0.06155", "+y=-0.01087", "+z=-0.04019",
        "+rx=-0.0394924", "+ry=-0.0327221", "+rz=-0.0328979", "+s=-0.009994",
        "+convention=coordinate_frame",
    ]
    pipeline = (["+proj=pipeline",
                 "+step", "+proj=unitconvert", "+xy_in=deg", "+xy_out=rad",
                 "+step", "+proj=cart", "+ellps=GRS80"]
                + helmert
                + ["+step", "+inv", "+proj=cart", "+ellps=GRS80",
                   "+step", "+proj=unitconvert", "+xy_in=rad", "+xy_out=deg"])
    out = subprocess.run(["cct", "-d", "12", *pipeline],
                         input=f"{lon} {lat} {h}\n", check=True, capture_output=True, text=True).stdout.split()
    return float(out[1]), float(out[0]), float(out[2])


PLACES = [
    ("Alice Springs, ALIC", -23.670124, 133.885513, 603.3466),
    ("Sydney Opera House", -33.856800, 151.215300, 20.0),
    ("Perth, Kings Park", -31.959600, 115.831300, 45.2),
    ("Darwin", -12.462700, 130.841800, 30.0),
    ("Hobart", -42.882200, 147.327200, 15.5),
    ("Brisbane", -27.470200, 153.025100, 27.0),
    ("Adelaide", -34.928500, 138.600700, 50.0),
    ("Cape York", -10.689600, 142.531700, 10.0),
    ("Norfolk Island", -29.040000, 167.954700, 110.0),
    ("Macquarie Island", -54.500000, 158.940000, 6.0),
]

rows = []
for name, lat, lon, h in PLACES:
    for direction, src, dst in (("to-gda2020", GDA94, GDA2020), ("to-gda94", GDA2020, GDA94)):
        la, lo, hh = cs2cs(src, dst, lat, lon, h)
        rows.append({
            "input": {"direction": direction, "lat": lat, "lon": lon, "height": f"{h} m"},
            "expect": {"ok": True, "result.lat.value": la, "result.lon.value": lo,
                       "result.height.value": hh,
                       "meta.warnings.*.code": "DISTORTION_NOT_MODELLED"},
            "source": f"PROJ 9.9.0 applying EPSG operation 8048, ICSM's conformal transformation: {name}, {direction}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
            "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9},
                          "result.height.value": {"abs": 1e-4}},
        })

# Out of the published area of use: the answer still comes, flagged.
la, lo, hh = cs2cs(GDA94, GDA2020, -41.3, 174.8, 10.0)
rows.append({
    "input": {"direction": "to-gda2020", "lat": -41.3, "lon": 174.8, "height": "10 m"},
    "expect": {"ok": True, "result.lat.value": la, "result.lon.value": lo,
               "meta.warnings.*.code": "OUTSIDE_AREA_OF_USE"},
    "source": "EPSG's area of use for operation 8048 is Australia and its offshore territories; Wellington is outside it, so the answer is extrapolation and says so",
    "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
    "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
})

for i, r in enumerate(rows, 1):
    print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))
