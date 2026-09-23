#!/usr/bin/env python3
"""Coordinate-parser fixture (standard library only):

  python3 tools/vectors/gen_parse_diff.py

Writes core/crates/gp-geodesy/tests/data/parse_diff.jsonl: seeded points
written in the notations people paste (decimal with signs or hemisphere
letters, DMS with symbols, primes, spaces, or hyphens, degrees and decimal
minutes, packed aviation, and labeled pairs), each with the exact decimal
value it encodes. The strings are built here from integer degrees,
minutes, and seconds, independently of the parser.
"""
import json
import random
from pathlib import Path

N = 1500


def split(v, sec_places):
    a = abs(v)
    d = int(a)
    m = int((a - d) * 60)
    s = round(((a - d) * 60 - m) * 60, sec_places)
    if s >= 60:
        s, m = 0.0, m + 1
    if m >= 60:
        m, d = 0, d + 1
    return d, m, s


def value(d, m, s, neg):
    x = d + m / 60 + s / 3600
    return -x if neg else x


def main():
    rnd = random.Random(44)
    rows = []
    for k in range(N):
        la, lo = rnd.uniform(-89.9, 89.9), rnd.uniform(-179.9, 179.9)
        style = k % 10
        if style == 0:  # signed decimal
            text, lat, lon = f"{la:.6f}, {lo:.6f}", round(la, 6), round(lo, 6)
        elif style == 1:  # decimal with hemisphere letters
            text = f"{abs(la):.5f}{'N' if la >= 0 else 'S'} {abs(lo):.5f}{'E' if lo >= 0 else 'W'}"
            lat, lon = round(abs(la), 5) * (1 if la >= 0 else -1), round(abs(lo), 5) * (1 if lo >= 0 else -1)
        elif style in (2, 3, 4, 5):  # DMS in four spellings
            (d1, m1, s1), (d2, m2, s2) = split(la, 1), split(lo, 1)
            h1, h2 = ("N" if la >= 0 else "S"), ("E" if lo >= 0 else "W")
            text = [
                f"{d1}°{m1:02d}'{s1:04.1f}\"{h1} {d2}°{m2:02d}'{s2:04.1f}\"{h2}",
                f"{d1}°{m1:02d}′{s1:04.1f}″{h1} {d2:03d}°{m2:02d}′{s2:04.1f}″{h2}",
                f"{d1} {m1} {s1} {h1} {d2} {m2} {s2} {h2}",
                f"{d1}-{m1:02d}-{s1:04.1f}{h1} {d2}-{m2:02d}-{s2:04.1f}{h2}",
            ][style - 2]
            lat, lon = value(d1, m1, s1, la < 0), value(d2, m2, s2, lo < 0)
        elif style == 6:  # degrees and decimal minutes
            def ddm(v):
                a = abs(v)
                d = int(a)
                return d, round((a - d) * 60, 3)
            (d1, m1), (d2, m2) = ddm(la), ddm(lo)
            if m1 >= 60 or m2 >= 60:
                continue
            text = f"{'N' if la >= 0 else 'S'}{d1}°{m1:06.3f}' {'E' if lo >= 0 else 'W'}{d2:03d}°{m2:06.3f}'"
            lat, lon = (d1 + m1 / 60) * (1 if la >= 0 else -1), (d2 + m2 / 60) * (1 if lo >= 0 else -1)
        elif style == 7:  # packed aviation ddmmssH dddmmssH
            (d1, m1, s1), (d2, m2, s2) = split(la, 0), split(lo, 0)
            s1, s2 = int(s1), int(s2)
            text = f"{d1:02d}{m1:02d}{s1:02d}{'N' if la >= 0 else 'S'}{d2:03d}{m2:02d}{s2:02d}{'E' if lo >= 0 else 'W'}"
            lat, lon = value(d1, m1, s1, la < 0), value(d2, m2, s2, lo < 0)
        elif style == 8:  # labeled, longitude first
            text, lat, lon = f"lon: {lo:.4f}, lat: {la:.4f}", round(la, 4), round(lo, 4)
        else:  # signed DMS
            (d1, m1, s1), (d2, m2, s2) = split(la, 2), split(lo, 2)
            text = f"{'-' if la < 0 else ''}{d1}°{m1}'{s1}\" {'-' if lo < 0 else ''}{d2}°{m2}'{s2}\""
            lat, lon = value(d1, m1, s1, la < 0), value(d2, m2, s2, lo < 0)
        rows.append({"text": text, "lat": lat, "lon": lon})
    out = Path("core/crates/gp-geodesy/tests/data/parse_diff.jsonl")
    out.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows))
    print(len(rows), "rows")
    bad = rejects(random.Random(45), 504)
    out = Path("core/crates/gp-geodesy/tests/data/parse_reject.jsonl")
    out.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in bad))
    print(len(bad), "rejections")


def rejects(rnd, n):
    """Strings that must not parse, built from the same parts as the good ones
    so the only thing wrong with each is the one thing named."""
    rows = []
    for k in range(n):
        la, lo = rnd.uniform(-89.9, 89.9), rnd.uniform(-179.9, 179.9)
        (d1, m1, s1), (d2, m2, s2) = split(la, 1), split(lo, 1)
        h1, h2 = ("N" if la >= 0 else "S"), ("E" if lo >= 0 else "W")
        tail = f"{d2}\u00b0{m2:02d}'{s2:04.1f}\""
        kind = k % 8
        if kind == 0:
            text, why = f"{d1}\u00b0{m1:02d}'60.0\" {tail}", "seconds must be less than 60"
        elif kind == 1:
            text, why = f"{d1}\u00b060'{s1:04.1f}\" {tail}", "minutes must be less than 60"
        elif kind == 2:
            text, why = f"{d1}\u00b0-{m1:02d}'{s1:04.1f}\" {tail}", "cannot be negative"
        elif kind == 3:
            text, why = f"{d1}:{m1:02d}:-{s1:04.1f} {d2}:{m2:02d}:{s2:04.1f}", "cannot be negative"
        elif kind == 4:
            text, why = f"{la:.6f}, {lo:.6f}xyz", "unexpected character"
        elif kind == 5:
            text, why = f"{91 + abs(la) % 9:.4f}{h1} {abs(lo):.4f}{h2}", "latitude must be within 90"
        elif kind == 6:
            text, why = f"{h1}{abs(la):.4f}{h1} {abs(lo):.4f}{h2}", "two hemisphere letters"
        else:
            text, why = f"-{abs(la):.4f}{h1} {abs(lo):.4f}{h2}", "contradicts hemisphere"
        rows.append({"text": text, "why": why})
    return rows


if __name__ == "__main__":
    main()
