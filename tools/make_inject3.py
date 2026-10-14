#!/usr/bin/env python3
"""Build keyboard injector v3: input.device IND_WRITE with InputEvents,
with a private stack, crash instrumentation, and color-phase readback.

Variants (diagnostic ladder; boots via c/Eval from the lean startup):
  stack   - switch/settle/restore only (proves the encodings)
  odo     - OpenDevice + CloseDevice on the private stack
  odotrace- odo + serial markers + trap handler + held beacons
  full    - odo + IND_WRITE key events into the input stream
  beacon  - palette probe: tight loop writing custom registers
  serloop - one serial marker byte, then a red tight-loop
  vtest   - full preamble (markers, switch, trap install), red tight-loop
  peekvec - display exec jump-table words/longs as color phases

Color protocol: single custom-register writes are repainted by the
display within a frame - only constant-rewrite spins are visible in
screenshots. Color 0 nibbles = palette 0xRGB (rr/17 etc).

Trap handler (vectors 2-5): on a CPU exception, cycles 5 phases
(~5s each) showing the fault PC bits 0-11, 12-15, 16-23, the pushed
SR, and the bus-error access address (low 12 bits).

Usage: python make_inject3.py <variant> [out.bin] [settle_outer_hex]
       [keycodes_csv] [devname]
"""
import struct
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0] if "\\" in __file__ else ".")
import capstone

DEVNAME = b"input.device\x00"
STACK_BYTES = 1024
DEFAULT_KEYS = "28,17,31,12,01,44"        # L I N E 1 RET


