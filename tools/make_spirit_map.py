#!/usr/bin/env python3
"""Turn the walkable ground level of the GPL Quake 2 map spirit2dm9 (by spirit) into a tile map.

    python3 tools/make_spirit_map.py REFERENCE_tobe_DELETED/spirit2dm9/maps/spirit2dm9.map > src/board/map_spirit.rs

Only brush geometry and entity positions are read (one tile = 128 map units = 4 m, one height step =
16 units = 0.5 m); the multi-level map is reduced to the part you can walk from a ground spawn
without jumping, jump pads or teleporters. No textures/sounds are used. See CREDITS.md.
"""
import sys, collections, statistics
sys.path.insert(0, __file__.rsplit('/', 1)[0])
import q2map

T = 8          # cells (16 units) per tile
HEAD = 4       # cells of head room
path = sys.argv[1]
ents = q2map.parse(path)
solid = q2map.voxelise(ents)
col = collections.defaultdict(set)
for i, j, k in solid: col[(i, j)].add(k)
surf = {(i, j, k) for (i, j), c in col.items() for k in c if all((k + d) not in c for d in range(1, HEAD + 1))}

def nb(n):
    i, j, k = n
    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        for dk in (-1, 0, 1):
            m = (i + dx, j + dy, k + dk)
            if m in surf: yield m

