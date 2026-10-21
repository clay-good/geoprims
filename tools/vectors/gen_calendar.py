#!/usr/bin/env python3
"""Golden vectors for time.scale.unix-time and time.scale.iso-week
(add-celestial-and-moon, time/sidereal-and-calendar), from Python's
datetime, an independent implementation of POSIX time
(datetime.fromtimestamp with UTC) and of ISO 8601 week dates
(date.isocalendar and date.fromisocalendar).

    gen_calendar.py unix    # print the Unix time vectors
    gen_calendar.py week    # print the ISO week vectors

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import datetime as dt
import json
import sys

UTC = dt.timezone.utc
SRC_U = "Python datetime (POSIX time, UTC), worked independently (tools/vectors/gen_calendar.py)"
SRC_W = "Python datetime.date.isocalendar (ISO 8601 week dates), worked independently (tools/vectors/gen_calendar.py)"
WEEKDAYS = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]


def iso(t):
    return t.strftime("%Y-%m-%dT%H:%M:%S") + "Z"


def unix():
    out = []
    # Spec scenarios first.
    out.append(({"unix": "1760832000000"}, {"result.utc": "2025-10-19T00:00:00Z", "result.unix": 1760832000.0, "meta.warnings.*.code": "INPUT_NORMALIZED"}))
    out.append(({"unix": "0"}, {"result.utc": "1970-01-01T00:00:00Z", "result.unix": 0.0, "result.jd": 2440587.5}))
    for s in (1, 86399, 951782400, 1234567890, 1483228799, 1483228800, 2000000000, 2147483647, 4102444800, -86400, -1):
        t = dt.datetime.fromtimestamp(s, UTC)
        out.append(({"unix": str(s)}, {"result.utc": iso(t), "result.unix": float(s), "result.jd": 2440587.5 + s / 86400}))
    for text in ("2000-02-29T12:00:00Z", "1999-12-31T23:59:59Z", "2038-01-19T03:14:08Z", "2026-10-21T06:30:00-05:00"):
        t = dt.datetime.fromisoformat(text.replace("Z", "+00:00"))
        out.append(({"utc": text}, {"result.unix": float(int(t.timestamp()))}))
    out.append(({"unix": "1760832000123", "unit": "ms"}, {"result.utc": "2025-10-19T00:00:00.123Z"}))
    out.append(({"unix": "1760832000000000000"}, {"result.utc": "2025-10-19T00:00:00Z"}))
    rows = [{"input": i, "expect": {**e, "ok": True}, "source": SRC_U, "sourceVersion": "POSIX.1-2024",
             "tolerance": {k: {"abs": 1e-6} for k, v in e.items() if isinstance(v, float)}} for i, e in out]
    rows.append({"input": {"utc": "2016-12-31T23:59:60Z"}, "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/utc"},
                 "source": "POSIX.1-2024 §4.19: a leap second has no Unix time", "sourceVersion": "POSIX.1-2024", "tolerance": {}})
    return rows


def week():
    rows = []
    for text in ("2027-01-01", "2026-01-01", "2026-10-21", "2020-12-31", "2021-01-03", "2021-01-04", "2015-12-31", "2016-01-03",
                 "2008-12-29", "2009-12-31", "2010-01-03", "1999-01-01", "2000-01-01", "2004-12-31", "2005-01-02", "2032-12-31", "2037-12-31"):
        d = dt.date.fromisoformat(text)
        y, w, k = d.isocalendar()
        mon = d - dt.timedelta(days=k - 1)
        rows.append({"input": {"date": text}, "expect": {"ok": True, "result.week_date": f"{y:04d}-W{w:02d}-{k}", "result.week_year": float(y),
                     "result.week": float(w), "result.weekday": WEEKDAYS[k - 1], "result.monday": mon.isoformat(),
                     "result.sunday": (mon + dt.timedelta(days=6)).isoformat()},
                     "source": SRC_W, "sourceVersion": "ISO 8601-1:2019", "tolerance": {"result.week_year": {"abs": 0}, "result.week": {"abs": 0}}})
    for y, w, k in ((2026, 53, 5), (2009, 53, 7), (2026, 1, 1), (2004, 53, 6)):
        d = dt.date.fromisocalendar(y, w, k)
        rows.append({"input": {"week_date": f"{y}-W{w:02d}-{k}"}, "expect": {"ok": True, "result.date": d.isoformat()},
                     "source": SRC_W, "sourceVersion": "ISO 8601-1:2019", "tolerance": {}})
    rows.append({"input": {"week_date": "2025-W53-1"}, "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/week_date"},
                 "source": "ISO 8601-1:2019: 2025 has 52 weeks", "sourceVersion": "ISO 8601-1:2019", "tolerance": {}})
    return rows


rows = unix() if sys.argv[1] == "unix" else week()
for i, r in enumerate(rows, 1):
    print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))
