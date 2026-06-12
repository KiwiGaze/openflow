import os

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

SS = 3
B = 1024
C = B * SS
OUT = os.path.dirname(os.path.abspath(__file__))
FONTS = "/Users/mac/.claude/skills/canvas-design/canvas-fonts"

INK = (0.086, 0.075, 0.169)
INK2 = (0.169, 0.145, 0.388)
VIOLET = (0.478, 0.416, 0.878)
PERI = (0.659, 0.627, 0.941)
MIST = (0.812, 0.788, 0.961)
MOON = (0.949, 0.937, 1.000)
AUBERGINE = (0.165, 0.078, 0.188)
AUBERGINE2 = (0.290, 0.125, 0.282)
ROSE = (0.910, 0.686, 0.718)
AMBER = (0.941, 0.788, 0.529)
CREAMWARM = (0.984, 0.910, 0.847)

MOTIF = np.array([22, 5, 12, 1, 20, 1], dtype=float)


def rgba(color, alpha):
    return tuple(int(round(c * 255)) for c in color) + (int(round(alpha * 255)),)


def to_arr(img):
    return np.asarray(img, dtype=np.float32) / 255.0


def to_img(arr):
    return Image.fromarray((np.clip(arr, 0, 1) * 255).astype(np.uint8), "RGBA")


def blank():
    return Image.new("RGBA", (C, C), (0, 0, 0, 0))


def gauss(img, r):
    return img.filter(ImageFilter.GaussianBlur(r))


def grid():
    y, x = np.mgrid[0:C, 0:C].astype(np.float32)
    return x, y


def squircle_alpha(frac=0.43, n=5.0, edge=1.5):
    x, y = grid()
    a = frac * C
    cx = cy = C / 2
    f = (np.abs(x - cx) / a) ** n + (np.abs(y - cy) / a) ** n
    r = f ** (1.0 / n)
    return np.clip((1.0 - r) * a / (edge * SS), 0, 1)


def vgrad(c0, c1, ease=True):
    t = np.linspace(0, 1, C, dtype=np.float32)
    if ease:
        t = t * t * (3 - 2 * t)
    rows = np.outer(1 - t, np.array(c0, np.float32)) + np.outer(t, np.array(c1, np.float32))
    arr = np.repeat(rows[:, None, :], C, axis=1)
    return np.concatenate([arr, np.ones((C, C, 1), np.float32)], axis=2)


def add_radial_light(arr, cx, cy, radius, color, amount):
    x, y = grid()
    d = np.sqrt((x - cx) ** 2 + (y - cy) ** 2)
    w = np.exp(-((d / radius) ** 2)) * amount
    out = arr.copy()
    for i in range(3):
        out[:, :, i] = arr[:, :, i] + (color[i] - arr[:, :, i]) * w
    return out


def mask_from_polygon(points):
    layer = Image.new("L", (C, C), 0)
    ImageDraw.Draw(layer).polygon([(float(px), float(py)) for px, py in points], fill=255)
    return np.asarray(layer, np.float32) / 255.0


def mix_by_mask(base_img, alt_img, mask):
    a, b = to_arr(base_img), to_arr(alt_img)
    m = mask[:, :, None]
    return to_img(a * (1 - m) + b * m)


def bezier(p0, c1, c2, p3, num=400):
    t = np.linspace(0, 1, num)[:, None]
    pts = ((1 - t) ** 3 * np.array(p0) + 3 * (1 - t) ** 2 * t * np.array(c1)
           + 3 * (1 - t) * t ** 2 * np.array(c2) + t ** 3 * np.array(p3))
    return pts * C


def curve_normals(pts):
    d = np.gradient(pts, axis=0)
    n = np.stack([-d[:, 1], d[:, 0]], axis=1)
    n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-9
    return n


def band_polygon(pts, normals, w, lo=-0.5, hi=0.5):
    left = pts + normals * (w * lo)[:, None]
    right = pts + normals * (w * hi)[:, None]
    return [tuple(p) for p in left] + [tuple(p) for p in right[::-1]]


def draw_poly(layer_img, points, color, alpha):
    ImageDraw.Draw(layer_img).polygon([(float(px), float(py)) for px, py in points], fill=rgba(color, alpha))


def finish(content_img, warm_shadow=False, frac=0.43, sh_off=11, sh_blur=14, sh_alpha=0.36):
    arr = to_arr(content_img)
    x, y = grid()
    d = np.sqrt((x - C / 2) ** 2 + (y - C / 2) ** 2)
    vig = np.clip((d / (0.62 * C)) ** 2.2, 0, 1) * 0.10
    arr[:, :, :3] *= (1 - vig)[:, :, None]

    sq = squircle_alpha(frac=frac)
    ring = np.clip(sq - squircle_alpha(frac=frac - 0.0045), 0, 1)
    topw = np.clip(1.15 - y / C * 1.6, 0, 1)
    hl = ring * topw * 0.30
    for i in range(3):
        arr[:, :, i] = arr[:, :, i] + (1.0 - arr[:, :, i]) * hl

    arr[:, :, 3] *= sq
    icon = to_img(arr).resize((B, B), Image.LANCZOS)

    iarr = to_arr(icon)
    rng = np.random.default_rng(7)
    noise = rng.normal(0, 0.0095, (B, B)).astype(np.float32)
    iarr[:, :, :3] = np.clip(iarr[:, :, :3] + noise[:, :, None] * iarr[:, :, 3:4], 0, 1)
    icon = to_img(iarr)

    out = Image.new("RGBA", (B, B), (0, 0, 0, 0))
    sh = Image.new("RGBA", (B, B), (0, 0, 0, 0))
    alpha_img = icon.split()[3]
    a8 = int(sh_alpha * 255)
    tone = (30, 16, 44, a8) if warm_shadow else (12, 10, 30, a8)
    sh.paste(Image.new("RGBA", (B, B), tone), (0, sh_off), alpha_img)
    sh = gauss(sh, sh_blur)
    out = Image.alpha_composite(out, sh)
    out = Image.alpha_composite(out, icon)
    return out


