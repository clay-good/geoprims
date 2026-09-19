# Practitioner review sign-offs

A domain counts as reviewed only when a qualified practitioner has checked its tools and a signed record is listed below. Until then, the domain's pages say "Not yet independently reviewed by a <practitioner>." Records expire after 12 months, or sooner when a reviewed tool's result changes. The build reads both tables (`tools/trust/signoffs.mjs`), and the claims gate fails any page that implies a review not recorded here.

## Reviewer needed per domain

| Domain | Practitioner |
|---|---|
| aviation | flight instructor (CFI) |
| drone | Part 107 remote pilot |
| survey | licensed surveyor |
| geodesy | geodesist |
| navigation | geodesist |
| geometry | GIS professional |
| indexing | GIS professional |
| raster | GIS professional |
| time | geodesist |
| units | metrologist |

## Sign-offs

Each row names the reviewer, their relevant qualification, the tools reviewed (ids, or `all` for the whole domain), the date (YYYY-MM-DD), and the scope of the review.

| Domain | Reviewer | Qualification | Tools | Date | Scope |
|---|---|---|---|---|---|
