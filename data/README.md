# data

Reference data that is small enough to live in git and is read by the build.

| File | What |
|---|---|
| `codes.json` | The codes registry: every error code and every warning code, with severity and message (contract B4). Tests keep the core in sync with it. |
| `taxonomy.json` | Domains and their groups. Every tool id's group must be listed here, or the catalog lint fails. |
