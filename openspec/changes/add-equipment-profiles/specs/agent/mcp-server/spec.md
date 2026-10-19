## ADDED Requirements

### Requirement: Inline profiles in run calls
`geoprims_run` SHALL accept an optional `profile` argument holding a profile object in the `geoprims-profile` file format. For a tool id, the server SHALL fill each kept field of `profile.tools[id]` that `args` does not set; for `workflow.<slug>`, it SHALL fill the workflow inputs declared for the profile's kind. Explicit `args` SHALL win. The server SHALL list the filled field names in `meta.profileFilled`.

#### Scenario: Camera profile fills GSD
- **WHEN** an agent runs `drone.photogrammetry.gsd` with `args: {height: "100 m"}` and a camera profile
- **THEN** the sensor, pixel, and focal inputs come from the profile, `meta.profileFilled` lists them, and the result equals the web page's for the same values

#### Scenario: Explicit argument wins
- **WHEN** `args` sets `focal_length` and the profile also has one
- **THEN** the call uses the argument's value and `meta.profileFilled` omits `focal_length`

### Requirement: Profiles are validated and stateless
The server SHALL validate the profile object against the file format, SHALL ignore tools and fields the profile's kind does not keep, and SHALL NOT read files or keep profile state between calls.

#### Scenario: Wrong kind
- **WHEN** a `gnss-receiver` profile is passed to a photogrammetry tool
- **THEN** the call runs with no fields filled and `meta.profileFilled` is empty

#### Scenario: Malformed profile
- **WHEN** the `profile` object lacks `format` or `kind`
- **THEN** the call returns `INVALID_INPUT` with `field` `/profile` and runs nothing
