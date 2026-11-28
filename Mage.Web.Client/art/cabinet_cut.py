#!/usr/bin/env python3
"""Cuts one painted cabinet part (art/cabinet.mjs) off its magenta ground and writes the shipped WebP.

    python3 art/cabinet_cut.py <answer.png> <out.webp> <width> [--wheel] [--grey]

Alpha comes from how magenta a pixel is (red and blue high, green low): brass, enamel and dark glass all have one of
red or blue low, so they stay solid while the ground goes. Half-covered edge pixels are unmixed from the ground's
colour so no pink fringe is left. The picture is trimmed to the object and scaled to <width>. --wheel centres the square on a round part's rim (its
width sets the circle; a grip sticking out at the top doesn't move the middle). --grey keeps only the light, so the
client can tint the part (a capsule takes its set's colour).
"""
import sys

import numpy as np
from PIL import Image


def main():
    source, out, width = sys.argv[1], sys.argv[2], int(sys.argv[3])
    wheel = '--wheel' in sys.argv
    rgb = np.asarray(Image.open(source).convert('RGB')).astype(np.float32)
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    magenta = np.minimum(r, b) - g
    # the ground's own colour, from the border
    edge = np.concatenate([rgb[:8].reshape(-1, 3), rgb[-8:].reshape(-1, 3), rgb[:, :8].reshape(-1, 3), rgb[:, -8:].reshape(-1, 3)])
    ground = np.median(edge, axis=0)
    full = float(np.minimum(ground[0], ground[2]) - ground[1])
    alpha = 1 - np.clip((magenta - full * 0.25) / (full * 0.6), 0, 1)
    # unmix the ground out of the edge pixels
    a = np.maximum(alpha, 1e-3)[..., None]
    colour = np.clip((rgb - (1 - a) * ground) / a, 0, 255)
    colour = np.where(alpha[..., None] > 0.98, rgb, colour)
    if '--grey' in sys.argv:
        light = colour @ np.array([0.299, 0.587, 0.114], dtype=np.float32)
        colour = np.dstack([light * 1.0, light * 0.985, light * 0.95])
    rgba = np.dstack([colour, alpha * 255]).astype(np.uint8)
    image = Image.fromarray(rgba, 'RGBA')

    ys, xs = np.nonzero(alpha > 0.5)
    left, right, top, bottom = xs.min(), xs.max() + 1, ys.min(), ys.max() + 1
    if wheel:
        cx, cy = (left + right) / 2, bottom - (right - left) / 2
        half = max((right - left) / 2, cy - top, bottom - cy) + 4
        box = (int(cx - half), int(cy - half), int(cx + half), int(cy + half))
    else:
        box = (left - 4, top - 4, right + 4, bottom + 4)
    image = image.crop(box)
    height = round(image.height * width / image.width)
    image.resize((width, height), Image.LANCZOS).save(out, 'WEBP', quality=88, method=6)
    print(f'{out}: {width}x{height} (cut {box} of {rgb.shape[1]}x{rgb.shape[0]})')


if __name__ == '__main__':
    main()
