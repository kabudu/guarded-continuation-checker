# GitHub release notes style

GitHub already displays the release title. A release-notes file must therefore
start with a short summary, not a duplicate level-one heading.

Use this structure:

1. One short opening paragraph that states the user-visible outcome.
2. A `## Release highlights` section with no more than seven concise bullets.
3. Optional focused sections for measurements, installation or migration.
4. A final `## Status and scope` section for maturity and claim boundaries.

Keep each bullet to one sentence and lead with a short bold label. Prefer a
table when several measurements share the same dimensions. Do not use em
dashes, manual HTML or research-log prose.

Run the formatting gate before publishing:

```sh
scripts/check-release-notes.sh docs/releases/vX.Y.Z.md
```

The tag-release workflow runs the same gate before it publishes or refreshes a
GitHub release.
