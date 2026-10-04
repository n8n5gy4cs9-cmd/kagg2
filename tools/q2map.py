#!/usr/bin/env python3
"""Parse a Quake 2 .map (text) into brushes/entities and voxelise the solid world.

Used by tools/make_spirit_map.py to turn the GPL-licensed map spirit2dm9 (by spirit) into one of our
tile maps. Only geometry/entity positions are read; no textures or sounds are copied.
"""
import re, itertools, collections

def parse(path):
    ents = []
    cur = None; brush = None
    for line in open(path, encoding='latin-1'):
        line = line.strip()
        if line.startswith('//') or not line:
            continue
        if line == '{':
            if cur is None:
                cur = {'brushes': []}
            else:
                brush = []
        elif line == '}':
            if brush is not None:
                cur['brushes'].append(brush); brush = None
            else:
                ents.append(cur); cur = None
        elif brush is not None:
            m = re.match(r'\(\s*(\S+) (\S+) (\S+)\s*\)\s*\(\s*(\S+) (\S+) (\S+)\s*\)\s*\(\s*(\S+) (\S+) (\S+)\s*\)\s*(\S+)', line)
            v = [float(x) for x in m.groups()[:9]]
            brush.append((tuple(v[0:3]), tuple(v[3:6]), tuple(v[6:9]), m.group(10)))
        elif cur is not None:
            m = re.match(r'"([^"]*)"\s+"([^"]*)"', line)
            if m: cur[m.group(1)] = m.group(2)
    return ents

def sub(a, b): return (a[0]-b[0], a[1]-b[1], a[2]-b[2])
def cross(a, b): return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])
def dot(a, b): return a[0]*b[0]+a[1]*b[1]+a[2]*b[2]

def planes(brush):
    out = []
    for p1, p2, p3, tex in brush:
        n = cross(sub(p3, p1), sub(p2, p1))
        l = dot(n, n) ** 0.5
        if l == 0: continue
        n = (n[0]/l, n[1]/l, n[2]/l)
        out.append((n, dot(n, p1), tex))
    return out

def solve3(a, b, c):
    (n1, d1), (n2, d2), (n3, d3) = a, b, c
    det = dot(n1, cross(n2, n3))
    if abs(det) < 1e-6: return None
    p = tuple((d1*cross(n2, n3)[i] + d2*cross(n3, n1)[i] + d3*cross(n1, n2)[i]) / det for i in range(3))
    return p

def bounds(pl):
    verts = []
    for a, b, c in itertools.combinations(pl, 3):
        p = solve3(a[:2], b[:2], c[:2])
        if p and all(dot(n, p) <= d + 0.01 for n, d, *_ in pl):
            verts.append(p)
    if not verts: return None
    return [(min(v[i] for v in verts), max(v[i] for v in verts)) for i in range(3)]

SKIP = ('sky', 'clip', 'hint', 'skip', 'trigger', 'origin', 'water', 'lava', 'slime')

def voxelise(ents, step=16):
    """Set of (i, j, k) cells (cell = step units) whose centre is inside a solid world brush."""
    solid = set()
    for br in ents[0]['brushes']:
        pl = planes(br)
        if any(any(s in t.lower() for s in SKIP) for _, _, t in pl):
            continue
        b = bounds([(n, d) for n, d, _ in pl])
        if not b: continue
        r = [range(int(lo // step), int(hi // step) + 1) for lo, hi in b]
        for i in r[0]:
            for j in r[1]:
                for k in r[2]:
                    p = ((i + .5) * step, (j + .5) * step, (k + .5) * step)
                    if all(dot(n, p) <= d for n, d, _ in pl):
                        solid.add((i, j, k))
    return solid

def origin(e):
    return tuple(float(x) for x in e['origin'].split()) if 'origin' in e else None
