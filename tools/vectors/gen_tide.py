#!/usr/bin/env python3
"""Golden vectors for navigation.tide.predict and navigation.tide.datums
(add-marine-navigation, navigation/tides).

The prediction vectors expect NOAA's own published heights: each feeds a
station's harmonic constituents and datum sheet (NOAA CO-OPS harcon.json
and datums.json, units=english) to the tool and checks four hourly heights
of one day against NOAA's predictions API (MLLW, GMT) within 0.02 ft, the
tolerance the spec states. The inputs and NOAA's numbers are read from
core/crates/gp-navigation/tests/data/tides/<station>.json, fetched
2026-10-20. The datum vectors use the Boston (8443970) sheet in meters.

    gen_tide.py predict    # print the prediction vectors as JSON lines
    gen_tide.py datums     # print the datum vectors

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "..", "core", "crates", "gp-navigation", "tests", "data", "tides")
STATIONS = ["8443970", "9447130", "9414290", "8724580", "1612340"]
NOAA = "NOAA CO-OPS predictions API for station {st}, MLLW, GMT, as published (fetched 2026-10-20)"
VER = "2026 predictions"


def compact(v):
    return json.dumps(v, separators=(",", ":"))


def predict():
    out = []
    for st in STATIONS:
        f = json.load(open(os.path.join(DATA, f"{st}.json")))
        hourly = {t: v for t, v in f["hourly"]}
        for day in ("03", "10", "17", "24"):
            exp = {"ok": True}
            tol = {}
            for k in (0, 6, 12, 18):
                exp[f"result.series.{k}.height.value"] = hourly[f"2026-10-{day} {k:02d}:00"]
                tol[f"result.series.{k}.height.value"] = {"abs": 0.02}
            out.append({"input": {"constituents": compact(f["harcon"]), "datum_sheet": compact(f["datums"]),
                                  "start": f"2026-10-{day}T00:00Z", "days": 1, "interval": "60 min"},
                        "expect": exp, "source": NOAA.format(st=st), "sourceVersion": VER, "tolerance": tol})
    f = json.load(open(os.path.join(DATA, "8443970.json")))
    out.append({"input": {"constituents": compact(f["harcon"]), "start": "2026-10-21", "days": 90},
                "expect": {"ok": False, "error.code": "LIMIT_EXCEEDED", "error.field": "/days"},
                "source": "add-marine-navigation tides spec: windows of at most 31 days", "sourceVersion": VER, "tolerance": {}})
    few = dict(f["harcon"]); few["HarmonicConstituents"] = f["harcon"]["HarmonicConstituents"][:20]
    out.append({"input": {"constituents": compact(few), "start": "2026-10-21"},
                "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/partial"},
                "source": "add-marine-navigation tides spec: a partial paste needs the acknowledgment", "sourceVersion": VER, "tolerance": {}})
    return out


def datums():
    sheet = {"units": "meters", "datums": [{"name": n, "value": v} for n, v in [
        ("STND", 0.0), ("MHHW", 4.205), ("MHW", 4.071), ("DTL", 2.64), ("MTL", 2.624), ("MSL", 2.66),
        ("MLW", 1.178), ("MLLW", 1.074), ("GT", 3.131), ("MN", 2.893), ("NAVD88", 2.752)]]}
    d = {x["name"]: x["value"] for x in sheet["datums"]}
    src = "NOAA CO-OPS datums for Boston, MA (8443970), epoch 1983-2001, by subtraction"
    out = []
    pairs = [("MLLW", "NAVD88", 3.0), ("MLLW", "MHHW", 0.0), ("MHHW", "MLLW", 0.0), ("NAVD88", "MLLW", 0.0), ("MSL", "MLLW", 1.5),
             ("MLW", "MHW", 2.0), ("MTL", "MSL", 0.25), ("DTL", "MTL", -0.5), ("STND", "MLLW", 5.0), ("MHHW", "NAVD88", -1.0),
             ("NAVD88", "MSL", 0.0), ("MLLW", "STND", 0.0), ("MHW", "MLW", 1.0), ("MSL", "NAVD88", 0.0), ("MLLW", "MLLW", 2.345),
             ("MLLW", "MSL", 10.0), ("MHHW", "MHW", 0.0), ("DTL", "NAVD88", 0.0), ("MLW", "STND", 0.0), ("MTL", "MLLW", 0.0)]
    for frm, to, h in pairs:
        out.append({"input": {"sheet": compact(sheet), "height": f"{h} m", "from": frm, "to": to},
                    "expect": {"ok": True, "result.converted.value": h + d[frm] - d[to]},
                    "source": src, "sourceVersion": "1983-2001 epoch", "tolerance": {"result.converted.value": {"abs": 1e-9}}})
    out.append({"input": {"sheet": compact(sheet), "height": "1 m", "from": "MLLW", "to": "LAT"},
                "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/to"}, "source": src, "sourceVersion": "1983-2001 epoch", "tolerance": {}})
    out.append({"input": {"sheet": compact(sheet), "height": "1 m", "from": "GT", "to": "MLLW"},
                "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/from"}, "source": src, "sourceVersion": "1983-2001 epoch", "tolerance": {}})
    return out


rows = predict() if sys.argv[1] == "predict" else datums()
for i, v in enumerate(rows, 1):
    print(json.dumps({"id": f"v{i:03d}", **v}, ensure_ascii=False, separators=(",", ":")))
