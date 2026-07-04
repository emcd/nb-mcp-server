## 1. Pre-Migration
- [ ] 1.1 Commit current state to preserve `openspec/AGENTS.md` in git history
- [ ] 1.2 Upgrade global `openspec` CLI to 1.x (>= 1.4.0); note coexistence with other 0.x repos
- [ ] 1.3 Verify `openspec` symlink resolves correctly

## 2. OpenSpec Init
- [ ] 2.1 Run `openspec init --tools opencode` non-interactively
- [ ] 2.2 Verify `openspec/AGENTS.md` deleted and `openspec/config.yaml` created
- [ ] 2.3 Verify `.opencode/commands/` and skills generated
- [ ] 2.4 Verify `openspec` symlink intact

## 3. Configuration
- [ ] 3.1 Review/customize `openspec/config.yaml` (init already creates it; add version pin comment >= 1.4.0)
- [ ] 3.2 Add `commands/` to `.auxiliary/configuration/coders/opencode/.gitignore` (OPSX 1.3+ writes plural, escapes existing gitignore)

## 4. Agent Workflow Instructions
- [ ] 4.1 Update `.auxiliary/configuration/AGENTS.md` OpenSpec Instructions section (replace `@openspec/AGENTS.md` reference with OPSX skill/command model; this is the real file behind both CLAUDE.md and AGENTS.md symlinks)
- [ ] 4.2 Create `documentation/agents/openspec.md` with OPSX workflow guidance

## 5. Validation
- [ ] 5.1 Run `openspec validate --all` — existing specs and changes pass unchanged
- [ ] 5.2 Verify no broken `@openspec/AGENTS.md` references remain in any instruction file
- [ ] 5.3 Verify generated skills/commands are functional: skills present under coder dirs, `openspec list` works, `/opsx:*` commands load in a fresh opencode session
