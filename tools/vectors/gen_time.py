#!/usr/bin/env python3
"""Golden vectors for time.scale.* computed with Python's datetime (proleptic
Gregorian, no host time zones) and the TAI - UTC table from IERS Bulletin C 72,
plus the add-practitioner-essentials spec scenarios."""
import json
import sys
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

SRC = "Python datetime and the IERS Bulletin C 72 leap-second table (tools/vectors/gen_time.py)"
VER = "Bulletin C 72 (2026-07-06)"
SPEC = "add-practitioner-essentials scenarios"
LEAPS = [(date(1980, 1, 1), 19), (date(1981, 7, 1), 20), (date(1982, 7, 1), 21), (date(1983, 7, 1), 22), (date(1985, 7, 1), 23),
         (date(1988, 1, 1), 24), (date(1990, 1, 1), 25), (date(1991, 1, 1), 26), (date(1992, 7, 1), 27), (date(1993, 7, 1), 28),
         (date(1994, 7, 1), 29), (date(1996, 1, 1), 30), (date(1997, 7, 1), 31), (date(1999, 1, 1), 32), (date(2006, 1, 1), 33),
         (date(2009, 1, 1), 34), (date(2012, 7, 1), 35), (date(2015, 7, 1), 36), (date(2017, 1, 1), 37)]
GPS_EPOCH = datetime(1980, 1, 6, tzinfo=timezone.utc)