def motif_heights(lo, hi):
    return lo + (hi - lo) * np.sqrt(MOTIF / MOTIF.max())


def candidate_veil():
    bg = vgrad(INK, INK2)
    bg = add_radial_light(bg, 0.34 * C, 0.28 * C, 0.55 * C, VIOLET, 0.30)
    base = to_img(bg)

    h_rel = motif_heights(0.30, 0.74)
    bar_w, gap = 0.052 * C, 0.042 * C
    total = 6 * bar_w + 5 * gap
    x0, cy = (C - total) / 2, 0.50 * C
    zone_h = 0.52 * C

    bars = blank()
    dr = ImageDraw.Draw(bars)
    grad = vgrad(MIST, PERI)
    for i, h in enumerate(h_rel):
        hx = h * zone_h
        x = x0 + i * (bar_w + gap)
        dr.rounded_rectangle([x, cy - hx / 2, x + bar_w, cy + hx / 2], radius=bar_w / 2, fill=(255, 255, 255, 255))
    barr = to_arr(bars)
    barr[:, :, :3] = grad[:, :, :3]
    bars = to_img(barr)

    glow = gauss(bars, 0.014 * C)
    ga = to_arr(glow); ga[:, :, 3] *= 0.5
    base = Image.alpha_composite(base, to_img(ga))

    edge_pts = bezier((0.67, -0.06), (0.45, 0.30), (0.52, 0.72), (0.31, 1.06))
    region = [tuple(p) for p in edge_pts] + [(1.12 * C, 1.06 * C), (1.12 * C, -0.06 * C)]
    vm = mask_from_polygon(region)

    frosted = gauss(bars, 0.017 * C)
    fa = to_arr(frosted)
    fa[:, :, :3] = fa[:, :, :3] * 0.62 + np.array(INK2, np.float32) * 0.38
    fa[:, :, 3] *= 0.92
    bars_mixed = mix_by_mask(bars, to_img(fa), vm)
    base = Image.alpha_composite(base, bars_mixed)

    tint = np.zeros((C, C, 4), np.float32)
    tint[:, :, :3] = np.array(MIST, np.float32)
    x, _ = grid()
    fade = np.clip((x / C - 0.28) / 0.8, 0, 1)
    tint[:, :, 3] = vm * (0.16 - 0.07 * fade)
    base = Image.alpha_composite(base, to_img(tint))

    line = blank()
    ImageDraw.Draw(line).line([tuple(p) for p in edge_pts], fill=rgba(MOON, 0.55), width=int(3.2 * SS), joint="curve")
    soft = gauss(line, 0.008 * C)
    sa = to_arr(soft); sa[:, :, 3] *= 0.5
    base = Image.alpha_composite(base, to_img(sa))
    base = Image.alpha_composite(base, line)
    return finish(base)


def panel(curve, w_top, w_tip, c_top, c_bot, sheen_alpha):
    pts = curve
    nrm = curve_normals(pts)
    t = np.linspace(0, 1, len(pts))
    w = (w_top + (w_tip - w_top) * t ** 1.2) * C

    layer = blank()
    steps = 90
    for k in range(steps):
        a0, a1 = k / steps, (k + 1) / steps
        i0, i1 = int(a0 * (len(pts) - 1)), int(a1 * (len(pts) - 1)) + 1
        col = tuple(np.array(c_top) + (np.array(c_bot) - np.array(c_top)) * ((a0 + a1) / 2))
        seg = band_polygon(pts[i0:i1 + 1], nrm[i0:i1 + 1], w[i0:i1 + 1])
        draw_poly(layer, seg, col, 1.0)

    sheen = blank()
    draw_poly(sheen, band_polygon(pts, nrm, w, lo=-0.12, hi=0.38), MIST, sheen_alpha)
    sheen = gauss(sheen, 0.012 * C)
    layer_arr = to_arr(layer)
    sheen_arr = to_arr(sheen)
    sheen_arr[:, :, 3] *= layer_arr[:, :, 3]
    layer = Image.alpha_composite(layer, to_img(sheen_arr))

    edge = blank()
    draw_poly(edge, band_polygon(pts, nrm, w, lo=-0.50, hi=-0.42), INK, 0.30)
    edge = gauss(edge, 0.003 * C)
    layer = Image.alpha_composite(layer, edge)
    return layer


