# Contributing

## Changelog Discipline

We maintain a strict changelog to help consumers, indexers, and integrators track behavior and wire-format changes.

If your Pull Request modifies any smart contracts (`contracts/**`), you **must** update the `CHANGELOG.md` file. Add a brief, actionable entry describing the change under the `## [Unreleased]` section.
Categorize your change appropriately (e.g., `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`).

A CI check is in place to remind you if your PR touches `contracts/**` but misses the `CHANGELOG.md` update.
