#!/usr/bin/env python3
"""Build the keyboard injector with phase beacons.

Program phases, signaled by painting the Workbench backdrop color
(color register 0 at $DFF180):
  RED    ($0F00)  - program entered
  GREEN  ($00F0)  - settle delay done, injecting keys
  YELLOW ($0FF0)  - key sequence complete, returning

Key injection: keycode -> CIA-A serial data register ($BFD00C), then
force the CIA serial interrupt via the ICR set protocol (write $88 to
$BFD00D). The ROM keyboard handler processes it as a real keypress.

Keycodes are rawkey matrix codes, sent as press (code) and release
(code|$80) pairs with delays. The settle delay lets the background Ed
(from `Run C:ED ...` in the startup-sequence) open its window first.

Branch/displacement conventions (MC68000 PRM):
  Bcc/BSR word form and DBcc: displacement is relative to the address
  of the displacement word = instruction + 2 (both are 2-word
  instructions; the Ghidra FDBcc issue applies only to the 3-word
  FPU form). PC-relative data addressing (lea): extension word = +2.

Usage: python make_inject.py [out.bin] [keycode_hex_csv]
"""
import struct
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0] if "\\" in __file__ else ".")
import capstone

KEYS = [
    0x28, 0x17, 0x31, 0x12, 0x01, 0x44,            # L I N E 1 RET
    0x28, 0x17, 0x31, 0x12, 0x02, 0x44,            # L I N E 2 RET
    0x45,                                          # ESC
    0x20, 0x40, 0x35, 0x18, 0x31, 0x12, 0x35, 0x29, 0x40,  # A SP / o n e / ;
    0x40, 0x20, 0x40, 0x35, 0x14, 0x11, 0x18, 0x35,        # SP A SP / t w o /
    0x44,                                          # RET
    0x45, 0x2D, 0x44,                              # ESC X RET
]


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "emulator/inject"
    keys = KEYS
    if len(sys.argv) > 2:
        keys = [int(x, 16) for x in sys.argv[2].split(",") if x]

    def w(d):
        return struct.pack(">h", d)

    def b8(d):
        assert -128 <= d <= 127
        return struct.pack(">b", d)

    # layout with computed labels
    body = b""
    # 0: red beacon
    body += bytes.fromhex("33fc0f00" + "00dff180")            # move.w #$f00,color0
    body += bytes.fromhex("6100")                            # 8: bsr settle
    settle_fix = len(body)                                   # patch pos
    body += b"\x00\x00"
    body += bytes.fromhex("33fc00f0" + "00dff180")           # 12: green
    body += bytes.fromhex("41fa")                            # 20: lea keys(pc),a0
    lea_fix = len(body)
    body += b"\x00\x00"
    body += bytes.fromhex("1018")                             # 24: loop:
    body += bytes.fromhex("0c0000ff")                         # cmpi.b #$ff,d0
    body += bytes.fromhex("67")                               # 30: beq done
    beq_fix = len(body)
    body += b"\x00"
    body += bytes.fromhex("6100")                             # 32: bsr sendkey
    bsr1 = len(body)
    body += b"\x00\x00"
    body += bytes.fromhex("00000080")                         # ori.b #$80,d0
    body += bytes.fromhex("6100")                             # 40: bsr sendkey
    bsr2 = len(body)
    body += b"\x00\x00"
    body += bytes.fromhex("6100")                             # 44: bsr gap
    bsr3 = len(body)
    body += b"\x00\x00"
    body += bytes.fromhex("60")                               # 48: bra loop
    bra_fix = len(body)
    body += b"\x00"
    # 50: done: yellow
    body += bytes.fromhex("33fc0ff0" + "00dff180")
    body += bytes.fromhex("4e75")                             # 58: rts
    sendkey = len(body)                                       # 60
    body += bytes.fromhex("13c0" + "00bfd00c")                # move.b d0,sdr
    body += bytes.fromhex("13fc" + "0088" + "00bfd00d")       # icr set bit3
    body += bytes.fromhex("6100")                             # 74: bsr keydelay
    bsr4 = len(body)
    body += b"\x00\x00"
    body += bytes.fromhex("4e75")                             # 78: rts
    keydelay = len(body)                                       # 80
    body += bytes.fromhex("323c2000")                         # move.w #$2000,d1
    body += bytes.fromhex("51c9fffe")                         # dbra d1,* (ref+2)
    body += bytes.fromhex("4e75")
    gap = len(body)                                            # 90
    body += bytes.fromhex("323c0800")
    body += bytes.fromhex("51c9fffe")
    body += bytes.fromhex("4e75")
    settle = len(body)                                          # 100
    body += bytes.fromhex("303c0384")                         # move.w #$384,d0
    s1 = len(body)                                              # 104
    body += bytes.fromhex("323c2000")                         # move.w #$2000,d1
    body += bytes.fromhex("51c9fffe")                         # s2: dbra d1,*
    body += bytes.fromhex("51c8")                             # 112: dbra d0,s1
    dbra_fix = len(body)
    body += b"\x00\x00"
    body += bytes.fromhex("4e75")                             # 116: rts
    keys_off = len(body)                                       # 118
    assert keys_off == 118, keys_off

    # patch displacements (ref = instruction address + 2)
    def patch(pos, disp):
        nonlocal body
        body = body[:pos] + w(disp) + body[pos + 2:]

    patch(settle_fix, settle - (8 + 2))
    patch(lea_fix, keys_off - (20 + 2))
    patch(beq_fix, 50 - (30 + 2)) if False else None
    # beq is byte displacement relative to end of 2-byte instruction (= +2)
    body = body[:beq_fix] + b8(50 - (30 + 2)) + body[beq_fix + 1:]
    patch(bsr1, sendkey - (32 + 2))
    patch(bsr2, sendkey - (40 + 2))
    patch(bsr3, gap - (44 + 2))
    body = body[:bra_fix] + b8(24 - (48 + 2)) + body[bra_fix + 1:]
    patch(bsr4, keydelay - (74 + 2))
    patch(dbra_fix, s1 - (112 + 2))

    ktab = bytes(keys) + b"\xff"
    blob = body + ktab
    while len(blob) % 4:
        blob += b"\x00"

    md = capstone.Cs(capstone.CS_ARCH_M68K, capstone.CS_MODE_BIG_ENDIAN | capstone.CS_MODE_M68K_000)
    print("disassembly:")
    for ins in md.disasm(body, 0):
        print("  %04x  %-8s %s" % (ins.address, ins.mnemonic, ins.op_str))
    print("keys @ %d: %s" % (keys_off, " ".join("%02x" % k for k in ktab)))

    nlongs = len(blob) // 4
    load = struct.pack(">I", 0x3F3)
    load += struct.pack(">I", 0)
    load += struct.pack(">I", 1)
    load += struct.pack(">I", 0)
    load += struct.pack(">I", 0)
    load += struct.pack(">I", nlongs)
    load += struct.pack(">I", 0x3E9)
    load += struct.pack(">I", nlongs)
    load += blob
    load += struct.pack(">I", 0x3F2)

    with open(out, "wb") as f:
        f.write(load)
    print("wrote %s (%d bytes)" % (out, len(load)))


if __name__ == "__main__":
    main()
