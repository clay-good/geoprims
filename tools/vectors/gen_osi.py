#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.osi-polynomial and osi-polynomial-inverse
(add-international-grids, geodesy/national-grids: "Irish Grid and ITM").

The oracle is Ordnance Survey's own OSGM15 Northern Ireland test files, in the
OSTN15-OSGM15 Developers Pack (pinned in tools/data/ostn15-assets.mjs): 100
points from Irish Grid to ETRS89 and 100 the other way, on the Belfast datum.
Positions are printed to a millionth of a second and a millimetre, heights to
0.1 mm. The first vector of the forward file is NI_IG_1, the specification's
own scenario.

    gen_osi.py DEVPACK_DIR FORWARD.jsonl INVERSE.jsonl
"""
import json
import re
import sys

SRC = "Ordnance Survey OSTN15-OSGM15 Developers Pack"
VER = "OSGM15 Northern Ireland test files of 2016-05-09"


def rows(path):
    text = open(path, encoding="utf-8-sig").read()
    return [l.split("\t") for l in text.split("\n")[1:] if l.strip()]


def dms(s):
    m = re.match(r"(\d+)° (\d+)' ([\d.]+)\" ([NSEW])", s.strip())
    v = int(m[1]) + int(m[2]) / 60 + float(m[3]) / 3600
    return -v if m[4] in "SW" else v


def main(d, fwd_path, inv_path):
    fwd = []
    for (pid, e, n, h), (_, la, lo, eh) in zip(rows(f"{d}/OSGM15_NI_TestInput_IGtoETRS89.txt"),
                                                 rows(f"{d}/OSGM15_NI_TestOutput_IGtoETRS89.txt")):
        fwd.append({
            "input": {"easting": f"{e} m", "northing": f"{n} m", "height": f"{h.strip()} m",
                      "datum": "Belfast"},
            "expect": {"ok": True, "result.lat.value": dms(la), "result.lon.value": dms(lo),
                       "result.ellipsoidal_height.value": float(eh)},
            "source": f"{SRC}, OSGM15_NI_TestOutput_IGtoETRS89.txt, {pid}",
            "sourceVersion": VER,
            # Half the printed millionth of a second, and the specification's
            # millimetre for the height: OSGM15's resampling to a 1 km ITM grid
            # is not specified exactly, and this resamples bilinearly.
            "tolerance": {"result.lat.value": {"abs": 1.4e-10}, "result.lon.value": {"abs": 1.4e-10},
                          "result.ellipsoidal_height.value": {"abs": 1e-3}},
        })
    inv = []
    for (pid, la, lo, eh), (_, e, n, h) in zip(rows(f"{d}/OSGM15_NI_TestInput_ETRS89toIG.txt"),
                                                 rows(f"{d}/OSGM15_NI_TestOutput_ETRS89toIG.txt")):
        inv.append({
            "input": {"lat": dms(la), "lon": dms(lo), "height": f"{eh.strip()} m", "datum": "Belfast"},
            "expect": {"ok": True, "result.easting.value": float(e), "result.northing.value": float(n),
                       "result.orthometric_height.value": float(h)},
            "source": f"{SRC}, OSGM15_NI_TestOutput_ETRS89toIG.txt, {pid}",
            "sourceVersion": VER,
            "tolerance": {"result.easting.value": {"abs": 5e-4}, "result.northing.value": {"abs": 5e-4},
                          "result.orthometric_height.value": {"abs": 1e-3}},
        })
    dump = lambda rs: "\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                           separators=(",", ":"))
                                for i, r in enumerate(rs, 1)) + "\n"
    open(fwd_path, "w").write(dump(fwd))
    open(inv_path, "w").write(dump(inv))


if __name__ == "__main__":
    main(*sys.argv[1:4])
