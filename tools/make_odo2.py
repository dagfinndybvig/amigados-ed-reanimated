#!/usr/bin/env python3
"""Build odo2: OpenDevice + CloseDevice isolation test with correct layout
(data placed after the code, displacements patched programmatically)."""
import struct
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0] if "\\" in __file__ else ".")
import capstone


def wd(d):
    return struct.pack(">h", d)


def main():
    body = b""
    body += bytes.fromhex("2c780004")                # 0: a6 = [4]
    body += bytes.fromhex("41fb") + b"\x00\x00"      # lea ioreq(pc),a0
    body += bytes.fromhex("7000")                   # moveq #0,d0
    for off in range(0, 48, 4):
        body += bytes.fromhex("42a8") + wd(off)     # clr.l off(a0)
    body += bytes.fromhex("317c" + "0030" + "0012") # mn_Length = 48
    body += bytes.fromhex("41fa") + b"\x00\x00"      # lea devname(pc),a0
    body += bytes.fromhex("7000")                   # d0 = 0 (unit)
    body += bytes.fromhex("41fb") + b"\x00\x00"      # lea ioreq(pc),a1
    body += bytes.fromhex("7200")                   # d1 = 0 (flags)
    body += bytes.fromhex("4eaefe44")               # jsr -444 OpenDevice
    body += bytes.fromhex("4a80")                   # tst.l d0
    body += bytes.fromhex("41fb") + b"\x00\x00"      # lea ioreq(pc),a0
    body += bytes.fromhex("4eaefe08")               # jsr -504 CloseDevice
    body += bytes.fromhex("7000")                   # moveq #0,d0
    body += bytes.fromhex("4e75")                   # rts

    # find patch points on ODD-2-byte alignment (lea opcodes are at even offsets)
    leas = [i for i in range(0, len(body) - 1, 2) if body[i:i + 2] == b"\x41\xfb"]
    devp = [i for i in range(0, len(body) - 1, 2) if body[i:i + 2] == b"\x41\xfa"]
    assert len(leas) == 3 and len(devp) == 1, (leas, devp)

    ioreq = len(body)
    dev = ioreq + 48

    def patch(pos, target):
        nonlocal body
        ref = pos + 2
        body = body[:pos + 2] + wd(target - ref) + body[pos + 4:]

    patch(leas[0], ioreq)
    patch(devp[0], dev)
    patch(leas[1], ioreq)
    patch(leas[2], ioreq)

    data = b"\x00" * 48 + b"input.device\x00"
    blob = body + data
    while len(blob) % 4:
        blob += b"\x00"
    n = len(blob) // 4
    load = (struct.pack(">I", 0x3F3) + struct.pack(">I", 0) +
            struct.pack(">I", 1) + struct.pack(">I", 0) + struct.pack(">I", 0) +
            struct.pack(">I", n) + struct.pack(">I", 0x3E9) + struct.pack(">I", n) +
            blob + struct.pack(">I", 0x3F2))
    out = sys.argv[1] if len(sys.argv) > 1 else "emulator/odo2"
    with open(out, "wb") as f:
        f.write(load)

    md = capstone.Cs(capstone.CS_ARCH_M68K, capstone.CS_MODE_BIG_ENDIAN | capstone.CS_MODE_M68K_000)
    print("body %d bytes; ioreq@%d dev@%d; file %d bytes" % (len(body), ioreq, dev, len(load)))
    print("leas:", leas, "dev lea:", devp)
    for ins in md.disasm(body, 0):
        print("  %04x  %-8s %s" % (ins.address, ins.mnemonic, ins.op_str))


if __name__ == "__main__":
    main()
