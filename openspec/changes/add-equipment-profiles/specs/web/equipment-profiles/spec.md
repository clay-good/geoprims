## Purpose

Lets a practitioner save their equipment once (aircraft, drone, camera, lidar sensor, GNSS receiver, radio link) and fill every tool and workflow that takes those values, in the browser, with nothing uploaded.

## ADDED Requirements

### Requirement: Profile kinds are data
Profile kinds and the tool fields each kind keeps SHALL be defined in `data/profiles.json`. The initial kinds SHALL be `aircraft`, `drone`, `camera`, `lidar`, `gnss-receiver`, and `radio`. The build SHALL fail if a listed field does not exist on the named tool, is marked `x-private`, or is a job value rather than an equipment value (a location, date, time, wind, or load).

#### Scenario: Field drift caught
- **WHEN** a tool renames the input a profile kind keeps
- **THEN** the build fails naming the kind, tool, and field

### Requirement: Profile file format
A profile SHALL export as JSON with `format: "geoprims-profile"`, `version`, `kind`, `name` (1 to 60 characters), `savedOn` (ISO 8601 date), and `tools`, keyed by tool id, holding each kept field in the same JSON the tool takes. Import SHALL accept this format and the `geoprims-aircraft` version 1 format (as kind `aircraft`), SHALL drop tools and fields the kind does not keep, and SHALL refuse files over 1 MB or without a valid name, stating why.

#### Scenario: Old aircraft file imports
- **WHEN** a reader imports a `.aircraft.json` file exported before this change
- **THEN** it becomes an `aircraft` profile with every table intact, and re-exporting writes the new format

#### Scenario: Unknown field dropped
- **WHEN** an imported camera profile carries a `wind_rating` field
- **THEN** the profile loads without it and the import report lists the dropped field

### Requirement: Local storage only
Profiles SHALL be stored in the browser under one key, with at most 50 profiles per kind. Nothing about a profile SHALL be sent over the network. The profile name SHALL NOT appear in permalinks, report payloads, or "copy as agent call" output; the values it filled SHALL, as ordinary inputs. "Erase all local data" SHALL remove every profile.

#### Scenario: Permalink carries values, not the name
- **WHEN** a reader fills the GSD tool from the profile "Zenmuse P1" and copies the permalink
- **THEN** the link holds the sensor and lens values and the word "Zenmuse" appears nowhere in it

#### Scenario: Egress test
- **WHEN** the privacy egress test saves a profile named with a sentinel and runs every profile-taking tool
- **THEN** no request contains the sentinel

### Requirement: Profile manager
A manager at `/settings/#profiles` SHALL list profiles by kind with name and saved date, and SHALL offer export, import, rename, and delete. Every tool that takes profile values SHALL offer, beside its inputs, the same save, load, export, and import actions for its kind, as the aircraft panel does today. Shipped example profiles SHALL be labeled "Example" and SHALL NOT be editable or counted.

#### Scenario: Save from a tool
- **WHEN** a reader enters a camera on the GSD tool and chooses "Save as profile" with the name "P1"
- **THEN** the camera kind gains "P1" holding the sensor, pixel, and focal values, and the same profile fills the trigger, footprint, and motion-blur tools

### Requirement: Filling a tool
Loading a profile SHALL set the tool's kept input fields and recompute. The answer card SHALL show "From profile: <name>" until a kept field is edited, then "Edited from <name>". Results, warnings, and sentences SHALL be identical to entering the same values by hand.

#### Scenario: Same answer either way
- **WHEN** the same camera values are entered by hand and loaded from a profile
- **THEN** the serialized results are byte-identical

### Requirement: Storage failure is explained
When the browser refuses storage (private window, quota), the manager SHALL say the profile lasts for this page only and offer export, as the aircraft panel does today.

#### Scenario: Private window
- **WHEN** a reader saves a profile in a private window where storage is blocked
- **THEN** the profile fills tools for the session and the manager states it will not persist
