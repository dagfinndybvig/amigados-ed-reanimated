#!/usr/bin/env python3
"""Patch a text file inside an AmigaDOS OFS ADF image.

Replaces the content of a named file with new text, reusing the file's
existing data blocks (new content must fit). Updates the file header
(size, high_seq, hash slots) and recomputes OFS block checksums.
The checksum routine is verified against an untouched block first.

Usage: python adf_patch.py <image.adf> <AmigaPath> <newfile.txt> <out.adf>
Example:
  python adf_patch.py wb13.adf s/Startup-Sequence newseq.txt edboot.adf
"""
import struct
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0] if "\\" in __file__ else ".")
from adf_extract import ADF, ROOT, T_HEADER, be32, bcpl

BLK = 512
ST_USERDIR, ST_FILE = 2, -3


def ofsum(block):
    """OFS block checksum: 32-bit wrapping sum of all longwords (checksum
    field zeroed), negated. Verified against all 1755 checksummed blocks
    of the Workbench 1.3 image."""
    b = bytearray(block)
    b[20:24] = b"\x00\x00\x00\x00"
    s = 0
    for i in range(0, len(b), 4):
        s = (s + int.from_bytes(b[i:i+4], "big")) & 0xFFFFFFFF
    return (-s) & 0xFFFFFFFF


def verify(adf):
    for n in range(ROOT + 1, adf.nblocks):
        b = adf.block(n)
        if be32(b, 0) in (2, 8) and be32(b, 20) != 0:
            assert ofsum(b) == be32(b, 20), "checksum algorithm mismatch on block %d" % n
            return n
    raise SystemExit("no verifiable block found")


def find_header(adf, path):
    parts = [p.upper() for p in path.split("/")]

    def resolve(dir_no, idx):
        for hn in adf.entries(dir_no):
            e = adf.block(hn)
            name = bcpl(e, 432).upper()
            if name == parts[idx]:
                sec = struct.unpack_from(">i", e, 508)[0]
                if idx == len(parts) - 1:
                    if sec != ST_FILE:
                        raise SystemExit("%s is not a file" % path)
                    return hn
                if sec != ST_USERDIR:
                    raise SystemExit("%s: %s is not a directory" % (path, name))
                return resolve(hn, idx + 1)
        raise SystemExit("path not found: %s (at %s)" % (path, parts[idx]))

    return resolve(ROOT, 0)


def patch(img, path, new_data, out):
    adf = ADF(img)
    verify(adf)
    hdr_no = find_header(adf, path)
    hdr = bytearray(adf.block(hdr_no))

    # existing chain
    chain, n = [], be32(hdr, 16)
    while n:
        chain.append(n)
        n = be32(adf.block(n), 16)
    capacity = len(chain) * 488
    if len(new_data) > capacity:
        raise SystemExit("new content %d > %d bytes capacity" % (len(new_data), capacity))

    k = (len(new_data) + 487) // 488
    img = bytearray(adf.img)
    for i, blk_no in enumerate(chain[:k]):
        b = bytearray(adf.block(blk_no))
        dsize = min(488, len(new_data) - i * 488)
        b[24:24+dsize] = new_data[i*488 : i*488 + dsize]
        if dsize < 488:
            b[24+dsize:512] = b"\x00" * (488 - dsize)
        struct.pack_into(">I", b, 8, i + 1)           # seq
        struct.pack_into(">I", b, 12, dsize)          # data_size
        nxt = chain[i + 1] if i + 1 < k else 0
        struct.pack_into(">I", b, 16, nxt)            # next
        struct.pack_into(">I", b, 20, ofsum(b))        # checksum
        img[blk_no*BLK:(blk_no+1)*BLK] = b

    # header: zero all 72 hash slots, then fill slots for seq 1..k
    for i in range(72):
        struct.pack_into(">I", hdr, 24 + 4 * i, 0)
    for i in range(k):
        struct.pack_into(">I", hdr, 24 + 4 * (71 - i), chain[i])
    struct.pack_into(">I", hdr, 8, k)                 # high_seq
    struct.pack_into(">I", hdr, 16, chain[0])         # first block
    struct.pack_into(">I", hdr, 324, len(new_data))   # data_size
    struct.pack_into(">I", hdr, 20, ofsum(hdr))       # checksum
    img[hdr_no*BLK:(hdr_no+1)*BLK] = hdr

    with open(out, "wb") as f:
        f.write(bytes(img))
    print("patched %s (%d bytes -> %d bytes, %d blocks) -> %s"
          % (path, be32(adf.block(hdr_no), 324), len(new_data), k, out))


def main():
    if len(sys.argv) != 5:
        raise SystemExit(__doc__)
    img, path, newfile, out = sys.argv[1:5]
    with open(newfile, "rb") as f:
        new_data = f.read()
    patch(img, path, new_data, out)


if __name__ == "__main__":
    main()
