#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.ch1903plus (add-international-grids,
geodesy/national-grids: "Canada, Europe, and the other national paths", the
CH1903+ datum step).

Two oracles. The first ten vectors are swisstopo's own numerical example
(Formulas and constants, December 2016, section 7.1 and 7.2): the five EUREF
points of Switzerland on CH1903+ and on ETRS89, each way, as printed. The rest
are PROJ 9.9.0 applying the same three parameters through cct, which is an
independent implementation of the geocentric translation and of the conversions
either side of it.

    gen_ch1903plus.py OUT.jsonl
"""
import json
import subprocess
import sys

PIPE_TO_ETRS = [
    "+proj=pipeline",
    "+step", "+proj=unitconvert", "+xy_in=deg", "+xy_out=rad",
    "+step", "+proj=cart", "+ellps=bessel",
    "+step", "+proj=helmert", "+x=674.374", "+y=15.056", "+z=405.346",
    "+step", "+inv", "+proj=cart", "+ellps=GRS80",
    "+step", "+proj=unitconvert", "+xy_in=rad", "+xy_out=deg",
]
PIPE_TO_PLUS = [
    "+proj=pipeline",
    "+step", "+proj=unitconvert", "+xy_in=deg", "+xy_out=rad",
    "+step", "+proj=cart", "+ellps=GRS80",
    "+step", "+inv", "+proj=helmert", "+x=674.374", "+y=15.056", "+z=405.346",
    "+step", "+inv", "+proj=cart", "+ellps=bessel",
    "+step", "+proj=unitconvert", "+xy_in=rad", "+xy_out=deg",
]


def run(pipe, lat, lon, h):
    o = subprocess.run(["cct", "-d", "11", *pipe], input=f"{lon} {lat} {h} 0\n",
                       check=True, capture_output=True, text=True).stdout.split()
    return float(o[1]), float(o[0]), float(o[2])


def dms(d, m, s):
    return d + m / 60.0 + s / 3600.0


# swisstopo's section 7.1: (name, CH1903+ lat, lon, h, ETRS89 lat, lon, h).
EUREF = [
    ("Zimmerwald", dms(46, 52, 42.269284), dms(7, 27, 58.416328), 897.361,
     dms(46, 52, 37.540562), dms(7, 27, 54.983506), 947.149),
    ("Chrischona", dms(47, 34, 6.404965), dms(7, 40, 10.574820), 457.138,
     dms(47, 34, 1.385301), dms(7, 40, 6.983077), 504.935),
    ("Pfaender", dms(47, 31, 0.092644), dms(9, 47, 8.465989), 1043.616,
     dms(47, 30, 55.172797), dms(9, 47, 3.697723), 1089.372),
    ("La Givrine", dms(46, 27, 19.272743), dms(6, 6, 9.983811), 1206.367,
     dms(46, 27, 14.690021), dms(6, 6, 7.326361), 1258.274),
    ("Monte Generoso", dms(45, 55, 49.707052), dms(9, 1, 20.606368), 1634.472,
     dms(45, 55, 45.438020), dms(9, 1, 16.389053), 1685.027),
]

# Places across Switzerland, as CH1903+ latitude, longitude and a plausible
# ellipsoidal height.
PLACES = [
    ("Bern, the Zytglogge", 46.947975, 7.447447, 590.0),
    ("Zurich, the Grossmunster", 47.370080, 8.544200, 455.0),
    ("Geneva, the Jet d'Eau", 46.207400, 6.155800, 420.0),
    ("Basel, the Munster", 47.555900, 7.592500, 320.0),
    ("Lausanne, the cathedral", 46.522700, 6.635400, 580.0),
    ("Lugano, the Piazza Riforma", 46.004400, 8.951600, 320.0),
    ("St. Moritz", 46.497800, 9.838300, 1870.0),
    ("Zermatt", 46.020700, 7.749100, 1660.0),
    ("Chur, the cathedral", 46.850300, 9.533200, 640.0),
    ("Sion, Valere", 46.233200, 7.361600, 670.0),
    ("Lucerne, the Chapel Bridge", 47.051500, 8.307600, 480.0),
    ("Neuchatel, the collegiate church", 46.993000, 6.927500, 480.0),
    ("Davos Platz", 46.796400, 9.823100, 1610.0),
    ("Interlaken", 46.686200, 7.863200, 620.0),
    ("Schaffhausen, the Munot", 47.697700, 8.638900, 450.0),
    ("Bellinzona, Castelgrande", 46.194400, 9.019400, 290.0),
    ("Fribourg, the cathedral", 46.806200, 7.161900, 640.0),
    ("Biel", 47.140800, 7.246400, 490.0),
    ("Winterthur", 47.499600, 8.726400, 490.0),
    ("Locarno, the Piazza Grande", 46.170000, 8.795000, 250.0),
]


def main(out):
    rows = []
    for name, plat, plon, ph, elat, elon, eh in EUREF:
        rows.append({
            "input": {"lat": plat, "lon": plon, "height": f"{ph} m"},
            "expect": {"ok": True, "result.lat.value": elat, "result.lon.value": elon,
                       "result.height.value": eh},
            "source": f"swisstopo, Formulas and constants (December 2016), section 7.1: {name}, CH1903+ to ETRS89 as printed",
            "sourceVersion": "swisstopo refsys, December 2016 edition",
            "tolerance": {"result.lat.value": {"abs": 2.8e-9},
                          "result.lon.value": {"abs": 2.8e-9},
                          "result.height.value": {"abs": 0.0005}},
        })
        rows.append({
            "input": {"lat": elat, "lon": elon, "height": f"{eh} m",
                      "direction": "to-ch1903-plus"},
            "expect": {"ok": True, "result.lat.value": plat, "result.lon.value": plon,
                       "result.height.value": ph},
            "source": f"swisstopo, Formulas and constants (December 2016), section 7.2, which is 7.1 in reverse: {name}, ETRS89 to CH1903+",
            "sourceVersion": "swisstopo refsys, December 2016 edition",
            "tolerance": {"result.lat.value": {"abs": 2.8e-9},
                          "result.lon.value": {"abs": 2.8e-9},
                          "result.height.value": {"abs": 0.0005}},
        })
    for name, lat, lon, h in PLACES:
        a, b, c = run(PIPE_TO_ETRS, lat, lon, h)
        rows.append({
            "input": {"lat": lat, "lon": lon, "height": f"{h} m"},
            "expect": {"ok": True, "result.lat.value": a, "result.lon.value": b,
                       "result.height.value": c},
            "source": f"PROJ 9.9.0 applying the same three parameters through cart and helmert: {name}, CH1903+ to ETRS89. PROJ's +ellps=bessel is EPSG's 1/f = 299.1528128 and this tool uses swisstopo's own 299.15281285, which is what its printed e-squared of 0.006674372230614 states; the two disagree by 4 nanometres of latitude and 2 micrometres of height, which is this tolerance",
            "sourceVersion": "PROJ 9.9.0; swisstopo parameters of 1997",
            "tolerance": {"result.lat.value": {"abs": 1e-10},
                          "result.lon.value": {"abs": 1e-10},
                          "result.height.value": {"abs": 1e-5}},
        })
    for name, lat, lon, h in PLACES[:8]:
        a, b, c = run(PIPE_TO_PLUS, lat, lon, h)
        rows.append({
            "input": {"lat": lat, "lon": lon, "height": f"{h} m",
                      "direction": "to-ch1903-plus"},
            "expect": {"ok": True, "result.lat.value": a, "result.lon.value": b,
                       "result.height.value": c},
            "source": f"PROJ 9.9.0 applying the same three parameters in reverse: {name}, ETRS89 to CH1903+, with the same 1/f difference between PROJ's Bessel and swisstopo's",
            "sourceVersion": "PROJ 9.9.0; swisstopo parameters of 1997",
            "tolerance": {"result.lat.value": {"abs": 1e-10},
                          "result.lon.value": {"abs": 1e-10},
                          "result.height.value": {"abs": 1e-5}},
        })
    with open(out, "w") as f:
        f.write("\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                     separators=(",", ":")) for i, r in enumerate(rows, 1)) + "\n")


if __name__ == "__main__":
    main(sys.argv[1])
