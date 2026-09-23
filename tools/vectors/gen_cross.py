import json, math
from pathlib import Path
OUT = Path("/Users/user/Documents/development/public/geoprims/.claude/worktrees/geospatial-aviation-spec-b79149/core/vectors")
SRC = "Cell-size tables for each system worked independently in Python (tools/vectors/gen_cross.py)"
VER = "2026-09-22"
A, F = 6378137.0, 1/298.257223563

def degs(lat):
    e2 = F*(2-F); s = math.sin(math.radians(lat)); w = math.sqrt(1-e2*s*s)
    return (math.radians(A*(1-e2)/w**3), math.radians(A/w)*math.cos(math.radians(lat)))

def geohash_span(p):
    bits = 5*p; lon_bits = (bits+1)//2; lat_bits = bits//2
    return (180/2**lat_bits, 360/2**lon_bits)

def olc_span(n):
    if n <= 10:
        size = 20/20**(n//2-1); return (size, size)
    extra = n-10
    return ((20/20**4)/5**extra, (20/20**4)/4**extra)

def qth_span(chars):
    dlat, dlon, k = 10.0, 20.0, 2
    while k < chars:
        d = 10.0 if (k//2) % 2 == 1 else 24.0
        dlat /= d; dlon /= d; k += 2
    return (dlat, dlon)

# H3 average areas (km2), resolutions 0-15, from the H3 documentation table.
H3_AREA = [4357449.4161, 609788.4417, 86801.7803, 12393.4348, 1770.3235, 252.9033,
           36.1290, 5.1613, 0.7373, 0.1053, 0.0150, 0.0021, 0.0003, 0.0000436, 0.0000062, 0.0000009]

def pick(options, target):
    return min(options, key=lambda o: abs(math.log(o[1]/target)))

def sizes(lat, target):
    per_lat, per_lon = degs(lat)
    mean = lambda w, h: math.sqrt(w*h)
    gh = pick([(p, mean(geohash_span(p)[1]*per_lon, geohash_span(p)[0]*per_lat)) for p in range(1, 13)], target)
    olc = pick([(n, mean(olc_span(n)[1]*per_lon, olc_span(n)[0]*per_lat)) for n in (2,4,6,8,10,11)], target)
    qth = pick([(c, mean(qth_span(c)[1]*per_lon, qth_span(c)[0]*per_lat)) for c in (2,4,6,8,10)], target)
    tile = pick([(z, 40075016.685578488*math.cos(math.radians(lat))/2**z) for z in range(0, 23)], target)
    mg = pick([(d, 100000/10**d) for d in range(0, 6)], target)
    return gh, olc, qth, tile, mg

# (lat, lon, target cell size in metres)
CASES = [(40.6892, -74.0445, 150.0), (0.0, 0.0, 150.0),
         (60.0, 10.0, 1000.0), (-33.8688, 151.2093, 10.0),
         (51.5074, -0.1278, 25000.0)]


def build(cases, start=0):
    """Vectors for (lat, lon, target size) cases, numbered from `start`."""
    rows = []
    for i, (lat, lon, target) in enumerate(cases, start + 1):
        gh, olc, qth, tile, mg = sizes(lat, target)
        s2 = pick([(lvl, math.sqrt(4*math.pi/(6*4**lvl)*6371008.8**2)) for lvl in range(0, 31)], target)
        e = {"result.cells.1.resolution": f"precision {gh[0]}",
             "result.cells.2.resolution": f"{olc[0]} characters",
             "result.cells.3.resolution": f"zoom {tile[0]}",
             "result.cells.4.resolution": f"{qth[0]} characters",
             "result.cells.5.resolution": f"level {s2[0]}",
             "result.cells.6.resolution": f"{mg[0]} digits, {int(mg[1])} m squares",
             "result.cells.1.cell_size.value": gh[1],
             "result.cells.3.cell_size.value": tile[1],
             "result.cells.5.cell_size.value": s2[1],
             "ok": True}
        t = {k: {"rel": 1e-9, "abs": 1e-6} for k, v in e.items() if isinstance(v, float)}
        rows.append({"id": f"v{i:03d}", "input": {"lat": lat, "lon": lon, "target_size": f"{target:g} m"},
                     "expect": e, "source": SRC, "sourceVersion": VER, "tolerance": t})
    return rows


def main():
    rows = build(CASES, 0)
    p = OUT / "indexing.convert.cross-index.jsonl"
    p.write_text("".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
    print(p.name, len(rows))


if __name__ == "__main__":
    main()
