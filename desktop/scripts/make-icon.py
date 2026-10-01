#!/usr/bin/env python3
"""Generate the XArchive application icon set.

The icon is described once, here, as vector primitives so the artwork stays
maintainable and every size is rendered from the same source instead of being
hand-tuned per size. The shapes are deliberately simple: a green rounded
square, a white archive box, and an `X` mark, which stays legible at 16 px
where fine detail would turn to noise.

Outputs into `desktop/src-tauri/icons`:
  * `icon.png`   512x512 master
  * `icon.ico`   16/24/32/48/64/128/256 px in one ICO resource
  * `32x32.png`, `128x128.png`, `128x128@2x.png` for the bundler and tray

Usage:  python3 desktop/scripts/make-icon.py
Requires Pillow. The generated files are committed, so this script does not
need to run during a normal build.
"""

from __future__ import annotations

import pathlib
import struct

from PIL import Image, ImageDraw

# Brand green, matching `--success` and the sidebar brand mark in style.css.
BRAND_GREEN = (14, 127, 69, 255)
WHITE = (255, 255, 255, 255)

SIZES = (16, 24, 32, 48, 64, 128, 256)
MASTER = 512


def rounded_square(size: int) -> Image.Image:
    """A rounded green plate with a transparent margin, like an app tile."""
    # The plate occupies most of the canvas but leaves a small transparent
    # border so the icon does not look cropped in a taskbar.
    #
    # The plate is one flat green on purpose. An earlier two-tone band behind
    # the box read as two separate color blocks at 16 px, where the edge landed
    # across the box and made the mark look noisy.
    inset = round(size * 0.06)
    radius = round(size * 0.22)
    plate = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(plate)
    draw.rounded_rectangle(
        [inset, inset, size - inset - 1, size - inset - 1],
        radius=radius,
        fill=BRAND_GREEN,
    )
    return plate


def draw_archive_box(size: int) -> Image.Image:
    """Overlay a white archive box with an `X` cut into its lid."""
    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)

    # Box body.
    left = round(size * 0.24)
    right = round(size * 0.76)
    top = round(size * 0.36)
    bottom = round(size * 0.70)
    draw.rounded_rectangle([left, top, right, bottom], radius=round(size * 0.05), fill=WHITE)

    # Lid, slightly wider than the body so the silhouette stays readable.
    lid_left = round(size * 0.19)
    lid_right = round(size * 0.81)
    lid_top = round(size * 0.26)
    lid_bottom = round(size * 0.40)
    draw.rounded_rectangle(
        [lid_left, lid_top, lid_right, lid_bottom], radius=round(size * 0.05), fill=WHITE
    )

    return layer


def draw_x_mark(size: int) -> Image.Image:
    """A green `X` on the white box body, cut out with transparency.

    Drawing it as a transparent overlay composited over the box keeps one
    source of truth for the mark instead of painting a second white shape.
    """
    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)

    stroke = max(1, round(size * 0.07))
    cx = size / 2
    cy = (size * 0.36 + size * 0.70) / 2
    arm = size * 0.085
    draw.line(
        [(cx - arm, cy - arm), (cx + arm, cy + arm)],
        fill=BRAND_GREEN,
        width=stroke,
    )
    draw.line(
        [(cx + arm, cy - arm), (cx - arm, cy + arm)],
        fill=BRAND_GREEN,
        width=stroke,
    )
    return layer


def render(size: int) -> Image.Image:
    plate = rounded_square(size)
    # Supersample and downscale so the rounded corners and the small `X` do not
    # turn jagged at 16 px.
    factor = 8 if size <= 64 else 4
    large = plate.resize((size * factor, size * factor), Image.LANCZOS)
    box = draw_archive_box(size * factor)
    mark = draw_x_mark(size * factor)
    composed = Image.alpha_composite(large, box)
    composed = Image.alpha_composite(composed, mark)
    return composed.resize((size, size), Image.LANCZOS)


def write_ico(images: list[Image.Image], path: pathlib.Path) -> None:
    """Write a multi-size ICO.

    Windows picks the best matching entry from the resource, so every size is
    stored as its own PNG-compressed image rather than resizing one bitmap.
    """
    payloads = []
    for image in images:
        from io import BytesIO

        buffer = BytesIO()
        image.save(buffer, format="PNG")
        payloads.append(buffer.getvalue())

    count = len(payloads)
    header = struct.pack("<HHH", 0, 1, count)
    offset = 6 + 16 * count
    directory = b""
    for image, payload in zip(images, payloads):
        width = 0 if image.width >= 256 else image.width
        height = 0 if image.height >= 256 else image.height
        directory += struct.pack(
            "<BBBBHHII",
            width,
            height,
            0,  # palette size
            0,  # reserved
            1,  # color planes
            32,  # bits per pixel
            len(payload),
            offset,
        )
        offset += len(payload)

    path.write_bytes(header + directory + b"".join(payloads))


def main() -> None:
    icons = pathlib.Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
    icons.mkdir(parents=True, exist_ok=True)

    master = render(MASTER)
    master.save(icons / "icon.png")

    rendered = [render(size) for size in SIZES]
    write_ico(rendered, icons / "icon.ico")

    by_size = dict(zip(SIZES, rendered))
    by_size[32].save(icons / "32x32.png")
    by_size[128].save(icons / "128x128.png")
    by_size[256].save(icons / "128x128@2x.png")

    for name in ("icon.png", "icon.ico", "32x32.png", "128x128.png", "128x128@2x.png"):
        print(f"wrote {icons / name}")


if __name__ == "__main__":
    main()
