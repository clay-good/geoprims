# Release runbook

Releases come from version tags on `main`. A release candidate deploys the exact static build to the `staging` GitHub environment and the `geoprims-preview` Worker. A stable tag deploys that build to `geoprims.com`. Only after deployment and the live header smoke pass does the workflow publish the GitHub release.

The MCP server is repository-only. It is not sent to npm or the MCP Registry. Each tag must already contain its rebuilt `mcp/dist`, so GitHub's source archive runs with Node 22 and no install step. The release also attaches a one-click MCPB bundle, the verification and evaluation reports, the CycloneDX SBOM, complete artifact digests, and `SHA256SUMS`. GitHub signs a provenance attestation for every attached file.

## One-time environment setup

Create `staging` and `production` GitHub environments. Give each environment `CLOUDFLARE_ACCOUNT_ID` and a least-privilege `CLOUDFLARE_API_TOKEN` that can deploy only its intended Worker. Protect `production` with a required reviewer. The repository currently has no deployment secrets, so the first release candidate will stop at deployment until these are configured.

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

The release workflow rejects malformed tags, version drift, a public MCP package, runtime dependencies, missing committed payload files, or a tag that is not on `main`. It rebuilds `mcp/dist` and rejects any byte difference before deploying.

## Verify the release

Download the attached files and run:

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify geoprims-VERSION.mcpb -R clay-good/geoprims
```

For a release candidate, also confirm the workflow's six-route staging header smoke and run the MCPB in a desktop host before promoting the same source to a stable version.
