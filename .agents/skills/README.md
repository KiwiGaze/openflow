# Agent skills

Repo-local skills for AI coding agents working on Velata. Each directory
holds one `SKILL.md` (name + description frontmatter, then purpose, files to
inspect, procedure, rules, commands, checklist, common mistakes, expected
output). They condense `docs/engineering/` into task-shaped procedures —
the docs stay authoritative.

| Skill                                                               | Reach for it when…                                    |
| ------------------------------------------------------------------- | ----------------------------------------------------- |
| [monorepo-structure](monorepo-structure/SKILL.md)                   | creating files, placing code, adding scripts/packages |
| [tauri-ipc](tauri-ipc/SKILL.md)                                     | touching commands, events, or any IPC-crossing type   |
| [rust-backend-simplification](rust-backend-simplification/SKILL.md) | cleaning up or refactoring the Rust core              |
| [react-ui-simplification](react-ui-simplification/SKILL.md)         | cleaning up the webviews, hooks, components           |
| [readability-maintainability](readability-maintainability/SKILL.md) | naming, comments, file organization, splits           |
| [local-first-privacy](local-first-privacy/SKILL.md)                 | audio, network, persistence, logging, consent         |
| [ci-quality-gate](ci-quality-gate/SKILL.md)                         | running/fixing/changing the check commands or CI      |
| [dependency-hygiene](dependency-hygiene/SKILL.md)                   | adding, updating, auditing, removing dependencies     |
| [code-review](code-review/SKILL.md)                                 | reviewing a diff/PR or processing review comments     |
