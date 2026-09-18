#!/usr/bin/env python3
"""Time-zone differential fixture from Python zoneinfo on the `tzdata`
package (same IANA release as core/crates/gp-time/data/tzdb.bin).

    PYTHONTZPATH= gen_tz_diff.py OUT.csv PER_ZONE SEED

Rows: zone,unix,utoff_seconds,abbr — instants uniform over 1970-2100."""
import random
import sys
from datetime import datetime, timezone
from zoneinfo import ZoneInfo, available_timezones

import tzdata

out, per, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
rng = random.Random(seed)
lo, hi = 0, int(datetime(2100, 1, 1, tzinfo=timezone.utc).timestamp())
with open(out, "w") as f:
    f.write(f"# zoneinfo on tzdata {tzdata.IANA_VERSION}; seed {seed}\n")
    for name in sorted(available_timezones()):
        if name in ("Factory", "localtime") or name.startswith("posix") or name.startswith("right"):
            continue
        z = ZoneInfo(name)
        for _ in range(per):
            t = rng.randint(lo, hi)
            d = datetime.fromtimestamp(t, z)
            f.write(f"{name},{t},{int(d.utcoffset().total_seconds())},{d.tzname()}\n")
