#!/usr/bin/env python3
"""Fit sliced ring pictures (plate frames, art/specs/plate-frame.json) to one geometry: every ring is centred on its
hole and scaled so the hole (the clear circle inside it) is the same share of the picture, so PlateFrame can lay any of them over an avatar disc
with one rule (LookParts.module.css .framePainted). A ring whose ornaments reach too far is scaled down to fit.

    python3 art/fit_rings.py public/career-art/plate-frame/*.webp [--hole 0.34]
"""
import argparse
import numpy as np
from PIL import Image
from scipy import ndimage


def fit(path, hole_share):
    image = Image.open(path).convert('RGBA')
    size = image.size[0]
    alpha = np.array(image)[..., 3]
    # the hole: the clear circle round the ring's centre (a wreath open at the top has no closed hole to count)
    painted_y, painted_x = np.where(alpha >= 64)
    cx, cy = (painted_x.min() + painted_x.max()) / 2, (painted_y.min() + painted_y.max()) / 2
    clearance = ndimage.distance_transform_edt(alpha < 64)
    hole = 2 * clearance[round(cy), round(cx)]
    if hole < size * 0.2:
        raise SystemExit(f'{path}: no clear hole round the centre')
    reach = np.sqrt((painted_x - cx) ** 2 + (painted_y - cy) ** 2).max()
    scale = min(hole_share * size / hole, 0.49 * size / reach)
    # move the hole's centre to the picture's, then scale about it
    out = image.transform(
        (size, size), Image.AFFINE,
        (1 / scale, 0, cx - size / 2 / scale, 0, 1 / scale, cy - size / 2 / scale),
        resample=Image.BICUBIC)
    out.save(path, 'WEBP', quality=90, method=6)
    return hole * scale / size


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('files', nargs='+')
    parser.add_argument('--hole', type=float, default=0.34, help="the hole's width as a share of the picture")
    args = parser.parse_args()
    for path in args.files:
        print(f'{path}  hole {fit(path, args.hole):.3f}')


if __name__ == '__main__':
    main()
