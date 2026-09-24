#!/usr/bin/env python3
"""US 1976 Standard Atmosphere at every tabulated kilometer from 0 to 81 km
(geometric), from the ambiance package, an independent implementation of the
standard (add-aviation-suite task 1.2, "Table agreement"). ambiance stops at
81.02 km, so the rows from 82 to 86 km, where the molecular-weight correction
applies, are not covered here.

Writes core/crates/gp-aviation/tests/data/us76_ambiance.csv: altitude (km),
temperature (K), pressure (Pa), density (kg/m3). Requires ambiance."""
from importlib.metadata import version
from pathlib import Path

from ambiance import Atmosphere

OUT = Path("core/crates/gp-aviation/tests/data/us76_ambiance.csv")


def main():
    rows = []
    for z in range(0, 82):
        a = Atmosphere(z * 1000.0)
        rows.append(f"{z},{a.temperature[0]!r},{a.pressure[0]!r},{a.density[0]!r}")
    OUT.write_text(f"# km,T_K,p_Pa,rho_kg_m3 (ambiance {version('ambiance')}, US 1976 Standard Atmosphere, geometric altitude)\n" + "\n".join(rows) + "\n")
    print(len(rows))


if __name__ == "__main__":
    main()
