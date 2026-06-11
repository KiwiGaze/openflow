---
name: code-review
description: Review an OpenFlow diff, branch, or PR — or process incoming review comments. Use this whenever asked to review changes, assess a PR, double-check work before merging, or respond to CodeRabbit/human review feedback in this repo; it encodes the repo's blockers, per-area rules, and the over-engineering filter.
---

# Code review

## Purpose

Apply OpenFlow's actual bar: privacy and contract violations are hard
blockers, boring-and-explicit beats clever, and review noise (style nits the
linter owns, speculative-abstraction demands) is itself a defect. The same
bar applies to reviews you write and reviews you receive.

## Files to inspect first

- `docs/engineering/review-checklist.md` — blockers and per-area rules
  (authoritative; this skill is its procedure).
- `docs/engineering/architecture-boundaries.md` — the seven break-prone
  constraints; check any touched file against them.
- `.github/PULL_REQUEST_TEMPLATE.md` — what the author attested to.
- `.coderabbit.yaml` — what automated review already covers (don't duplicate
  it).

## Procedure

1. Read the diff end to end before judging anything; map each file to its
   area (Rust / React / core / IPC / CI / docs).
2. Scan for blockers first: telemetry or default-on network, audio/transcript
   persistence, one-sided IPC change, new endpoint absent from PRIVACY.md,
   `unwrap()` outside tests, silently droppable dictation output. Any hit →
   the review leads with it.
3. Walk the matching per-area sections of the review checklist; for touched
   constraint files (pipeline/output/hud/shortcuts), verify against the
   seven constraints explicitly.
4. Verify claims, don't trust them: run `pnpm check:all` (or read CI), and
   for "already fixed" review comments, check the code at HEAD.
5. Write findings as: `file:line — what — why it matters — smallest fix`,
   ordered by severity, with a confidence tag when below certain.
6. Filter your own findings before posting (see Rules) — then state clearly:
   approve, approve-with-nits, or request-changes with the blocker list.

**Processing incoming review comments:** verify each against the codebase
(some are already fixed), apply the over-engineering filter below, implement
only valid fixes, run the gates, then summarize: addressed / rejected (with
reason) / already resolved.

## Rules

Reject — in both directions — findings that demand:

- an abstraction for a single use site (helper, wrapper, factory, generic);
- indirection without measurable benefit, or speculative
  configurability/future-proofing;
- defensive code for states the type system already forbids, or validation
  beyond system boundaries;
- patterns that add lines without reducing complexity;
- cosmetic renames/restructures (including wire-visible IPC names).

And hold the floor: blockers are non-negotiable regardless of how much work
the PR contains; tests for logic changes; both gates green; comments that
explain why.

## Commands

```sh
git diff main...HEAD             # the changeset under review
pnpm check:all                   # verify the author's claims
gh pr view <n> --comments        # incoming review threads (when using gh)
```

## Checklist

- [ ] Whole diff read; blockers scanned first
- [ ] Per-area checklist sections applied; constraints checked for touched
      files
- [ ] Gates run (or CI verified), not assumed
- [ ] Findings filtered for over-engineering before posting
- [ ] Verdict explicit; every request-changes item is actionable

## Common mistakes

- Reviewing the diff in isolation and missing that it breaks an invariant
  documented two files away (the constraint list exists for this).
- Demanding `get_history` → `list_history`-style renames — wire churn, zero
  behavior.
- Accepting "CI is green" while the diff edited the gate itself.
- Letting a large, mostly-good PR carry one small privacy violation through
  on goodwill.

## Expected output

A review whose first line is the verdict, blockers (if any) up top, findings
as `file:line — what — why — smallest fix`, no filtered-out noise — or, when
processing comments, an addressed/rejected/already-resolved summary with the
gates green.
