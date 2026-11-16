#!/usr/bin/env python3
"""Golden vectors for geodesy.projection.lo-forward and lo-inverse
(add-international-grids, geodesy/national-grids: "Hartebeesthoek94 Lo zones").

Three sources, each catching what the others cannot:

  * IOGP Guidance Note 7-2 (September 2019), section 3.2.3.3: the Lo29
    worked example as printed. Its formula is the shorter USGS series, so its
    westing is 1 cm larger than the exact projection; the row says so and
    carries a 2 cm tolerance.
  * PROJ 9.9.0 through cs2cs applying each belt's own EPSG record (2046 to
    2055). EPSG declares the axes westing (Y) then southing (X), and cs2cs
    keeps that order. These rows carry the coordinates at a millimetre.
  * GeographicLib's TransverseMercatorProj on WGS 84, k = 1, negated for the
    south-orientated axes: the convergence, the point scale factor, and the
    coordinates to a micrometre.

    gen_lo.py FORWARD.jsonl INVERSE.jsonl
"""
import json
import subprocess
import sys

WGS84_A = "6378137"
WGS84_F = repr(1 / 298.257223563)


def epsg(cm):
    return 2046 + (cm - 15) // 2


def check_against_epsg():
    for cm in range(15, 34, 2):
        want = subprocess.run(["projinfo", "-o", "PROJ", f"EPSG:{epsg(cm)}"],
                              check=True, capture_output=True, text=True).stdout
        for p in ("+proj=tmerc", "+axis=wsu", "+lat_0=0", f"+lon_0={cm} ", "+k=1 ",
                  "+x_0=0", "+y_0=0", "+ellps=WGS84"):
            assert p in want, f"EPSG:{epsg(cm)} no longer has {p}: {want}"
        name = subprocess.run(["projinfo", "-q", "-o", "WKT2:2019", f"EPSG:{epsg(cm)}"],
                              check=True, capture_output=True, text=True).stdout
        assert f'Hartebeesthoek94 / Lo{cm}"' in name, name[:100]


def cs2cs_fwd(cm, lat, lon):
    out = subprocess.run(["cs2cs", "-d", "10", "EPSG:4148", "+to", f"EPSG:{epsg(cm)}"],
                         input=f"{lat} {lon}\n", check=True, capture_output=True,
                         text=True).stdout.split()
    return float(out[0]), float(out[1])  # westing Y, southing X


def cs2cs_inv(cm, y, x):
    out = subprocess.run(["cs2cs", "-d", "12", f"EPSG:{epsg(cm)}", "+to", "EPSG:4148"],
                         input=f"{y} {x}\n", check=True, capture_output=True,
                         text=True).stdout.split()
    return float(out[0]), float(out[1])


def tmproj(cm, lat, lon):
    out = subprocess.run(
        ["TransverseMercatorProj", "-l", str(cm), "-k", "1",
         "-e", WGS84_A, WGS84_F, "-p", "9"],
        input=f"{lat} {lon}\n", check=True, capture_output=True, text=True,
    ).stdout.split()
    x, y, gam, k = (float(v) for v in out[:4])
    return -x, -y, gam, k


