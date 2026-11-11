#!/usr/bin/env python3
"""Generate the gm-desktop application icon set.

WHY THIS EXISTS
---------------
`src-tauri/icons/` originally held 1x1-pixel placeholders (69-76 bytes each).
They did not block `tauri build` - the bundler accepted them - but every
surface that shows an icon (taskbar, Start menu, Alt-Tab, the tray) rendered
blank, and a blank taskbar entry reads as a broken install to anyone who sees
it.

WHAT IT PRODUCES
----------------
* `icons/icon-source.png` - a 1024x1024 RGBA master. This is the input to
  Tauri's own icon generator (`pnpm tauri icon icons/icon-source.png`), which
  is what produces the correctly-sized PNGs plus `icon.ico` and `icon.icns`.
  Tauri's generator is used rather than hand-rolling the multi-resolution
  containers, because `.ico` and `.icns` have binary layouts that are easy to
  get subtly wrong and are exactly the kind of thing that should come from the
  tool that has to read them back.

* `icons/tray.png` - 64x64 RGBA, drawn for a tray rather than for a taskbar.
  It is referenced directly by `src-tauri/lib.rs` via `include_bytes!` and is
  NOT part of Tauri's generated set, so it is written here.

THE MARK
--------
A commit graph: one trunk with a branch peeling off near the top, rendered as
filled nodes joined by thick strokes. The geometry is deliberately simple and
the strokes deliberately heavy (48/1024 of the canvas) because the smallest
shipped size is 32x32, where a hairline stroke disappears entirely.

REPLACING THIS
--------------
If real brand artwork arrives, delete this script, drop the artwork in as
`icons/icon-source.png`, re-run `pnpm tauri icon`, and nothing else in the
build needs to change. No code references these files by content.
"""

from __future__ import annotations

import os
from PIL import Image, ImageDraw

# Draw at 4x and downscale: ImageDraw has no antialiasing, and the shipped
# sizes are small enough that jaggies would be plainly visible.
SS = 4
CANVAS = 1024

# Two-stop vertical gradient, deep indigo into near-black slate. Dark enough
# that a white mark stays legible on both light and dark taskbars, which is
# the constraint that matters for an icon that has to work unconfigured.
GRAD_TOP = (45, 58, 140)      # #2D3A8C
GRAD_BOTTOM = (15, 23, 42)    # #0F172A

MARK = (255, 255, 255, 255)


def _lerp(a: tuple[int, ...], b: tuple[int, ...], t: float) -> tuple[int, ...]:
    return tuple(round(av + (bv - av) * t) for av, bv in zip(a, b))


def _rounded_gradient(size: int, radius_ratio: float = 0.22) -> Image.Image:
    """A rounded square filled with a vertical gradient.

    Windows draws the taskbar/Start icon itself and applies its own mask, but
    the rounded corners here are what make the icon read as intentional rather
    than as a square photo cropped badly.
    """
    grad = Image.new("RGBA", (1, size))
    for y in range(size):
        grad.putpixel((0, y), _lerp(GRAD_TOP, GRAD_BOTTOM, y / max(size - 1, 1)) + (255,))
    grad = grad.resize((size, size))

    mask = Image.new("L", (size * SS, size * SS), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, size * SS - 1, size * SS - 1],
        radius=int(size * SS * radius_ratio),
        fill=255,
    )
    mask = mask.resize((size, size), Image.LANCZOS)

    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.paste(grad, (0, 0), mask)
    return out


def draw_mark(img: Image.Image) -> None:
    """Draw the commit-graph mark, in canvas coordinates on `img`."""
    d = ImageDraw.Draw(img)
    s = SS
    stroke = int(48 * s)
    node_r = int(76 * s)

    trunk_x = int(400 * s)
    top_y = int(300 * s)
    mid_y = int(512 * s)
    bot_y = int(724 * s)
    branch_x = int(640 * s)
    branch_end_y = int(724 * s)

    # Trunk and branch. Drawn first so the node fills sit on top and hide the
    # stroke ends cleanly, rather than butting against them.
    d.line([(trunk_x, top_y), (trunk_x, bot_y)], fill=MARK, width=stroke)
    d.line([(trunk_x, top_y), (branch_x, top_y)], fill=MARK, width=stroke)
    d.line([(branch_x, top_y), (branch_x, branch_end_y)], fill=MARK, width=stroke)

    for cx, cy in ((trunk_x, top_y), (trunk_x, mid_y), (trunk_x, bot_y),
                   (branch_x, top_y), (branch_x, branch_end_y)):
        d.ellipse([cx - node_r, cy - node_r, cx + node_r, cy + node_r], fill=MARK)


def build_app_icon() -> Image.Image:
    big = _rounded_gradient(CANVAS * SS)
    draw_mark(big)
    return big.resize((CANVAS, CANVAS), Image.LANCZOS)


def build_tray_icon() -> Image.Image:
    """64x64 RGBA on transparency.

    A tray icon needs a real alpha channel: the taskbar supplies its own
    background, and an opaque square would show as a visible block against it.
    The rounded-square plate is therefore omitted entirely here - just the mark.
    """
    size = 64
    big = Image.new("RGBA", (size * SS, size * SS), (0, 0, 0, 0))
    d = ImageDraw.Draw(big)

    # Same geometry as the app icon, scaled to the tray canvas, and shifted up
    # slightly because a tray glyph reads better optically centred than
    # mathematically centred.
    k = size * SS / CANVAS
    stroke = max(1, int(48 * k))
    node_r = max(1, int(76 * k))
    pts = [(400, 300), (400, 512), (400, 724), (640, 300), (640, 724)]
    ox = oy = 0
    sx = lambda x: int((x * k) + ox)  # noqa: E731
    sy = lambda y: int((y * k) + oy)  # noqa: E731

    d.line([(sx(400), sy(300)), (sx(400), sy(724))], fill=MARK, width=stroke)
    d.line([(sx(400), sy(300)), (sx(640), sy(300))], fill=MARK, width=stroke)
    d.line([(sx(640), sy(300)), (sx(640), sy(724))], fill=MARK, width=stroke)
    for cx, cy in pts:
        d.ellipse([sx(cx) - node_r, sy(cy) - node_r, sx(cx) + node_r, sy(cy) + node_r],
                  fill=MARK)
    return big.resize((size, size), Image.LANCZOS)


def main() -> None:
    here = os.path.dirname(os.path.abspath(__file__))
    icons = os.path.normpath(os.path.join(here, "..", "src-tauri", "icons"))
    os.makedirs(icons, exist_ok=True)

    source = os.path.join(icons, "icon-source.png")
    build_app_icon().save(source, "PNG", optimize=True)

    tray = os.path.join(icons, "tray.png")
    build_tray_icon().save(tray, "PNG", optimize=True)

    for p in (source, tray):
        print(f"wrote {p} ({os.path.getsize(p)} bytes)")


if __name__ == "__main__":
    main()
