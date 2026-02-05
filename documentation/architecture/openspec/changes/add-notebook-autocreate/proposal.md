# Change: Auto-create missing notebooks

## Why
Creating a notebook is a manual prerequisite today, which makes first-run
experiences fail in new projects. Auto-creation reduces friction while keeping
opt-out control.

## What Changes
- Automatically create a missing notebook after name resolution.
- Add a `--no-create-notebook` flag to disable auto-creation.
- Document the auto-creation behavior and opt-out flag.

## Impact
- Affected specs: `notebook-resolution`
- Affected code: `src/main.rs`, `src/nb.rs`, docs/README