def candidate_vfold():
    bg = vgrad(INK, (0.125, 0.094, 0.255))
    bg = add_radial_light(bg, 0.50 * C, 0.17 * C, 0.32 * C, MOON, 0.62)
    bg = add_radial_light(bg, 0.50 * C, 0.95 * C, 0.55 * C, INK2, 0.5)
    base = to_img(bg)

    left_curve = bezier((0.215, 0.135), (0.27, 0.46), (0.41, 0.70), (0.500, 0.858))
    right_curve = bezier((0.785, 0.135), (0.73, 0.46), (0.59, 0.70), (0.500, 0.860))

    deep_violet = tuple(np.array(VIOLET) * 0.55 + np.array(INK2) * 0.45)
    left = panel(left_curve, 0.066, 0.024, tuple(np.array(PERI) * 0.88), deep_violet, 0.30)
    base = Image.alpha_composite(base, left)

    right = panel(right_curve, 0.072, 0.026, PERI, tuple(np.array(VIOLET) * 0.85), 0.36)
    shadow = gauss(right.split()[3], 0.012 * C)
    sh = np.zeros((C, C, 4), np.float32)
    sh[:, :, 3] = (np.asarray(shadow, np.float32) / 255.0) * (np.asarray(left.split()[3], np.float32) / 255.0) * 0.55
    base = Image.alpha_composite(base, to_img(sh))
    base = Image.alpha_composite(base, right)
    return finish(base)


def candidate_halo():
    bg = vgrad(tuple(np.array(INK) * 0.8), INK)
    bg = add_radial_light(bg, 0.5 * C, 0.56 * C, 0.45 * C, INK2, 0.55)
    base = to_img(bg)

    cx, cy = 0.5 * C, 0.575 * C
    ring_alpha = [0.80, 0.58, 0.38, 0.22, 0.12]
    for i in range(5):
        r = (0.150 + 0.076 * i) * C
        wdt = (8.5 - 1.1 * i) * 2.2 * SS
        col = tuple(np.array(MIST) + (np.array(PERI) - np.array(MIST)) * (i / 4))
        ring = blank()
        ImageDraw.Draw(ring).ellipse([cx - r, cy - r, cx + r, cy + r], outline=rgba(col, 1.0), width=int(wdt))
        ring = gauss(ring, (1.0 + i * 3.2) * SS)
        ra = to_arr(ring); ra[:, :, 3] *= ring_alpha[i]
        base = Image.alpha_composite(base, to_img(ra))

    core = blank()
    r0 = 0.082 * C
    ImageDraw.Draw(core).ellipse([cx - r0, cy - r0, cx + r0, cy + r0], fill=rgba(MOON, 1.0))
    bloom1 = gauss(core, 0.012 * C); b1 = to_arr(bloom1); b1[:, :, 3] *= 0.85
    bloom2 = gauss(core, 0.05 * C); b2 = to_arr(bloom2); b2[:, :, 3] *= 0.5
    base = Image.alpha_composite(base, to_img(b2))
    base = Image.alpha_composite(base, to_img(b1))
    base = Image.alpha_composite(base, core)
    return finish(base)


def candidate_halflight():
    bg = vgrad(INK, INK2)
    bg = add_radial_light(bg, 0.30 * C, 0.42 * C, 0.5 * C, VIOLET, 0.22)
    base = to_img(bg)

    cx, cy, r = 0.5 * C, 0.5 * C, 0.272 * C
    disc = blank()
    ImageDraw.Draw(disc).ellipse([cx - r, cy - r, cx + r, cy + r], fill=(255, 255, 255, 255))
    da = to_arr(disc)
    x, y = grid()
    t = np.clip(((x - (cx - r)) + (y - (cy - r))) / (4 * r), 0, 1)
    for i in range(3):
        da[:, :, i] = MIST[i] + (VIOLET[i] - MIST[i]) * t
    disc = to_img(da)

    glow = gauss(disc, 0.02 * C)
    ga = to_arr(glow); ga[:, :, 3] *= 0.4
    base = Image.alpha_composite(base, to_img(ga))
    base = Image.alpha_composite(base, disc)

    px = 0.53 * C
    pm = np.clip((x - px) / (1.2 * SS) + 0.5, 0, 1)
    frosted = gauss(base, 0.018 * C)
    fa = to_arr(frosted)
    fa[:, :, :3] = fa[:, :, :3] * 0.82 + np.array(INK2, np.float32) * 0.10
    base = mix_by_mask(base, to_img(fa), pm)

    tint = np.zeros((C, C, 4), np.float32)
    tint[:, :, :3] = np.array(MIST, np.float32)
    ty = np.linspace(0.15, 0.06, C, dtype=np.float32)[:, None]
    tint[:, :, 3] = pm * ty
    base = Image.alpha_composite(base, to_img(tint))

    line = blank()
    ImageDraw.Draw(line).line([(px, 0.10 * C), (px, 0.90 * C)], fill=rgba(MOON, 0.42), width=int(3 * SS))
    lg = gauss(line, 0.007 * C)
    la = to_arr(lg); la[:, :, 3] *= 0.5
    base = Image.alpha_composite(base, to_img(la))
    base = Image.alpha_composite(base, line)
    return finish(base)


