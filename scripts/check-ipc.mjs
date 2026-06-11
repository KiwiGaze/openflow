#!/usr/bin/env node
/**
 * Verifies the hand-mirrored IPC contract stays in sync. There is no codegen:
 * Rust command/event names and their TypeScript mirrors in
 * `packages/core/src/types.ts` are written by hand, so this script is the only
 * automated guard against drift. It checks:
 *
 *   1. every `#[tauri::command]` fn ↔ a `COMMANDS` value (both directions)
 *   2. every command is registered in main.rs `generate_handler![…]`
 *   3. every Rust `*_EVENT` constant ↔ an `EVENTS` value (both directions)
 *   4. `.emit(…)` call sites use a named constant, never a string literal
 *   5. `COMMANDS` keys are the camelCase form of their snake_case values
 *
 * Exits non-zero with a per-finding message on any drift. Conventions:
 * docs/engineering/ipc-contract-conventions.md
 */
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const rustSrcDir = join(repoRoot, 'apps/desktop/src-tauri/src');
const typesPath = join(repoRoot, 'packages/core/src/types.ts');

const problems = [];

// Walked recursively so commands, event constants, and emit sites are found
// even in nested modules (e.g. a future `pipeline/cloud.rs`) — a flat listing
// would silently under-check. `name` is the src/-relative path.
function rustSourceFiles(dir) {
  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...rustSourceFiles(path));
    else if (entry.name.endsWith('.rs')) files.push(path);
  }
  return files;
}

const rustFiles = rustSourceFiles(rustSrcDir).map((path) => ({
  name: path.slice(rustSrcDir.length + 1),
  text: readFileSync(path, 'utf8'),
}));

// --- Rust side -------------------------------------------------------------

const commandFns = new Set();
for (const { text } of rustFiles) {
  for (const match of text.matchAll(
    /#\[tauri::command\]\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([a-z0-9_]+)/g,
  )) {
    commandFns.add(match[1]);
  }
}

const mainRs = rustFiles.find((f) => f.name === 'main.rs');
const handlerBlock = /generate_handler!\[([^\]]*)\]/s.exec(mainRs?.text ?? '');
const registered = new Set(
  (handlerBlock?.[1] ?? '')
    .split(',')
    .map((entry) => entry.trim().split('::').pop())
    .filter(Boolean),
);

const rustEvents = new Set();
for (const { text } of rustFiles) {
  for (const match of text.matchAll(/pub const [A-Z0-9_]*_EVENT: &str = "([^"]+)"/g)) {
    rustEvents.add(match[1]);
  }
}

// Literal event names defeat the whole point of the constants; flag the call.
// Matched against the whole file, not per line, so rustfmt-wrapped multiline
// calls like `emit(\n  "event", …)` can't slip past (`\s` spans newlines).
// Covers both call forms: `app.emit("…")` and `tauri::Emitter::emit(app, "…")`.
const literalEmitPatterns = [
  /\.emit\(\s*"/g,
  /\.emit_to\(\s*[^,]+,\s*"/g,
  /Emitter::emit\(\s*[^,]+,\s*"/g,
  /Emitter::emit_to\(\s*[^,]+,\s*[^,]+,\s*"/g,
];
for (const { name, text } of rustFiles) {
  for (const pattern of literalEmitPatterns) {
    for (const match of text.matchAll(pattern)) {
      const line = text.slice(0, match.index).split('\n').length;
      problems.push(
        `${name}:${line} emits a string-literal event name — use a named *_EVENT constant`,
      );
    }
  }
}

// --- TypeScript side --------------------------------------------------------

const typesTs = readFileSync(typesPath, 'utf8');

function extractMirror(constName) {
  // Lazy match up to `} as const` so a nested brace can never silently
  // truncate the set this guard checks.
  const block = new RegExp(`const ${constName} = \\{([\\s\\S]*?)\\} as const`).exec(typesTs);
  if (!block) {
    problems.push(`packages/core/src/types.ts: could not find \`const ${constName} = { … }\``);
    return new Map();
  }
  const entries = new Map();
  for (const match of block[1].matchAll(/([A-Za-z0-9_]+):\s*'([^']+)'/g)) {
    entries.set(match[1], match[2]);
  }
  return entries;
}

const tsCommands = extractMirror('COMMANDS');
const tsEvents = extractMirror('EVENTS');

const camelCase = (snake) => snake.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());
for (const [key, value] of tsCommands) {
  if (camelCase(value) !== key) {
    problems.push(`COMMANDS.${key}: expected key \`${camelCase(value)}\` for value '${value}'`);
  }
}

// --- Compare ----------------------------------------------------------------

function diff(label, leftName, left, rightName, right) {
  for (const item of left) {
    if (!right.has(item)) {
      problems.push(`${label} \`${item}\` is in ${leftName} but missing from ${rightName}`);
    }
  }
}

const tsCommandValues = new Set(tsCommands.values());
const tsEventValues = new Set(tsEvents.values());

diff('command', 'Rust (#[tauri::command])', commandFns, 'COMMANDS in types.ts', tsCommandValues);
diff('command', 'COMMANDS in types.ts', tsCommandValues, 'Rust (#[tauri::command])', commandFns);
diff('command', 'Rust (#[tauri::command])', commandFns, 'main.rs generate_handler![…]', registered);
diff('command', 'main.rs generate_handler![…]', registered, 'Rust (#[tauri::command])', commandFns);
diff('event', 'Rust *_EVENT constants', rustEvents, 'EVENTS in types.ts', tsEventValues);
diff('event', 'EVENTS in types.ts', tsEventValues, 'Rust *_EVENT constants', rustEvents);

if (problems.length > 0) {
  console.error(`IPC contract drift (${problems.length}):\n`);
  for (const problem of problems) console.error(`  - ${problem}`);
  console.error('\nSee docs/engineering/ipc-contract-conventions.md for the rules.');
  process.exit(1);
}

console.log(
  `IPC contract in sync: ${commandFns.size} commands, ${rustEvents.size} events, mirrors match.`,
);