def belt(lon):
    return 2 * int(lon // 2) + 1


PLACES = [
    ("Cape Town, the Castle of Good Hope", -33.925800, 18.427800),
    ("Saldanha", -33.010000, 17.940000),
    ("Springbok", -29.664300, 17.886500),
    ("Upington", -28.447800, 21.256100),
    ("Beaufort West", -32.356700, 22.583000),
    ("Port Elizabeth (Gqeberha)", -33.960800, 25.602200),
    ("Kimberley, the Big Hole", -28.738300, 24.758300),
    ("Johannesburg, Park Station", -26.195800, 28.040600),
    ("Pretoria, the Union Buildings", -25.740300, 28.212200),
    ("Bloemfontein", -29.118700, 26.214100),
    ("Maseru, Lesotho", -29.310000, 27.480000),
    ("Durban, the Moses Mabhida Stadium", -29.828600, 31.030000),
    ("Mbabane, Eswatini", -26.305400, 31.136700),
    ("Musina", -22.338100, 30.041700),
    ("Kosi Bay", -26.900000, 32.880000),
    ("Alexander Bay", -28.596000, 16.485000),
    ("The Atlantic off Namaqualand, in Lo15", -31.000000, 15.500000),
]


def rows():
    fwd, inv = [], []
    # IOGP's own example, as printed, from both ends.
    fwd.append({
        "input": {"lat": "25°43'55.302\"S", "lon": "28°16'57.479\"E"},
        "expect": {"ok": True, "result.zone": 29, "result.westing.value": 71984.49,
                   "result.southing.value": 2847342.74},
        "source": "IOGP Guidance Note 7-2 (September 2019), section 3.2.3.3, Transverse Mercator (South Orientated): the Hartebeesthoek94 / Lo29 example as printed. Its formula is the shorter USGS series, whose westing is 1.0 cm larger than the exact projection's 71984.480 m, hence the 2 cm tolerance",
        "sourceVersion": "IOGP 373-7-2, revised September 2019",
        "tolerance": {"result.zone": {"abs": 0}, "result.westing.value": {"abs": 0.02},
                      "result.southing.value": {"abs": 0.01}},
    })
    inv.append({
        "input": {"zone": 29, "westing": "71984.49 m", "southing": "2847342.74 m"},
        "expect": {"ok": True,
                   "result.lat.value": -(25 + 43 / 60 + 55.302 / 3600),
                   "result.lon.value": 28 + 16 / 60 + 57.479 / 3600},
        "source": "IOGP Guidance Note 7-2 (September 2019), section 3.2.3.3, reverse: Lo29, Y 71984.49 m, X 2847342.74 m back to 25°43'55.302\"S 28°16'57.479\"E, to the 1 cm the printed westing carries",
        "sourceVersion": "IOGP 373-7-2, revised September 2019",
        "tolerance": {"result.lat.value": {"abs": 2e-7}, "result.lon.value": {"abs": 2e-7}},
    })
    for name, lat, lon in PLACES:
        cm = belt(lon)
        assert 15 <= cm <= 33, (name, cm)
        y, x = cs2cs_fwd(cm, lat, lon)
        src = f"EPSG:{epsg(cm)} (Hartebeesthoek94 / Lo{cm})"
        fwd.append({
            "input": {"lat": lat, "lon": lon},
            "expect": {"ok": True, "result.zone": cm,
                       "result.westing.value": y, "result.southing.value": x},
            "source": f"PROJ 9.9.0 cs2cs applying {src}: {name}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
            "tolerance": {"result.zone": {"abs": 0}, "result.westing.value": {"abs": 1e-3},
                          "result.southing.value": {"abs": 1e-3}},
        })
        blat, blon = cs2cs_inv(cm, y, x)
        inv.append({
            "input": {"zone": cm, "westing": f"{y:.6f} m", "southing": f"{x:.6f} m"},
            "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
            "source": f"PROJ 9.9.0 cs2cs applying {src} in reverse: {name}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
            "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
        })

    ANGLES = [
        ("Pretoria, the Union Buildings", -25.740300, 28.212200, 29),
        ("Cape Town, the Castle of Good Hope", -33.925800, 18.427800, 19),
        ("Durban, the Moses Mabhida Stadium", -29.828600, 31.030000, 31),
        ("On Lo27's central meridian at Bloemfontein's latitude", -29.118700, 27.000000, 27),
    ]
    tol = {"result.convergence.value": {"abs": 1e-7}, "result.scale_factor": {"abs": 1e-11}}
    for name, lat, lon, cm in ANGLES:
        y, x, gam, k = tmproj(cm, lat, lon)
        fwd.append({
            "input": {"lat": lat, "lon": lon},
            "expect": {"ok": True, "result.zone": cm, "result.westing.value": y,
                       "result.southing.value": x, "result.convergence.value": gam,
                       "result.scale_factor": k},
            "source": f"GeographicLib TransverseMercatorProj on WGS 84 about {cm}°E with k = 1, negated for the south-orientated axes: Y, X, the convergence, and the point scale factor at {name}",
            "sourceVersion": "GeographicLib TransverseMercatorProj 2.7",
            "tolerance": {**tol, "result.zone": {"abs": 0}, "result.westing.value": {"abs": 1e-6},
                          "result.southing.value": {"abs": 1e-6}},
        })
        inv.append({
            "input": {"zone": cm, "westing": f"{y:.9f} m", "southing": f"{x:.9f} m"},
            "expect": {"ok": True, "result.lat.value": lat, "result.lon.value": lon,
                       "result.convergence.value": gam, "result.scale_factor": k},
            "source": f"GeographicLib TransverseMercatorProj on Lo{cm}'s parameters, read back: the place, the convergence, and the point scale factor at {name}",
            "sourceVersion": "GeographicLib TransverseMercatorProj 2.7",
            "tolerance": {**tol, "result.lat.value": {"abs": 1e-11},
                          "result.lon.value": {"abs": 1e-11}},
        })

    # A forced neighbouring belt: Pretoria is in Lo29, and a job west of it
    # may want Lo27. PROJ gives the answer; the tool must say it is forced.
    lat, lon = -25.740300, 28.212200
    y, x = cs2cs_fwd(27, lat, lon)
    fwd.append({
        "input": {"lat": lat, "lon": lon, "zone": 27},
        "expect": {"ok": True, "result.zone": 27, "result.westing.value": y,
                   "result.southing.value": x, "meta.warnings.*.code": "NONSTANDARD_ZONE"},
        "source": "PROJ 9.9.0 cs2cs applying EPSG:2052 (Hartebeesthoek94 / Lo27): the Union Buildings, which fall in Lo29, projected onto Lo27 as a job spanning the belt edge would",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.zone": {"abs": 0}, "result.westing.value": {"abs": 1e-3},
                      "result.southing.value": {"abs": 1e-3}},
    })
    return fwd, inv


def dump(rs):
    return "\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                separators=(",", ":"))
                     for i, r in enumerate(rs, 1)) + "\n"


def main():
    check_against_epsg()
    fwd, inv = rows()
    if len(sys.argv) != 3:
        sys.stdout.write(dump(fwd) + "--- inverse ---\n" + dump(inv))
        return
    with open(sys.argv[1], "w") as f:
        f.write(dump(fwd))
    with open(sys.argv[2], "w") as f:
        f.write(dump(inv))


if __name__ == "__main__":
    main()
