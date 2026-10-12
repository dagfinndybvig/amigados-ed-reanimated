#!/usr/bin/env python3
"""Build keyboard injector v2: input.device IND_WRITE with InputEvents.

Opens input.device (exec OpenDevice A0=name, D0=unit, A1=ioreq, D1=flags,
convention verified against kick13.rom call sites), then for each keycode
sends a RAWKEY press/release InputEvent pair via DoIO (IND_WRITE=0x409),
then CloseDevice, exit 0 (the 1.x Shell treats a nonzero d0 at rts as
the command's failure code).

Layout is hand-computed; all instructions verified with Capstone.

Usage: python make_inject2.py [out.bin] [settle_outer_hex] [keycodes_csv]
"""
import struct
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0] if "\\" in __file__ else ".")
import capstone

DEVNAME = b"input.device\x00"

KEYS = [
    0x28, 0x17, 0x31, 0x12, 0x01, 0x44,            # L I N E 1 RET
    0x28, 0x17, 0x31, 0x12, 0x02, 0x44,            # L I N E 2 RET
    0x45,                                          # ESC
    0x20, 0x40, 0x35, 0x18, 0x31, 0x12, 0x35, 0x29, 0x40,
    0x40, 0x20, 0x40, 0x35, 0x14, 0x11, 0x18, 0x35,
    0x44,
    0x45, 0x2D, 0x44,                              # ESC X RET
]


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "emulator/inject2"
    keys = KEYS
    if len(sys.argv) > 3 and sys.argv[3]:
        keys = [int(x, 16) for x in sys.argv[3].split(",") if x]
    settle_outer = int(sys.argv[2], 16) if len(sys.argv) > 2 else 0x384

    def w(d):
        return struct.pack(">h", d)

    def b8(d):
        assert -128 <= d <= 127
        return struct.pack(">b", d)

    # labels
    LOOP = 114
    DONE = 172
    FAIL = 184
    KEYDELAY = 188
    IOREQ = 198
    EVENTS = 246
    KEYS_OFF = 278
    DEV = 315

    # labels (beacon build)
    LOOP = 130
    DONE = 196
    FAIL = 216
    KEYDELAY = 220
    IOREQ = 230
    EVENTS = 278
    KEYS_OFF = 310
    DEV = 347

    # --- code region with phase beacons ---
    body = b""
    body += bytes.fromhex("2c780004")                     # 0: a6 = ExecBase
    body += bytes.fromhex("41fb") + w(IOREQ - 6)          # 4: a0 = ioreq
    for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44):
        body += bytes.fromhex("42a8") + w(off)            # clr.l off(a0)
    body += bytes.fromhex("317c" + "0030" + "0012")       # 56: mn_Length = 48
    body += bytes.fromhex("33fc" + "0f00" + "00dff180")   # 62: RED = entered
    body += bytes.fromhex("41fa") + w(DEV - 72)           # 70: a0 = "input.device"
    body += bytes.fromhex("7000")                         # 74: d0 = 0
    body += bytes.fromhex("41fb") + w(IOREQ - 78)         # 76: a1 = ioreq
    body += bytes.fromhex("7200")                         # 80: d1 = 0
    body += bytes.fromhex("4eaefe44")                     # 82: jsr -444 OpenDevice
    body += bytes.fromhex("4a80")                         # 86: tst.l d0
    body += bytes.fromhex("6600") + w(FAIL - 90)          # 88: bne.w fail
    body += bytes.fromhex("33fc" + "00f0" + "00dff180")   # 92: GREEN = device open
    body += bytes.fromhex("303c%04x" % settle_outer)      # 100: settle
    body += bytes.fromhex("323c2000")                     # 104: s1
    body += bytes.fromhex("51c9fffe")                     # 108: dbra d1,*
    body += bytes.fromhex("51c8") + w(104 - 114)          # 112: dbra d0,s1
    body += bytes.fromhex("41fb") + w(EVENTS - 118)       # 116: a0 = events
    body += bytes.fromhex("43e80010")                     # 120: a1 = ev2
    body += bytes.fromhex("2089")                         # 124: ev1.next = ev2
    body += bytes.fromhex("41fc") + w(KEYS_OFF - 128)     # 126: a2 = keys
    assert len(body) == LOOP, (len(body), LOOP)
    body += bytes.fromhex("144a")                         # 130: loop:
    body += bytes.fromhex("0c0200ff")                     # cmpi.b #$ff,d2
    body += bytes.fromhex("6700") + w(DONE - 138)         # 136: beq.w done
    body += bytes.fromhex("41fd") + w(EVENTS - 142)       # 140: a3 = events
    body += bytes.fromhex("3742" + "0006")                # 144: ev1.code = d2
    body += bytes.fromhex("00020080")                     # 148: ori.b #$80,d2
    body += bytes.fromhex("3742" + "0016")                # 152: ev2.code = d2
    body += bytes.fromhex("33fc" + "000f" + "00dff180")   # 156: BLUE = in loop
    body += bytes.fromhex("41fb") + w(IOREQ - 166)        # 164: a0 = ioreq
    body += bytes.fromhex("317c" + "0409" + "001c")       # 168: cmd = IND_WRITE
    body += bytes.fromhex("43fb") + w(EVENTS - 176)        # 174: a1 = events
    body += bytes.fromhex("21490024")                     # 178: io_Data = events (36)
    body += bytes.fromhex("317c" + "0002" + "0020")       # 182: io_Length = 2 (32)
    body += bytes.fromhex("4eaefe38")                     # 188: jsr -456 DoIO
    body += bytes.fromhex("61") + b8(KEYDELAY - 194)       # 192: bsr keydelay
    body += bytes.fromhex("60") + b8(LOOP - 196)          # 194: bra loop
    assert len(body) == DONE, (len(body), DONE)
    body += bytes.fromhex("33fc" + "0ff0" + "00dff180")   # 196: done: YELLOW
    body += bytes.fromhex("41fb") + w(IOREQ - 206)        # 204: a0 = ioreq
    body += bytes.fromhex("4eaefe08")                     # 208: jsr -504 CloseDevice
    body += bytes.fromhex("7000")                          # 212: moveq #0,d0
    body += bytes.fromhex("4e75")                          # 214: rts
    assert len(body) == FAIL, (len(body), FAIL)
    body += bytes.fromhex("7000") + bytes.fromhex("4e75")  # 216: fail
    assert len(body) == KEYDELAY, (len(body), KEYDELAY)
    body += bytes.fromhex("323c1800")                      # 220: keydelay
    body += bytes.fromhex("51c9fffe")
    body += bytes.fromhex("4e75")
    assert len(body) == IOREQ, (len(body), IOREQ)

    data = b"\x00" * 48                                    # ioreq (zeroed; mn_Length set at runtime)
    ev = struct.pack(">I", 0) + bytes([0x0A, 0x00]) + struct.pack(">HH", 0, 0) + b"\x00" * 6
    data += ev + ev                                        # 2 InputEvents (16 bytes each)
    assert len(ev) == 16, len(ev)
    assert len(data) == 80, len(data)
    data += bytes(keys) + b"\xff"
    data += DEVNAME
    blob = body + data
    while len(blob) % 4:
        blob += b"\x00"

    # verify disassembly of the code region
    md = capstone.Cs(capstone.CS_ARCH_M68K, capstone.CS_MODE_BIG_ENDIAN | capstone.CS_MODE_M68K_000)
    print("disassembly:")
    for ins in md.disasm(body, 0):
        print("  %04x  %-8s %s" % (ins.address, ins.mnemonic, ins.op_str))

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
    print("wrote %s (%d bytes; blob %d, keys @%d)" % (out, len(load), len(blob), KEYS_OFF))


if __name__ == "__main__":
    main()
