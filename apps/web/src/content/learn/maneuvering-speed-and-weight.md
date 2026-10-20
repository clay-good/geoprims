---
title: Maneuvering speed and weight
description: Why the maneuvering speed in your flight manual is a number for maximum weight, why a lighter airplane needs a lower one, and how to work it out with the square root of the weight ratio.
summary: Why VA drops when the airplane is light, the square-root rule, and what VA does and does not protect.
audience: Pilots
published: 2026-10-20
tools:
  - aviation.performance.va-for-weight
  - aviation.performance.turn
  - aviation.loading.weight-balance
sources:
  - title: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
    issuer: Federal Aviation Administration
    edition: 2023
    locator: Chapter 5, Load Factors, design maneuvering speed (page 5-35)
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak
  - title: 14 CFR 23.335, Design airspeeds
    issuer: FAA (eCFR)
    edition: Text in force until August 30, 2017 (removed by the Part 23 rewrite)
    locator: Paragraph (c), design maneuvering speed, VA not less than VS √n
    url: https://www.ecfr.gov/api/versioner/v1/full/2017-01-01/title-14.xml?section=23.335
---

The maneuvering speed printed in most flight manuals is good for one weight: the maximum. Fly lighter and the safe number goes down, not up. That surprises a lot of pilots, because a lighter airplane feels like it should be the easier one on its wings.

The fix is one square root. Multiply the published speed by the square root of your weight over the maximum weight.

## What maneuvering speed protects

The Pilot's Handbook defines design maneuvering speed (VA) as the speed below which you can move one flight control, once, to its full deflection, in one axis, in smooth air, without risk of damage to the airplane.

Read that definition slowly. It is narrow on purpose. It does not cover full inputs in two axes at once. It does not cover a full input and then a full input the other way. The handbook says so plainly: flying at or below VA "does not provide structural protection" against those.

So VA is a limit, not a comfort zone. It is also the speed many pilots slow to in turbulence, which is where weight starts to matter.

## Why weight changes it

VA works because the wing stalls before it can break the airplane. Pull hard below VA and the wing runs out of lift (it stalls) before it makes enough load to bend anything. Above VA the wing can make more load than the structure was built to take.

How much load can a wing make before it stalls? That depends on how fast you are going compared with the stall speed. The handbook's example: an airplane that stalls at 60 knots, stalled at 102 knots, pulls a load factor of 1.7 squared, about 2.89 g. Load factor grows with the square of the speed ratio.

Now make the airplane lighter. A lighter airplane needs less lift to fly, so it stalls at a lower speed. Stall speed goes with the square root of weight. At the same airspeed, the light airplane is flying further above its own stall speed, so a sharp pull can make more g before the wing quits.

The structure did not get stronger when you burned fuel. Its limit load factor is the same number it always was. To keep the stall arriving before that limit, the speed has to come down with the stall speed. That is the old 14 CFR 23.335(c) relation: VA is not less than VS times the square root of the limit load factor, and VS moves with the square root of weight.

## How it is worked out

Put those together and the weight cancels into one ratio:

- **Weight ratio** = weight now / maximum weight
- **VA now** = published VA × √(weight ratio)

That's it. No load factor, no stall speed, just the number in the book and two weights. A quarter of the maximum weight gives half the speed, since the square root of 1/4 is 1/2.

Older airplanes may not print VA at all. For those, the handbook offers a rough guide: about 1.7 times the normal stall speed. The 1.7 comes from √2.89, the same square-root logic run backward. Treat it as a cross-check, not a substitute for the manual.

## A worked example

Take an airplane whose manual gives VA as 105 knots at its maximum weight of 2,550 lb. Today you are flying solo with partial fuel at 2,000 lb.

- Weight ratio: 2,000 / 2,550 = 0.784
- Square root: 0.886
- VA now: 105 × 0.886 = **about 93 knots**

That is 12 knots slower than the number on the placard. If you hit turbulence and slow to 105 knots out of habit, you are above your actual maneuvering speed for the day.

Two more things to know before you lean on the result:

- **The manual wins.** Many newer manuals list VA at several weights. Use theirs when they do.
- **Weight changes in flight.** Burning fuel lowers the number as you go. Working it out for the landing weight gives you the most conservative figure for the whole flight.

The [maneuvering speed tool](/aviation/performance/va-for-weight/) works this out for your weights, and the [weight and balance tool](/aviation/loading/weight-balance/) gives you the weight to put in it. The [turn tool](/aviation/performance/turn/) shows the load factor a bank angle adds, which is the same load the wing is carrying when you pull.
