#!/usr/bin/env node
/**
 * Mechanical tripwires for the privacy invariants ("privacy is the product").
 * These checks cannot prove the app is private — they catch the accidents
 * that are cheap to catch:
 *
 *   1. HTTP client crates appear only in the three audited network modules
 *      (llm.rs — refinement, models.rs — model downloads, cloud_stt.rs —
 *      opt-in cloud STT). A network call creeping into audio, settings, or
 *      clipboard code is a privacy regression by construction.
 *   2. Webview code never does its own network I/O (fetch/XHR/sendBeacon/
 *      WebSocket) — every byte that leaves the app goes through the Rust
 *      core where the consent gates live.
 *   3. No known telemetry/analytics SDK appears in any dependency manifest.
 *
 * Exits non-zero with a per-finding message. The human-judgment privacy rules
 * (no audio persistence, consent gates) live in PRIVACY.md and the review
 * checklist — this script does not replace them.
 */
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const problems = [];

// --- 1. HTTP crates stay in the audited network modules ---------------------

const NETWORK_MODULES = new Set(['llm.rs', 'models.rs', 'cloud_stt.rs']);
const HTTP_CRATE_PATTERN = /\b(reqwest|ureq|hyper|isahc|curl)::/;

const rustSrcDir = join(repoRoot, 'apps/desktop/src-tauri/src');
for (const name of readdirSync(rustSrcDir).filter((n) => n.endsWith('.rs'))) {
  if (NETWORK_MODULES.has(name)) continue;
  const text = readFileSync(join(rustSrcDir, name), 'utf8');
  text.split('\n').forEach((line, i) => {
    if (HTTP_CRATE_PATTERN.test(line)) {
      problems.push(
        `${name}:${i + 1} uses an HTTP client outside the audited network modules ` +
          `(${[...NETWORK_MODULES].join(', ')})`,
      );
    }
  });
}

// --- 2. The webviews do no network I/O of their own --------------------------

const WEBVIEW_NETWORK_PATTERN = /\b(fetch\(|XMLHttpRequest|sendBeacon|new WebSocket)/;

function walkTsFiles(dir) {
  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...walkTsFiles(path));
    else if (/\.(ts|tsx)$/.test(entry.name)) files.push(path);
  }
  return files;
}

for (const dir of ['apps/desktop/src', 'packages/core/src']) {
  for (const path of walkTsFiles(join(repoRoot, dir))) {
    const text = readFileSync(path, 'utf8');
    text.split('\n').forEach((line, i) => {
      if (WEBVIEW_NETWORK_PATTERN.test(line)) {
        problems.push(
          `${path.slice(repoRoot.length + 1)}:${i + 1} does network I/O from the webview — ` +
            `route it through a Rust IPC command instead`,
        );
      }
    });
  }
}

// --- 3. No telemetry SDK in any dependency manifest ---------------------------

const TELEMETRY_NAMES =
  /\b(sentry|posthog|amplitude|mixpanel|segment|bugsnag|datadog|telemetry|firebase-analytics)\b/i;

// Derived from the workspace layout so a future package can't silently
// escape the scan.
const manifests = [
  'package.json',
  'apps/desktop/src-tauri/Cargo.toml',
  ...['apps', 'packages'].flatMap((dir) =>
    readdirSync(join(repoRoot, dir), { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => join(dir, entry.name, 'package.json')),
  ),
].filter((manifest) => existsSync(join(repoRoot, manifest)));
for (const manifest of manifests) {
  const text = readFileSync(join(repoRoot, manifest), 'utf8');
  text.split('\n').forEach((line, i) => {
    if (TELEMETRY_NAMES.test(line)) {
      problems.push(`${manifest}:${i + 1} references a telemetry/analytics package`);
    }
  });
}

if (problems.length > 0) {
  console.error(`Privacy tripwires (${problems.length}):\n`);
  for (const problem of problems) console.error(`  - ${problem}`);
  console.error('\nSee PRIVACY.md and docs/engineering/review-checklist.md.');
  process.exit(1);
}

console.log('Privacy tripwires clear: HTTP confined, webviews IPC-only, no telemetry deps.');
