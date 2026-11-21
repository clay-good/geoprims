#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.chenyx06 (add-international-grids,
geodesy/national-grids: "Canada, Europe, and the other national paths", the
CHENyx06 half).

Two oracles. The first five vectors are swisstopo's own numerical example
(Formulas and constants, December 2016, section 7.1): the five EUREF points of
Switzerland in LV03 and the LV95 coordinates swisstopo's FINELTRA
transformation of CHENyx06 gives for them, which the grid approximation this
tool applies reproduces within 8.1 mm. The rest are PROJ 9.9.0 applying the
same grid through hgridshift between EPSG 21781 (LV03) and EPSG 2056 (LV95),
both ways.

    gen_chenyx06.py GRID_TIF OUT.jsonl
"""
import json
import subprocess
import sys

TIF = sys.argv[1]
LV03 = ("+proj=somerc +lat_0=46.9524055555556 +lon_0=7.43958333333333 +k_0=1 "
        "+x_0=600000 +y_0=200000 +ellps=bessel").split()
LV95 = ("+proj=somerc +lat_0=46.9524055555556 +lon_0=7.43958333333333 +k_0=1 "
        "+x_0=2600000 +y_0=1200000 +ellps=bessel").split()


def convert(east, north, to_lv95=True):
    src, dst = (LV03, LV95) if to_lv95 else (LV95, LV03)
    step = ["+step", "+proj=hgridshift", f"+grids={TIF}"]
    if not to_lv95:
        step = ["+step", "+inv", "+proj=hgridshift", f"+grids={TIF}"]
    pipe = ["+proj=pipeline", "+step", "+inv", *src, *step, "+step", *dst]
    o = subprocess.run(["cct", "-d", "6", *pipe], input=f"{east} {north} 0 0\n",
                       check=True, capture_output=True, text=True).stdout.split()
    return float(o[0]), float(o[1])


# swisstopo's section 7.1 example: LV03 in, the LV95 it prints out.
EUREF = [
    ("Zimmerwald", 602030.680, 191775.030, 2602030.740, 1191775.030),
    ("Chrischona", 617306.300, 268507.300, 2617306.920, 1268507.870),
    ("Pfaender", 776668.105, 265372.681, 2776668.590, 1265372.250),
    ("La Givrine", 497313.292, 145625.438, 2497312.650, 1145626.140),
    ("Monte Generoso", 722758.810, 87649.670, 2722759.060, 1087648.190),
]

# Places across the country, in LV03.
PLACES = [
    ("Bern, the Federal Palace", 600421.0, 199498.0),
    ("Zurich main station", 683206.0, 248234.0),
    ("Basel, Marktplatz", 611265.0, 267593.0),
    ("Geneva, Jet d'Eau", 500675.0, 117769.0),
    ("Lausanne cathedral", 538232.0, 152414.0),
    ("Lugano, Piazza Riforma", 717252.0, 96025.0),
    ("St. Gallen, Abbey", 745509.0, 254309.0),
    ("Chur, the cathedral", 759594.0, 190851.0),
    ("Sion, Valere", 594105.0, 120260.0),
    ("Lucerne, the Chapel Bridge", 665793.0, 211705.0),
    ("Neuchatel, the collegiate church", 561148.0, 204581.0),
    ("Davos Platz", 783654.0, 185094.0),
    ("Zermatt", 624222.0, 97277.0),
    ("Interlaken", 633077.0, 170651.0),
    ("Schaffhausen, the Munot", 689620.0, 283498.0),
    ("Bellinzona, Castelgrande", 721571.0, 117019.0),
    ("Fribourg cathedral", 578736.0, 183828.0),
    ("Biel", 585211.0, 220961.0),
    ("Winterthur", 697167.0, 261544.0),
    ("Locarno, Piazza Grande", 704264.0, 114261.0),
    ("Bulle", 571010.0, 163260.0),
    ("Thun", 614253.0, 178835.0),
]


def main(out):
    rows = []
    for name, e03, n03, e95, n95 in EUREF:
        rows.append({
            "input": {"easting": f"{e03} m", "northing": f"{n03} m"},
            "expect": {"ok": True, "result.easting.value": e95, "result.northing.value": n95},
            "source": f"swisstopo, Formulas and constants (December 2016), section 7.1: {name}, LV03 to LV95 by FINELTRA with CHENyx06",
            "sourceVersion": "swisstopo refsys, December 2016 edition; CHENyx06",
            "tolerance": {"result.easting.value": {"abs": 0.0081},
                          "result.northing.value": {"abs": 0.0081}},
        })
    for name, e03, n03 in PLACES:
        e95, n95 = convert(e03, n03)
        rows.append({
            "input": {"easting": f"{e03} m", "northing": f"{n03} m"},
            "expect": {"ok": True, "result.easting.value": e95, "result.northing.value": n95,
                       "result.east_shift.value": round(e95 - e03 - 2_000_000.0, 6),
                       "result.north_shift.value": round(n95 - n03 - 1_000_000.0, 6)},
            "source": f"PROJ 9.9.0 hgridshift applying ch_swisstopo_CHENyx06a.tif between EPSG 21781 and EPSG 2056: {name}, LV03 to LV95",
            "sourceVersion": "PROJ 9.9.0; swisstopo CHENyx06",
            "tolerance": {"result.easting.value": {"abs": 1e-5},
                          "result.northing.value": {"abs": 1e-5},
                          "result.east_shift.value": {"abs": 2e-5},
                          "result.north_shift.value": {"abs": 2e-5}},
        })
    for name, e03, n03 in PLACES[:10]:
        e95, n95 = convert(e03, n03)
        a, b = convert(round(e95, 4), round(n95, 4), to_lv95=False)
        rows.append({
            "input": {"easting": f"{round(e95, 4)} m", "northing": f"{round(n95, 4)} m",
                      "direction": "to-lv03"},
            "expect": {"ok": True, "result.easting.value": a, "result.northing.value": b},
            "source": f"PROJ 9.9.0 hgridshift in reverse applying the same grid: {name}, LV95 to LV03",
            "sourceVersion": "PROJ 9.9.0; swisstopo CHENyx06",
            "tolerance": {"result.easting.value": {"abs": 1e-5},
                          "result.northing.value": {"abs": 1e-5}},
        })
    # Off the grid: Milan is south of 45°28'N, Munich east of 11°03'E.
    for name, e03, n03 in [("Milan, which is south of the grid", 530000.0, -40000.0),
                           ("Munich, which is east of the grid", 1000000.0, 300000.0)]:
        rows.append({
            "input": {"easting": f"{e03} m", "northing": f"{n03} m"},
            "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
            "source": f"CHENyx06 covers 5°33′E to 11°03′E and 45°28′N to 48°04′N: {name}",
            "sourceVersion": "swisstopo CHENyx06",
            "tolerance": {},
        })
    with open(out, "w") as f:
        f.write("\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                     separators=(",", ":")) for i, r in enumerate(rows, 1)) + "\n")


if __name__ == "__main__":
    main(sys.argv[2])
