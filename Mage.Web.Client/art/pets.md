# Pets

Low-poly 3D companions for the battlefield (`src/app/pets`), served from `public/pets/*.glb`. Unlike the painted art
here, they come from Retro Diffusion's Low-Poly v2 API through the voxel-automation repo's runner
(`npm run models:rd -- generate ...` in `/Volumes/Cosos/code/voxel-automation`), copied in as generated.

The scene animates parts by node name: `tail`, `head`, `ear`/`ear~x` (with `inner_ear`/`inner_ear~x`), `hat`, `pom`.
A model without one of them just doesn't move that part.

| Pet | File | Run | Prompt | Settings | Cost |
| --- | --- | --- | --- | --- | ---: |
| Debug duck (`debug-duck`), the contributors' pet | `public/pets/debug-duck.glb` | `playmat-debug-duck-v1`, asset `659ff48c-50d3-48d7-b859-952d5362d82e` | Cute yellow rubber duck sitting upright, wearing a tiny orange hard hat with a headlamp, holding a small magnifying glass under one wing, big friendly eyes, orange beak. Cute mascot, chunky low poly, flat colors, cozy game. | size 64, `rd_lowpoly__detailed`, quality | $0.85 |
| Party corgi (`party-corgi`) | `public/pets/party-corgi.glb` | `playmat-corgi-party-v1`, asset `7b8291ec-11ab-48e8-9549-73e9314266b9` | Corgi dog sitting proudly, orange and white fur, big upright ears, wearing a pointy striped party hat with a pom-pom and black sunglasses. Cute mascot, chunky low poly, flat colors, cozy game. | size 64, `rd_lowpoly__detailed`, quality | $0.85 |

The duck's locker still (`public/pets/debug-duck-still.png`, 320×352, transparent) is one frame of `PetScene` at idle,
read back from the canvas and fitted with a 16px margin.

## The /support GIF

`public/support/party-corgi.gif` (and its still, `party-corgi-still.png`, for reduced motion) is the corgi as a 1997
homepage would show it. Frames: 24 transparent 240×240 turntable renders of the GLB (three.js in the dev server, driven
by Playwright); then `python3 art/pets-gif.py <frames dir> public/support/party-corgi.gif`.
