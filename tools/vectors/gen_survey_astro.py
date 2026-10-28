#!/usr/bin/env python3
"""Golden vectors for the two astronomic-azimuth tools of survey/astro
(add-celestial-and-moon task 6), from USNO's celestial-navigation API, read out
of core/crates/gp-survey/tests/data/usno_azimuth.json.

A pointing with a horizontal angle of zero makes the line's azimuth the body's,
so each USNO row becomes one vector: the azimuth the hour-angle method must
return at that place and instant. Rows with the body under a degree of altitude
are left out, since a pointing cannot be made on a body below the horizon.

The spec's worked Polaris observation is the first vector of the Polaris file:
2016-02-23 04:21:15 UT at 29 deg 31.0' N, 074 deg 30.0' W with a horizontal
angle of 45 deg, which USNO's own azimuth for Polaris there puts at 314.250 deg.
Bowditch's section 1504 is the 2019 edition's numbering and no such worked
example could be found in the 2024 edition, so the check rests on USNO.

    gen_survey_astro.py survey.astro.polaris-azimuth
    gen_survey_astro.py survey.astro.solar-azimuth

Writes nothing: new vectors are appended by hand so published files stay
frozen (see the repository's vector rules).
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
USNO = json.loads(
    (ROOT / "core/crates/gp-survey/tests/data/usno_azimuth.json").read_text()
)
SOURCE = "USNO celnav API 4.0.1 (aa.usno.navy.mil/api/celnav), times as UT1, fetched 2026-10-27"
SPEC = "add-celestial-and-moon survey/astro scenario"


def tol(pairs):
    return {k: {"abs": round(v, 9)} for k, v in pairs}


def rows_for(body):
    return [
        r for r in USNO["rows"] if r["body"] == body and r["hc"] >= 1.0
    ]


def one(tool, row, angle="0 00 00", expect_azimuth=None):
    return {
        "input": {
            "lat": row["lat"],
            "lon": row["lon"],
            "dut1": "0 s",
            "pointings": [
                {"time": row["ut1"].replace(" ", "T") + "Z", "angle": angle}
            ],
        },
        "expect": {
            "ok": True,
            "result.azimuth.value": row["zn"] if expect_azimuth is None else expect_azimuth,
        },
        "source": SOURCE + "; a pointing with a zero horizontal angle makes the line's azimuth the body's",
        "sourceVersion": "API 4.0.1",
        # A thousandth of a degree is 3.6 seconds of arc, well inside what a
        # theodolite reads and well outside the agreement measured in
        # core/crates/gp-survey/tests/astro.rs, which is 1.2 seconds.
        "tolerance": tol([("result.azimuth.value", 0.001)]),
    }


def polaris_azimuth():
    rows = [one("polaris", r) for r in rows_for("POLARIS")]
    spec = next(r for r in USNO["rows"] if r["ut1"].startswith("2016-02-23"))
    rows.insert(
        0,
        {
            "input": {
                "lat": "29 31.0 N",
                "lon": "74 30.0 W",
                "dut1": "0 s",
                "pointings": [
                    {"time": "2016-02-23T04:21:15Z", "angle": "45 00 00"}
                ],
            },
            "expect": {
                "ok": True,
                "result.azimuth.value": (spec["zn"] - 45.0) % 360.0,
                "meta.warnings.*.code": "ASTRONOMIC_AZIMUTH",
            },
            "source": (
                "add-celestial-and-moon survey/astro, the spec's worked Polaris observation, checked "
                "against USNO celnav: Polaris at 359.250 degrees for 2016-02-23 04:21:15 UT at "
                "29 deg 31.0' N, 074 deg 30.0' W, so a 45 degree horizontal angle puts the line at "
                "314.250 degrees"
            ),
            "sourceVersion": "API 4.0.1",
            "tolerance": tol([("result.azimuth.value", 0.001)]),
        },
    )
    rows.append(
        {
            "input": {
                "lat": 29.5,
                "lon": -74.5,
                "dut1": "0 s",
                "pointings": [{"time": "1750-01-01T00:00:00Z", "angle": "0 00 00"}],
            },
            "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
            "source": SPEC + ": the body's place is computed from 1800 to 2100",
            "sourceVersion": "2026",
            "tolerance": {},
        }
    )
    rows.append(
        {
            "input": {
                "lat": 29.5,
                "lon": -74.5,
                "dut1": "0 s",
                "pointings": [{"time": "2016-02-23T04:21:15Z", "angle": "400 00 00"}],
            },
            "expect": {"ok": False, "error.code": "INVALID_INPUT"},
            "source": SPEC + ": a horizontal angle runs from 0 up to 360 degrees",
            "sourceVersion": "2026",
            "tolerance": {},
        }
    )
    return rows


def solar_azimuth():
    rows = [one("solar", r) for r in rows_for("Sun")]
    # The limb correction: the same instant with each limb, which must sit the
    # semidiameter over the cosine of the altitude either side of the centre.
    base = {
        "lat": "39 44 21.1 N",
        "lon": "104 59 25.1 W",
        "dut1": "0 s",
        "pointings": [{"time": "2026-06-21T15:40:00Z", "angle": "112 18 40"}],
    }
    # USNO's celnav for that same instant and place gives the sun's
    # semidiameter as 0.262323 degrees and its geocentric altitude as
    # 44.707665 degrees, and its azimuth as 95.458546 degrees. So the limb
    # correction the tool must report is
    #     0.262323 * 3600 / cos(44.707665 deg) = 1328.3 seconds of arc,
    # which makes this an oracle for the correction, not a restatement of the
    # tool's own output.
    usno_sd_deg, usno_hc_deg, usno_zn = 0.262323, 44.707665, 95.458546
    correction = usno_sd_deg * 3600 / math.cos(math.radians(usno_hc_deg))
    for limb in ("leading", "trailing"):
        rows.append(
            {
                "input": {**base, "limb": limb},
                "expect": {
                    "ok": True,
                    "result.limb_correction.value": correction,
                    "result.pointings.0.body_azimuth.value": usno_zn,
                },
                "source": (
                    SOURCE
                    + f"; a pointing on the {limb} limb is carried to the centre by the semidiameter "
                    "over the cosine of the altitude, both of which USNO publishes for this instant "
                    "and place: 0.262323 deg / cos(44.707665 deg)"
                ),
                "sourceVersion": "API 4.0.1",
                # Two seconds of arc covers the small difference between this
                # core's semidiameter and altitude and USNO's own.
                "tolerance": tol(
                    [
                        ("result.limb_correction.value", 2.0),
                        ("result.pointings.0.body_azimuth.value", 0.001),
                    ]
                ),
            }
        )
    rows.append(
        {
            "input": {
                **base,
                "laplace": "-2.4 arcsec",
            },
            "expect": {"ok": True, "meta.warnings.*.code": "ASTRONOMIC_AZIMUTH"},
            "source": SPEC + ": a Laplace correction gives the geodetic azimuth beside the astronomic one",
            "sourceVersion": "2026",
            "tolerance": {},
        }
    )
    return rows


GENERATORS = {
    "survey.astro.polaris-azimuth": polaris_azimuth,
    "survey.astro.solar-azimuth": solar_azimuth,
}

if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in GENERATORS:
        sys.exit(f"usage: {sys.argv[0]} {{{'|'.join(GENERATORS)}}}")
    for i, row in enumerate(GENERATORS[sys.argv[1]](), start=1):
        print(json.dumps({"id": f"v{i:03d}", **row}, ensure_ascii=False))
