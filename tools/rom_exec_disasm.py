#!/usr/bin/env python3
"""Disassemble exec routines from kick13.rom via the ROM's own library
vector table (105 entries at ROM offset 0x1a7c, word offsets relative
to the table; per TODO 6b notes, code-verified against V36+ names)."""
import struct
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0] if "\\" in __file__ else ".")
import capstone

ROM = r"C:\Users\dybvig\Reverse\Ed\emulator\kick13.rom"
VTAB = 0x1A7C            # ROM offset of the exec vector table
NENT = 105


def main():
    d = open(ROM, "rb").read()
    lvo = int(sys.argv[1], 0)              # negative LVO, e.g. -444
    count = int(sys.argv[2]) if len(sys.argv) > 2 else 40
    idx = (-lvo) // 6 - 1
    assert 0 <= idx < NENT, idx
    off = struct.unpack_from(">H", d, VTAB + idx * 2)[0]
    code = VTAB + off
    print("vector index %d, word offset 0x%04x -> ROM 0x%05x" % (idx, off, code))
    md = capstone.Cs(capstone.CS_ARCH_M68K,
                     capstone.CS_MODE_BIG_ENDIAN | capstone.CS_MODE_M68K_000)
    n = 0
    pos = code
    while n < count:
        for ins in md.disasm(d[pos:pos + 10], pos):
            print("  %06x  %-8s %s" % (ins.address, ins.mnemonic, ins.op_str))
            pos = ins.address + ins.size
            n += 1
            break
        else:
            print("  %06x  .dc.w    $%04x" % (pos, struct.unpack_from(">H", d, pos)[0]))
            pos += 2
            n += 1


if __name__ == "__main__":
    main()
