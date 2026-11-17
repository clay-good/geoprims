#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.nzgd2000 (add-international-grids,
geodesy/national-grids: "NZGD2000 and its deformation model", the distortion
grid half).

The oracle is PROJ 9.9.0's hgridshift applying nz_linz_nzgd2kgrid0005.tif,
the same LINZ grid the asset is packed from (pinned in
tools/data/nzgd2000-grid-assets.mjs), forward and in reverse. The first
vector is the specification's LINZ example.

    gen_nzgd2000.py GRID_TIF OUT.jsonl
"""
import json
import subprocess
import sys

TIF = sys.argv[1]


def cct(lat, lon, inverse=False):
    args = ["cct", "-d", "12"] + (["-I"] if inverse else []) + ["+proj=hgridshift", f"+grids={TIF}"]
    out = subprocess.run(args, input=f"{lon} {lat} 0 0\n", check=True, capture_output=True,
                         text=True).stdout.split()
    return float(out[1]), float(out[0])


PLACES = [
    ("Auckland, the Sky Tower", -36.848460, 174.762189),
    ("Wellington, the Beehive", -41.278400, 174.776700),
    ("Christchurch Cathedral", -43.531000, 172.637000),
    ("Dunedin, the Octagon", -45.874100, 170.503600),
    ("Invercargill", -46.413200, 168.353800),
    ("Queenstown", -45.031200, 168.662600),
    ("Nelson", -41.270600, 173.284000),
    ("Napier", -39.492800, 176.912000),
    ("Gisborne", -38.662300, 178.017600),
    ("Tauranga", -37.687800, 176.165100),
    ("Hamilton", -37.787000, 175.279300),
    ("New Plymouth", -39.057200, 174.075200),
    ("Whangarei", -35.725200, 174.323700),
    ("Cape Reinga", -34.428800, 172.677800),
    ("Greymouth", -42.450400, 171.210800),
    ("Milford Sound", -44.671600, 167.925100),
    ("Stewart Island, Oban", -46.898900, 168.128200),
    ("East Cape", -37.689300, 178.548000),
    ("Mount Cook", -43.595000, 170.141800),
    ("Kaikoura", -42.400800, 173.681400),
]


def main(out):
    rows = []
    lat, lon = -36.5, 175.0
    rows.append({
        "input": {"lat": "36°30'00\"S", "lon": "175°00'00\"E"},
        "expect": {"ok": True, "result.lat.value": -(36 + 29 / 60 + 53.48482 / 3600),
                   "result.lon.value": 175 + 0.69469 / 3600},
        "source": "The national-grids specification's LINZ example: 36°30'00\"S 175°00'00\"E on NZGD1949 is 36°29'53.48482\"S 175°00'00.69469\"E on NZGD2000, within 0.0001\"",
        "sourceVersion": "LINZ distortion grid nzgd2kgrid0005",
        "tolerance": {"result.lat.value": {"abs": 2.8e-8}, "result.lon.value": {"abs": 2.8e-8}},
    })
    for name, la, lo in PLACES:
        bla, blo = cct(la, lo)
        rows.append({
            "input": {"lat": la, "lon": lo},
            "expect": {"ok": True, "result.lat.value": bla, "result.lon.value": blo},
            "source": f"PROJ 9.9.0 hgridshift applying nz_linz_nzgd2kgrid0005.tif: {name}, NZGD1949 to NZGD2000",
            "sourceVersion": "PROJ 9.9.0; LINZ grid nzgd2kgrid0005",
            "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
        })
    for name, la, lo in PLACES[:6]:
        bla, blo = cct(la, lo, inverse=True)
        rows.append({
            "input": {"lat": la, "lon": lo, "direction": "to-nzgd1949"},
            "expect": {"ok": True, "result.lat.value": bla, "result.lon.value": blo},
            "source": f"PROJ 9.9.0 hgridshift in reverse applying nz_linz_nzgd2kgrid0005.tif: {name}, NZGD2000 to NZGD1949",
            "sourceVersion": "PROJ 9.9.0; LINZ grid nzgd2kgrid0005",
            "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
        })
    rows.append({
        "input": {"lat": -44.0, "lon": -176.5},
        "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
        "source": "LINZ's grid covers 166°E to 180°E; the Chatham Islands, at 176°30'W, are outside it",
        "sourceVersion": "LINZ distortion grid nzgd2kgrid0005",
        "tolerance": {},
    })
    with open(out, "w") as f:
        f.write("\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                     separators=(",", ":")) for i, r in enumerate(rows, 1)) + "\n")


if __name__ == "__main__":
    main(sys.argv[2])
