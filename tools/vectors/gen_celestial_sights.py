#!/usr/bin/env python3
"""Golden vectors for the two sight tools of navigation/celestial
(add-celestial-and-moon task 3). Two independent sources:

- Bowditch 2024 (NGA Pub. 9). Chapter 20 §2010 Table 2010b works a four-star
  fix right through, so its LHA, declination, computed altitude, azimuth, and
  intercept are published vectors for the reduction. Chapter 16 §1618 to
  §1622 works eight sextant altitude corrections, Sun and Moon limbs, a
  planet, a star, and two low sights. Table 2010b's Procyon column is left
  out: its increment for 35m59s is printed as 8 deg 56.2' where 9 deg 01.2'
  is right, so its GHA, LHA, and Hc are each about 5' adrift (USNO gives GHA
  265 deg 09.9' for that second), and the table repeats its GHA as its SHA.
- USNO's celnav API (aa.usno.navy.mil/api/celnav, version 4.0.1), read from
  core/crates/gp-navigation/tests/data/usno_sight_reduction.json: the
  computed altitude and azimuth at an assumed position, and its own
  refraction, semidiameter, and parallax in altitude. Those corrections are
  evaluated at the apparent altitude ha = hc - refr - sd - pa, so a sight
  taken from a zero height of eye with no index error has exactly that
  sextant altitude. Bennett's refraction matches USNO's within 0.02 minutes
  of arc from 1.4 to 89 degrees of apparent altitude, so rows down to
  2 degrees are used.

    gen_celestial_sights.py navigation.celestial.sight-reduction
    gen_celestial_sights.py navigation.celestial.altitude-correction

Writes nothing: new vectors are appended by hand so published files stay
frozen (see the repository's vector rules).
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
USNO = json.loads(
    (ROOT / "core/crates/gp-navigation/tests/data/usno_sight_reduction.json").read_text()
)
TENTH = 0.1 / 60  # a tenth of a minute of arc, in degrees
FIFTH = 0.2 / 60
SOURCE = "USNO celnav API 4.0.1 (aa.usno.navy.mil/api/celnav), times as UT1, fetched 2026-10-27"


def dm(d, m):
    return (abs(d) + m / 60) * (-1 if d < 0 else 1)


def utc(row):
    return row["ut1"].replace(" ", "T") + "Z"


def tol(pairs):
    return {k: {"abs": round(v, 6)} for k, v in pairs}


def sight_reduction():
    rows = []
    bow = "Bowditch 2024 (NGA Pub. 9) §2010, Table 2010b, the complete solution of a four-star fix"
    for body, t, lat, lon, ho, lha, dec, hc, zn, a in [
        ("Spica", "08:24:03", "41 00.0 S", "128 18.7 E", "32 53.4",
         304.0, dm(-11, 17.4), dm(32, 50.4), 75.4, 3.0),
        ("Regulus", "08:29:58", "41 00.0 S", "127 37.3 E", "37 05.6",
         354.0, dm(11, 50.9), dm(36, 51.6), 7.3, 14.0),
        ("Canopus", "08:41:55", "41 00.0 S", "128 19.4 E", "52 11.7",
         54.0, dm(-52, 42.6), dm(52, 15.0), 233.2, -3.3),
    ]:
        rows.append({
            "input": {"body": body, "utc": f"2024-06-02T{t}Z", "dut1": "0 s",
                      "lat": lat, "lon": lon, "ho": ho},
            "expect": {"ok": True, "result.lha.value": lha, "result.dec.value": dec,
                       "result.hc.value": hc, "result.zn.value": zn,
                       "result.intercept.value": a,
                       "result.direction": "Toward" if a >= 0 else "Away"},
            "source": (bow + f": {body} at {t}, LHA {lha:.1f} deg, Hc and Zn from Pub. No. 229, "
                       "intercept in nautical miles"),
            "sourceVersion": "2024 edition",
            # Every input the table prints is rounded to a tenth of a minute,
            # and its Hc carries Pub. No. 229's own interpolation.
            "tolerance": tol([("result.lha.value", FIFTH), ("result.dec.value", TENTH),
                              ("result.hc.value", FIFTH), ("result.zn.value", 0.1),
                              ("result.intercept.value", 0.2)]),
        })
    picked = [r for r in USNO["rows"] if r["hc"] > 2.0][::6]
    for r in picked:
        lim = 0.5 / 60 if r["body"] == "Venus" else 0.25 / 60 if r["body"] == "Moon" else TENTH
        rows.append({
            "input": {"body": r["body"], "utc": utc(r), "dut1": "0 s",
                      "lat": r["lat"], "lon": r["lon"], "ho": 0.0},
            "expect": {"ok": True, "result.gha.value": r["gha"], "result.dec.value": r["dec"],
                       "result.hc.value": r["hc"], "result.zn.value": r["zn"]},
            "source": SOURCE,
            "sourceVersion": "API 4.0.1",
            # USNO's Venus departs from JPL within weeks of inferior
            # conjunction and its Moon drifts about 0.005' a year, both
            # recorded in the gp-astro tests and the change's design note.
            "tolerance": tol([("result.gha.value", lim), ("result.dec.value", lim),
                              ("result.hc.value", lim), ("result.zn.value", 0.05)]),
        })
    rows.append({
        "input": {"body": "Sun", "utc": "2024-06-02T00:00:00Z", "dut1": "0 s",
                  "lat": 51.48, "lon": 0.0, "ho": "-5 00.0"},
        "expect": {"ok": True, "meta.warnings.*.code": "BODY_BELOW_HORIZON"},
        "source": "add-celestial-and-moon navigation/celestial scenario: a body below the horizon still reports Hc and Zn",
        "sourceVersion": "2026", "tolerance": {},
    })
    rows.append({
        "input": {"body": "Sirius B", "utc": "2024-06-02T08:24:03Z", "lat": 0.0,
                  "lon": 0.0, "ho": 10.0},
        "expect": {"ok": False, "error.code": "INVALID_INPUT"},
        "source": "add-celestial-and-moon navigation/celestial: only the Sun, Moon, four planets, and the 58 almanac stars are named",
        "sourceVersion": "2026", "tolerance": {},
    })
    return rows


BOWDITCH_16 = [
    ("§1618 Example 1, the Sun's lower limb from 38 feet",
     {"hs": "51 28.4", "body": "Sun", "limb": "lower", "height_of_eye": "38 ft",
      "utc": "2024-06-02T11:00:00Z", "index_correction": "-2.0 arcmin", "dut1": "0 s"},
     dm(51, 20.4), dm(51, 35.6), 0.15),
    ("§1618 Example 2, the Sun's upper limb from 45 feet",
     {"hs": "32 47.9", "body": "Sun", "limb": "upper", "height_of_eye": "45 ft",
      "utc": "2024-06-02T11:00:00Z", "index_correction": "1.0 arcmin", "dut1": "0 s"},
     dm(32, 42.4), dm(32, 25.1), 0.15),
    ("§1619 Example 1, the Moon's lower limb with HP 59.6'",
     {"hs": "18 04.6", "body": "Moon", "limb": "lower", "height_of_eye": "32 ft",
      "utc": "2024-06-02T11:00:00Z", "index_correction": "3.2 arcmin",
      "hp": "59.6 arcmin", "dut1": "0 s"},
     dm(18, 2.3), dm(19, 12.2), 0.1),
    ("§1619 Example 2, the Moon's upper limb with HP 59.6'",
     {"hs": "66 47.3", "body": "Moon", "limb": "upper", "height_of_eye": "70 ft",
      "utc": "2024-06-02T09:00:00Z", "index_correction": "-1.6 arcmin",
      "hp": "59.6 arcmin", "dut1": "0 s"},
     dm(66, 37.6), dm(66, 44.5), 0.1),
    ("§1620, Venus from 28 feet",
     {"hs": "44 21.3", "body": "Venus", "limb": "center", "height_of_eye": "28 ft",
      "utc": "2024-12-19T18:00:00Z", "dut1": "0 s"},
     dm(44, 16.2), dm(44, 15.3), 0.1),
    ("§1621, the star Miaplacidus from 50 feet",
     {"hs": "27 54.0", "body": "Miaplacidus", "limb": "center", "height_of_eye": "50 ft",
      "utc": "2024-06-02T11:00:00Z", "index_correction": "1.0 arcmin", "dut1": "0 s"},
     dm(27, 48.1), dm(27, 46.3), 0.1),
    ("§1622 Example 1, the Sun low down at 88 degrees Fahrenheit",
     {"hs": "1 24.4", "body": "Sun", "limb": "lower", "height_of_eye": "45 ft",
      "utc": "2024-06-02T11:00:00Z", "index_correction": "1.8 arcmin",
      "temperature": "88 degF", "pressure": "29.78 inHg", "dut1": "0 s"},
     dm(1, 19.7), dm(1, 15.4), 0.25),
    ("§1622 Example 3, the Moon's upper limb low down",
     {"hs": "2 35.4", "body": "Moon", "limb": "upper", "height_of_eye": "33 ft",
      "utc": "2024-06-02T17:14:27Z", "temperature": "63 degF",
      "pressure": "29.81 inHg", "hp": "59.6 arcmin", "dut1": "0 s"},
     dm(2, 29.8), dm(2, 57.4), 0.25),
]


def altitude_correction():
    rows = []
    for title, inp, ha, ho, limit in BOWDITCH_16:
        rows.append({
            "input": inp,
            "expect": {"ok": True, "result.ha.value": ha, "result.ho.value": ho},
            "source": ("Bowditch 2024 (NGA Pub. 9) chapter 16, Sextant Altitude Corrections, "
                       + title),
            "sourceVersion": "2024 edition",
            "tolerance": tol([("result.ha.value", 0.05 / 60), ("result.ho.value", limit / 60)]),
        })
    # The Moon is left to Bowditch's four examples: USNO augments its
    # semidiameter for the altitude and takes its parallax from the observer's
    # own geocentric radius, where the almanac convention these tools follow
    # uses the tabulated horizontal parallax and the equatorial radius. The
    # two part by up to 0.2' in parallax and 0.15' in semidiameter.
    picked = [r for r in USNO["rows"]
              if "refr" in r and r["body"] != "Moon"
              and r["hc"] - r["refr"] - r["sd"] - r["pa"] > 2.0][::5]
    for r in picked:
        ha = r["hc"] - r["refr"] - r["sd"] - r["pa"]
        rows.append({
            "input": {"hs": round(ha, 6), "body": r["body"],
                      "limb": "lower" if r["sd"] > 0 else "center",
                      "height_of_eye": "0 m", "utc": utc(r), "dut1": "0 s"},
            "expect": {"ok": True,
                       "result.refraction.value": round(r["refr"] * 60, 4),
                       "result.semidiameter.value": round(r["sd"] * 60, 4),
                       "result.parallax.value": round(r["pa"] * 60, 4),
                       "result.ho.value": r["hc"]},
            "source": SOURCE + ", its own corrections at the apparent altitude",
            "sourceVersion": "API 4.0.1",
            "tolerance": tol([("result.refraction.value", 0.1), ("result.semidiameter.value", 0.1),
                              ("result.parallax.value", 0.1), ("result.ho.value", 0.0025)]),
        })
    rows.append({
        "input": {"hs": "1 24.4", "body": "Sun", "limb": "lower", "height_of_eye": "45 ft",
                  "utc": "2024-06-02T11:00:00Z", "dut1": "0 s"},
        "expect": {"ok": True, "meta.warnings.*.code": "LOW_ALTITUDE_REFRACTION"},
        "source": "add-celestial-and-moon navigation/celestial: a sight below 10 degrees of apparent altitude says so",
        "sourceVersion": "2026", "tolerance": {},
    })
    rows.append({
        "input": {"hs": "27 54.0", "body": "Spica", "limb": "lower", "height_of_eye": "50 ft",
                  "utc": "2024-06-02T11:00:00Z", "dut1": "0 s"},
        "expect": {"ok": False, "error.code": "INVALID_INPUT"},
        "source": "add-celestial-and-moon navigation/celestial: a star shows no disc, so it has no limb",
        "sourceVersion": "2026", "tolerance": {},
    })
    return rows


GENERATORS = {
    "navigation.celestial.sight-reduction": sight_reduction,
    "navigation.celestial.altitude-correction": altitude_correction,
}

if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in GENERATORS:
        sys.exit("usage: gen_celestial_sights.py <%s>" % "|".join(GENERATORS))
    for i, row in enumerate(GENERATORS[sys.argv[1]](), 1):
        print(json.dumps({"id": f"v{i:03d}", **row}, ensure_ascii=False,
                         separators=(",", ":")))
