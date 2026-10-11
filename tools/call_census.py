#!/usr/bin/env python3
"""Census of runtime calls in the Ed disassembly.

Scans the Capstone listing for the BCPL call pattern:
    moveq  #<funcnum>, d0
    ...
    jsr    (a5)            <- a5 = runtime dispatch (globvec)
and reports which function numbers Ed calls, where, and the nearest d1/d2
immediates (call arguments) for context.

Usage: python call_census.py <asm-listing>
"""
import re
import sys
from collections import defaultdict

path = sys.argv[1] if len(sys.argv) > 1 else r"emulator\ed.asm"
call_re = re.compile(r"seg(\d+)\+0x([0-9a-f]+)\s+jsr\s+\(a5\)")
move_re = re.compile(r"seg(\d+)\+0x([0-9a-f]+)\s+moveq\s+#\$([0-9a-f-]+), (d\d)")
other_jsr_re = re.compile(r"seg(\d+)\+0x([0-9a-f]+)\s+jsr\s+(\S+)")

lines = open(path, encoding="latin-1").read().splitlines()

# index instructions
insns = []
for ln in lines:
    m = re.match(r"\s*seg(\d+)\+0x([0-9a-f]+)\s+(\S+)\s*(.*)", ln)
    if m:
        insns.append((int(m.group(1)), int(m.group(2), 16), m.group(3), m.group(4), ln))

calls = []
for idx, (seg, off, mn, ops, ln) in enumerate(insns):
    if mn == "jsr" and ops.startswith("(a5)"):
        # walk back up to 6 instructions for the moveq into d0
        func = None
        args = {}
        for back in range(idx - 1, max(0, idx - 7), -1):
            bseg, boff, bmn, bops, _ = insns[back]
            if bmn == "moveq":
                m = re.match(r"#\$([0-9a-f-]+), (d\d)", bops)
                if m:
                    reg = m.group(2)
                    val = int(m.group(1), 16)
                    if reg == "d0" and func is None:
                        func = val
                    elif func is not None and reg != "d0":
                        args.setdefault(reg, val)
            if func is not None and bmn == "jsr":
                break
        calls.append((seg, off, func, args))

byfunc = defaultdict(list)
for seg, off, func, args in calls:
    byfunc[func].append((seg, off, args))

print("total jsr (a5) calls: %d" % len(calls))
print("distinct function numbers: %d" % len([f for f in byfunc if f is not None]))
print()
for func in sorted(byfunc, key=lambda f: (f is None, f)):
    sites = byfunc[func]
    name = "???" if func is None else str(func)
    print("func %s (%d call sites)" % (name, len(sites)))
    for seg, off, args in sites[:6]:
        argtxt = " ".join("%s=%s" % (k, v) for k, v in sorted(args.items()))
        print("   seg%d+0x%04x  %s" % (seg, off, argtxt))
    if len(sites) > 6:
        print("   ... +%d more" % (len(sites) - 6))