def vec(i, inp, exp, src=SRC, ver=VER):
    e = dict(exp)
    e.setdefault("ok", True)
    tol = {k: {"rel": 1e-15, "abs": 1e-6} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


def tai_utc(d):
    return [o for (start, o) in LEAPS if d >= start][-1]


def gps(dt):
    off = tai_utc(dt.date()) - 19
    g = (dt - GPS_EPOCH).total_seconds() + off
    week = int(g // 604800)
    return g, week, g - week * 604800, off


def gps_week():
    stamps = ["2026-09-18T00:00:00Z", "1999-08-21T23:59:47Z", "2019-04-06T23:59:42Z", "2006-01-01T00:00:00Z", "2012-06-30T12:34:56.500Z"]
    out = []
    for i, s in enumerate(stamps, 1):
        dt = datetime.fromisoformat(s.replace("Z", "+00:00"))
        g, w, sow, off = gps(dt)
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        out.append(vec(i, {"utc": s}, {"result.gps_week": float(w), "result.seconds_of_week": sow, "result.week_10bit": float(w % 1024),
                                       "result.rollover_era": float(w // 1024), "result.gps_minus_utc.value": float(off)}, src, ver))
    out.append(vec(6, {"utc": "2026-09-17T19:00:00-05:00"}, {"result.gps_week": 2436.0, "result.seconds_of_week": 432018.0}, SPEC, "2026"))
    out.append(vec(7, {"utc": "2027-09-18T00:00:00Z"}, {"meta.warnings.1.code": "LEAP_SECOND_TABLE_EXPIRED"}, SPEC, "2026"))
    out.append(vec(8, {"utc": "1979-12-31T00:00:00Z"}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}, SPEC, "2026"))
    return out


def gps_to_utc():
    cases = [(2436, 432018, None, "2026-09-18T00:00:00Z"), (388, 432018, 2, "2026-09-18T00:00:00Z"), (1930, 18, None, "2017-01-01T00:00:00Z"),
             (1930, 17, None, "2016-12-31T23:59:60Z"), (1000, 86400, 0, None)]
    out = []
    for i, (w, sow, era, want) in enumerate(cases, 1):
        full = w + 1024 * (era or 0)
        if want is None:
            g = full * 604800 + sow
            guess = GPS_EPOCH + timedelta(seconds=g)
            dt = guess - timedelta(seconds=tai_utc(guess.date()) - 19)
            want = dt.strftime("%Y-%m-%dT%H:%M:%SZ")
        inp = {"week": w, "seconds_of_week": sow}
        if era is not None:
            inp["era"] = era
        out.append(vec(i, inp, {"result.utc": want, "result.gps_week": float(full)}, SPEC if i <= 2 else SRC, "2026" if i <= 2 else VER))
    out.append(vec(6, {"week": 388, "seconds_of_week": 0}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    return out


def julian():
    out = []
    for i, s in enumerate(["2026-09-18T00:00:00Z", "2000-01-01T12:00:00Z", "2024-12-31T18:00:00Z", "1858-11-17T00:00:00Z", "2026-03-01T06:00:00Z"], 1):
        dt = datetime.fromisoformat(s.replace("Z", "+00:00"))
        jd = 2440587.5 + (dt - datetime(1970, 1, 1, tzinfo=timezone.utc)).total_seconds() / 86400
        doy = dt.timetuple().tm_yday
        out.append(vec(i, {"utc": s}, {"result.jd": jd, "result.mjd": jd - 2400000.5, "result.day_of_year": float(doy),
                                       "result.rinex_name": f"ssss{doy:03d}0.{dt.year % 100:02d}o"},
                       SPEC if i == 1 else "USNO Julian date definition evaluated with Python datetime (tools/vectors/gen_time.py)", "2026" if i == 1 else "2024"))
    out.append(vec(6, {"jd": 2451545.0}, {"result.utc": "2000-01-01T12:00:00Z"}, "USNO Julian date definition", "2024"))
    out.append(vec(7, {"mjd": 61301}, {"result.utc": "2026-09-18T00:00:00Z", "result.day_of_year": 261.0}, SPEC, "2026"))
    return out


def decimal_hours():
    cases = [("1.3", 78, "1 h 18 min"), ("0.1", 6, "6 min"), ("2", 120, "2 h"), ("1:18", 78, "1 h 18 min"), ("12.75", 765, "12 h 45 min")]
    return [vec(i, {"time": t}, {"result.minutes": float(m), "result.duration": d, "result.hm": f"{m // 60}:{m % 60:02d}", "result.hours": m / 60},
                SPEC if i == 1 else "Hours-to-minutes arithmetic (tools/vectors/gen_time.py)", "2026")
            for i, (t, m, d) in enumerate(cases, 1)]


def block_time():
    cases = [("2215Z", "0140Z"), ("0800", "1130"), ("23:50", "00:10"), ("2026-09-18T22:15-05:00", "2026-09-19T01:40-05:00"), ("2026-12-31T20:00Z", "2027-01-01T09:05Z")]
    out = []
    for i, (o, n) in enumerate(cases, 1):
        if "T" in o:
            m = int((datetime.fromisoformat(n.replace("Z", "+00:00")) - datetime.fromisoformat(o.replace("Z", "+00:00"))).total_seconds() // 60)
        else:
            c = lambda s: int(s.replace(":", "").rstrip("Z")[:2]) * 60 + int(s.replace(":", "").rstrip("Z")[2:])
            m = (c(n) - c(o)) % 1440
        out.append(vec(i, {"out_time": o, "in_time": n}, {"result.minutes": float(m), "result.hm": f"{m // 60}:{m % 60:02d}"}, "Clock arithmetic with Python datetime (tools/vectors/gen_time.py)", "2026"))
    out.append(vec(6, {"out_time": "2215", "in_time": "2026-09-19T01:40Z"}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    return out


def utc_offset():
    cases = [("2026-07-01T14:05", "-05:00", "local-to-utc"), ("2026-07-01T20:30", "-06:00", "local-to-utc"), ("2026-01-15T03:00", "+05:30", "local-to-utc"),
             ("2026-07-01T01:30", "-06:00", "utc-to-local"), ("2026-12-31T23:30", "+13:45", "utc-to-local")]
    out = []
    for i, (t, off, d) in enumerate(cases, 1):
        sign = -1 if off[0] == "-" else 1
        tz = timezone(sign * timedelta(hours=int(off[1:3]), minutes=int(off[4:6])))
        naive = datetime.fromisoformat(t)
        if d == "local-to-utc":
            u = naive.replace(tzinfo=tz).astimezone(timezone.utc)
            local = naive.replace(tzinfo=tz)
        else:
            u = naive.replace(tzinfo=timezone.utc)
            local = u.astimezone(tz)
        out.append(vec(i, {"time": t, "offset": off, "direction": d},
                       {"result.zulu": u.strftime("%H%MZ"), "result.utc": u.strftime("%Y-%m-%dT%H:%M:%SZ"),
                        "result.local": local.strftime("%Y-%m-%dT%H:%M:%S") + off, "result.day_shift": float((u.date() - local.date()).days)},
                       SPEC if i == 1 else "Python datetime with fixed offsets (tools/vectors/gen_time.py)", "2026"))
    out.append(vec(6, {"time": "2026-07-01T14:05", "offset": "America/Chicago"}, {"ok": False, "error.code": "UNSUPPORTED"}, SPEC, "2026"))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    files = {"time.scale.gps-week": gps_week(), "time.scale.gps-to-utc": gps_to_utc(), "time.scale.julian-date": julian(),
             "time.scale.decimal-hours": decimal_hours(), "time.scale.block-time": block_time(), "time.scale.utc-offset": utc_offset()}
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