class Asm:
    """Tiny two-pass assembler: raw hex words + PC-relative fixups.

    Word-form pcrel (lea, bcc.w): displacement = target - extension addr
    (the project's settled Bcc/BSR/lea convention).
    Byte-form pcrel (bra.s/bsr.s): displacement = target - (opcode + 2).
    """

    def __init__(self):
        self.parts = []
        self.labels = {}
        self.fixups = []
        self._n = 0

    def here(self):
        return sum(len(p) for p in self.parts)

    def label(self, name):
        assert name not in self.labels
        self.labels[name] = self.here()

    def raw(self, hexs):
        self.parts.append(bytes.fromhex(hexs))

    def pcrel_w(self, opcode_hex, label):
        self.raw(opcode_hex)
        pos = self.here()
        self.parts.append(b"\x00\x00")
        self.fixups.append((pos, "w", label))

    def pcrel_b(self, opcode_hex, label):
        self.raw(opcode_hex)
        pos = self.here()
        self.parts.append(b"\x00")
        self.fixups.append((pos, "b", label))

    def assemble(self):
        blob = bytearray(b"".join(self.parts))
        for pos, kind, label in self.fixups:
            assert label in self.labels, "undefined label " + label
            if kind == "w":
                disp = self.labels[label] - pos
                assert -0x8000 <= disp <= 0x7FFF, "word fixup overflow %s" % label
                struct.pack_into(">h", blob, pos, disp)
            else:
                # byte form: PC = opcode + 2 = one past the displacement byte
                disp = self.labels[label] - (pos + 1)
                assert -128 <= disp <= 127, "byte fixup overflow %s" % label
                blob[pos] = disp & 0xFF
        return bytes(blob)

    # ----- instrumentation helpers -----

    def holdbeacon(self, color, seconds):
        """Hold custom color 0 for ~seconds, rewriting constantly
        (single writes are repainted by the display within a frame)."""
        self._n += 1
        tag = "hb%d" % self._n
        self.raw("33fc%04x00dff180" % color)      # color0 = <color>
        self.raw("303c%04x" % (seconds * 700))     # d0 budget (~0.7ms/iter)
        self.label(tag)
        self.raw("33fc%04x00dff180" % color)      # rewrite
        self.raw("323c0400")                      # ~9ms inner gap
        self.label(tag + "s")
        self.raw("51c9fffe")                      # dbra d1,gap
        self.pcrel_w("51c8", tag)                 # dbra d0,loop

    def install_traphandler(self):
        """Install a 68000 exception handler on vectors 2-5 (bus error,
        address error, illegal instruction, zero divide). Cycles 5
        phases (~5s each) showing: fault PC bits 0-11, bits 12-15,
        bits 16-23, pushed SR (& 0xfff), bus-error access address
        (low 12 bits). Frame after the d0 push: SR@4, PChi@6, PClo@8,
        IR@10, SSW@12, ADDRhi@14, ADDRlo@16."""
        self._n += 1
        vh = "vexh%d" % self._n
        hold = "vh%d" % self._n
        self.pcrel_w("41fa", vh)               # lea handler(pc),a0
        for vec in (0x08, 0x0C, 0x10, 0x14):   # vectors 2,3,4,5
            self.raw("23c8%08x" % vec)         # move.l a0,<vector>
        self.pcrel_b("60", vh + "_skip")       # bra past the handler!
        self.label(vh)
        self.raw("2f00")                       # move.l d0,-(a7)
        self.label(vh + "cyc")
        # A: PClo & $0fff
        self.raw("302f0008")
        self.raw("02400fff")
        self.pcrel_w("6100", hold)
        # B: PClo & $f000
        self.raw("302f0008")
        self.raw("0240f000")
        self.pcrel_w("6100", hold)
        # C: PChi & $00ff
        self.raw("302f0006")
        self.raw("024000ff")
        self.pcrel_w("6100", hold)
        # D: SR & $0fff
        self.raw("302f0004")
        self.raw("02400fff")
        self.pcrel_w("6100", hold)
        # E: access address lo & $0fff
        self.raw("302f0010")
        self.raw("02400fff")
        self.pcrel_w("6100", hold)
        self.pcrel_b("60", vh + "cyc")         # bra cycle
        self.label(hold)                       # hold ~5s, color = d0
        self.raw("33c000dff180")               # move.w d0,color0
        self.raw("343c0d48")                   # d2 = 3400 outer
        self.label(hold + "o")
        self.raw("33c000dff180")               # rewrite
        self.raw("323c0400")                   # d1 = 1024 inner
        self.label(hold + "s")
        self.raw("51c9fffe")                   # dbra d1,inner
        self.pcrel_w("51ca", hold + "o")       # dbra d2,outer
        self.raw("4e75")                       # rts
        self.label(vh + "_skip")

    def showphases(self):
        """Call site: bsr to the showphases routine (emitted by
        emit_showphases); d0.l = value to display."""
        if not hasattr(self, "_sph_label"):
            self._sph_label = "sph"
        self.pcrel_w("6100", self._sph_label)

    def emit_showphases(self):
        """Emit the showphases + hold routines (call once at build tail).
        showphases: d0.l -> 4 color phases (~5s each): bits 0-11,
        12-15, 16-23, 24-31."""
        hold = "sphh"
        self.label("sph")
        self.raw("2f00")                   # move.l d0,-(a7)
        self.raw("02400fff")               # andi.w #$0fff,d0
        self.pcrel_w("6100", hold)
        self.raw("302f0000")               # move.w (a7),d0
        self.raw("0240f000")
        self.pcrel_w("6100", hold)
        self.raw("302f0002")               # move.w 2(a7),d0 (hi word)
        self.raw("024000ff")
        self.pcrel_w("6100", hold)
        self.raw("302f0002")
        self.raw("0240f000")
        self.pcrel_w("6100", hold)
        self.raw("4fef0004")               # lea 4(a7),a7
        self.raw("4e75")
        self.label(hold)                   # hold ~5s, color = d0
        self.raw("33c000dff180")           # move.w d0,color0
        self.raw("343c0d48")               # d2 = 3400 outer
        self.label(hold + "o")
        self.raw("33c000dff180")           # rewrite
        self.raw("323c0400")               # d1 = 1024 inner
        self.label(hold + "s")
        self.raw("51c9fffe")               # dbra d1,inner
        self.pcrel_w("51ca", hold + "o")   # dbra d2,outer
        self.raw("4e75")

    def marker(self, ch):
        """serout a constant marker char (d1/d2 clobbered)."""
        self.raw("74%02x" % ord(ch))           # moveq #<ch>,d2
        self.pcrel_w("6100", "serout")         # bsr.w serout

    def print_d0(self):
        """serout the low byte of d0 (d1/d2 clobbered)."""
        self.raw("1400")                      # move.b d0,d2
        self.pcrel_w("6100", "serout")


