# Dictating code and identifiers

Status: exploration. A brainstorm sketch, not a committed spec.

## Why

Developers dictate too — and code is hostile to everything the prose pipeline does well.
Identifiers are `camelCase` or `snake_case`, "dot" and "open paren" are literal symbols not
punctuation, and the capitalization rules are inverted. Literal mode rightly skips the LLM and
the prose cleanup, but it still cannot turn "user service dot get user by id" into
`userService.getUserById`. The result is that the one group most comfortable talking to their
computer gets the worst dictation experience.

## Idea

A **code-aware formatting layer** that recognises spoken identifiers and symbols and emits
source-like text. It is reached through a dedicated mode ("Code") or — better — through the
per-app activation already planned, so it turns on automatically in an editor and off in Mail,
with no new trigger to remember.

Two tiers, deterministic first:

### Tier 1 — deterministic symbol + casing grammar (no network)

A bounded spoken vocabulary, handled in `text.rs`, fast and predictable:

```
"open paren" "close paren"   → ( )
"dot"  "comma"  "semicolon"  → .  ,  ;
"new line"                   → \n              (already understood today)
"camel case user id"         → userId
"snake case user id"         → user_id
"pascal case user service"   → UserService
"constant max size"          → MAX_SIZE
```

A casing command consumes the next few words up to a delimiter and joins them in the requested
convention. This is a small, testable state machine — the same shape as the existing "new line /
new paragraph" handling, just richer.

### Tier 2 — LLM-assisted formatting (optional, off by default)

A "Code" mode prompt that takes a loose spoken description and renders it in the surrounding
convention, using the dictionary as the source of known project symbols. Needs an AI profile;
purely additive on top of Tier 1.

## Identifier survival

Both tiers lean on the dictionary. Project identifiers (`getUserById`, `TanStack`, `OAuth`) only
survive if whisper is biased toward them and the cleanup never "fixes" them. Two reuses:

- The dictionary already feeds whisper as `initial_prompt` and is listed to the LLM — tagging
  some entries as **code terms** lets the Code path treat them as atomic and skip prose
  capitalisation.
- [living-dictionary](living-dictionary.md)'s internal-caps detector is exactly the signal that
  finds these identifiers in the first place. The two features share one detector.

## What this is _not_

Deep editor integration — reading the open file, tagging files in a chat, knowing the project's
symbols by scraping the IDE — is **out of scope**. It would require editor-specific automation
and accessibility/screen access that cut against the privacy posture, for value that the
on-device formatting already captures. OpenFlow formats what you _say_; it does not look at what
is on your screen.

## How it fits

Tier 1 is a new pure function in `text.rs`, gated by the active mode/context so prose dictation
is untouched (you do not want "dot" becoming "." in an email):

```rust
// text.rs — only when the Code path is active
pub fn apply_code_formatting(text: &str, code_terms: &[&str]) -> String { /* symbols + casing */ }
```

Tier 2 is just another mode prompt in `modes.rs` plus an optional `codeTerms` flavour on
`DictionaryEntry`. No pipeline restructuring — Code is a mode like the others, with one extra
deterministic pass when it is the active path.

## Privacy fit

Tier 1 is pure local rules — no network, no model, nothing stored. Tier 2 is the existing refine
call (text only, BYO key). Crucially, **nothing reads the editor or the screen**; the only input
is the user's voice, same as every other path.

## Open questions

- Vocabulary size and discoverability: the casing/symbol commands are invisible unless taught —
  they need a cheat-sheet and probably a Code-mode hint on first use.
- Conflict with prose: "new line" already maps to a newline; make sure the code grammar is only
  live on the Code path so prose is unaffected.
- Is Code a **mode**, a **toggle**, or strictly **per-app**? Per-app activation is the cleanest
  (it matches how you actually work), but it depends on per-app behavior landing first.
- Language coverage: start with the conventions common across most languages (camel/snake/pascal/
  constant) rather than per-language symbol sets.