def candidate_silkwave():
    bg = vgrad(AUBERGINE, AUBERGINE2)
    bg = add_radial_light(bg, 0.30 * C, 0.78 * C, 0.55 * C, (0.45, 0.20, 0.30), 0.5)
    base = to_img(bg)

    n = 2400
    xs = np.linspace(-0.05 * C, 1.05 * C, n)
    amps = (0.052 + 0.105 * MOTIF / MOTIF.max()) * C
    seg = n // 6
    ys = np.zeros(n)
    th = np.zeros(n)
    for k in range(6):
        s = slice(k * seg, n if k == 5 else (k + 1) * seg)
        u = np.linspace(0, 1, ys[s].shape[0])
        sign = -1 if k % 2 == 0 else 1
        ys[s] = sign * amps[k] * np.sin(np.pi * u)
        th[s] = 0.072 * C + 0.042 * C * np.sin(np.pi * u) * (amps[k] / amps.max())
    kern = np.ones(120) / 120
    ys = np.convolve(ys, kern, mode="same") + 0.50 * C
    th = np.convolve(th, kern, mode="same")

    ribbon = blank()
    steps = 120
    for k in range(steps):
        i0, i1 = int(k / steps * (n - 1)), int((k + 1) / steps * (n - 1)) + 1
        tmid = (k + 0.5) / steps
        col = tuple(np.array(ROSE) + (np.array(AMBER) - np.array(ROSE)) * tmid)
        top = [(xs[i], ys[i]) for i in range(i0, min(i1 + 1, n))]
        bot = [(xs[i], ys[i] + th[i]) for i in range(min(i1, n - 1), i0 - 1, -1)]
        draw_poly(ribbon, top + bot, col, 1.0)

    rsh = gauss(ribbon, 0.018 * C)
    ra = to_arr(rsh); ra[:, :, :3] = np.array(AUBERGINE, np.float32) * 0.5; ra[:, :, 3] *= 0.55
    shifted = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    shifted.paste(to_img(ra), (0, int(0.035 * C)))
    base = Image.alpha_composite(base, shifted)

    wglow = gauss(ribbon, 0.02 * C)
    wa = to_arr(wglow); wa[:, :, 3] *= 0.28
    base = Image.alpha_composite(base, to_img(wa))
    base = Image.alpha_composite(base, ribbon)

    sheen = blank()
    top_edge = [(xs[i], ys[i]) for i in range(n)] + [(xs[i], ys[i] + 0.013 * C) for i in range(n - 1, -1, -1)]
    draw_poly(sheen, top_edge, CREAMWARM, 0.55)
    sheen = gauss(sheen, 0.0035 * C)
    base = Image.alpha_composite(base, sheen)
    return finish(base, warm_shadow=True)


def motif_bar_mask(bar_w, gap_w, zone_h, cy, lo=0.30, hi=0.74):
    h_rel = motif_heights(lo, hi)
    total = 6 * bar_w + 5 * gap_w
    x0 = (C - total) / 2
    layer = Image.new("L", (C, C), 0)
    dr = ImageDraw.Draw(layer)
    for i, h in enumerate(h_rel):
        hx = h * zone_h
        bx = x0 + i * (bar_w + gap_w)
        dr.rounded_rectangle([bx, cy - hx / 2, bx + bar_w, cy + hx / 2], radius=bar_w / 2, fill=255)
    return np.asarray(layer, np.float32) / 255.0


def disc_mask_arr(cx, cy, r, edge=1.5):
    x, y = grid()
    d = np.sqrt((x - cx) ** 2 + (y - cy) ** 2)
    return np.clip((r - d) / (edge * SS), 0, 1)


def candidate_eclipse():
    bg = vgrad(INK, INK2)
    bg = add_radial_light(bg, 0.70 * C, 0.24 * C, 0.50 * C, INK2, 0.5)
    base = to_img(bg)

    cx, cy, r = 0.5 * C, 0.515 * C, 0.245 * C
    dm = disc_mask_arr(cx, cy, r)
    x, y = grid()
    shade = np.clip(((x - (cx - r)) + (y - (cy - r))) / (4 * r), 0, 1)
    disc = np.zeros((C, C, 4), np.float32)
    for i in range(3):
        disc[:, :, i] = MOON[i] + (MIST[i] - MOON[i]) * shade
    disc[:, :, 3] = dm
    disc_img = to_img(disc)

    halo = gauss(disc_img, 0.028 * C)
    ha = to_arr(halo); ha[:, :, 3] *= 0.45
    base = Image.alpha_composite(base, to_img(ha))
    base = Image.alpha_composite(base, disc_img)

    bm = motif_bar_mask(0.052 * C, 0.042 * C, 0.52 * C, cy)
    silhouette = tuple(np.array(INK) * 0.85)
    bars = np.zeros((C, C, 4), np.float32)
    for i in range(3):
        bars[:, :, i] = silhouette[i] * dm + MIST[i] * (1 - dm)
    bars[:, :, 3] = bm

    outer = np.zeros((C, C, 4), np.float32)
    for i in range(3):
        outer[:, :, i] = MIST[i]
    outer[:, :, 3] = bm * (1 - dm)
    og = gauss(to_img(outer), 0.012 * C)
    oa = to_arr(og); oa[:, :, 3] *= 0.5
    base = Image.alpha_composite(base, to_img(oa))
    base = Image.alpha_composite(base, to_img(bars))
    return finish(base)


