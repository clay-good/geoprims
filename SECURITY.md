# Security

## Reporting a vulnerability

Open a private advisory at
[github.com/clay-good/geoprims/security/advisories/new](https://github.com/clay-good/geoprims/security/advisories/new).
Please do not open a public issue for a vulnerability. You should get a first reply within seven days.

Include what you found, how to reproduce it, and what an attacker could do with it. If it involves the website,
the permalink is usually enough to reproduce; the fragment holds the whole input state.

## What is in scope

| Area | What matters |
|---|---|
| The website | Anything that gets script or style past the enumerated CSP, reads another origin's data, or sends a user's inputs anywhere |
| The core (`core/`) | Memory safety in the Wasm modules, and any input that makes a module trap or loop instead of returning an error |
| The MCP server (`mcp/`) | Anything that makes it open a socket, read outside its own directory, or execute input |
| The report Worker (`worker/`) | Anything that stores a reporter's address, bypasses the bot check or the caps, or reads another row |
| Releases | A build that does not reproduce, or an artifact whose published SHA-256 does not match |

A wrong answer is not a vulnerability: report it with the "Report a problem" button on the tool, or the
["Wrong answer" issue form](https://github.com/clay-good/geoprims/issues/new?template=wrong-answer.yml).

## What is not in scope

- Reports from automated scanners with no demonstrated impact.
- Missing headers on endpoints that serve no data, or absent rate limits on static files.
- Anything requiring a compromised device, a malicious browser extension, or physical access.
- Social engineering, and denial of service by volume.

## What the design already rules out

There is no account system, no session, no cookie, and no server-side calculation. Every tool runs in the visitor's
own browser, so there is no stored user data to reach. The only server code is the problem-report Worker, which
stores no address and answers every acceptable request identically. See [the privacy page](https://geoprims.com/privacy/).

## Supported versions

The current release. Fixes go into the next release, and a security fix is labeled in
[the changelog](https://geoprims.com/changelog/).
