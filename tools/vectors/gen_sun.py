#!/usr/bin/env python3
"""Golden vectors for time.sun.* (needs pvlib for the SPA vectors).

Rise, set, transit, and civil twilight times are USNO Astronomical Applications
API values (aa.usno.navy.mil/api/rstt/oneday, retrieved 2026-09-18), which the
core matches to the minute at these sites. Sun position and the day's highest
sun are the NOAA solar equations transcribed independently in Python."""
import json
import math
import sys
from datetime import datetime, timezone
from pathlib import Path

USNO = "USNO Astronomical Applications API rstt/oneday, retrieved 2026-09-18"
USNO_VER = "API 4.0.1"
NOAA = "NOAA solar calculator equations transcribed in Python (tools/vectors/gen_sun.py)"
NOAA_VER = "NOAA GML 2023"
SPEC = "add-practitioner-essentials scenarios"
R = math.pi / 180


def vec(i, inp, exp, src, ver, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


def noaa(lat, lon, dt):
    """NOAA spreadsheet equations: (apparent elevation, azimuth, declination)."""
    jd = 2440587.5 + dt.timestamp() / 86400
    t = (jd - 2451545) / 36525
    l0 = (280.46646 + t * (36000.76983 + 0.0003032 * t)) % 360
    m = 357.52911 + t * (35999.05029 - 0.0001537 * t)
    e = 0.016708634 - t * (0.000042037 + 0.0000001267 * t)
    c = math.sin(m * R) * (1.914602 - t * (0.004817 + 0.000014 * t)) + math.sin(2 * m * R) * (0.019993 - 0.000101 * t) + math.sin(3 * m * R) * 0.000289
    om = 125.04 - 1934.136 * t
    lam = l0 + c - 0.00569 - 0.00478 * math.sin(om * R)
    eps = 23 + (26 + (21.448 - t * (46.815 + t * (0.00059 - t * 0.001813))) / 60) / 60 + 0.00256 * math.cos(om * R)
    dec = math.asin(math.sin(eps * R) * math.sin(lam * R)) / R
    y = math.tan(eps * R / 2) ** 2
    eot = 4 / R * (y * math.sin(2 * l0 * R) - 2 * e * math.sin(m * R) + 4 * e * y * math.sin(m * R) * math.cos(2 * l0 * R)
                   - 0.5 * y * y * math.sin(4 * l0 * R) - 1.25 * e * e * math.sin(2 * m * R))
    minutes = ((jd + 0.5) % 1) * 1440
    ha = ((minutes + eot + 4 * lon) % 1440) / 4 - 180
    cz = math.sin(lat * R) * math.sin(dec * R) + math.cos(lat * R) * math.cos(dec * R) * math.cos(ha * R)
    el = 90 - math.acos(max(-1, min(1, cz))) / R
    az = (math.atan2(math.sin(ha * R), math.cos(ha * R) * math.sin(lat * R) - math.tan(dec * R) * math.cos(lat * R)) / R + 180) % 360
    if el > 85:
        ref = 0
    elif el > 5:
        tn = math.tan(el * R)
        ref = 58.1 / tn - 0.07 / tn ** 3 + 0.000086 / tn ** 5
    elif el > -0.575:
        ref = 1735 + el * (-518.2 + el * (103.4 + el * (-12.79 + el * 0.711)))
    else:
        ref = -20.772 / math.tan(el * R)
    return el + ref / 3600, az, dec


def position():
    """SPA vectors from pvlib's port of NREL SPA (run with a Python that has
    pvlib, e.g. a scratch virtualenv), plus the NREL published example."""
    import random
    import pandas as pd
    from pvlib.solarposition import spa_python
    rng = random.Random(20260918)
    out = []
    nrel = {"lat": 39.742476, "lon": -105.1786, "time": "2003-10-17T12:30:30-07:00", "height": "1830.14 m", "pressure": "820 hPa",
            "temperature": "11 degC", "delta_t": "67 s"}
    out.append(vec(1, nrel, {"result.zenith.value": 50.11162, "result.azimuth.value": 194.34024, "result.equation_of_time.value": 14.641503},
                   "NREL/TP-560-34302 Table A5.1 (published to 1e-5)", "2008", tol=1e-5))
    for i in range(2, 27):
        la, lo = rng.uniform(-66, 66), rng.uniform(-180, 180)
        stamp = f"{rng.randint(1950, 2080)}-{rng.randint(1, 12):02d}-{rng.randint(1, 28):02d}T{rng.randint(0, 23):02d}:{rng.randint(0, 59):02d}:{rng.randint(0, 59):02d}Z"
        h, pr, tc, dt = round(rng.uniform(0, 3000), 1), round(rng.uniform(700, 1030), 1), round(rng.uniform(-20, 35), 1), round(rng.uniform(30, 90), 2)
        inp = {"lat": round(la, 6), "lon": round(lo, 6), "time": stamp, "height": f"{h} m", "pressure": f"{pr} hPa", "temperature": f"{tc} degC", "delta_t": f"{dt} s"}
        r = spa_python(pd.DatetimeIndex([stamp]), inp["lat"], inp["lon"], altitude=h, pressure=pr * 100, temperature=tc, delta_t=dt, atmos_refract=0.5667, how="numpy").iloc[0]
        out.append(vec(i, inp, {"result.zenith.value": float(r["apparent_zenith"]), "result.azimuth.value": float(r["azimuth"]),
                                "result.elevation_true.value": float(r["elevation"]), "result.equation_of_time.value": float(r["equation_of_time"])},
                       "pvlib.solarposition.spa_python (NREL SPA port)", "pvlib 0.13.0", tol=1e-8))
    out.append(vec(27, {"lat": 39.7392, "lon": -104.9903, "time": "2026-06-21T12:00"}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    out.append(vec(28, {"lat": 39.7392, "lon": -104.9903, "time": "2026-06-21T12:00Z", "precision": "survey"}, {"meta.warnings.1.code": "UT1_APPROXIMATED"}, SPEC, "2026"))
    return out


def ev(d, hm, ud, uhm):
    return f"{d} {hm} local ({ud} {uhm}Z)"


def events():
    den = {"lat": 39.7392, "lon": -104.9903, "date": "2026-06-21", "offset": "-06:00"}
    pit = {"lat": 40.4406, "lon": -79.9959, "date": "2026-09-18", "offset": "-04:00"}
    syd = {"lat": -33.8688, "lon": 151.2093, "date": "2026-12-21", "offset": "+11:00"}
    return [
        vec(1, den, {"result.civil_dawn": ev("2026-06-21", "05:00", "2026-06-21", "1100"), "result.sunrise": ev("2026-06-21", "05:32", "2026-06-21", "1132"),
                     "result.solar_noon": ev("2026-06-21", "13:02", "2026-06-21", "1902"), "result.sunset": ev("2026-06-21", "20:31", "2026-06-22", "0231"),
                     "result.civil_dusk": ev("2026-06-21", "21:04", "2026-06-22", "0304")}, USNO, USNO_VER),
        vec(2, pit, {"result.civil_dawn": ev("2026-09-18", "06:36", "2026-09-18", "1036"), "result.sunrise": ev("2026-09-18", "07:04", "2026-09-18", "1104"),
                     "result.solar_noon": ev("2026-09-18", "13:14", "2026-09-18", "1714"), "result.sunset": ev("2026-09-18", "19:24", "2026-09-18", "2324"),
                     "result.civil_dusk": ev("2026-09-18", "19:51", "2026-09-18", "2351")}, USNO, USNO_VER),
        vec(3, syd, {"result.sunrise": ev("2026-12-21", "05:41", "2026-12-20", "1841"), "result.solar_noon": ev("2026-12-21", "12:53", "2026-12-21", "0153"),
                     "result.sunset": ev("2026-12-21", "20:05", "2026-12-21", "0905")}, USNO, USNO_VER),
        vec(4, {"lat": 71.29, "lon": -156.79, "date": "2026-12-21", "offset": "-09:00"}, {"result.state": "polar-night", "result.sunrise": "polar-night", "result.day_minutes": 0.0}, USNO, USNO_VER),
        vec(5, {"lat": 69.65, "lon": 18.96, "date": "2026-06-21", "offset": "+02:00"},
            {"result.state": "polar-day", "result.civil_dusk": "no-civil-twilight-end", "result.solar_noon": ev("2026-06-21", "12:46", "2026-06-21", "1046")}, USNO, USNO_VER),
        vec(6, {"lat": 51.5074, "lon": -0.1278, "date": "2026-01-15", "offset": "+00:00"},
            {"result.solar_noon": ev("2026-01-15", "12:10", "2026-01-15", "1210"), "result.sunset": ev("2026-01-15", "16:21", "2026-01-15", "1621"),
             "result.civil_dusk": ev("2026-01-15", "16:59", "2026-01-15", "1659")}, USNO, USNO_VER),
        vec(7, dict(den, offset="America/Denver"), {"result.sunset": ev("2026-06-21", "20:31", "2026-06-22", "0231")}, USNO, USNO_VER),
        vec(8, dict(den, offset="Mars/Olympus"), {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"),
    ]


def nights():
    den = {"lat": 39.7392, "lon": -104.9903, "date": "2026-06-21", "offset": "-06:00"}
    pit = {"lat": 40.4406, "lon": -79.9959, "date": "2026-09-18", "offset": "-04:00"}
    lon_ = {"lat": 51.5074, "lon": -0.1278, "date": "2026-01-15", "offset": "+00:00"}
    src = "USNO sunset and civil twilight (retrieved 2026-09-18) with the 14 CFR 61.57(b) and 1.1 definitions"
    return [
        vec(1, dict(den, landing_time="21:20"), {"result.currency_from": "21:31", "result.landing_logs_night": "yes", "result.landing_counts_currency": "no"}, SPEC, "2026"),
        vec(2, dict(den, landing_time="21:40"), {"result.landing_logs_night": "yes", "result.landing_counts_currency": "yes"}, src, USNO_VER),
        vec(3, dict(pit, landing_time="19:45"), {"result.currency_from": "20:24", "result.landing_logs_night": "no", "result.landing_counts_currency": "no"}, src, USNO_VER),
        vec(4, dict(lon_, landing_time="17:30"), {"result.currency_from": "17:21", "result.landing_counts_currency": "yes"}, src, USNO_VER),
        vec(5, dict(den), {"result.part107_evening": ev("2026-06-21", "20:31", "2026-06-22", "0231") + " to " + ev("2026-06-21", "21:01", "2026-06-22", "0301")}, src, USNO_VER),
        vec(6, {"lat": 71.29, "lon": -156.79, "date": "2026-12-21", "offset": "-09:00", "landing_time": "18:00"}, {"result.landing_counts_currency": "yes", "result.landing_logs_night": "yes"}, SPEC, "2026"),
    ]


def mapping():
    out = []
    sites = [(39.7392, -104.9903, "2026-06-21", "-06:00", "2026-06-21T19:02:00+00:00"), (40.4406, -79.9959, "2026-09-18", "-04:00", "2026-09-18T17:14:00+00:00"),
             (51.5074, -0.1278, "2026-01-15", "+00:00", "2026-01-15T12:10:00+00:00")]
    for i, (la, lo, d, off, noon) in enumerate(sites, 1):
        el, _, _ = noaa(la, lo, datetime.fromisoformat(noon))
        # Near the transit the elevation changes by < 0.01° per minute.
        out.append(vec(i, {"lat": la, "lon": lo, "date": d, "offset": off}, {"result.max_elevation.value": el}, NOAA, NOAA_VER, tol=0.01))
    out.append(vec(4, {"lat": 51.5074, "lon": -0.1278, "date": "2026-01-15", "offset": "+00:00", "threshold": 30}, {"result.window": "at no time that day"}, NOAA, NOAA_VER))
    out.append(vec(5, {"lat": 69.65, "lon": 18.96, "date": "2026-06-21", "offset": "+02:00", "threshold": 2}, {"result.window": "all day"}, NOAA, NOAA_VER))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    files = {"time.sun.position": position(), "time.sun.events": events(), "time.sun.aviation-nights": nights(), "time.sun.mapping-window": mapping()}
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
