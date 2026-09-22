#!/usr/bin/env python3
"""Golden vectors for the night currency counter, worked by hand. Events sit
at least 20 minutes inside or outside the 14 CFR 61.57(b) period at Denver
(sunset near 19:53 and sunrise near 05:58 MDT in early May, from USNO), so
their yes/no does not hang on the solver's last minute. Currency runs through
the 90th day after the older of the third most recent qualifying takeoff and
landing."""
import datetime as dt
import json
import sys
from pathlib import Path

SRC = "Counting worked by hand (tools/vectors/gen_currency.py)"
SPEC = "add-practitioner-essentials currency scenario"


def vec(i, inp, exp, src=SRC):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": 0} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def ev(when, t=1, l=1, a="ASEL"):
    return {"when": when, "lat": 39.86, "lon": -104.67, "takeoffs": t, "landings": l, "aircraft": a}


def plus90(d):
    return (dt.date.fromisoformat(d) + dt.timedelta(days=90)).isoformat()


def cases():
    three = [ev("2026-05-01T22:30-06:00"), ev("2026-05-10T22:30-06:00"), ev("2026-06-02T23:00-06:00")]
    out = []
    out.append(vec(1, {"as_of": "2026-07-15", "events": three},
                   {"result.through": plus90("2026-05-01"), "result.current": "yes", "result.aircraft.0.takeoffs": 3, "result.aircraft.0.landings": 3,
                    "result.events.0.counts": "yes", "result.events.1.counts": "yes", "result.events.2.counts": "yes"},
                   SPEC + " (May 1, May 10, and June 2 give currency through July 30, 2026)"))
    out.append(vec(2, {"as_of": "2026-07-30", "events": three}, {"result.current": "yes", "result.through": "2026-07-30"}))
    out.append(vec(3, {"as_of": "2026-07-31", "events": three}, {"result.current": "no", "result.aircraft.0.landings": 2}))
    # Too early in the evening, a morning landing inside the period, and one after it ends.
    mixed = [ev("2026-05-01T20:30-06:00"), ev("2026-05-10T03:30-06:00", 1, 2), ev("2026-06-02T05:30-06:00"), ev("2026-06-03T23:00-06:00", 0, 1, "amel")]
    out.append(vec(4, {"as_of": "2026-07-15", "events": mixed},
                   {"result.through": "none", "result.current": "no", "result.events.0.counts": "no", "result.events.1.counts": "yes",
                    "result.events.2.counts": "no", "result.events.3.counts": "yes", "result.aircraft.0.takeoffs": 1, "result.aircraft.0.landings": 2,
                    "result.aircraft.1.aircraft": "AMEL", "result.aircraft.1.landings": 1}))
    # Several landings in one flight: the third most recent landing is the older one.
    batch = [ev("2026-05-01T22:30-06:00", 1, 1), ev("2026-05-20T22:30-06:00", 2, 3)]
    out.append(vec(5, {"as_of": "2026-06-01", "events": batch}, {"result.through": plus90("2026-05-01"), "result.current": "yes"}))
    out.append(vec(6, {"as_of": "2026-07-15", "events": [{**three[0], "when": "2026-05-01T22:30"}]}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "time.sun.night-currency.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in cases()))


if __name__ == "__main__":
    main()
