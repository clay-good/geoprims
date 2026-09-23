---
title: How to estimate drone flight time
description: Drone flight time is usable battery energy divided by the power the drone draws. How to work it out from mAh and volts, with reserves and cold packs.
summary: From the numbers on the battery label to minutes in the air, with a landing reserve and a cold-weather allowance.
audience: Drone operators
published: 2026-09-23
tools:
  - drone.power.endurance
  - drone.power.battery-energy
  - drone.power.hover-power
sources:
  - title: Range, Endurance, and Optimal Speed Estimates for Multicopters
    issuer: Bauersfeld, L., and Scaramuzza, D., IEEE Robotics and Automation Letters 7(2)
    edition: 2022
    locator: Section VII-E (endurance from effective capacity, nominal voltage, and electrical power)
    url: https://arxiv.org/abs/2109.04741
  - title: Principles of Helicopter Aerodynamics
    issuer: Leishman, J. G., Cambridge University Press
    edition: 2nd edition, 2006
    locator: Chapter 2, momentum theory (ideal hover power and figure of merit)
    url: https://doi.org/10.1017/CBO9780511809569
---

A drone's flight time is the battery energy you are willing to use, divided by the power the drone draws while it flies. Energy is in watt-hours (Wh) and power is in watts (W), so the answer comes out in hours. A pack with 72.3 Wh to spend and a drone that draws 150.6 W hovers for about 28.8 minutes.

Everything else is about getting those two numbers right: how much of the pack you really use, how much you keep back for landing, how cold the battery is, and how much power the drone needs to stay up.

## Why it matters

The flight time on the box is measured in calm air, often with a fresh pack flown until it is nearly empty. Your flights carry payloads, fight wind, climb, and land with charge to spare. If you plan a mapping mission from the box figure, you may run short over the far end of the site. A few minutes of planning shows how many batteries a job needs and when to turn for home.

## How it is worked out

### Step 1: battery energy

Battery labels give capacity in milliamp-hours (mAh) and a nominal voltage. Energy is the two multiplied together:

**Energy (Wh) = capacity (mAh) × voltage (V) ÷ 1,000**

A 5,870 mAh pack at 15.4 V holds 90.4 Wh. If the label shows only the cell count, the [battery energy tool](/drone/power/battery-energy/) uses 3.7 V per lithium-polymer cell and tells you so. Capacity alone, in mAh, is not enough to compare two packs. A higher-voltage pack with fewer mAh can hold more energy.

### Step 2: usable energy

Few pilots drain a pack to zero. Running a lithium pack very low shortens its life, and the voltage sags near the end. The **usable share** (also called the depth-of-discharge limit) is the part of the pack you allow yourself to use, such as 80%.

### Step 3: landing reserve

A **landing reserve** is energy you plan never to spend, so you can land at a safe spot if something changes. A common way to write it is a percentage, such as 20%. The catch is that "20%" can mean two different things, and the geoprims tools use one each:

- The [flight time tool](/drone/power/endurance/) takes the reserve as a share of the **usable** energy. With 80% usable and a 20% reserve, it keeps 20% of the 80%.
- The [battery energy tool](/drone/power/battery-energy/) takes the reserve as a share of the **whole pack**. With an 80% limit and a 20% reserve, it subtracts 20 points from the 80, leaving 60% of the pack to fly on.

Neither is wrong. They answer slightly different questions, and the worked example below shows how far apart they land. Pick the one that matches how your team writes its reserve, and say which one you mean.

### Step 4: cold batteries

Lithium packs deliver less energy when they are cold. If you enter a battery temperature, the flight time tool applies a labeled rule of thumb: no derating at 20 °C or warmer, rising 1% per degree below that (20% at 0 °C), with a cap at 50%. It is a planning heuristic, not a measurement. If you have your own test data, enter your own derating instead.

### Step 5: power

The power figure matters as much as the battery. Take it from your own flight log if you can: average watts in a steady hover. Without one, the [hover power tool](/drone/power/hover-power/) estimates it from takeoff mass, rotor count, rotor size, and air density, using momentum theory with a figure of merit for real rotor losses and an efficiency for the motors and speed controllers. That tool is still marked experimental, so treat its answer as a starting point and check it against a test flight.

Then:

**Flight time = energy × usable share × (1 − cold derating) × (1 − reserve) ÷ power**

## A worked example

A quadcopter with a 5,870 mAh, 15.4 V pack, a takeoff mass of 1.4 kg, and four 9.4 in rotors, flown at sea level:

| Step | Result |
|---|---|
| Pack energy | 90.4 Wh |
| Hover power, estimated | 150.6 W |
| Usable energy at 80% | 72.3 Wh |
| Hover time, no reserve | **28.8 min** |
| Hover time with a 20% reserve (14.5 Wh kept) | 23.1 min |
| The same, with the battery at 10 °C | 20.7 min |
| The same, with the battery at 0 °C | 18.4 min |

Now the two reserve conventions side by side, both with an 80% limit and a 20% reserve:

| Reserve counted as | Energy left to fly on | Hover time |
|---|---|---|
| 20% of the usable energy (flight time tool) | 57.9 Wh | 23.1 min |
| 20% of the whole pack (battery energy tool) | 54.2 Wh | 21.6 min |

The difference is 1.5 minutes on a single pack. On a mission planned to the last minute, that is the difference between landing with a margin and landing on the reserve.

Height and heat raise the power a drone needs to hover, because thinner air gives the rotors less to push on. The same quadcopter at a pressure altitude of 5,000 ft on a 30 °C day needs about 169.3 W, and its hover time with 80% usable drops from 28.8 to 25.6 minutes.

## Rules of thumb, and where they drift

- **"Flight time scales with mAh."** Only at the same voltage. Compare packs in watt-hours.
- **"A bigger battery always flies longer."** A heavier pack also raises hover power. Past a point, the extra weight eats most of the extra energy.
- **"The spec sheet tells me my flight time."** That figure is usually a no-wind, no-payload, near-empty number. Plan from your own logged power and the reserve you actually keep.

## Common mistakes

- **Using the whole pack.** Set a usable share and a reserve before you plan the mission, not during it.
- **Mixing up the two reserve conventions.** Twenty percent of the usable energy is less than 20% of the pack.
- **Forgetting the cold.** A pack that sat in a cold truck will not deliver its warm-weather energy. Warm packs before flight where the maker says to.
- **Planning a survey on hover time.** Forward flight, climbs, and headwinds draw different power. Log a real mission and use its average power if you have one.
- **Trusting an estimate over a test.** A short test flight that logs power beats any formula.

## Where the numbers come from

The endurance formula, energy times voltage divided by electrical power, follows Bauersfeld and Scaramuzza's multicopter range and endurance paper. The hover power estimate uses momentum theory as set out in Leishman's *Principles of Helicopter Aerodynamics*. The [flight time tool](/drone/power/endurance/) shows every step with your numbers under "How we got this." Start with the [battery energy tool](/drone/power/battery-energy/) if your pack is labeled only in mAh, and the [hover power tool](/drone/power/hover-power/) if you have no logged power yet. If you plan to fly near the height limit, see [the Part 107 altitude limit](/learn/part-107-altitude-limit/).
