/**
 * Accelerator strings use the Tauri global-shortcut grammar:
 * `Modifier+...+Key`, e.g. `Alt+Space`, `Cmd+Shift+R`, `F5`.
 */

export const MODIFIER_TOKENS = [
  'Cmd',
  'Command',
  'Super',
  'Meta',
  'CmdOrCtrl',
  'CommandOrControl',
  'Ctrl',
  'Control',
  'Alt',
  'Option',
  'Shift',
] as const;

/** Non-modifier key tokens we allow users to bind. One place to extend. */
export const KEY_TOKENS = [
  ...'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
  ...'0123456789',
  ...Array.from({ length: 24 }, (_, i) => `F${i + 1}`),
  'Space',
  'Backquote',
  'Minus',
  'Equal',
  'BracketLeft',
  'BracketRight',
  'Backslash',
  'Semicolon',
  'Quote',
  'Comma',
  'Period',
  'Slash',
  'Up',
  'Down',
  'Left',
  'Right',
  'Enter',
  'Tab',
] as const;

const MODIFIER_SET = new Set<string>(MODIFIER_TOKENS.map((m) => m.toLowerCase()));
const KEY_SET = new Set<string>(KEY_TOKENS.map((k) => k.toLowerCase()));
const F_KEY_RE = /^f([1-9]|1[0-9]|2[0-4])$/i;

export interface ParsedAccelerator {
  modifiers: string[];
  key: string;
}

/** Parse an accelerator string. Returns null when invalid. */
export function parseAccelerator(accel: string): ParsedAccelerator | null {
  const parts = accel.split('+').map((p) => p.trim());
  if (parts.some((p) => p.length === 0)) return null;
  const key = parts[parts.length - 1];
  if (key === undefined) return null;
  const modifiers = parts.slice(0, -1);
  if (!KEY_SET.has(key.toLowerCase())) return null;
  if (!modifiers.every((m) => MODIFIER_SET.has(m.toLowerCase()))) return null;
  // A bare letter/digit/punctuation binding would shadow normal typing
  // system-wide; only F-keys may stand alone.
  if (modifiers.length === 0 && !F_KEY_RE.test(key)) return null;
  return { modifiers, key };
}

export function isValidAccelerator(accel: string): boolean {
  return parseAccelerator(accel) !== null;
}

const MAC_MODIFIER_GLYPHS: Record<string, string> = {
  cmd: '⌘',
  command: '⌘',
  super: '⌘',
  meta: '⌘',
  cmdorctrl: '⌘',
  commandorcontrol: '⌘',
  ctrl: '⌃',
  control: '⌃',
  alt: '⌥',
  option: '⌥',
  shift: '⇧',
};

const MAC_KEY_GLYPHS: Record<string, string> = {
  space: 'Space',
  backquote: '`',
  minus: '-',
  equal: '=',
  bracketleft: '[',
  bracketright: ']',
  backslash: '\\',
  semicolon: ';',
  quote: "'",
  comma: ',',
  period: '.',
  slash: '/',
  up: '↑',
  down: '↓',
  left: '←',
  right: '→',
  enter: '↩',
  tab: '⇥',
};

/** Render an accelerator with macOS glyphs, e.g. `Alt+Space` → `⌥ Space`. */
export function formatAcceleratorMac(accel: string): string {
  const parsed = parseAccelerator(accel);
  if (!parsed) return accel;
  const mods = parsed.modifiers.map((m) => MAC_MODIFIER_GLYPHS[m.toLowerCase()] ?? m);
  const key = MAC_KEY_GLYPHS[parsed.key.toLowerCase()] ?? parsed.key.toUpperCase();
  return [...mods, key].join(' ');
}

export interface KeyboardEventLike {
  code: string;
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}

const EVENT_CODE_TO_TOKEN: Record<string, string> = {
  Space: 'Space',
  Backquote: 'Backquote',
  Minus: 'Minus',
  Equal: 'Equal',
  BracketLeft: 'BracketLeft',
  BracketRight: 'BracketRight',
  Backslash: 'Backslash',
  Semicolon: 'Semicolon',
  Quote: 'Quote',
  Comma: 'Comma',
  Period: 'Period',
  Slash: 'Slash',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Enter: 'Enter',
  Tab: 'Tab',
};

/**
 * Build an accelerator from a captured keydown, for the hotkey recorder UI.
 * Returns null while only modifiers are held or for unbindable keys.
 */
export function acceleratorFromKeyboardEvent(ev: KeyboardEventLike): string | null {
  let key: string | null = null;
  if (/^Key[A-Z]$/.test(ev.code)) key = ev.code.slice(3);
  else if (/^Digit[0-9]$/.test(ev.code)) key = ev.code.slice(5);
  else if (F_KEY_RE.test(ev.code)) key = ev.code.toUpperCase();
  else key = EVENT_CODE_TO_TOKEN[ev.code] ?? null;
  if (key === null) return null;

  const mods: string[] = [];
  if (ev.metaKey) mods.push('Cmd');
  if (ev.ctrlKey) mods.push('Ctrl');
  if (ev.altKey) mods.push('Alt');
  if (ev.shiftKey) mods.push('Shift');

  const accel = [...mods, key].join('+');
  return isValidAccelerator(accel) ? accel : null;
}
