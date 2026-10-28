#!/usr/bin/env python3
"""Golden vectors for the Polaris and compass-check tools of
navigation/celestial (add-celestial-and-moon task 3). Two sources:

- Bowditch 2024 (NGA Pub. 9). Chapter 20 §2015 works a Polaris sight right
  through, so its LHA Aries, correction, latitude, and azimuth are published
  vectors. §2016 works two gyro errors by Pub. No. 229 and §2018 one by the
  amplitude tables; the amplitude example fixes only a latitude and a
  declination, and the Sun holds that declination on 2024-07-24 at 14h30m UT,
  setting then at longitude 80°44.4'E, so the whole example is reproducible.
  Its tabulated values are printed to a tenth of a degree and reached by
  double interpolation, so 0.1° is the closest the table can be read.
- USNO's celnav API (aa.usno.navy.mil/api/celnav, version 4.0.1), read from
  core/crates/gp-navigation/tests/data/usno_sight_reduction.json: the
  computed altitude and azimuth of a body at an assumed position. For the
  Polaris tool the row is used backwards, feeding USNO's own computed
  altitude as the observed altitude and expecting the row's latitude, which
  tests Polaris's place and the latitude solver together.

    gen_celestial_checks.py navigation.celestial.polaris-latitude
    gen_celestial_checks.py navigation.celestial.sun-azimuth-check

Writes nothing: new vectors are appended by hand so published files stay
frozen (see the repository's vector rules).
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
USNO = json.loads(
    (ROOT / "core/crates/gp-navigation/tests/data/usno_sight_reduction.json").read_text()
)
TENTH = 0.1 / 60  # a tenth of a minute of arc, in degrees
FIFTH = 0.2 / 60
SOURCE = "USNO celnav API 4.0.1 (aa.usno.navy.mil/api/celnav), times as UT1, fetched 2026-10-27"
SPEC = "add-celestial-and-moon navigation/celestial scenario"


def dm(d, m):
    return (abs(d) + m / 60) * (-1 if d < 0 else 1)


def utc(row):
    return row["ut1"].replace(" ", "T") + "Z"


def tol(pairs):
    return {k: {"abs": round(v, 6)} for k, v in pairs}


def rows_for(body):
    return [r for r in USNO["rows"] if r["body"] == body]


def polaris_latitude():
    rows = [{
        "input": {"ho": "16 11.3", "utc": "2024-06-02T12:25:12Z", "dut1": "0 s",
                  "lon": "110 07.3 W"},
        "expect": {"ok": True, "result.lat.value": dm(16, 3.2),
                   "result.correction.value": -8.1, "result.zn.value": 0.8,
                   "result.gha_aries.value": dm(77, 45.9)},
        "source": ("Bowditch 2024 (NGA Pub. 9) §2015, Latitude by Polaris: LHA Aries "
                   "327°38.8', a0 51.2' + a1 0.4' + a2 0.3' - 1° = -8.1', Ho 16°11.3', "
                   "latitude 16°03.2'N, azimuth 000.8°. GHA Aries is the example's own "
                   "71°26.9' + 6°19.0' = 77°45.9'; its printed LHA Aries 327°38.8' is 0.2' "
                   "off that figure less the longitude, 327°38.6'"),
        "sourceVersion": "2024 edition",
        # The three table corrections are each rounded to a tenth of a minute
        # and the azimuth is read from a table printed to a tenth of a degree.
        "tolerance": tol([("result.lat.value", FIFTH), ("result.correction.value", 0.2),
                          ("result.zn.value", 0.2), ("result.gha_aries.value", FIFTH)]),
    }]
    for r in rows_for("POLARIS"):
        rows.append({
            "input": {"ho": r["hc"], "utc": utc(r), "dut1": "0 s", "lon": r["lon"]},
            "expect": {"ok": True, "result.lat.value": r["lat"],
                       "result.dec.value": r["dec"], "result.gha.value": r["gha"],
                       "result.zn.value": r["zn"]},
            "source": SOURCE + "; the row read backwards, its computed altitude taken as the observed altitude",
            "sourceVersion": "API 4.0.1",
            "tolerance": tol([("result.lat.value", TENTH), ("result.dec.value", TENTH),
                              ("result.gha.value", TENTH), ("result.zn.value", 0.05)]),
        })
    rows.append({
        "input": {"ho": "95 00.0", "utc": "2024-06-02T12:25:12Z", "lon": 0.0},
        "expect": {"ok": False, "error.code": "INVALID_INPUT"},
        "source": SPEC + ": an altitude over 90 degrees is not an altitude",
        "sourceVersion": "2026", "tolerance": {},
    })
    rows.append({
        "input": {"ho": 89.9, "utc": "2024-06-02T12:25:12Z", "dut1": "0 s", "lon": 0.0},
        "expect": {"ok": True},
        "source": SPEC + ": a sight from under the pole still answers",
        "sourceVersion": "2026", "tolerance": {},
    })
    return rows


def sun_azimuth_check():
    rows = [{
        "input": {"utc": "2024-07-24T14:30:00Z", "dut1": "0 s", "lat": "51 24.6 N",
                  "lon": "80 44.4 E", "bearing": "303 deg", "compass": "gyro"},
        "expect": {"ok": True, "result.dec.value": dm(19, 40.4),
                   "result.amplitude.value": 32.6, "result.amplitude_azimuth.value": 302.6,
                   "result.error.value": -0.4, "result.amplitude_label": "W 32.7 N"},
        "source": ("Bowditch 2024 (NGA Pub. 9) §2018, Amplitude of a Body Observed on the "
                   "Celestial Horizon: latitude 51°24.6'N, declination N 19°40.4', amplitude "
                   "W 32.6°N, true bearing 302.6°, gyro reading 303.0°, error 0.4° W"),
        "sourceVersion": "2024 edition",
        # Table 22 is printed to a tenth of a degree and reached by double
        # interpolation; the closed form gives 32.67 degrees.
        "tolerance": tol([("result.dec.value", TENTH), ("result.amplitude.value", 0.1),
                          ("result.amplitude_azimuth.value", 0.1), ("result.error.value", 0.1)]),
    }]
    for r in rows_for("Sun"):
        rows.append({
            "input": {"utc": utc(r), "dut1": "0 s", "lat": r["lat"], "lon": r["lon"]},
            "expect": {"ok": True, "result.zn.value": r["zn"], "result.dec.value": r["dec"],
                       "result.lha.value": (r["gha"] + r["lon"]) % 360.0},
            "source": SOURCE,
            "sourceVersion": "API 4.0.1",
            "tolerance": tol([("result.zn.value", 0.05), ("result.dec.value", TENTH),
                              ("result.lha.value", TENTH)]),
        })
    rows.append({
        "input": {"utc": "2024-07-24T14:30:00Z", "lat": "51 24.6 N", "lon": "80 44.4 E",
                  "bearing": "303 deg", "compass": "magnetic"},
        "expect": {"ok": False, "error.code": "INVALID_INPUT"},
        "source": SPEC + ": a magnetic compass cannot be checked without its variation",
        "sourceVersion": "2026", "tolerance": {},
    })
    rows.append({
        "input": {"utc": "2024-07-24T14:30:00Z", "dut1": "0 s", "lat": "51 24.6 N",
                  "lon": "80 44.4 E", "bearing": "303 deg", "compass": "magnetic",
                  "variation": "-1.5 deg"},
        "expect": {"ok": True, "result.deviation.value": 1.166667},
        "source": SPEC + ": the deviation is the compass error less the variation supplied",
        "sourceVersion": "2026",
        "tolerance": tol([("result.deviation.value", 0.1)]),
    })
    rows.append({
        "input": {"utc": "2024-06-21T12:00:00Z", "dut1": "0 s", "lat": "80 00.0 N",
                  "lon": 0.0},
        "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
        "source": SPEC + ": where the Sun does not cross the celestial horizon it has no amplitude",
        "sourceVersion": "2026", "tolerance": {},
    })
    return rows


GENERATORS = {
    "navigation.celestial.polaris-latitude": polaris_latitude,
    "navigation.celestial.sun-azimuth-check": sun_azimuth_check,
}

if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in GENERATORS:
        sys.exit(f"usage: {sys.argv[0]} {{{'|'.join(GENERATORS)}}}")
    for i, row in enumerate(GENERATORS[sys.argv[1]](), start=1):
        print(json.dumps({"id": f"v{i:03d}", **row}, ensure_ascii=False))
