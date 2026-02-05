# Design

## Notebook Resolution Order
1. Per-command notebook argument
2. Server configuration (`--notebook` or `NB_MCP_NOTEBOOK`)
3. Git-derived default from the master worktree path
4. Error with guidance if none of the above resolve

## Git-Derived Default
Use `git rev-parse --git-common-dir` to find the common `.git` directory.
If the path ends in `.git`, use its parent directory as the master worktree
root. The notebook name is the basename of that root directory.

If Git discovery fails, do not fall back to nb defaults.