def candidate_curtain():
    bg = vgrad(INK, tuple(np.array(INK2) * 0.92))
    bg = add_radial_light(bg, 0.5 * C, 0.78 * C, 0.27 * C, MOON, 0.85)
    bg = add_radial_light(bg, 0.5 * C, 0.18 * C, 0.45 * C, INK2, 0.35)
    base = to_img(bg)

    n = 13
    w = 0.026 * C
    span = 0.66 * C
    gap = (span - n * w) / (n - 1)
    x0, top = (C - span) / 2, 0.155 * C

    strands = blank()
    dr = ImageDraw.Draw(strands)
    for i in range(n):
        sx = x0 + i * (w + gap)
        xc = (sx + w / 2) / C
        bell = np.exp(-(((xc - 0.5) / 0.155) ** 2))
        jitter = (MOTIF[i % 6] / 22 - 0.5) * 0.035
        ln = (0.72 - 0.34 * bell + jitter) * C
        dr.rounded_rectangle([sx, top, sx + w, top + ln], radius=w / 2, fill=(255, 255, 255, 236))
    sarr = to_arr(strands)
    grad = vgrad(PERI, VIOLET)
    sarr[:, :, :3] = grad[:, :, :3]
    strands = to_img(sarr)

    sg = gauss(strands, 0.010 * C)
    sga = to_arr(sg); sga[:, :, 3] *= 0.35
    base = Image.alpha_composite(base, to_img(sga))
    base = Image.alpha_composite(base, strands)
    return finish(base)


def candidate_fade():
    bg = vgrad(INK, INK2)
    bg = add_radial_light(bg, 0.30 * C, 0.30 * C, 0.55 * C, VIOLET, 0.30)
    base = to_img(bg)

    h_rel = motif_heights(0.30, 0.74)
    alphas = [1.0, 1.0, 0.80, 0.55, 0.34, 0.20]
    bar_w, gap_w = 0.052 * C, 0.042 * C
    total = 6 * bar_w + 5 * gap_w
    x0, cy, zone_h = (C - total) / 2, 0.50 * C, 0.52 * C

    bars = blank()
    dr = ImageDraw.Draw(bars)
    for i, h in enumerate(h_rel):
        hx = h * zone_h
        bx = x0 + i * (bar_w + gap_w)
        dr.rounded_rectangle([bx, cy - hx / 2, bx + bar_w, cy + hx / 2],
                             radius=bar_w / 2, fill=(255, 255, 255, int(alphas[i] * 255)))
    barr = to_arr(bars)
    grad = vgrad(MIST, PERI)
    barr[:, :, :3] = grad[:, :, :3]
    bars = to_img(barr)

    glow = gauss(bars, 0.014 * C)
    ga = to_arr(glow); ga[:, :, 3] *= 0.5
    base = Image.alpha_composite(base, to_img(ga))
    base = Image.alpha_composite(base, bars)
    return finish(base)


DUSK_PAL = dict(bg0=(0.073, 0.064, 0.144), bg1=INK2, amb=MOON, amb_amt=0.22,
                disc_hi=MIST, disc_lo=VIOLET, halo=0.5, warm=False)

THEMES = [
    ("theme-a-lagoon", "LAGOON", dict(
        bg0=(0.027, 0.090, 0.098), bg1=(0.071, 0.247, 0.267), amb=(0.875, 0.961, 0.941), amb_amt=0.20,
        disc_hi=(0.871, 0.957, 0.933), disc_lo=(0.208, 0.584, 0.541), halo=0.5, warm=False)),
    ("theme-b-noir", "NOIR", dict(
        bg0=(0.055, 0.055, 0.071), bg1=(0.165, 0.165, 0.200), amb=(0.961, 0.937, 0.886), amb_amt=0.18,
        disc_hi=(0.965, 0.945, 0.906), disc_lo=(0.663, 0.651, 0.710), halo=0.45, warm=False)),
    ("theme-c-moss", "MOSS", dict(
        bg0=(0.039, 0.078, 0.051), bg1=(0.122, 0.255, 0.161), amb=(0.918, 0.961, 0.875), amb_amt=0.20,
        disc_hi=(0.910, 0.953, 0.875), disc_lo=(0.341, 0.627, 0.424), halo=0.5, warm=False)),
    ("theme-d-vino", "VINO", dict(
        bg0=(0.110, 0.055, 0.094), bg1=(0.306, 0.129, 0.212), amb=(0.984, 0.929, 0.867), amb_amt=0.22,
        disc_hi=(0.980, 0.925, 0.875), disc_lo=(0.831, 0.518, 0.557), halo=0.5, warm=True)),
    ("theme-e-carta", "CARTA", dict(
        bg0=(0.957, 0.945, 0.910), bg1=(0.898, 0.875, 0.808), amb=(1.0, 1.0, 1.0), amb_amt=0.25,
        disc_hi=(0.353, 0.318, 0.455), disc_lo=(0.102, 0.082, 0.188), halo=0.32, warm=False)),
]


