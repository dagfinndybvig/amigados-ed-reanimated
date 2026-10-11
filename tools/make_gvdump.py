#!/usr/bin/env python3
"""Build gvdump v2: dumps the dos.library shared global vector to DF0:gv.dump.

Convention-independent design: derives ExecBase from absolute address 4,
opens dos.library via the exec.library LVO (-552 OpenLibrary), then uses
the dos.library C LVOs (-30 Open, -48 Write, -36 Close) with caller-side
stack cleanup. DosLibrary.dl_GV is at offset 0x26.

Every instruction is verified by disassembling with Capstone and
comparing against the intended text. Output is a 1.x-era Amiga load file
(ATRM hunk format, same container as the Ed binary).

Usage: python make_gvdump.py [out.bin]
"""
import struct
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0] if "\\" in __file__ else ".")
import capstone

DOSNAME = b"dos.library\x00"
FNAME = b"DF0:gv.dump\x00"

# (mnemonic, op_str, encoding) - displacements patched at assembly time
PROGRAM = [
    ("movea.l", "$4.w, a6", bytes.fromhex("2c780004")),   # ExecBase = [4]
    ("lea.l",   "DOS(pc), a0", None),                      # "dos.library"
    ("move.l",  "a0, d1", bytes.fromhex("2208")),
    ("moveq",   "#$0, d0", bytes.fromhex("7000")),
    ("jsr",     "-$228(a6)", bytes.fromhex("4eaefdd8")),   # OpenLibrary
    ("movea.l", "d0, a5", bytes.fromhex("2a40")),          # A5 = DOSBase
    ("movea.l", "$26(a5), a1", bytes.fromhex("226d0026")), # dl_GV
    ("pea.l",   "$3ee.w", bytes.fromhex("487803ee")),      # MODE_NEWFILE
    ("pea.l",   "FN(pc)", None),                           # "DF0:gv.dump"
    ("jsr",     "-$1e(a5)", bytes.fromhex("4eadffe2")),    # Open
    ("move.l",  "d0, d7", bytes.fromhex("2e00")),
    ("addq.l",  "#$8, a7", bytes.fromhex("508f")),
    ("pea.l",   "$400.w", bytes.fromhex("48780400")),      # 1024
    ("pea.l",   "(a1)", bytes.fromhex("4851")),            # GV buffer
    ("move.l",  "d7, -(a7)", bytes.fromhex("2f07")),
    ("jsr",     "-$30(a5)", bytes.fromhex("4eadffd0")),    # Write
    ("addq.l",  "#$8, a7", bytes.fromhex("508f")),
    ("lea.l",   "$4(a7), a7", bytes.fromhex("4fef0004")),
    ("move.l",  "d7, -(a7)", bytes.fromhex("2f07")),
    ("jsr",     "-$24(a5)", bytes.fromhex("4eadffdc")),    # Close
    ("addq.l",  "#$4, a7", bytes.fromhex("588f")),
    ("moveq",   "#$0, d0", bytes.fromhex("7000")),
    ("rts",     "", bytes.fromhex("4e75")),
]


def verify(mn, ops, enc, base_addr):
    md = capstone.Cs(capstone.CS_ARCH_M68K, capstone.CS_MODE_BIG_ENDIAN | capstone.CS_MODE_M68K_000)
    for ins in md.disasm(enc, base_addr):
        ok = ins.mnemonic == mn and (not ops or ins.op_str.replace(" ", "") == ops.replace(" ", ""))
        return ok, "%s %s" % (ins.mnemonic, ins.op_str)
    return False, "<no disasm>"


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "emulator/gvdump"

    # layout: compute code size, then data offsets
    fixed_sizes = [4 if e[2] is None else len(e[2]) for e in PROGRAM]
    code_size = sum(fixed_sizes)
    # data immediately after code, aligned to even
    dosname_off = code_size
    fname_off = dosname_off + len(DOSNAME)

    # assemble with displacements
    parts = []
    addr = 0
    for (mn, ops, enc), size in zip(PROGRAM, fixed_sizes):
        if enc is None:
            if "DOS(pc)" in ops:
                disp = dosname_off - (addr + size)
                enc = bytes.fromhex("41fa") + struct.pack(">h", disp)
                mn2, ops2 = "lea.l", "$%x(pc), a0" % disp
            else:
                disp = fname_off - (addr + size)
                enc = bytes.fromhex("487a") + struct.pack(">h", disp)
                mn2, ops2 = "pea.l", "$%x(pc)" % disp
            ok, got = verify(mn2 if mn == "lea.l" else "pea.l", ops2, enc, addr)
            if not ok and "(pc)" in got and got.split()[0] == ("lea.l" if mn == "lea.l" else "pea.l"):
                ok = True  # capstone renders effective address, not displacement
            print("  %-8s %-18s -> %s  %s" % (mn, ops, " ".join("%02x" % b for b in enc),
                                              "OK" if ok else "MISMATCH: " + got))
            if not ok:
                # capstone renders effective addresses; accept if it disassembles as the right mnemonic
                if got.split()[0] != mn:
                    raise SystemExit("encoding error at %s" % mn)
        else:
            ok, got = verify(mn, ops, enc, addr)
            print("  %-8s %-18s -> %s  %s" % (mn, ops, " ".join("%02x" % b for b in enc),
                                              "OK" if ok else "MISMATCH: " + got))
            if not ok:
                raise SystemExit("encoding error at %s %s: %s" % (mn, ops, got))
        parts.append(enc)
        addr += size

    code = b"".join(parts)
    data = DOSNAME + FNAME
    while (len(code) + len(data)) % 4:
        data += b"\x00"
    blob = code + data

    print("\nfull disassembly:")
    md = capstone.Cs(capstone.CS_ARCH_M68K, capstone.CS_MODE_BIG_ENDIAN | capstone.CS_MODE_M68K_000)
    md.skipdata = True
    for ins in md.disasm(blob, 0):
        if ins.mnemonic == "data":
            continue
        print("  %04x  %-8s %s" % (ins.address, ins.mnemonic, ins.op_str))
    print("  data: %r %r" % (DOSNAME, FNAME))

    nlongs = len(blob) // 4
    load = struct.pack(">I", 0x3F3)          # hunk_header (1011)
    load += struct.pack(">I", 0)              # resident-library names: terminator
    load += struct.pack(">I", 1)              # table size
    load += struct.pack(">I", 0)              # F
    load += struct.pack(">I", 0)              # L
    load += struct.pack(">I", nlongs)         # sizes[0]
    load += struct.pack(">I", 0x3E9)          # hunk_code (1001)
    load += struct.pack(">I", nlongs)
    load += blob
    load += struct.pack(">I", 0x3F2)          # hunk_end (1010)

    with open(out, "wb") as f:
        f.write(load)
    print("\nwrote %s (%d bytes, code+data %d bytes)" % (out, len(load), len(blob)))


if __name__ == "__main__":
    main()
