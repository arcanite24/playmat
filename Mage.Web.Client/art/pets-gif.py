"""
The party corgi's 90s "web 1.0" GIF for the /support page (public/support/party-corgi.gif): a starfield, the corgi
spinning, a blinking NEW! burst and rainbow text, doubled with nearest-neighbour and cut to 64 colours.
Usage: python3 art/pets-gif.py <frames dir with f00.png..f23.png, 240x240 RGBA turntable renders> <out.gif>
"""
import math, random, sys
from PIL import Image, ImageDraw, ImageFont
src, out = sys.argv[1], sys.argv[2]
N = 24
W, H = 160, 176
random.seed(1997)
stars = [(random.randrange(W), random.randrange(H), random.randrange(N)) for _ in range(70)]
def burst(d, cx, cy, r1, r2, points, color):
    pts = []
    for i in range(points * 2):
        a = math.pi * i / points - math.pi / 2
        r = r2 if i % 2 == 0 else r1
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    d.polygon(pts, fill=color, outline=(0, 0, 0))

def text(d, xy, s, fill, shadow=(0, 0, 0)):
    x, y = xy
    d.text((x + 1, y + 1), s, font=f, fill=shadow)
    d.text((x, y), s, font=f, fill=fill)

# Pillow's built-in bitmap font: very 1997
f = ImageFont.load_default()

frames = []
for i in range(N):
    im = Image.new('RGB', (W, H), (0, 0, 32))
    d = ImageDraw.Draw(im)
    for x, y, phase in stars:
        on = (i + phase) % 8
        if on < 5:
            d.point((x, y), fill=(255, 255, 255) if on < 2 else (120, 120, 200))
        if on == 0 and phase % 5 == 0:
            d.point([(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)], fill=(255, 255, 160))
    corgi = Image.open(f'{src}/f{i:02d}.png').convert('RGBA').resize((120, 120), Image.NEAREST)
    bob = int(round(abs(math.sin(i / N * math.pi * 4)) * -4))
    im.paste(corgi, (20, 26 + bob), corgi)
    # NEW! burst, blinking
    if i % 6 < 4:
        burst(d, 132, 22, 11, 19, 10, (255, 32, 32) if i % 2 else (255, 230, 0))
        text(d, (121, 16), 'NEW!', (255, 255, 255) if i % 2 else (255, 0, 0))
    # rainbow text, one hue step per frame
    msg = 'THANK U 4 SUPPORTING!!'
    tw = d.textlength(msg, font=f)
    x = (W - tw) / 2
    for k, ch in enumerate(msg):
        h = ((k * 25 + i * 15) % 360) / 60
        c = [(255, int(255 * h), 0), (int(255 * (2 - h)), 255, 0), (0, 255, int(255 * (h - 2))),
             (0, int(255 * (4 - h)), 255), (int(255 * (h - 4)), 0, 255), (255, 0, int(255 * (6 - h)))][int(h) % 6]
        text(d, (x, 156), ch, tuple(max(0, min(255, v)) for v in c))
        x += d.textlength(ch, font=f)
    # a 3D-ish bevel border, Netscape style
    d.rectangle([0, 0, W - 1, H - 1], outline=(192, 192, 192))
    d.line([(1, H - 2), (W - 2, H - 2), (W - 2, 1)], fill=(64, 64, 64))
    big = im.resize((W * 2, H * 2), Image.NEAREST)
    frames.append(big.quantize(colors=64, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE))
frames[0].save(out, save_all=True, append_images=frames[1:], duration=90, loop=0, optimize=True, disposal=1)
print('ok')
