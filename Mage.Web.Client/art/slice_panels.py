#!/usr/bin/env python3
"""Cut a grid of opaque painted panels (full-art card styles, art/specs/style-*.json) into one card picture per cell.

    python3 art/slice_panels.py GRID.png --cols 4 --rows 2 --targets out/a.webp:384x536,out/b.webp:360x504,- \
        [--radius 0.045] [--sheet contact.jpg]

A panel grid is asked for as paintings separated by flat gutters, so unlike art/slice.py nothing is keyed: a
painting may use any colour, the gutter's included. Each cell's panel is the run of rows and columns that are mostly
not gutter colour (a painting that happens to hold some of it stays whole); the panel is inset a little to lose its
anti-aliased edge, cover-fitted to the target size and given rounded corners. A target of "-" skips that cell.
"""
import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw


def parse_target(text):
    if text == '-':
        return None
    path, size = text.rsplit(':', 1)
    width, height = size.lower().split('x')
    return Path(path), (int(width), int(height))


def gutter_colour(rgb):
    edges = np.concatenate([rgb[:6].reshape(-1, 3), rgb[-6:].reshape(-1, 3), rgb[:, :6].reshape(-1, 3), rgb[:, -6:].reshape(-1, 3)])
    return np.median(edges, axis=0)


def span(painted, length):
    """The longest run of indexes whose painted share is over half: the panel's extent along one axis."""
    on = painted > 0.5
    best, start = (0, length), None
    best_len = 0
    for index, value in enumerate(list(on) + [False]):
        if value and start is None:
            start = index
        elif not value and start is not None:
            if index - start > best_len:
                best, best_len = (start, index), index - start
            start = None
    return best


def cover(image, size):
    width, height = size
    scale = max(width / image.width, height / image.height)
    resized = image.resize((max(width, round(image.width * scale)), max(height, round(image.height * scale))), Image.LANCZOS)
    left, top = (resized.width - width) // 2, (resized.height - height) // 2
    return resized.crop((left, top, left + width, top + height))


def round_corners(image, radius):
    width, height = image.size
    mask = Image.new('L', (width * 4, height * 4), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, width * 4 - 1, height * 4 - 1), radius=round(width * 4 * radius), fill=255)
    out = image.convert('RGBA')
    out.putalpha(mask.resize((width, height), Image.LANCZOS))
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('grid')
    parser.add_argument('--cols', type=int, required=True)
    parser.add_argument('--rows', type=int, required=True)
    parser.add_argument('--targets', required=True)
    parser.add_argument('--radius', type=float, default=0.045)
    parser.add_argument('--inset', type=float, default=0.012, help='share of the panel trimmed off each edge')
    parser.add_argument('--sheet')
    args = parser.parse_args()

    targets = [parse_target(text) for text in args.targets.split(',')]
    if len(targets) > args.cols * args.rows:
        sys.exit(f'{len(targets)} targets for {args.cols * args.rows} cells')
    grid = Image.open(args.grid).convert('RGB')
    rgb = np.array(grid, dtype=np.float32)
    gutter = np.linalg.norm(rgb - gutter_colour(rgb), axis=-1) < 40
    height, width = gutter.shape
    cut = []
    for index, target in enumerate(targets):
        if target is None:
            continue
        row, col = divmod(index, args.cols)
        x0, x1 = round(width * col / args.cols), round(width * (col + 1) / args.cols)
        y0, y1 = round(height * row / args.rows), round(height * (row + 1) / args.rows)
        cell = ~gutter[y0:y1, x0:x1]
        left, right = span(cell.mean(axis=0), x1 - x0)
        top, bottom = span(cell.mean(axis=1), y1 - y0)
        if right - left < (x1 - x0) * 0.4 or bottom - top < (y1 - y0) * 0.4:
            print(f'error: cell {index + 1} ({target[0].name}) has no panel', file=sys.stderr)
            continue
        inset_x, inset_y = round((right - left) * args.inset), round((bottom - top) * args.inset)
        panel = grid.crop((x0 + left + inset_x, y0 + top + inset_y, x0 + right - inset_x, y0 + bottom - inset_y))
        aspect = panel.width / panel.height
        wanted = target[1][0] / target[1][1]
        if abs(aspect - wanted) / wanted > 0.12:
            print(f'warning: {target[0].name} panel is {aspect:.2f} wide for {wanted:.2f}; cover-fit crops it', file=sys.stderr)
        image = round_corners(cover(panel, target[1]), args.radius)
        target[0].parent.mkdir(parents=True, exist_ok=True)
        image.save(target[0], 'WEBP', quality=90, method=6)
        cut.append(image)
        print(target[0])
    if args.sheet and cut:
        tile_h = max(image.height for image in cut)
        sheet = Image.new('RGB', (sum(image.width for image in cut), tile_h), (20, 30, 26))
        x = 0
        for image in cut:
            sheet.paste(image, (x, 0), image)
            x += image.width
        sheet.save(args.sheet, quality=85)


if __name__ == '__main__':
    main()
