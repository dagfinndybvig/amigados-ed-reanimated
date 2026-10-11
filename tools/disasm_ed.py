#!/usr/bin/env python3
"""Disassemble the Ed binary (old TRIPOS load format) with Capstone m68k.

Linear disassembly per segment, with BCPL strings annotated where the byte
stream looks like a string constant. Output: text listing or analysis.

Usage:
  python disasm_ed.py <file> [--seg N] [--out file]
  python disasm_ed.py <file> --immediates   (list CMPI/CMP immediate values)
"""
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0] if "\\" in __file__ else ".")
from hunk_dump import parse, bcpl_strings

import capstone


def disassemble(path, seg_no=None):
    segs, data, fmt = parse(path)
    md = capstone.Cs(capstone.CS_ARCH_M68K, capstone.CS_MODE_BIG_ENDIAN | capstone.CS_MODE_M68K_000)
    md.detail = True
    md.skipdata = True  # keep going through embedded data (strings, tables)
    for s in segs:
        if seg_no is not None and s["n"] != seg_no:
            continue
        strings = dict(bcpl_strings(s["raw"]))
        yield s, md.disasm(s["raw"], 0), strings


def printable_str(b):
    return all(32 <= c < 127 for c in b)


def annotate(s, strings):
    """Yield (kind, payload) items: instructions or string constants."""
    raw = s["raw"]
    covered = set()
    for off, st in strings.items():
        for i in range(off, off + 1 + len(st)):
            covered.add(i)
    return covered


def main():
    args = sys.argv[1:]
    if not args:
        raise SystemExit(__doc__)
    path = args[0]
    mode = None
    seg_no = None
    out = None
    if "--immediates" in args:
        mode = "imm"
    if "--seg" in args:
        seg_no = int(args[args.index("--seg") + 1])
    if "--out" in args:
        out = args[args.index("--out") + 1]

    lines = []
    for s, insns, strings in disassemble(path, seg_no):
        lines.append("== segment %d (%d bytes)" % (s["n"], s["size"]))
        if mode == "imm":
            for ins in insns:
                if ins.mnemonic == "data":
                    continue
                try:
                    ops = ins.operands
                except capstone.CsError:
                    continue
                for op in ops:
                    if op.type == capstone.m68k.M68K_OP_IMM:
                        v = op.imm & 0xFF
                        if 0x20 <= v <= 0x7E and ins.mnemonic.startswith(("cmp", "cmpi")):
                            lines.append("  seg%d+0x%04x  %-8s %-28s  '%s'"
                                         % (s["n"], ins.address, ins.mnemonic, ins.op_str, chr(v)))
        else:
            for ins in insns:
                if ins.mnemonic == "data":
                    lines.append("  seg%d+0x%04x  .data" % (s["n"], ins.address))
                    continue
                try:
                    ops = ins.operands
                except capstone.CsError:
                    ops = []
                ann = ""
                for a in ops:
                    if a.type == capstone.m68k.M68K_OP_IMM:
                        v = a.imm & 0xFFFFFFFF
                        ann = "  ; 0x%x" % v if v > 9 else ""
                lines.append("  seg%d+0x%04x  %-8s %s%s"
                             % (s["n"], ins.address, ins.mnemonic, ins.op_str, ann))
    text = "\n".join(lines)
    if out:
        open(out, "w", encoding="latin-1").write(text)
        print("wrote %d lines to %s" % (len(lines), out))
    else:
        print(text)


if __name__ == "__main__":
    main()
