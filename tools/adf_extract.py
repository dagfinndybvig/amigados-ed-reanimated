#!/usr/bin/env python3
"""Extract files from an AmigaDOS OFS (DOS\0) ADF floppy image.

On-disk layout verified empirically against the Workbench 1.3 image:

Header block (root / dir / file header):
    0   type (2 = T_HEADER)
    4   header_key (own block number; 0 for root)
    8   high_seq (file: highest data block seq)
    12  hash_size (root: 72) / unused
    16  first data block (file)
    20  checksum
    24..308  hash table, 72 slots
    312 info / extension block pointer (root: bitmap flag = -1)
    320 protection
    324 data_size (file)
    416 date (days, mins, ticks)
    432 BCPL name
    508 secondary type (1=root, 2=dir, -3=file)

OFS data block (type 8):
    0   type, 4 header_key, 8 seq, 12 data_size(<=488),
    16  next data block, 20 checksum, 24..511 data

File data blocks are read by following the next-block chain from the header's
first data block, cross-checking seq numbers against the header's data_size.
Hash collision chains are followed through the same slot in each chained
block, but only into blocks of type 2 (a file's other slots hold data block
pointers of type 8).

Usage: python adf_extract.py <image.adf> <outdir> [--list]
"""
import os
import struct
import sys

BLK = 512
ROOT = 880
T_HEADER, T_DATA = 2, 8
ST_ROOT, ST_USERDIR, ST_FILE = 1, 2, -3

be32 = lambda b, off: struct.unpack_from(">I", b, off)[0]
be32s = lambda b, off: struct.unpack_from(">i", b, off)[0]


def bcpl(b, off):
    n = b[off]
    return b[off + 1 : off + 1 + n].decode("latin-1")


class ADF:
    def __init__(self, path):
        with open(path, "rb") as f:
            self.img = f.read()
        if len(self.img) % BLK:
            sys.exit("not a raw ADF image (size %d)" % len(self.img))
        self.nblocks = len(self.img) // BLK

    def block(self, n):
        if not 0 <= n < self.nblocks:
            sys.exit("block %d out of range" % n)
        return self.img[n * BLK : (n + 1) * BLK]

    def read_file(self, hdr_no):
        hdr = self.block(hdr_no)
        size = be32(hdr, 324)
        n, total = be32(hdr, 16), 0
        parts = []
        while n and len(parts) < self.nblocks:
            b = self.block(n)
            if be32(b, 0) != T_DATA:
                sys.exit("file at block %d: chain leads to non-data block %d" % (hdr_no, n))
            seq = be32(b, 8)
            if seq != len(parts) + 1:
                sys.exit("file at block %d: seq gap at block %d (got %d)" % (hdr_no, n, seq))
            ds = be32(b, 12)
            parts.append(b[24 : 24 + ds])
            total += ds
            n = be32(b, 16)
        data = b"".join(parts)
        if total < size:
            sys.exit("file at block %d: got %d bytes, header says %d" % (hdr_no, total, size))
        return data[:size]

    def entries(self, dir_no):
        """Yield header block numbers reachable from dir's hash table."""
        blk = self.block(dir_no)
        for i in range(72):
            ptr = be32(blk, 24 + 4 * i)
            while ptr:
                e = self.block(ptr)
                if be32(e, 0) != T_HEADER:
                    sys.exit("block %d in dir %d chain: not a header block" % (ptr, dir_no))
                yield ptr
                nxt = be32(e, 24 + 4 * i)
                # follow chain only while target looks like a header block;
                # a file's other slots point at type-8 data blocks
                ptr = nxt if nxt and be32(self.block(nxt), 0) == T_HEADER else 0

    def walk(self, dir_no, prefix, out, listing):
        for n in self.entries(dir_no):
            e = self.block(n)
            name = bcpl(e, 432)
            sec = be32s(e, 508)
            path = prefix + name
            if sec == ST_USERDIR:
                self.walk(n, path + "/", out, listing)
            elif sec == ST_FILE:
                data = self.read_file(n)
                listing.append((path, len(data)))
                if out:
                    dest = os.path.join(out, *path.split("/"))
                    os.makedirs(os.path.dirname(dest), exist_ok=True)
                    with open(dest, "wb") as f:
                        f.write(data)
            else:
                listing.append((path + " (sec %d, skipped)" % sec, -1))

    def run(self, out=None):
        listing = []
        self.walk(ROOT, "", out, listing)
        return listing


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        sys.exit(__doc__)
    adf = ADF(args[0])
    listing = adf.run(args[1])
    print("Volume: %s  (%d files)" % (bcpl(adf.block(ROOT), 432), len(listing)))
    for p, sz in sorted(listing):
        print("%9d  %s" % (sz, p))


if __name__ == "__main__":
    main()
