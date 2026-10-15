#!/usr/bin/env python3
"""Triage an Amiga HUNK executable: segment list, and string dumps.

Handles both layouts:
- standard: magic 0x3F3 + HUNK_HEADER (0x3F4) + hunks
- old TRIPOS (AmigaDOS 1.x BCPL commands, e.g. Ed):
    +0 magic 0x3F3, +4 0, +8 num_hunks, +12 first, +16 last,
    +20 sizes[num] (longwords), then per hunk [0x3E9][size][code],
    hunks delimited by a 0x3F2 longword.

Modes:
  default       list segments
  --strings     dump BCPL-style strings (len byte + chars, word padded)
  --raw N       hex dump the first 0x100 bytes of segment N

Usage: python hunk_dump.py <file> [--strings] [--raw N]
"""
import struct
import sys

MAGIC = 0x3F3


def be32(b, off):
    return struct.unpack_from(">I", b, off)[0]


def parse(path):
    data = open(path, "rb").read()
    if be32(data, 0) != MAGIC:
        sys.exit("not an Amiga HUNK executable (magic %08x)" % be32(data, 0))
    if be32(data, 4) == 0x3F4:
        return parse_standard(data)
    return parse_old(data)


def parse_old(data):
    n, first, last = be32(data, 8), be32(data, 12), be32(data, 16)
    sizes = [be32(data, 20 + 4 * i) for i in range(n)]
    segs, off = [], 20 + 4 * n
    for i in range(n):
        marker, size = be32(data, off), be32(data, off + 4)
        # 0x3E9 = code hunk, 0x3EA = data hunk (Ed 2.00 has both)
        if marker not in (0x3E9, 0x3EA) or size != sizes[i]:
            sys.exit("old format: hunk %d marker/size mismatch at 0x%x" % (i, off))
        start = off + 8
        segs.append({"n": i, "start": start, "size": size * 4,
                     "raw": data[start : start + size * 4]})
        off = start + size * 4
        # optional HUNK_RELOC32 blocks: 0x3EC, then [count, hunk, count
        # offsets] groups, terminated by a zero count
        while be32(data, off) == 0x3EC:
            off += 4
            while True:
                cnt = be32(data, off)
                off += 4
                if cnt == 0:
                    break
                off += 4 + 4 * cnt
        if be32(data, off) == 0x3F2:  # segment delimiter
            off += 4
    return segs, data, "old TRIPOS"


def parse_standard(data):
    segs, off = [], 4
    while off + 4 <= len(data):
        t = be32(data, off)
        off += 4
        if t in (0x3EA, 0x3EB):  # CODE/DATA
            size = be32(data, off) * 4
            off += 4
            segs.append({"n": len(segs), "start": off, "size": size,
                         "raw": data[off : off + size]})
            off += size
        elif t == 0x3EC:  # BSS
            segs.append({"n": len(segs), "start": off, "size": be32(data, off) * 4,
                         "raw": b""})
            off += 4
        elif t in (0x3F3,):
            break
        elif t in (0x3F4,):  # HEADER
            n = be32(data, off + 8)
            off += 20 + 4 * n
        else:
            break
    return segs, data, "standard"


def printable(b):
    return all(32 <= c < 127 or c in (9, 10, 13) for c in b)


def bcpl_strings(raw, minlen=3):
    out, i = [], 0
    while i < len(raw):
        n = raw[i]
        if minlen <= n <= 100 and i + 1 + n <= len(raw):
            s = raw[i + 1 : i + 1 + n]
            if printable(s):
                out.append((i, s.decode("latin-1")))
                # a BCPL string occupies 1+n bytes, padded to an even total
                i += 1 + n + ((1 + n) & 1)
                continue
        i += 1
    return out


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    segs, data, fmt = parse(sys.argv[1])
    print("== %s (%d bytes, %s format)" % (sys.argv[1], len(data), fmt))
    for s in segs:
        print("  segment %d: %5d bytes at file 0x%05x..0x%05x"
              % (s["n"], s["size"], s["start"], s["start"] + s["size"]))
    if "--strings" in sys.argv:
        print("\n== BCPL strings")
        for s in segs:
            for off, st in bcpl_strings(s["raw"]):
                print("  seg%d+0x%04x  %r" % (s["n"], off, st))
    if "--raw" in sys.argv:
        n = int(sys.argv[sys.argv.index("--raw") + 1])
        raw = segs[n]["raw"]
        for i in range(0, min(len(raw), 0x100), 16):
            print("%04x  %s" % (i, " ".join("%02x" % c for c in raw[i:i+16])))


if __name__ == "__main__":
    main()
