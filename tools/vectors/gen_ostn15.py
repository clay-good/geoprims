#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.ostn15 and geodesy.datum.ostn15-inverse
(add-international-grids, geodesy/national-grids: "OSTN15 and OSGM15").

The oracle is Ordnance Survey itself: the test files in OS's
OSTN15-OSGM15-DevelopersPack.zip (pinned in tools/data/ostn15-assets.mjs),
which give for 40 points the result and the interpolation intermediates both
ways, and for the inverse every iteration. The first vector of each file is
the user guide's Caister Water Tower worked example.

    gen_ostn15.py DEVPACK_DIR FORWARD.jsonl INVERSE.jsonl
"""
import json
import sys

SRC = "Ordnance Survey OSTN15-OSGM15 Developers Pack"
VER = "OSTN15/OSGM15, test files of 2016-05-09"


def rows(path):
    lines = [l for l in open(path, encoding="latin-1").read().split("\n")]
    head = lines[0].split(",")
    return head, [l.split(",") for l in lines[1:] if l.strip()]


def main(d, fwd_path, inv_path):
    _, tin = rows(f"{d}/OSTN15_OSGM15_TestInput_ETRStoOSGB.txt")
    head, tout = rows(f"{d}/OSTN15_OSGM15_TestOutput_ETRStoOSGB.txt")
    col = {k.strip(): i for i, k in enumerate(head)}
    fwd = [{
        "input": {"lat": "52°39'28.8282\"N", "lon": "1°42'57.8663\"E", "height": "108.05 m"},
        "expect": {"ok": True, "result.easting.value": 651409.804,
                   "result.northing.value": 313177.450,
                   "result.orthometric_height.value": 63.822, "result.datum_flag": 1,
                   "result.east_shift.value": 102.801, "result.north_shift.value": -78.236,
                   "result.geoid_height.value": 44.228},
        "source": "Ordnance Survey, Transformations and OSGM15 User Guide v1.3 (2020), annexe A: Caister Water Tower, with the interpolated shifts it prints",
        "sourceVersion": "User Guide v1.3, November 2020",
        "tolerance": {k: {"abs": 5e-4} for k in ("result.easting.value", "result.northing.value",
                                                  "result.orthometric_height.value",
                                                  "result.east_shift.value",
                                                  "result.north_shift.value",
                                                  "result.geoid_height.value")},
    }]
    inputs = {r[0]: r for r in tin}
    for r in tout:
        pid = r[0]
        _, lat, lon, h = inputs[pid]
        g = lambda k: float(r[col[k]])
        exp = {"ok": True, "result.easting.value": g("OSGBEast"),
               "result.northing.value": g("OSGBNorth"),
               "result.orthometric_height.value": g("ODNHeight"),
               "result.datum_flag": int(r[col["OSGBDatumFlag"]]),
               "result.east_shift.value": g("Se"), "result.north_shift.value": g("Sn"),
               "result.geoid_height.value": g("Sg")}
        tol = {"result.easting.value": {"abs": 5e-4}, "result.northing.value": {"abs": 5e-4},
               "result.orthometric_height.value": {"abs": 5e-4},
               "result.east_shift.value": {"abs": 5e-6}, "result.north_shift.value": {"abs": 5e-6},
               "result.geoid_height.value": {"abs": 5e-6}}
        fwd.append({"input": {"lat": float(lat), "lon": float(lon), "height": f"{h} m"},
                    "expect": exp,
                    "source": f"{SRC}, OSTN15_OSGM15_TestOutput_ETRStoOSGB.txt, {pid}: the result, its datum flag, and the interpolated shifts",
                    "sourceVersion": VER, "tolerance": tol})

    head, tout = rows(f"{d}/OSTN15_OSGM15_TestOutput_OSGBtoETRS.txt")
    col = {k.strip(): i for i, k in enumerate(head)}
    _, tin = rows(f"{d}/OSTN15_OSGM15_TestInput_OSGBtoETRS.txt")
    inputs = {r[0]: r for r in tin}
    inv = [{
        "input": {"easting": "651409.804 m", "northing": "313177.450 m", "height": "63.822 m"},
        "expect": {"ok": True, "result.etrs89_easting.value": 651307.0032,
                   "result.etrs89_northing.value": 313255.6860,
                   "result.ellipsoidal_height.value": 108.0504, "result.iterations": 3,
                   "result.east_shift.value": 102.8008, "result.north_shift.value": -78.2360,
                   "result.geoid_height.value": 44.2284,
                   "result.lat.value": 52.658007833, "result.lon.value": 1.716073972},
        "source": "Ordnance Survey, Transformations and OSGM15 User Guide v1.3 (2020), annexe A: Caister Water Tower inverted, converging in three iterations; the latitude and longitude are the example's own, which the 0.1 mm convergence test leaves 0.3 mm from",
        "sourceVersion": "User Guide v1.3, November 2020",
        "tolerance": {**{k: {"abs": 5e-5} for k in ("result.etrs89_easting.value",
                                                    "result.etrs89_northing.value",
                                                    "result.ellipsoidal_height.value",
                                                    "result.east_shift.value",
                                                    "result.north_shift.value",
                                                    "result.geoid_height.value")},
                      "result.iterations": {"abs": 0},
                      "result.lat.value": {"abs": 1e-8}, "result.lon.value": {"abs": 1e-8}},
    }]
    by = {}
    for r in tout:
        by.setdefault(r[0], []).append(r)
    for pid, its in by.items():
        res = [r for r in its if r[1] == "RESULT"][0]
        last = [r for r in its if r[1] != "RESULT"][-1]
        n_it = len([r for r in its if r[1] != "RESULT"])
        _, e, n, h = inputs[pid]
        g = lambda r, k: float(r[col[k]])
        inv.append({
            "input": {"easting": f"{e} m", "northing": f"{n} m", "height": f"{h.strip()} m"},
            "expect": {"ok": True, "result.etrs89_easting.value": g(last, "ETRSEast/Lat"),
                       "result.etrs89_northing.value": g(last, "ETRSNorth/Long"),
                       "result.ellipsoidal_height.value": g(last, "ETRSHeight"),
                       "result.east_shift.value": g(res, "Se"),
                       "result.north_shift.value": g(res, "Sn"),
                       "result.geoid_height.value": g(res, "Sg"),
                       "result.datum_flag": int(res[col["OSGBDatumFlag"]]),
                       "result.lat.value": g(res, "ETRSEast/Lat"),
                       "result.lon.value": g(res, "ETRSNorth/Long")},
            "source": f"{SRC}, OSTN15_OSGM15_TestOutput_OSGBtoETRS.txt, {pid}: the converged estimate and the final shifts, to the 0.1 mm OS's convergence test allows; the latitude and longitude OS prints are the test point's own, which that test leaves within a millimetre of. The number of iterations is not pinned: OS's printed count does not follow its stated stopping rule (TP02 stops after two with the shift still moving 0.9 mm)",
            "sourceVersion": VER,
            "tolerance": {**{k: {"abs": 1e-4} for k in ("result.etrs89_easting.value",
                                                        "result.etrs89_northing.value",
                                                        "result.ellipsoidal_height.value",
                                                        "result.east_shift.value",
                                                        "result.north_shift.value",
                                                        "result.geoid_height.value")},
                          "result.lat.value": {"abs": 1e-8}, "result.lon.value": {"abs": 1e-8}},
        })
    for r in fwd + inv:
        for k, v in r["expect"].items():
            if isinstance(v, (int, float)) and not isinstance(v, bool) and k not in r["tolerance"]:
                r["tolerance"][k] = {"abs": 0}
    dump = lambda rs: "\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                           separators=(",", ":"))
                                for i, r in enumerate(rs, 1)) + "\n"
    open(fwd_path, "w").write(dump(fwd))
    open(inv_path, "w").write(dump(inv))


if __name__ == "__main__":
    main(*sys.argv[1:4])