def build(variant, keys, settle_outer):
    a = Asm()
    trace = variant == "odotrace"
    # --- entry: beacon + switch to private stack ---
    a.raw("2c780004")                 # a6 = ExecBase
    if trace:
        a.marker("A")                 # entry reached
    a.raw("33fc0f0000dff180")         # RED: entered
    a.pcrel_w("41fa", "stacktop")     # lea stacktop(pc),a0
    a.raw("210f")                     # move.l a7,-(a0): save old SP
    a.raw("2e48")                     # movea.l a0,a7: SP switched
    if trace:
        a.marker("B")                 # running on the private stack

    if variant == "beacon":
        # color-register probe: tight loop, constant writes
        a.label("b_loop")
        a.raw("33fc0f0000dff182")         # color1 = red (text pen)
        a.raw("33fc0f0000dff180")         # color0 = red (bg)
        a.pcrel_b("60", "b_loop")
    elif variant == "serloop":
        # one serial marker byte, then a red tight-loop
        a.marker("A")
        a.label("sl_loop")
        a.raw("33fc0f0000dff180")
        a.pcrel_b("60", "sl_loop")
    elif variant == "vtest":
        # full odotrace preamble, then red tight-loop
        a.marker("A")
        a.raw("33fc0f0000dff180")
        a.pcrel_w("41fa", "stacktop")
        a.raw("210f")
        a.raw("2e48")
        a.marker("B")
        a.install_traphandler()
        a.label("vt_loop")
        a.raw("33fc0f0000dff180")
        a.pcrel_b("60", "vt_loop")
    elif variant == "peekvec":
        # display exec jump-table values as color phases:
        # RED  -> longword at $4 dereferenced (*SysBase)
        # GREEN-> word at [SysBase-444], then longword at [-442]
        # BLUE -> word at [SysBase-552], then longword at [-550]
        # YELLOW loop = end
        a.raw("2c780004")                 # a6 = SysBase
        a.raw("20780004")                 # a0 = [4]
        a.raw("2010")                     # d0 = (a0)
        a.holdbeacon(0x0f00, 3)           # RED 3s
        a.showphases()
        a.holdbeacon(0x00f0, 3)           # GREEN 3s
        a.raw("41eefe44")                 # lea -444(a6),a0
        a.raw("3010")                     # d0.w = word there
        a.showphases()
        a.raw("41eefe46")                 # lea -442(a6),a0
        a.raw("2010")                     # d0.l
        a.showphases()
        a.holdbeacon(0x000f, 3)          # BLUE 3s
        a.raw("41eefdd8")                 # lea -552(a6),a0
        a.raw("3010")
        a.showphases()
        a.raw("41eefdae")                 # lea -550(a6),a0
        a.raw("2010")
        a.showphases()
        a.label("pv_loop")
        a.raw("33fc0ff000dff180")         # yellow tight loop = end
        a.pcrel_b("60", "pv_loop")
    elif variant == "stack":
        a.raw("303c%04x" % settle_outer)
        a.label("set_loop")
        a.raw("323c2000")
        a.label("set1")
        a.raw("51c9fffe")             # dbra d1,set1
        a.pcrel_w("51c8", "set_loop")  # dbra d0,set_loop
        a.raw("33fc00f000dff180")     # GREEN: ran on the private stack
    else:
        a.install_traphandler()        # vectors 2-5 -> crash PC as color
        a.holdbeacon(0x0f00, 3)         # RED 3s: entry, about to set up ioreq
        a.pcrel_w("41fb", "ioreq")    # a0 = ioreq
        for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44):
            a.raw("42a8%04x" % off)   # clr.l off(a0)
        a.raw("317c00300012")         # mn_Length = 48
        a.holdbeacon(0x0ff0, 2)         # YELLOW 2s: ioreq ready
        if trace:
            a.marker("C")             # ioreq initialized
        a.pcrel_w("41fa", "dev")      # a0 = devname
        a.raw("7000")                 # d0 = unit 0
        a.pcrel_w("41fb", "ioreq")    # a1 = ioreq
        if trace:
            a.marker("D")             # args set, calling OpenDevice
        a.raw("7200")                 # d1 = flags 0 (after marker)
        a.raw("4eaefe44")             # jsr OpenDevice(-444,a6)
        if trace:
            a.marker("E")             # OpenDevice returned
            a.print_d0()              # error code byte
        a.raw("4a80")                 # tst.l d0 (beacons clobber d0)
        a.pcrel_w("6600", "fail")     # bne fail
        a.holdbeacon(0x0fff, 3)         # WHITE 3s: OpenDevice returned, d0==0

    if variant == "full":
        a.raw("303c%04x" % settle_outer)
        a.label("set_loop")
        a.raw("323c2000")
        a.label("set1")
        a.raw("51c9fffe")
        a.pcrel_w("51c8", "set_loop")
        a.pcrel_w("41fb", "events")   # a0 = ev1
        a.raw("43e80010")             # a1 = ev2 = 16(a0)
        a.raw("2089")                 # ev1.next = ev2
        a.pcrel_w("41fc", "keys")     # a2 = keys
        a.label("loop")
        a.raw("1442")                 # move.b (a2)+,d2
        a.raw("0c0200ff")             # cmpi.b #$ff,d2
        a.pcrel_w("6700", "done")     # beq done
        a.pcrel_w("41fd", "events")   # a3 = events
        a.raw("37420006")             # ev1.ie_Code = d2
        a.raw("00020080")             # ori.b #$80,d2 (release bit)
        a.raw("37420016")             # ev2.ie_Code = d2
        a.pcrel_w("41fb", "ioreq")    # a0 = ioreq
        a.raw("317c0409001c")         # io_Command = IND_WRITE
        a.pcrel_w("43fb", "events")   # a1 = events
        a.raw("21490024")             # io_Data = events (36)
        a.raw("317c00020020")         # io_Length = 2 (32)
        a.raw("4eaefe38")             # jsr DoIO(-456,a6)
        a.pcrel_b("61", "keydelay")   # bsr keydelay
        a.pcrel_b("60", "loop")       # bra loop

    if variant != "stack":
        a.label("done")
        a.pcrel_w("41fb", "ioreq")    # a0 = ioreq
        a.raw("4eaefe08")             # jsr CloseDevice(-504,a6)
        if trace:
            a.marker("F")             # CloseDevice returned
        a.raw("33fc0ff000dff180")     # YELLOW: closed and done
        a.raw("2e5f")                 # movea.l (a7)+,a7: restore SP
        if trace:
            a.marker("G")             # SP restored
        a.raw("7000")                 # d0 = 0
        a.raw("4e75")                 # rts
        a.label("fail")
        if trace:
            a.marker("H")             # OpenDevice failed path
        a.holdbeacon(0x000f, 4)         # BLUE 4s: returned with error
        a.raw("33fc0fff00dff180")     # WHITE: fail exit
        a.raw("2e5f")
        a.raw("7000")
        a.raw("4e75")
        a.label("keydelay")
        a.raw("323c1800")             # d1 = 0x1800
        a.raw("51c9fffe")             # dbra d1,*
        a.raw("4e75")
        if trace or variant in ("serloop", "vtest"):
            # serout: send the byte in d2 (clobbers d1). UART direct.
            # Port enable copied from devs/serial.device (0x316):
            # DDR bits 6,7 outputs (RTS/DTR), PRA 5-7 asserted.
            a.label("serout")
            a.raw("003900c000bfd200")
            a.raw("0239001f00bfd000")
            a.raw("023900c700bfd200")
            a.raw("33fc000b00dff032")      # SERPER = 11 (9600-ish)
            a.raw("d442")                  # add.w d2,d2 (data << 1)
            a.raw("33c200dff030")          # SERDAT
            # fixed gap instead of a TBE spin: a stuck bridge would
            # hang an unbounded wait (TBE held low with no consumer)
            a.raw("323c1800")               # d1 = 0x1800 (~9ms)
            a.label("sergap")
            a.raw("51c9fffe")              # dbra d1,sergap
            a.raw("4e75")
        if variant == "peekvec":
            a.emit_showphases()
    else:
        a.raw("33fc0ff000dff180")     # YELLOW: restored (stack variant)
        a.raw("2e5f")                 # restore SP
        a.raw("7000")
        a.raw("4e75")

    while a.here() % 4:
        a.raw("4e71")           # nop: code region must be longword-aligned
    body_len = a.here()      # measure before data labels are bound

    # --- data region (strictly after code) ---
    data = b"\x00" * 48                                  # ioreq @ body+0
    ev = struct.pack(">I", 0) + bytes([0x0A, 0x00]) + \
        struct.pack(">HH", 0, 0) + b"\x00" * 6
    assert len(ev) == 16
    data += ev + ev                                      # events
    keys_blob = bytes(keys) + b"\xff"
    data += keys_blob                                    # keys
    data += DEVNAME                                      # dev
    data += b"\x00" * STACK_BYTES                       # stack buffer
    while len(data) % 4:
        data += b"\x00"

    # bind the data labels (emitted as pcrel from code) to their offsets
    ioreq = body_len
    events = ioreq + 48
    keys = events + 32
    dev = keys + len(keys_blob)
    stacktop = dev + len(DEVNAME) + STACK_BYTES
    for name, val in (("ioreq", ioreq), ("events", events),
                      ("keys", keys), ("dev", dev), ("stacktop", stacktop)):
        a.labels[name] = val
    body = a.assemble()          # now with data labels bound
    blob = body + data

    # --- byte-level verification of the stack-switch words ---
    assert bytes.fromhex("210f") in body, "push old SP (move.l a7,-(a0))"
    assert bytes.fromhex("2e48") in body, "switch (movea.l a0,a7)"
    assert bytes.fromhex("2e5f") in body, "restore (movea.l (a7)+,a7)"

    # --- verify disassembly of the code region ---
    md = capstone.Cs(capstone.CS_ARCH_M68K,
                     capstone.CS_MODE_BIG_ENDIAN | capstone.CS_MODE_M68K_000)
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
    print("wrote %s (file %d bytes; code %d, ioreq@%d events@%d keys@%d "
          "dev@%d stacktop@%d)" % (out_path, len(load), len(body),
                                   ioreq, events, keys, dev, stacktop))
    with open(out_path, "wb") as f:
        f.write(load)


variant = sys.argv[1] if len(sys.argv) > 1 else "full"
out_path = sys.argv[2] if len(sys.argv) > 2 else "emulator/inject3-" + variant
keys = [int(x, 16) for x in
        (sys.argv[4] if len(sys.argv) > 4 else DEFAULT_KEYS).split(",") if x]
settle_outer = int(sys.argv[3], 16) if len(sys.argv) > 3 else 0x384
devname = (sys.argv[5] if len(sys.argv) > 5 else "input.device").encode("latin1") + b"\x00"
DEVNAME = devname
build(variant, keys, settle_outer)