MONO_THEMES = [
    ("mono-a-nero", "NERO", dict(
        bg0=(0.020, 0.020, 0.020), bg1=(0.075, 0.075, 0.075), amb=(1.0, 1.0, 1.0), amb_amt=0.10,
        disc_hi=(1.0, 1.0, 1.0), disc_lo=(0.906, 0.906, 0.906), halo=0.35, warm=False)),
    ("mono-b-graphite", "GRAPHITE", dict(
        bg0=(0.031, 0.035, 0.039), bg1=(0.106, 0.110, 0.125), amb=(0.902, 0.910, 0.941), amb_amt=0.14,
        disc_hi=(0.969, 0.973, 0.973), disc_lo=(0.788, 0.796, 0.820), halo=0.40, warm=False)),
    ("mono-c-bianco", "BIANCO", dict(
        bg0=(0.969, 0.969, 0.965), bg1=(0.910, 0.910, 0.894), amb=(1.0, 1.0, 1.0), amb_amt=0.22,
        disc_hi=(0.235, 0.235, 0.259), disc_lo=(0.043, 0.043, 0.051), halo=0.25, warm=False)),
    ("mono-d-glow", "GLOW", dict(
        bg0=(0.031, 0.035, 0.039), bg1=(0.075, 0.078, 0.098), amb=(0.369, 0.416, 0.824), amb_amt=0.40,
        disc_hi=(0.969, 0.973, 0.973), disc_lo=(0.839, 0.851, 0.894), halo=0.40, warm=False)),
]

LINEA_PAL = dict(bg0=(0.031, 0.035, 0.039), bg1=(0.090, 0.094, 0.110), amb=(0.369, 0.416, 0.824),
                 amb_amt=0.18, disc_hi=(0.969, 0.973, 0.973), disc_lo=(0.969, 0.973, 0.973),
                 halo=0.45, warm=False)


def candidate_core_outline(pal=LINEA_PAL):
    bg = vgrad(pal["bg0"], pal["bg1"])
    bg = add_radial_light(bg, 0.5 * C, 0.5 * C, 0.34 * C, pal["amb"], pal["amb_amt"])
    base = to_img(bg)

    cx, cy, r = 0.5 * C, 0.5 * C, 0.27 * C
    mark = blank()
    dr = ImageDraw.Draw(mark)
    dr.ellipse([cx - r, cy - r, cx + r, cy + r], outline=rgba(pal["disc_hi"], 0.96), width=int(0.0145 * C))
    bm = motif_bar_mask(0.034 * C, 0.027 * C, 0.30 * C, cy)
    bars = np.zeros((C, C, 4), np.float32)
    bars[:, :, :3] = np.array(pal["disc_hi"], np.float32)
    bars[:, :, 3] = bm * 0.96
    mark = Image.alpha_composite(mark, to_img(bars))

    glow = gauss(mark, 0.012 * C)
    ga = to_arr(glow); ga[:, :, 3] *= pal["halo"]
    base = Image.alpha_composite(base, to_img(ga))
    base = Image.alpha_composite(base, mark)
    return finish(base)


def candidate_core(pal=DUSK_PAL, **fin):
    bg = vgrad(pal["bg0"], pal["bg1"])
    bg = add_radial_light(bg, 0.5 * C, 0.5 * C, 0.34 * C, pal["amb"], pal["amb_amt"])
    base = to_img(bg)

    cx, cy, r = 0.5 * C, 0.5 * C, 0.27 * C
    dm = disc_mask_arr(cx, cy, r)
    x, y = grid()
    t = np.clip(((x - (cx - r)) + (y - (cy - r))) / (4 * r), 0, 1)
    disc = np.zeros((C, C, 4), np.float32)
    for i in range(3):
        disc[:, :, i] = pal["disc_hi"][i] + (pal["disc_lo"][i] - pal["disc_hi"][i]) * t
    bm = motif_bar_mask(0.034 * C, 0.027 * C, 0.30 * C, cy)
    disc[:, :, 3] = np.clip(dm - bm, 0, 1)
    disc_img = to_img(disc)

    halo = gauss(disc_img, 0.024 * C)
    ha = to_arr(halo); ha[:, :, 3] *= pal["halo"]
    base = Image.alpha_composite(base, to_img(ha))
    base = Image.alpha_composite(base, disc_img)
    return finish(base, warm_shadow=pal["warm"], **fin)


def rr_mask(cx, cy, wd, ht, rad, angle):
    layer = Image.new("L", (C, C), 0)
    ImageDraw.Draw(layer).rounded_rectangle(
        [cx - wd / 2, cy - ht / 2, cx + wd / 2, cy + ht / 2], radius=rad, fill=255)
    if angle:
        layer = layer.rotate(angle, resample=Image.BICUBIC, center=(cx, cy))
    return np.asarray(layer, np.float32) / 255.0


