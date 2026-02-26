# Homebrew Automation Roadmap

This document captures the plan for automating Homebrew tap updates now that the
JSON-aware workflow is stable.

## Goals

- Publish `hashline` releases to the existing tap without manual edits.
- Verify the generated formula and man page artifacts before pushing.
- Keep the automation opt-in so maintainers can still cut hotfixes manually.

## Proposed Workflow

1. **Add a release workflow** (`.github/workflows/release.yml`):
   - Trigger on tags that match `v*`.
   - Build the release binaries and man pages (current `release` job already
     does this).
   - Compute SHA256 checksums for the macOS/Linux tarballs.
   - Store the checksum list as an artifact for the tap update step.
2. **Update the tap automatically**:
   - Check out `github.com/lispmeister/homebrew-hashline` (create if missing).
   - Run `scripts/update-brew-formula.sh` (new script) which:
     - Reads the checksum artifact.
     - Updates version, URL, and SHA256 fields in `hashline.rb`.
     - Commits with message `hashline <version>`.
   - Push the commit back to the tap repository (requires a PAT with
     `repo` scope stored as `BREW_TAP_TOKEN`).
3. **Surface the plan to maintainers**:
   - Document the workflow in `CONTRIBUTING.md`.
   - Keep a manual fallback (`bin/update-brew-formula`) that mirrors the script
     for one-off releases.

## Open Questions

- Do we want to cut Windows bottles as part of the tap? (Currently the tap
  only publishes macOS and Linux.)
- Should the workflow post a PR to the tap instead of pushing directly?

## Next Steps

1. Implement the release workflow + script in a follow-up PR.
2. Add tests that exercise the script with fixture checksums.
3. Update the README once the automation is proven stable.
