# Velata brand guidelines

Velata (Italian: _veiled_) is a local-first, privacy-first voice workspace. The brand's one
idea: **the signal is present, but held** — luminous work behind a quiet layer. The full
aesthetic manifesto lives in `velata-icon/velata-icon-philosophy.md` ("Veiled Signal").

## Name

- Product and company name: **Velata**. Domain: **velata.app**. Never "VelataApp" or "Velata.app"
  in prose — the name is the word, the domain is the address.

## The mark

The app icon is the **Core** mark in the **Glow** register: a silver disc with the waveform
carved through it, on layered graphite, lit from behind by periwinkle.

- The carved bar rhythm is not decoration: bar heights encode **V·E·L·A·T·A** by alphabet
  position (22·5·12·1·20·1). Keep this rhythm in any waveform the brand draws (HUD, site, docs).
- The icon stays monochrome. No color inside the mark; light may glow **behind** it.
- The menu-bar icon is the Core silhouette — a solid disc with the six-bar motif carved through
  it — as a macOS template image (black + alpha; the system recolors it per menu-bar appearance).
- Source of truth: `docs/design/velata-icon/render.py` (`python render.py prod` regenerates every
  asset; needs a venv with Pillow + numpy). Master: `velata-icon/prod/icon-1024.png`.

## Color

Graphite scale (surfaces, marketing backdrops):

| Token      | Hex       | Use                              |
| ---------- | --------- | -------------------------------- |
| graphite-0 | `#08090A` | darkest backdrop, icon top       |
| graphite-1 | `#1B1C20` | icon base, dark surfaces         |
| silver-0   | `#F7F8F8` | the mark, light text on graphite |
| silver-1   | `#C9CBD1` | secondary silver, disc shading   |

Accent — **periwinkle `#5E6AD2`** (the one color the brand owns):

- UI tokens: `--accent: #5e6ad2` (light), `#7e89e8` (dark); hovers `#4d59c0` / `#97a0ef`;
  HUD bars `#9aa4f2` (`apps/desktop/src/app/styles.css`).
- Rule: the accent appears **behind or around** the brand (glow, focus, record state, links) —
  never inside the mark, never as a fill for the wordmark.
- The retired dusk-violet palette and the warm "Vino" register live on only as illustration
  options inside `render.py`; they are not product colors.

## Typography

- Wordmark: **Italiana** (display serif), generous tracking, all caps — `V E L A T A`.
- Product UI keeps the existing system font stack; do not bring Italiana into the app.
- Specimen/diagram labels (site, boards): a quiet monospace (Geist Mono or equivalent).

## Voice

Quiet, precise, a little Italian warmth. Say what the product does not do ("your audio never
leaves this Mac") as plainly as what it does. No exclamation marks, no AI sparkle clichés,
no "supercharge". The veil is the promise: presence without exposure.
