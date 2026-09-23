---
title: What is the Part 107 altitude limit?
description: Under 14 CFR 107.51(b), a drone may fly 400 ft above the ground, or higher near a structure. What the rule says, AGL vs MSL, and where waivers fit.
summary: The 400 ft rule, the structure exception, and how to turn "above ground" into a number your drone or map can use.
audience: Drone operators
published: 2026-09-23
tools:
  - drone.ops.part107-altitude
  - drone.sensors.vlos
  - geodesy.geoid.geoid-height
sources:
  - title: 14 CFR 107.51, Operating limitations for small unmanned aircraft
    issuer: Federal Aviation Administration, via the Electronic Code of Federal Regulations
    edition: Current text, checked September 23, 2026
    locator: Paragraph (b), altitude; (c) visibility; (d) cloud clearance
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.51
  - title: 14 CFR Part 107, Small Unmanned Aircraft Systems
    issuer: Federal Aviation Administration, via the Electronic Code of Federal Regulations
    edition: Current text, checked September 23, 2026
    locator: §107.41 (operation in certain airspace) and §107.205 (regulations subject to waiver)
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107
  - title: Low Altitude Authorization and Notification Capability (LAANC)
    issuer: Federal Aviation Administration
    edition: Web page, current
    locator: What is LAANC, and further coordination requests
    url: https://www.faa.gov/uas/getting_started/laanc
  - title: Part 107 Waivers
    issuer: Federal Aviation Administration
    edition: Web page, current
    locator: How to apply for a Part 107 waiver
    url: https://www.faa.gov/uas/commercial_operators/part_107_waivers
  - title: Normalizing Unmanned Aircraft Systems Beyond Visual Line of Sight Operations (Proposed rule)
    issuer: Federal Aviation Administration, Federal Register
    edition: Notice of proposed rulemaking, August 7, 2025
    locator: Proposed 14 CFR Part 108
    url: https://www.federalregister.gov/documents/2025/08/07/2025-14992/normalizing-unmanned-aircraft-systems-beyond-visual-line-of-sight-operations
---

Under Part 107, a small drone may fly no higher than 400 ft above ground level (AGL). The one exception is near a structure: within a 400 ft radius of it, you may fly up to 400 ft above the structure's top. The rule is 14 CFR 107.51(b), and it is measured from the ground below the drone, not from sea level and not from where you took off.

This article is a planning and education aid, not legal advice. Check the current regulation, your authorizations, and any local restrictions before you fly.

## What the rule says

Section 107.51(b) says the drone's altitude "cannot be higher than 400 feet above ground level," unless the drone is both:

1. flown within a 400-foot radius of a structure, and
2. no higher than 400 feet above the structure's "immediate uppermost limit," meaning its highest point, such as the top of an antenna.

Both conditions must hold at once. Step outside the 400 ft radius and the limit drops back to 400 ft above the ground.

The same section sets other limits that often come up in the same flight plan: a groundspeed of no more than 87 knots (100 mph), flight visibility of at least 3 statute miles from the control station, and at least 500 ft below and 2,000 ft horizontally from clouds.

## Why it matters

The 400 ft ceiling keeps small drones below most crewed aircraft outside airport areas. The structure exception lets inspectors look at towers, stacks, and tall buildings without a waiver. Getting the number wrong in either direction costs you: too high risks conflict with aircraft and enforcement, too low can leave the top of a tower out of your inspection.

## How it is worked out

The [Part 107 altitude tool](/drone/ops/part107-altitude/) applies the rule in three steps.

1. **Structure or open ground.** If you give a structure height and your horizontal distance from it, and you are within 400 ft, the limit is the structure height plus 400 ft. Otherwise it is 400 ft.
2. **Above sea level (MSL).** Add the ground elevation. Charts, obstacle data, and many flight planning tools work in MSL.
3. **Above the ellipsoid (HAE).** GNSS receivers work on the ellipsoid, and some drones log height that way. The height of the ellipsoid above or below mean sea level at your site is the geoid height, N. Ellipsoid height = MSL height + N. The [geoid height tool](/geodesy/geoid/geoid-height/) gives N from EGM96 for any point.

## A worked example

A tower 300 ft tall, with the drone 200 ft from it:

| Case | Maximum altitude |
|---|---|
| Within 400 ft of the 300 ft tower | **700 ft AGL** |
| The same, with the ground at 5,280 ft | 5,980 ft MSL |
| 500 ft from the tower | 400 ft AGL |

Open ground at 5,280 ft elevation near Denver, where the geoid height is about −17 m (the geoid tool gives −16.993 m):

| Height reference | Maximum altitude |
|---|---|
| Above ground (AGL) | 400 ft |
| Above mean sea level (MSL) | 5,680 ft |
| Above the ellipsoid (HAE) | 5,624 ft |

The HAE limit is lower than the MSL limit here because the geoid sits below the ellipsoid over most of the continental United States. A drone that logs ellipsoid height and a map that shows MSL disagree by more than 50 ft at this site.

## AGL, MSL, and height above takeoff

Most drone apps show **height above the takeoff point**, not height above the ground under the drone. The two match only over flat ground.

- Take off from a hilltop and fly out over a valley, and the ground falls away. The app may read 350 ft while the drone is well over 400 ft AGL.
- Take off from a valley floor and fly toward rising ground, and the drone can be much closer to the terrain than the app suggests.

Over sloping ground, work out the limit from the ground elevation under the drone's path, not from the launch point.

## Controlled airspace, LAANC, and waivers

The altitude limit is not the only ceiling. Under §107.41, flying in Class B, C, or D airspace, or in the surface area of Class E airspace designated for an airport, needs prior authorization from air traffic control. The FAA's LAANC system gives automated authorizations for controlled airspace at or below 400 ft, up to the ceiling shown on the FAA's UAS facility maps. To fly above a facility map ceiling, up to 400 ft, you submit a further coordination request.

To fly higher than 107.51(b) allows, you need a waiver. Section 107.205 lists 107.51 among the rules the FAA may waive, and the FAA's Part 107 waiver page explains how to apply.

A separate, **Proposed** rule for beyond-visual-line-of-sight operations (a new Part 108) was published for comment in August 2025. It is not in effect. Recreational flyers operate under a separate statutory exception, not Part 107, and this article does not cover their rules.

## Common mistakes

- **Reading the app's height as AGL.** It is usually height above takeoff.
- **Stretching the structure exception.** It applies only within 400 ft of the structure, and only up to 400 ft above its highest point.
- **Mixing MSL and HAE.** Know which one your drone logs and which one your map uses.
- **Forgetting the airspace.** A 400 ft limit does not mean 400 ft is available. A facility map ceiling may be lower.
- **Trusting a summary over the rule.** Rules change. The tool shows the date its summary was checked and links the current text.

## Where the numbers come from

The rule text is from the current eCFR, checked on September 23, 2026. The [Part 107 altitude tool](/drone/ops/part107-altitude/) shows its basis and the regulation link with every answer. The [geoid height tool](/geodesy/geoid/geoid-height/) supplies N for the ellipsoid limit. How far out you can fly is often set by eyesight rather than altitude. See [how far you can see a drone](/learn/visual-line-of-sight/) and the [visual line of sight tool](/drone/sensors/vlos/).
