## ADDED Requirements

### Requirement: Every visual serves the job
A tool page SHALL show a visual only when it draws the reader's own inputs or the tool's answer. It SHALL redraw on every new result and help answer the question the tool exists for. A tool whose answer is a number or a table SHALL show no visual. No visual SHALL come before the answer or delay it.

#### Scenario: GSD shows the footprint, not a map
- **WHEN** a reader opens the GSD tool and presses "Try the example" (1-inch 20 MP camera at 100 m)
- **THEN** the page shows a side-view diagram of the camera 100 m above the ground, a footprint 150 m across, and "1 pixel = 2.741 cm of ground", and it shows no map

#### Scenario: The visual follows the inputs
- **WHEN** the reader changes the GSD tool's height from 100 m to 60 m
- **THEN** the diagram redraws with the new height, footprint, and GSD from the new result

### Requirement: The map is for places
A page SHALL carry the map only when the tool's inputs or outputs include places on the earth: latitude and longitude, a geographic polygon or line, or a grid cell. Coordinates on a local plane (northing and easting on an assumed or project grid) SHALL NOT be drawn on the map; they SHALL get a plane diagram instead.

If the current inputs give the map nothing to draw (for example, an optional location left blank), the map SHALL hide. When the tool takes a latitude and longitude, one line SHALL say how to see it: "Enter a latitude and longitude to see this on the map." The map SHALL reappear, framed on the result, as soon as there is something to draw.

#### Scenario: A plane traverse is sketched, not mapped
- **WHEN** a reader runs traverse closure on courses from an assumed point at northing 5,000, easting 5,000
- **THEN** the page shows the adjusted traverse in plan with numbered corners and its precision, and it shows no map

#### Scenario: Optional location
- **WHEN** a reader runs glide range without a location
- **THEN** there is no map, and the line "Enter a latitude and longitude to see this on the map." is shown
- **WHEN** the reader then enters a latitude and longitude
- **THEN** the map appears with the glide range ring around that point

### Requirement: Declared visuals are drawn
A tool whose manifest declares a `vector-diagram` SHALL draw one from its primary example. A tool whose manifest declares a geographic layer SHALL draw at least one layer from its primary example, unless that example leaves out an optional location. Every length and angle in a drawing SHALL come from the core's result or from the reader's input. The renderer SHALL NOT compute a value the result does not hold.

#### Scenario: Photo overlap
- **WHEN** a reader runs the trigger tool with 75% front and 65% side overlap
- **THEN** the diagram shows frames along two flight lines, overlapping areas reading darker, with "Photo every 25 m (2.5 s), 75% front overlap" and "Lines 52.5 m apart, 65% side overlap"

#### Scenario: Gate on the catalog
- **WHEN** a manifest declares a map layer its outputs cannot fill
- **THEN** `visual-purpose.test.mjs` fails and names the tool

### Requirement: Grid cells draw on the map
Tools that return S2 cells SHALL draw each cell's outline from the core (`indexing.s2.cell-info`), just as H3 tools draw theirs from `indexing.h3.cell-info`.

#### Scenario: S2 covering
- **WHEN** a reader runs the S2 covering example over downtown Pittsburgh
- **THEN** the map outlines each covering cell the result lists, eight at level 12
