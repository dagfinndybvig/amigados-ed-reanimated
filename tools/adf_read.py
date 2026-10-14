#!/usr/bin/env python3
"""Scratchpad helper: read a file out of an ADF image.

Usage: python adf_read.py <image.adf> <AmigaPath> [outfile]
Without outfile, prints text (latin1). Also lists dir contents for dirs.
"""
import struct
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0] if "\\" in __file__ else ".")
from adf_extract import ADF, ROOT, bcpl


def find_dir(adf, name, d=ROOT):
    for e in adf.entries(d):
        b = adf.block(e)
        if struct.unpack_from(">i", b, 508)[0] == 2 and bcpl(b, 432).lower() == name:
            return e
    raise SystemExit("dir not found: " + name)


def resolve(adf, path):
    parts = path.strip("/").split("/")
    d = ROOT
    for p in parts[:-1]:
        d = find_dir(adf, p.lower(), d)
    for e in adf.entries(d):
        b = adf.block(e)
        if bcpl(b, 432).lower() == parts[-1].lower():
            return e
    raise SystemExit("file not found: " + path)


def read_file(adf, hdr_no):
    h = adf.block(hdr_no)
    chain, n = [], struct.unpack_from(">I", h, 16)[0]
    while n:
        chain.append(n)
        n = struct.unpack_from(">I", adf.block(n), 16)[0]
    data = b""
    for bl in chain:
        b = adf.block(bl)
        data += b[24:24 + struct.unpack_from(">I", b, 12)[0]]
    return data


def main():
    adf = ADF(sys.argv[1])
    path = sys.argv[2]
    try:
        e = resolve(adf, path)
    except SystemExit:
        e = find_dir(adf, path.strip("/").split("/")[-1])
        for h in adf.entries(e):
            b = adf.block(h)
            print("blk %4d %-20r size=%d" % (
                h, bcpl(b, 432), struct.unpack_from(">I", b, 324)[0]))
        return
    data = read_file(adf, e)
    if len(sys.argv) > 3:
        with open(sys.argv[3], "wb") as f:
            f.write(data)
        print("wrote %s (%d bytes)" % (sys.argv[3], len(data)))
    else:
        sys.stdout.buffer.write(data.replace(b"\r", b"\n"))
        print()


if __name__ == "__main__":
    main()