def candidate_strata():
    bg = vgrad(INK, INK2)
    bg = add_radial_light(bg, 0.28 * C, 0.22 * C, 0.5 * C, VIOLET, 0.25)
    base = to_img(bg)

    wd, ht, rad = 0.58 * C, 0.30 * C, 0.055 * C
    panes = [
        (0.640 * C, -8.0, tuple(np.array(VIOLET) * 0.8), tuple(np.array(INK2) * 1.1), 0.34),
        (0.545 * C, -4.0, PERI, VIOLET, 0.58),
        (0.430 * C, 0.0, MIST, PERI, 1.0),
    ]
    _, y = grid()
    for cyp, ang, ctop, cbot, al in panes:
        m = rr_mask(0.5 * C, cyp, wd, ht, rad, ang)
        sh = np.zeros((C, C, 4), np.float32)
        sh[:, :, :3] = np.array(INK, np.float32) * 0.6
        sh[:, :, 3] = m * 0.45
        sh_img = Image.new("RGBA", (C, C), (0, 0, 0, 0))
        sh_img.paste(to_img(sh), (0, int(0.016 * C)))
        base = Image.alpha_composite(base, gauss(sh_img, 0.012 * C))

        t = np.clip((y - (cyp - ht / 2)) / ht, 0, 1)
        arr = np.zeros((C, C, 4), np.float32)
        for i in range(3):
            arr[:, :, i] = ctop[i] + (cbot[i] - ctop[i]) * t
        arr[:, :, 3] = m * al
        base = Image.alpha_composite(base, to_img(arr))

    bm = motif_bar_mask(0.030 * C, 0.024 * C, 0.17 * C, 0.430 * C)
    bars = np.zeros((C, C, 4), np.float32)
    bars[:, :, :3] = np.array(INK2, np.float32) * 0.85
    bars[:, :, 3] = bm * 0.9
    base = Image.alpha_composite(base, to_img(bars))
    return finish(base)


def text_tracked(draw, xy, text, font, fill, tracking):
    x, y = xy
    for ch in text:
        draw.text((x, y), ch, font=font, fill=fill)
        x += draw.textlength(ch, font=font) + tracking
    return x - tracking


def tracked_width(draw, text, font, tracking):
    return sum(draw.textlength(ch, font=font) for ch in text) + tracking * (len(text) - 1)


def board(icons, names, numerals, study, note):
    W, H = 2200, 1300
    paper = (245, 243, 239, 255)
    inkt = (43, 39, 58, 255)
    gray = (118, 113, 132, 255)
    hair = (208, 204, 214, 255)
    img = Image.new("RGBA", (W, H), paper)
    d = ImageDraw.Draw(img)

    italiana = ImageFont.truetype(f"{FONTS}/Italiana-Regular.ttf", 132)
    serif_it = ImageFont.truetype(f"{FONTS}/InstrumentSerif-Italic.ttf", 34)
    mono = ImageFont.truetype(f"{FONTS}/GeistMono-Regular.ttf", 23)
    mono_s = ImageFont.truetype(f"{FONTS}/GeistMono-Regular.ttf", 17)

    title = "VELATA"
    tw = tracked_width(d, title, italiana, 30)
    text_tracked(d, ((W - tw) / 2, 96), title, italiana, inkt, 30)
    sub = "velata  ·  italian  ·  “veiled”"
    sw = d.textlength(sub, font=serif_it)
    d.text(((W - sw) / 2, 252), sub, font=serif_it, fill=gray)

    d.line([(140, 330), (W - 140, 330)], fill=hair, width=2)
    d.text((140, 344), study, font=mono_s, fill=gray)
    d.text((W - 140 - d.textlength(note, font=mono_s), 344), note, font=mono_s, fill=gray)

    icon_size, gap, x0, y0 = 320, 80, 140, 420
    for i, (icon, name) in enumerate(zip(icons, names)):
        x = x0 + i * (icon_size + gap)
        big = icon.resize((icon_size, icon_size), Image.LANCZOS)
        img.alpha_composite(big, (x, y0))

        label = f"{numerals[i]} — {name}"
        lw = d.textlength(label, font=mono)
        d.text((x + (icon_size - lw) / 2, y0 + icon_size + 38), label, font=mono, fill=inkt)

        m64 = icon.resize((64, 64), Image.LANCZOS)
        m32 = icon.resize((32, 32), Image.LANCZOS)
        mx = x + (icon_size - (64 + 24 + 32)) / 2
        my = y0 + icon_size + 96
        img.alpha_composite(m64, (int(mx), my))
        img.alpha_composite(m32, (int(mx) + 64 + 24, my + 32))
        cap = "64 / 32 px"
        cw = d.textlength(cap, font=mono_s)
        d.text((x + (icon_size - cw) / 2, my + 84), cap, font=mono_s, fill=gray)

    d = ImageDraw.Draw(img)
    d.line([(140, H - 120), (W - 140, H - 120)], fill=hair, width=2)
    d.text((140, H - 100), "VEILED SIGNAL — one mark per canvas · superellipse n=5", font=mono_s, fill=gray)
    ft = "BAR RHYTHM ENCODES V·E·L·A·T·A (22·5·12·1·20·1)"
    d.text((W - 140 - d.textlength(ft, font=mono_s), H - 100), ft, font=mono_s, fill=gray)
    return img


