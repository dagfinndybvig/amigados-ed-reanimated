#!/usr/bin/env python3
"""Inspect blocks of an AmigaDOS ADF image (OFS).

Scans every block, classifies it, and prints a summary. Used to verify
on-disk layout assumptions against real images.

Usage: python inspect_adf.py <image.adf> [--block N]
"""
import struct
import sys

BLK = 512


def be32(b, off):
    return struct.unpack_from(">I", b, off)[0]


def be32s(b, off):
    return struct.unpack_from(">i", b, off)[0]


def bcpl(b, off):
    n = b[off]
    return b[off + 1 : off + 1 + n].decode("latin-1")


def main():
    path = sys.argv[1]
    with open(path, "rb") as f:
        img = f.read()
    nblocks = len(img) // BLK

    if len(sys.argv) > 3 and sys.argv[2] == "--block":
        n = int(sys.argv[3])
        b = img[n * BLK : (n + 1) * BLK]
        print("block %d: type=%d sec=%d name=%r" % (n, be32(b, 0), be32s(b, 508), bcpl(b, 432)))
        print("  hdr_key=%d high=%d f12=%d chk16=%x" % (be32(b, 4), be32(b, 8), be32(b, 12), be32(b, 16)))
        print("  slots 20..:", [be32(b, 20 + 4 * i) for i in range(12)] + ["..."])
        print("  o312=%d o496=%d o500=%d" % (be32(b, 312), be32(b, 496), be32(b, 500)))
        print("  date d/m/t=%d/%d/%d  prot=%08x" % (be32s(b, 488), be32s(b, 492), be32s(b, 496) and 0, be32(b, 484) if be32(b, 0) == 2 else 0))
        return

    counts = {}
    headers = []
    for n in range(nblocks):
        b = img[n * BLK : (n + 1) * BLK]
        t = be32(b, 0)
        counts[t] = counts.get(t, 0) + 1
        if t == 2:
            headers.append((n, be32s(b, 508), bcpl(b, 432)))
    print("block type counts:", dict(sorted(counts.items())))
    print("%d header blocks:" % len(headers))
    for n, sec, name in headers:
        parent = be32s(img[n * BLK : (n + 1) * BLK], 496) if sec in (-3, 2) else None
        print("  blk %4d  sec %2d  parent@496 %s  name %r" % (n, sec, parent, name))


if __name__ == "__main__":
    main()
