#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.etrf (add-international-grids,
geodesy/national-grids).

The oracle is PROJ 9.9.0 through cct, applying the same two steps the tool
does: the IERS tie onto ITRF2014, then EPSG's ITRF2014 to ETRF2000 (1), a
time-dependent position-vector transformation with a reference epoch of
2010.0. The parameters therefore come from the EPSG dataset and from IERS,
not from this repository.

    gen_etrf.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import subprocess

# EPSG's ITRF2014 to ETRF2000 (1), as PROJ writes it.
TIE = ["+proj=helmert", "+x=0.0547", "+y=0.0522", "+z=-0.0741",
       "+rx=0.001701", "+ry=0.01029", "+rz=-0.016632", "+s=0.00212",
       "+dx=0.0001", "+dy=0.0001", "+dz=-0.0019",
       "+drx=0.000081", "+dry=0.00049", "+drz=-0.000792", "+ds=0.00011",
       "+t_epoch=2010", "+convention=position_vector"]
# IERS ITRF2020 to ITRF2014, the leg the tool chains through.
ITRF2020_TO_2014 = ["+proj=helmert", "+x=-0.0014", "+y=-0.0009", "+z=0.0014", "+s=-0.00042",
                    "+dx=0.0", "+dy=-0.0001", "+dz=0.0002", "+ds=0.0",
                    "+t_epoch=2015", "+convention=position_vector"]


def run(steps, lat, lon, h, t):
    pipeline = (["+proj=pipeline",
                 "+step", "+proj=unitconvert", "+xy_in=deg", "+xy_out=rad",
                 "+step", "+proj=cart", "+ellps=GRS80"]
                + steps
                + ["+step", "+inv", "+proj=cart", "+ellps=GRS80",
                   "+step", "+proj=unitconvert", "+xy_in=rad", "+xy_out=deg"])
    out = subprocess.run(["cct", "-d", "12", *pipeline],
                         input=f"{lon} {lat} {h} {t}\n",
                         check=True, capture_output=True, text=True).stdout.split()
    return float(out[1]), float(out[0]), float(out[2])


PLACES = [
    ("Dublin", 53.349806, -6.260247, 85.0),
    ("Zurich", 47.378100, 8.540200, 450.0),
    ("Madrid", 40.416800, -3.703800, 700.0),
    ("Helsinki", 60.169900, 24.938400, 30.0),
    ("Lisbon", 38.722300, -9.139300, 50.0),
    ("Warsaw", 52.229700, 21.012200, 120.0),
    ("Edinburgh", 55.953300, -3.188300, 70.0),
    ("Athens", 37.983800, 23.727500, 90.0),
]
EPOCHS = [2026.0, 2015.0, 2000.0]

rows = []
for name, lat, lon, h in PLACES:
    for t in EPOCHS[:1] if name not in ("Dublin", "Zurich") else EPOCHS:
        la, lo, hh = run(["+step"] + ITRF2020_TO_2014 + ["+step"] + TIE, lat, lon, h, t)
        rows.append({
            "input": {"direction": "to-etrs89", "frame": "ITRF2020", "epoch": f"{t}",
                      "lat": lat, "lon": lon, "height": f"{h} m"},
            "expect": {"ok": True, "result.lat.value": la, "result.lon.value": lo,
                       "result.height.value": hh},
            "source": f"PROJ 9.9.0: the IERS tie from ITRF2020 onto ITRF2014, then EPSG's ITRF2014 to ETRF2000 (1): {name} at epoch {t}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102; IERS ITRF2020",
            "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9},
                          "result.height.value": {"abs": 1e-4}},
        })
        # And back again, which must return the point it started from.
        rows.append({
            "input": {"direction": "to-itrf", "frame": "ITRF2020", "epoch": f"{t}",
                      "lat": la, "lon": lo, "height": f"{hh} m"},
            "expect": {"ok": True, "result.lat.value": lat, "result.lon.value": lon},
            "source": f"The same chain run backwards returns {name} to where it started",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102; IERS ITRF2020",
            "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
        })

# Outside the area EPSG publishes the tie for: answered, and flagged.
la, lo, hh = run(["+step"] + ITRF2020_TO_2014 + ["+step"] + TIE, 40.7128, -74.006, 10.0, 2026.0)
rows.append({
    "input": {"direction": "to-etrs89", "frame": "ITRF2020", "epoch": "2026.0",
              "lat": 40.7128, "lon": -74.006, "height": "10 m"},
    "expect": {"ok": True, "result.lat.value": la, "result.lon.value": lo,
               "meta.warnings.*.code": "OUTSIDE_AREA_OF_USE"},
    "source": "New York is outside the ETRS89 tie's published area, so the answer is extrapolation and says so",
    "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
    "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
})
rows.append({
    "input": {"direction": "to-etrs89", "frame": "ITRF2020", "epoch": "1800",
              "lat": 53.349806, "lon": -6.260247},
    "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
    "source": "add-international-grids geodesy/national-grids: the epoch is held to 1980 through 2100",
    "sourceVersion": "2026", "tolerance": {},
})

for i, r in enumerate(rows, 1):
    print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))