start_ent = next(e for e in ents[1:] if e['classname'] == 'info_player_deathmatch' and e['origin'] == '0 -256 14')
sx, sy, sz = q2map.origin(start_ent)
st = min((s for s in surf if s[0] == int(sx // 16) and s[1] == int(sy // 16)), key=lambda s: abs(s[2] * 16 - sz))
seen = {st}; q = collections.deque([st])
while q:
    n = q.popleft()
    for m in nb(n):
        if m not in seen: seen.add(m); q.append(m)

tiles = collections.defaultdict(list)
for i, j, k in seen: tiles[(i // T, j // T)].append(k + 1)
walk = {t: int(statistics.median(ks)) for t, ks in tiles.items() if len(ks) >= 12}
# keep the connected part, then relax heights so neighbours differ by at most one step
start_tile = (st[0] // T, st[1] // T)
comp = {start_tile}; q = collections.deque([start_tile])
while q:
    x, y = q.popleft()
    for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        n = (x + d[0], y + d[1])
        if n in walk and n not in comp: comp.add(n); q.append(n)
base = min(walk[t] for t in comp)
h = {t: walk[t] - base for t in comp}
changed = True
while changed:
    changed = False
    for (x, y) in comp:
        for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            n = (x + d[0], y + d[1])
            if n in h and h[(x, y)] > h[n] + 1:
                h[(x, y)] = h[n] + 1; changed = True
h = {t: min(v, 35) for t, v in h.items()}

xs = [t[0] for t in comp]; ys = [t[1] for t in comp]
x0, y0 = min(xs) - 1, min(ys) - 1
W, H = max(xs) - min(xs) + 3, max(ys) - min(ys) + 3
loc = lambda t: (t[0] - x0, t[1] - y0)
floor = {loc(t) for t in comp}

def full3(c):
    return all((c[0] + dx, c[1] + dy) in floor for dx in (-1, 0, 1) for dy in (-1, 0, 1))
# airlock: nearest fully walkable 3x3 around the spawn tile
sc = loc(start_tile)
air = min((c for c in floor if full3(c)), key=lambda c: abs(c[0] - sc[0]) + abs(c[1] - sc[1]))
# distances for the target room
dist = {air: 0}; q = collections.deque([air])
while q:
    c = q.popleft()
    for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        n = (c[0] + d[0], c[1] + d[1])
        if n in floor and n not in dist: dist[n] = dist[c] + 1; q.append(n)
tgt = max((c for c in floor if full3(c) and abs(c[0] - air[0]) + abs(c[1] - air[1]) > 6), key=lambda c: dist[c])
far = max(dist.values())

layout = [['#'] * W for _ in range(H)]
things = [['.'] * W for _ in range(H)]
heights = [['0'] * W for _ in range(H)]
digit = lambda v: '0123456789abcdefghijklmnopqrstuvwxyz'[v]
for (x, y) in floor:
    layout[y][x] = 'SBA'[((x // 4) + 2 * (y // 4)) % 3]
    heights[y][x] = digit(h[(x + x0, y + y0)])
for dx in (-1, 0, 1):
    for dy in (-1, 0, 1):
        layout[air[1] + dy][air[0] + dx] = 'L'
        layout[tgt[1] + dy][tgt[0] + dx] = 'T'
things[air[1]][air[0]] = '@'
things[air[1] - 1][air[0] - 1] = '!'
things[tgt[1]][tgt[0]] = 'G'
occupied = {air, tgt, (air[0] - 1, air[1] - 1)}

def put(c, ch):
    if c in floor and c not in occupied and not (abs(c[0] - air[0]) <= 1 and abs(c[1] - air[1]) <= 1):
        things[c[1]][c[0]] = ch; occupied.add(c); return True
    return False
tile_of = lambda e: loc((int(q2map.origin(e)[0] // 128), int(q2map.origin(e)[1] // 128)))
ITEMS = {'weapon_shotgun': 'w', 'weapon_supershotgun': 'w', 'weapon_machinegun': 'n', 'weapon_chaingun': 'n',
         'weapon_rocketlauncher': 'o', 'weapon_grenadelauncher': 'o', 'weapon_railgun': 'v', 'weapon_hyperblaster': 'v',
         'item_health_small': 'h', 'item_health_large': 'H', 'item_health_mega': 'H', 'item_armor_shard': 's',
         'item_armor_jacket': 'a', 'item_armor_combat': 'a', 'item_armor_body': 'A', 'ammo_shells': 'e',
         'ammo_bullets': 'u', 'ammo_rockets': 'r', 'ammo_slugs': 'l', 'ammo_cells': 'u', 'ammo_grenades': 'r'}
for e in ents[1:]:
    c = e['classname']
    if 'origin' not in e: continue
    t = tile_of(e)
    if c in ITEMS: put(t, ITEMS[c])
    elif c == 'light': put(t, '*')
    elif c == 'info_player_deathmatch': put(t, 'm')
# one lamp and a few monsters per 4x4 block, farther blocks get tougher monsters
blocks = collections.defaultdict(list)
for c in sorted(floor): blocks[(c[0] // 4, c[1] // 4)].append(c)
for k, cs in sorted(blocks.items()):
    free = [c for c in cs if c not in occupied and abs(c[0] - air[0]) > 2 or abs(c[1] - air[1]) > 2]
    free = [c for c in free if c in floor and c not in occupied]
    if not free: continue
    put(free[len(free) // 2], '^' if (k[0] + k[1]) % 2 else '*')
    n = 1 + (len(free) > 8) + (dist[free[0]] > far // 2)
    for idx in range(n):
        c = free[(idx * 5 + 3) % len(free)]
        put(c, 'M' if dist[c] > far * 0.6 and idx == 0 else 'm')

out = []
out.append("// Generated by tools/make_spirit_map.py from spirit2dm9 (c) spirit, GPL-2.0-or-later. Do not edit.\n")
for name, g in (('LAYOUT', layout), ('THINGS', things), ('HEIGHTS', heights)):
    out.append(f"pub(super) const SPIRIT_{name}: &[&str] = &[\n" + ''.join(f'    "{"".join(r)}",\n' for r in g) + "];\n")
sys.stdout.write(''.join(out))
sys.stderr.write(f"{W}x{H}, {len(floor)} floor tiles, max height {max(h.values())}\n")
for r, t, z in zip(layout, things, heights):
    sys.stderr.write(''.join(tt if tt != '.' else (z_ if l != '#' and z_ != '0' else l) for l, tt, z_ in zip(r, t, z)) + '\n')
