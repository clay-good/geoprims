#!/usr/bin/env python3
"""Regenerates core/crates/gp-astro/src/elp_tables.rs: the lunar theory
ELP 2000-82B (Chapront-Touze and Chapront 1983, 1988; constants fitted to
DE200/LE200) as terms A t^p sin(c0 + c1 t + c2 t^2 + c3 t^3 + c4 t^4), t in
Julian centuries of TDB from J2000.0, A in arcseconds (longitude, latitude)
or kilometers (distance). The arguments, the fitted corrections, and the
truncation rule (a term is kept when its tabulated amplitude is at least
PREC radians, or PREC times the mean distance) follow the authors'
subroutine ELP82B in elp82b.f. At PREC = 1e-7 the series keep 769 of
37,872 terms and hold the apparent Moon within 0.7 arcseconds of JPL
Horizons from 1950 to 2050.

Download ELP1 to ELP36 from VizieR (https://cdsarc.cds.unistra.fr/ftp/VI/79/)
into a directory, then:

    elp_tables.py DIR [OUT]

Standard library only."""
import math
import sys
from pathlib import Path

PREC = 1e-7
pi = math.pi
rad = 648000 / pi
deg = pi / 180
ath = 384747.9806743165
am = 0.074801329518
alfa = 0.002571881335
dtasm = 2 * alfa / (3 * am)


def dms(d, m, s):
    return (d + m / 60 + s / 3600) * deg


# Mean arguments (elp82b.f): constant, then rates for t to t^4 in arcseconds.
w = [[dms(218, 18, 59.95571), 1732559343.73604, -5.8883, 0.6604e-2, -0.3169e-4],
     [dms(83, 21, 11.67475), 14643420.2632, -38.2776, -0.45047e-1, 0.21301e-3],
     [dms(125, 2, 40.39816), -6967919.3622, 6.3622, 0.7625e-2, -0.3586e-4]]
eart = [dms(100, 27, 59.22059), 129597742.2758, -0.0202, 0.9e-5, 0.15e-6]
peri = [dms(102, 56, 14.42753), 1161.2283, 0.5327, -0.138e-3, 0.0]
for row in w + [eart, peri]:
    row[1:] = [x / rad for x in row[1:]]
preces = 5029.0966 / rad
planets = [(dms(252, 15, 3.25986), 538101628.68898), (dms(181, 58, 47.28305), 210664136.43355),
           (eart[0], eart[1] * rad), (dms(355, 25, 59.78866), 68905077.59284),
           (dms(34, 21, 5.34212), 10925660.42861), (dms(50, 4, 38.89694), 4399609.65932),
           (dms(314, 3, 18.01841), 1542481.19393), (dms(304, 20, 55.19575), 786550.32074)]
p = [[a, b / rad] for a, b in planets]
delnu = 0.55604 / rad / w[0][1]
dele = 0.01789 / rad
delg = -0.08066 / rad
delnp = -0.06424 / rad / w[0][1]
delep = -0.12879 / rad
# Delaunay arguments D, l', l, F.
de = [[w[0][i] - eart[i] for i in range(5)], [eart[i] - peri[i] for i in range(5)],
      [w[0][i] - w[1][i] for i in range(5)], [w[0][i] - w[2][i] for i in range(5)]]
de[0][0] += pi
zeta = [w[0][0], w[0][1] + preces]


def fields(line, ints, widths):
    v = [int(line[3 * i:3 * i + 3]) for i in range(ints)]
    pos = 3 * ints
    for wd in widths:
        v.append(float(line[pos:pos + wd]))
        pos += wd
    return v


src = Path(sys.argv[1])
dest = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parents[2] / "core" / "crates" / "gp-astro" / "src" / "elp_tables.rs"
series = [[[] for _ in range(3)] for _ in range(3)]  # [coordinate][power]
total = 0
for n in range(1, 37):
    iv = (n - 1) % 3
    limit = PREC * ath if iv == 2 else PREC * rad
    lines = [l for l in (src / f"ELP{n}").read_text().splitlines()[1:] if l.strip()]
    total += len(lines)
    for line in lines:
        c = [0.0] * 5
        if n <= 3:
            v = fields(line, 4, [15] + [12] * 6)
            ilu, coef = v[:4], v[4:]
            if abs(coef[0]) < limit:
                continue
            x = coef[0]
            if n == 3:
                x -= 2 * x * delnu / 3
            x += (coef[1] + dtasm * coef[5]) * (delnp - am * delnu) + coef[2] * delg + coef[3] * dele + coef[4] * delep
            for k in range(5):
                c[k] = sum(ilu[i] * de[i][k] for i in range(4))
            if iv == 2:
                c[0] += pi / 2
            power = 0
        elif n <= 9 or n >= 22:
            v = fields(line, 5, [10, 10, 10])
            iz, ilu, pha, x = v[0], v[1:5], v[5], v[6]
            if x < limit:
                continue
            for k in range(2):
                c[k] = iz * zeta[k] + sum(ilu[i] * de[i][k] for i in range(4))
            c[0] += pha * deg
            power = 1 if 7 <= n <= 9 or 25 <= n <= 27 else 2 if n >= 34 else 0
        else:
            v = fields(line, 11, [10, 10, 10])
            ipla, pha, x = v[:11], v[11], v[12]
            if x < limit:
                continue
            for k in range(2):
                if n < 16:
                    c[k] = ipla[8] * de[0][k] + ipla[9] * de[2][k] + ipla[10] * de[3][k] + sum(ipla[i] * p[i][k] for i in range(8))
                else:
                    c[k] = sum(ipla[i + 7] * de[i][k] for i in range(4)) + sum(ipla[i] * p[i][k] for i in range(7))
            c[0] += pha * deg
            power = 1 if 13 <= n <= 15 or 19 <= n <= 21 else 0
        c[0] = math.fmod(c[0], 2 * pi)
        series[iv][power].append([x] + c)
kept = sum(len(s) for row in series for s in row)
assert total == 37872, total
out = [
    "//! ELP 2000-82B lunar series (Chapront-Touze and Chapront, VizieR VI/79),",
    "//! fitted to DE200/LE200: terms A t^p sin(c0 + c1 t + c2 t^2 + c3 t^3 + c4 t^4)",
    "//! as [A, c0, c1, c2, c3, c4], t in Julian centuries of TDB from J2000.0, A in",
    f"//! arcseconds or kilometers, truncated at {PREC} rad as the authors' ELP82B",
    "//! subroutine does. Generated by tools/codegen/elp_tables.py. Do not edit by hand.",
    "",
    "#![allow(clippy::approx_constant, clippy::unreadable_literal, clippy::excessive_precision)]",
    "",
    "/// Longitude, latitude, and distance, each by power of t (0, 1, 2).",
    "pub const MOON: [[&[[f64; 6]]; 3]; 3] = [",
]
for iv in range(3):
    out.append("    [")
    for pw in range(3):
        out.append("        &[" + ", ".join("[" + ", ".join(repr(float(x)) for x in t) + "]" for t in series[iv][pw]) + "],")
    out.append("    ],")
out.append("];")
dest.write_text("\n".join(out) + "\n")
print(f"wrote {dest}: {kept} of {total} terms")
