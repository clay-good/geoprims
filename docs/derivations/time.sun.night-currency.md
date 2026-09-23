<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Night passenger currency (`time.sun.night-currency`)

## Method

14 CFR 61.57(b) says a pilot may not carry passengers at night unless, within the preceding 90 days, they have made three takeoffs and three full-stop landings — each in the same category and class — during the period from **one hour after sunset to one hour before sunrise**. Three questions follow, and the tool answers all three: which of your logged events count, whether you are current on a date, and the day currency lapses.

Which events count is a sun question. For each event the tool finds sunset at its place on the evening the event belongs to, and sunrise the next morning, with sunrise and sunset at the sun's centre −0.8333° — 34′ of refraction plus the 16′ semidiameter. An hour is added to the first and subtracted from the second. A time before local noon belongs to the night that began the evening before, which is how a 04:00 landing gets counted against the right sunset.

The counting is then bookkeeping, and the two parts of it are easy to get wrong in opposite directions. Takeoffs and landings are counted **separately** — three takeoffs and two landings is not currency. And they are counted **per category and class** — a multi-engine night does nothing for single-engine currency.

Currency runs through the 90th day after the older of the third most recent qualifying takeoff and the third most recent qualifying full-stop landing. The count reported alongside is of qualifying events in the 90 days before the date asked about, which is a different window and will differ from the events used for `through` near a lapse.

## Equations

- Night period for an event at place p on the night beginning evening D: [sunset(p, D) + 1 h, sunrise(p, D+1) − 1 h], sun centre at −0.8333°.
- An event before local noon belongs to evening D − 1.
- Per category and class: t₃ = date of the third most recent qualifying takeoff; l₃ likewise for full-stop landings.
- through = min(t₃, l₃) + 90 days; current on a date when date ≤ through.

## Symbols and units

`as_of` is the date to judge; each event carries `when` (with its offset), `lat`, `lon`, `takeoffs`, `landings` and `aircraft` (category and class). Out come `current`, `through`, `state`, the `rule` text, a per-aircraft breakdown, and each event marked `counts` yes or no.

## Domain

Any place and date. At high latitudes in summer the period can be empty — the sun never gets low enough for an hour after sunset to precede an hour before sunrise — and then nothing counts, which is the correct answer and not a failure.

## Approximations

Sun times are within about a minute of USNO. An event within a minute of a boundary is therefore worth checking against the Air Almanac, which is the legal source; the tool says so in its own limitations rather than in a footnote. This is a planning aid: the logbook and the regulation govern, including simulator credit, which is not counted here.

## Worked example

- sourcePublisher: Federal Aviation Administration; National Renewable Energy Laboratory; pvlib community
- sourceTitle: 14 CFR 61.57(b), Recent flight experience: pilot in command; NREL Solar Position Algorithm; pvlib-python
- sourceEdition: Current as of the review date; NREL/TP-560-34302 revised January 2008; pvlib 0.13.0
- sourceLocator: 61.57(b)(1) — three takeoffs and three full-stop landings within 90 days, one hour after sunset to one hour before sunrise, same category and class
- independent: yes
- inputs: 22 logbooks, including three qualifying nights checked inside and outside the 90 days, an event 20 minutes before the period opens, a 04:00 landing belonging to the previous evening, a 05:30 one that does not, three takeoffs against two landings, two categories on one logbook, and winter nights at Denver and Seattle
- outputs: the currency verdict, the through date, the per-aircraft counts, and each event's yes or no
- tolerance: exact
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

Sunset and sunrise come from pvlib's SPA by bisection, and 61.57(b) is then applied in the generator from the regulation's own words. Every event sits at least 20 minutes inside or outside its boundary, so no yes-or-no in these vectors hangs on the last minute of either solver.

Writing the reference got the bookkeeping wrong twice, and the tool was right both times: first by pooling all categories into one count, which made a multi-engine night extend single-engine currency, and then by counting qualifying events over all time rather than the 90 days the report covers. Both are exactly the mistakes the rule's two-part structure invites.

## Differential tests

- `tools/vectors/gen_sun_pvlib.py`: 16 of the 22 vectors, sun from pvlib and 61.57(b) applied independently
- `core/crates/gp-time/tests/usno_nights.rs`: the underlying night windows against USNO at 100 places
- `core/vectors/time.sun.night-currency.jsonl`: 22 vectors across two places, two seasons and two categories

## Invariants

- `core/crates/gp-time/tests/time.rs` `night_currency_invariants`: three qualifying nights give currency and two do not, which is the rule's threshold; takeoffs and landings are counted apart, so three takeoffs against two landings is not current; category and class are counted apart, so a multi-engine night leaves single-engine currency untouched; an event an hour and a half after sunset counts and one half an hour after sunset does not, pinning the one-hour margin rather than sunset itself; an event at 04:00 counts against the previous evening while the same logbook at 05:30 does not; currency holds on the through date and not the day after; and adding an event that does not qualify changes nothing at all
