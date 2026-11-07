# Release runbook

Releases come from version tags on `main`. A tag builds and verifies the payload, then publishes the GitHub release. It does not deploy: the site and the report Worker are deployed by hand, so that every Cloudflare credential stays on the maintainer's machine and the repository holds no deployment secret. The other sites here are run the same way.

The MCP server is repository-only. It is not sent to npm or the MCP Registry. Each tag must already contain its rebuilt `mcp/dist`, so GitHub's source archive runs with Node 22 and no install step. The release also attaches a one-click MCPB bundle, the verification and evaluation reports, the CycloneDX SBOM, complete artifact digests, and `SHA256SUMS`. GitHub signs a provenance attestation for every attached file.

## Deploying

Deploying is a separate, manual step, and deliberately so: automating it would
mean putting a long-lived Cloudflare API token in a repository secret, and
nothing here needs one. `wrangler` uses the maintainer's own OAuth login.

Run the full gate sequence first, then both targets:

```bash
cd apps/web && npx wrangler deploy --config wrangler.production.jsonc
cd ../worker && npx wrangler deploy
```

Both, whenever the tool catalog changed: the Worker regenerates
`src/catalog-tools.json` on each deploy, and a stale copy once validated reports
against 170 of 323 tools. Then smoke the live headers:

```bash
node apps/web/scripts/header-smoke.mjs https://geoprims.com --production
```

A tag's verified site build is attached to the workflow run as the `release-site`
artifact, so a release can be deployed from exactly the bytes CI checked rather
than from a local rebuild.

To rehearse against the preview Worker first, `npx wrangler deploy` from
`apps/web` (no `--config`) publishes to `geoprims-preview.hi-6b2.workers.dev`.

Watch for Cloudflare's 25 MiB per-asset limit: `wrangler` aborts the whole
deploy on the first file over it, which once left the live site a month stale.
`apps/web/test/asset-size.test.mjs` fails the build before that can happen
again.

## Prepare a tag

1. Start from a green `main` commit and choose a semantic version. Use a prerelease such as `v0.2.0-rc.1` first.
2. Set the same version in `mcp/package.json`, `mcp/manifest.json`, and the Rust workspace packages. Update the changelog.
3. Run the release builds, then force-add the ignored clone payload:

   ```bash
   npm run build
   npm run build:mcp
   npm run build:sbom
   git add -f mcp/dist
   ```

4. Commit the release preparation, push `main`, and wait for CI to pass.
5. Create an annotated tag on that exact commit and push it. Do not move a published tag.

The release workflow rejects malformed tags, version drift, a public MCP package, runtime dependencies, missing committed payload files, or a tag that is not on `main`. It rebuilds `mcp/dist` and rejects any byte difference before publishing.

## Verify the release

Download the attached files and run:

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify geoprims-VERSION.mcpb -R clay-good/geoprims
```

For a release candidate, deploy it to the preview Worker, run the six-route header smoke against that URL, and run the MCPB in a desktop host before promoting the same source to a stable version.
