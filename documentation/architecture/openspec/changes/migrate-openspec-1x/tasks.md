## 1. Pre-Migration
- [x] 1.1 Commit current state to preserve `openspec/AGENTS.md` in git history
- [x] 1.2 Upgrade global `openspec` CLI to 1.x (>= 1.4.0); note coexistence with other 0.x repos
- [x] 1.3 Verify `openspec` symlink resolves correctly

## 2. OpenSpec Init
- [x] 2.1 Run `openspec init --tools opencode` non-interactively
- [x] 2.2 Verify `openspec/AGENTS.md` deleted and `openspec/config.yaml` created
- [x] 2.3 Verify `.opencode/commands/` generated
- [x] 2.4 Verify `openspec` symlink intact

## 3. Configuration
- [x] 3.1 Review/customize `openspec/config.yaml` (init already creates it; add version pin comment >= 1.4.0)
- [x] 3.2 Add `commands/` to `.auxiliary/configuration/coders/opencode/.gitignore` (OPSX 1.3+ writes plural, escapes existing gitignore)

## 4. Agent Workflow Instructions
- [x] 4.1 Update `.auxiliary/configuration/AGENTS.md` OpenSpec Instructions section (replace `@openspec/AGENTS.md` reference with OPSX command model; this is the real file behind both CLAUDE.md and AGENTS.md symlinks)
- [x] 4.2 Create `documentation/agents/openspec.md` with OPSX workflow guidance

## 5. Validation
- [x] 5.1 Run `openspec validate --all` — existing specs and changes pass unchanged
- [x] 5.2 Verify no broken `@openspec/AGENTS.md` references remain in any instruction file
- [x] 5.3 Verify generated commands are functional: commands present under coder dirs, `openspec list` works, `/opsx:*` commands load in a fresh opencode session
