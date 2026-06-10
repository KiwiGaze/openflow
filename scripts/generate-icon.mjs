#!/usr/bin/env node
/**
 * Generates the 1024x1024 app icon (rounded gradient square with a waveform
 * glyph) without any image dependencies, then writes it to
 * apps/desktop/app-icon.png. All platform icons are derived from it via
 * `pnpm --filter @openflow/desktop tauri icon app-icon.png`.
 */
import { deflateSync } from 'node:zlib';
import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SIZE = 1024;
// Big Sur style: the visible rounded square sits inset within the canvas.
const MARGIN = 100;
const RADIUS = 185;

const TOP = [0x4f, 0x46, 0xe5]; // indigo; gradient runs to violet #7c3aed
const BAR_COLOR = [0xff, 0xff, 0xff];

const BAR_HEIGHTS = [220, 420, 620, 420, 220];
const BAR_WIDTH = 72;
const BAR_GAP = 56;

function insideRoundedRect(x, y) {
  const min = MARGIN;
  const max = SIZE - MARGIN;
  if (x < min || x >= max || y < min || y >= max) return false;
  const rx = Math.max(min + RADIUS - x, x - (max - RADIUS), 0);
  const ry = Math.max(min + RADIUS - y, y - (max - RADIUS), 0);
  return rx * rx + ry * ry <= RADIUS * RADIUS;
}

function insideBar(x, y) {
  const totalW = BAR_HEIGHTS.length * BAR_WIDTH + (BAR_HEIGHTS.length - 1) * BAR_GAP;
  const startX = (SIZE - totalW) / 2;
  for (let i = 0; i < BAR_HEIGHTS.length; i++) {
    const cx = startX + i * (BAR_WIDTH + BAR_GAP) + BAR_WIDTH / 2;
    const hw = BAR_WIDTH / 2;
    const hh = BAR_HEIGHTS[i] / 2;
    const dx = Math.abs(x - cx);
    const dy = Math.abs(y - SIZE / 2);
    if (dx <= hw && dy <= hh - hw) return true;
    const capDy = dy - (hh - hw);
    if (capDy > 0 && dx * dx + capDy * capDy <= hw * hw) return true;
  }
  return false;
}

const raw = Buffer.alloc((SIZE * 4 + 1) * SIZE);
let off = 0;
for (let y = 0; y < SIZE; y++) {
  raw[off++] = 0; // filter: none
  for (let x = 0; x < SIZE; x++) {
    let [r, g, b, a] = [0, 0, 0, 0];
    if (insideRoundedRect(x, y)) {
      const t = (x + y) / (2 * (SIZE - 1));
      r = Math.round(TOP[0] + (0x7c - TOP[0]) * t);
      g = Math.round(TOP[1] + (0x3a - TOP[1]) * t);
      b = Math.round(TOP[2] + (0xed - TOP[2]) * t);
      a = 255;
      if (insideBar(x, y)) [r, g, b] = BAR_COLOR;
    }
    raw[off++] = r;
    raw[off++] = g;
    raw[off++] = b;
    raw[off++] = a;
  }
}

const CRC_TABLE = new Int32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c;
});

function crc32(buf) {
  let c = 0xffffffff;
  for (const byte of buf) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const out = join(root, 'apps', 'desktop', 'app-icon.png');
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, png);
console.log(`wrote ${out} (${png.length} bytes)`);

// Menu-bar template icon: black waveform on transparency, 44x44 (22pt @2x).
// macOS recolors template images for light/dark menu bars.
const TRAY = 44;
const trayBars = [10, 20, 32, 20, 10];
const trayBarW = 4;
const trayBarGap = 3;
const trayRaw = Buffer.alloc((TRAY * 4 + 1) * TRAY);
let toff = 0;
for (let y = 0; y < TRAY; y++) {
  trayRaw[toff++] = 0;
  for (let x = 0; x < TRAY; x++) {
    const totalW = trayBars.length * trayBarW + (trayBars.length - 1) * trayBarGap;
    const startX = (TRAY - totalW) / 2;
    let inside = false;
    for (let i = 0; i < trayBars.length; i++) {
      const x0 = startX + i * (trayBarW + trayBarGap);
      const half = trayBars[i] / 2;
      if (x >= x0 && x < x0 + trayBarW && Math.abs(y - TRAY / 2) <= half) {
        inside = true;
        break;
      }
    }
    trayRaw[toff++] = 0;
    trayRaw[toff++] = 0;
    trayRaw[toff++] = 0;
    trayRaw[toff++] = inside ? 255 : 0;
  }
}
const trayIhdr = Buffer.alloc(13);
trayIhdr.writeUInt32BE(TRAY, 0);
trayIhdr.writeUInt32BE(TRAY, 4);
trayIhdr[8] = 8;
trayIhdr[9] = 6;
const trayPng = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', trayIhdr),
  chunk('IDAT', deflateSync(trayRaw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);
const trayOut = join(root, 'apps', 'desktop', 'src-tauri', 'icons', 'tray.png');
mkdirSync(dirname(trayOut), { recursive: true });
writeFileSync(trayOut, trayPng);
console.log(`wrote ${trayOut} (${trayPng.length} bytes)`);
