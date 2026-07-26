# Release-note style

GitHub release notes are product communication, not an internal experiment
log. They must help a new reader understand the release before presenting
implementation detail.

Use this order:

1. One short paragraph stating the user-visible outcome.
2. An optional second paragraph explaining the central capability.
3. `## What is new`, with no more than seven compact bullets.
4. `## Measured result` when the release includes measurements.
5. `## Install`, with copyable commands.
6. `## Project status`, with the honest support and claim boundary.

Each bullet must express one idea and should fit on one rendered line where
practical. Move qualifications, benchmark context and detailed methodology into
short prose below the list. Avoid packing several clauses, measurements or tool
names into one bullet. Use inline code only for literal commands, identifiers,
versions or file names.

Before publishing:

- preview the Markdown in GitHub;
- check it on a narrow viewport;
- confirm that wrapped bullets remain easy to scan;
- verify every command and link;
- retain negative measurements and claim boundaries without letting them
  dominate the opening;
- confirm that the committed file and published release body are identical.

The canonical notes live at `docs/releases/v<version>.md`. The release workflow
publishes that exact file, so live-only edits must be copied back into the
repository.