SET1 = [
    ("candidate-I-the-veil", candidate_veil, "THE VEIL"),
    ("candidate-II-v-fold", candidate_vfold, "V-FOLD"),
    ("candidate-III-halo", candidate_halo, "HALO"),
    ("candidate-IV-half-light", candidate_halflight, "HALF-LIGHT"),
    ("candidate-V-silk-wave", candidate_silkwave, "SILK WAVE"),
]
SET2 = [
    ("candidate-VI-eclipse", candidate_eclipse, "ECLIPSE"),
    ("candidate-VII-curtain", candidate_curtain, "CURTAIN"),
    ("candidate-VIII-fade", candidate_fade, "FADE"),
    ("candidate-IX-core", candidate_core, "CORE"),
    ("candidate-X-strata", candidate_strata, "STRATA"),
]


def render_set(makers):
    icons, names = [], []
    for fname, fn, label in makers:
        icon = fn()
        icon.save(f"{OUT}/{fname}.png")
        icons.append(icon)
        names.append(label)
        print("rendered", fname)
    return icons, names


def tray_template(size=44, scale=8):
    s = size * scale
    cx = cy = s / 2

    disc = Image.new("L", (s, s), 0)
    r = 18 * scale
    ImageDraw.Draw(disc).ellipse([cx - r, cy - r, cx + r, cy + r], fill=255)

    bars = Image.new("L", (s, s), 0)
    db = ImageDraw.Draw(bars)
    h_rel = motif_heights(0.30, 0.74)
    bw, gap = 2.8 * scale, 1.9 * scale
    total = 6 * bw + 5 * gap
    x0, zone = (s - total) / 2, 20 * scale
    for i, h in enumerate(h_rel):
        hx = h * zone
        bx = x0 + i * (bw + gap)
        db.rounded_rectangle([bx, cy - hx / 2, bx + bw, cy + hx / 2], radius=bw / 2, fill=255)

    alpha = np.clip(
        np.asarray(disc, np.float32) - np.asarray(bars, np.float32), 0, 255
    ).astype(np.uint8)
    arr = np.zeros((s, s, 4), np.uint8)
    arr[:, :, 3] = alpha
    return Image.fromarray(arr, "RGBA").resize((size, size), Image.LANCZOS)


ICONSET = [
    ("icon_16x16.png", 16), ("icon_16x16@2x.png", 32), ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64), ("icon_128x128.png", 128), ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256), ("icon_256x256@2x.png", 512), ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]
TAURI_PNGS = [("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128),
              ("128x128@2x.png", 256), ("icon.png", 512)]


def production():
    pal = dict(MONO_THEMES[3][2])
    pdir = f"{OUT}/prod"
    os.makedirs(f"{pdir}/velata.iconset", exist_ok=True)

    master = candidate_core(pal, frac=0.402, sh_off=10, sh_blur=22, sh_alpha=0.30)
    master.save(f"{pdir}/icon-1024.png")
    for name, px in ICONSET:
        master.resize((px, px), Image.LANCZOS).save(f"{pdir}/velata.iconset/{name}")
    for name, px in TAURI_PNGS:
        master.resize((px, px), Image.LANCZOS).save(f"{pdir}/{name}")
    master.resize((256, 256), Image.LANCZOS).save(
        f"{pdir}/icon.ico",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    tray_template().save(f"{pdir}/tray.png")
    print("production assets in", pdir)


if __name__ == "__main__":
    import sys
    which = sys.argv[1] if len(sys.argv) > 1 else "all"
    if which == "prod":
        production()
        sys.exit(0)
    if which in ("all", "1"):
        icons, names = render_set(SET1)
        board(icons, names, ["I", "II", "III", "IV", "V"],
              "ICON STUDY 01", "FIVE CANDIDATES — DUSK REGISTER").save(f"{OUT}/velata-icon-candidates.png")
        print("board 01 done")
    if which in ("all", "2"):
        icons, names = render_set(SET2)
        board(icons, names, ["VI", "VII", "VIII", "IX", "X"],
              "ICON STUDY 02", "THE VEIL UNFROSTED · WORKSPACE MARKS").save(f"{OUT}/velata-icon-candidates-2.png")
        print("board 02 done")
    if which in ("all", "3"):
        icons, names = [], []
        for fname, label, pal in THEMES:
            icon = candidate_core(pal)
            icon.save(f"{OUT}/{fname}.png")
            icons.append(icon)
            names.append(label)
            print("rendered", fname)
        board(icons, names, ["A", "B", "C", "D", "E"],
              "ICON STUDY 03", "COLOR REGISTERS — MARK IX · CORE").save(f"{OUT}/velata-icon-colors.png")
        print("board 03 done")
    if which in ("all", "4"):
        icons, names = [], []
        for fname, label, pal in MONO_THEMES:
            icon = candidate_core(pal)
            icon.save(f"{OUT}/{fname}.png")
            icons.append(icon)
            names.append(label)
            print("rendered", fname)
        icon = candidate_core_outline()
        icon.save(f"{OUT}/mono-e-linea.png")
        icons.append(icon)
        names.append("LINEA")
        print("rendered mono-e-linea")
        board(icons, names, ["A", "B", "C", "D", "E"],
              "ICON STUDY 04", "MONOCHROME · LINEAR REGISTER").save(f"{OUT}/velata-icon-mono.png")
        print("board 04 done")
