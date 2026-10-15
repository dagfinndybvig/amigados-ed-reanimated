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
    elif variant == "dump444":
        # dump bytes as ~10s color phases (palette = 0xF00 | byte, since
        # the upper d0 byte stays 0xFF after dbra loops):
        #   after RED: bytes at $4..$7 (the SysBase pointer longword)
        #   after GREEN: bytes at [SysBase-446 .. -437] (the -444 entry)
        a.holdbeacon(0x0f00, 3)           # RED 3s: [4] dump starts
        for i in range(4):
            a.raw("1038%04x" % (4 + i))   # move.b $4+i,d0 (abs.W)
            a.pcrel_w("6100", "dhold")
        a.raw("2c780004")                 # a6 = SysBase
        a.holdbeacon(0x00f0, 3)           # GREEN 3s: table dump starts
        for i in range(10):
            a.raw("102e%04x" % (0x10000 - 446 + i))  # move.b -446+i(a6),d0
            a.pcrel_w("6100", "dhold")
        a.label("d444_loop")
        a.raw("33fc0ff000dff180")         # yellow tight loop = end
        a.pcrel_b("60", "d444_loop")
    elif variant == "dumpod":
        # dump 8 bytes at 0x00C0509C (the runtime OpenDevice target we
        # measured via dump444) as 8 color phases (~10s each)
        a.holdbeacon(0x0f00, 3)           # RED 3s: dump starts
        a.raw("207c00c0509c")             # movea.l #$c0509c,a0
        for i in range(8):
            a.raw("1028%04x" % i)         # move.b i(a0),d0
            a.pcrel_w("6100", "dhold")
        a.label("dod_loop")
        a.raw("33fc0ff000dff180")         # yellow tight loop = end
        a.pcrel_b("60", "dod_loop")
    elif variant == "oddirect":
        # bypass the runtime jump table: call the ROM's OpenDevice
        # implementation directly (ROM 0x6a2, mapped at 0x00F806A2).
        # First a control probe: OpenLibrary(-552) on dos.library
        # (proven-working LVO), then the direct ROM OpenDevice call.
        a.install_traphandler()
        a.raw("2c780004")                 # a6 = SysBase
        a.holdbeacon(0x0f00, 3)           # RED 3s: direct call begins
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq (41fb is a1!)
        for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44):
            a.raw("42a8%04x" % off)       # clr.l off(a0)
        a.raw("317c00300012")             # mn_Length = 48
        a.pcrel_w("41fa", "dev")          # a0 = devname
        a.raw("7000")                     # d0 = unit 0
        a.pcrel_w("41fb", "ioreq")        # a1 = ioreq
        a.marker("C")                      # args set
        a.raw("7200")                      # d1 = flags 0
        a.raw("4eb900f806a2")             # jsr ROM OpenDevice directly
        a.marker("D")                      # OpenDevice returned
        a.print_d0()
        a.raw("4a80")                      # tst.l d0
        a.pcrel_w("6600", "odd_fail")
        a.holdbeacon(0x0fff, 4)           # WHITE 4s: d0 == 0
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq
        a.raw("4eb900f806f0")             # jsr ROM CloseDevice (0x6f0)
        a.marker("E")
        a.holdbeacon(0x0ff0, 3)           # YELLOW 3s: done
        a.label("odd_loop")
        a.raw("33fc000f00dff180")         # hold BLUE = end reached
        a.pcrel_b("60", "odd_loop")
        a.label("odd_fail")
        a.marker("H")
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq
        a.raw("1028001f")                 # d0.b = io_Error (ioreq+0x1f)
        a.pcrel_w("6100", "dhold")        # display it ~10s
        a.holdbeacon(0x000f, 4)           # BLUE: returned with error
        a.label("oddf_loop")
        a.raw("33fc000f00dff180")
        a.pcrel_b("60", "oddf_loop")
    elif variant == "odsetup":
        # bisect: trap handler + entry RED + ioreq setup (leas, clrs,
        # mn_Length) + GREEN hold + yellow loop. Crash = the ioreq
        # setup; yellow = setup is clean.
        a.install_traphandler()
        a.holdbeacon(0x0f00, 3)           # RED 3s: entry
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq (41fb is a1!)
        for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44):
            a.raw("42a8%04x" % off)       # clr.l off(a0)
        a.raw("317c00300012")             # mn_Length = 48
        a.pcrel_w("41fa", "dev")          # a0 = devname
        a.raw("7000")                     # d0 = 0
        a.pcrel_w("41fb", "ioreq")        # a1 = ioreq
        a.raw("7200")                     # d1 = 0
        a.holdbeacon(0x00f0, 3)           # GREEN 3s: setup done
        a.label("ods_loop")
        a.raw("33fc0ff000dff180")         # yellow tight loop
        a.pcrel_b("60", "ods_loop")
    elif variant == "odclr":
        # bisect 2: RED + ioreq lea + 12 clr.l + mn_Length + GREEN
        a.install_traphandler()
        a.holdbeacon(0x0f00, 3)           # RED 3s: entry
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq (41fb is a1!)
        for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44):
            a.raw("42a8%04x" % off)       # clr.l off(a0)
        a.raw("317c00300012")             # mn_Length = 48
        a.holdbeacon(0x00f0, 3)           # GREEN 3s: done
        a.label("odc_loop")
        a.raw("33fc0ff000dff180")
        a.pcrel_b("60", "odc_loop")
    elif variant == "odlea":
        # bisect 3: RED + the three leas only + GREEN
        a.install_traphandler()
        a.holdbeacon(0x0f00, 3)           # RED 3s: entry
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq (41fb is a1!)
        a.pcrel_w("41fa", "dev")          # a0 = devname
        a.pcrel_w("41fb", "ioreq")        # a1 = ioreq
        a.holdbeacon(0x00f0, 3)           # GREEN 3s: done
        a.label("odl_loop")
        a.raw("33fc0ff000dff180")
        a.pcrel_b("60", "odl_loop")
    elif variant == "odbase":
        # display our segment base (lea 0(pc)) and the ioreq address
        # as color phases, to compare with fault addresses.
        a.install_traphandler()
        a.holdbeacon(0x0f00, 3)           # RED 3s: base display
        a.raw("41fa0000")                 # lea 0(pc),a0 = segment base
        a.raw("2008")                     # d0 = a0
        a.showphases()
        a.holdbeacon(0x00f0, 3)           # GREEN 3s: ioreq display
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq (41fb is a1!)
        a.raw("2008")                     # d0 = a0
        a.showphases()
        a.label("odb_loop")
        a.raw("33fc0ff000dff180")
        a.pcrel_b("60", "odb_loop")
    elif variant == "fulldirect":
        # THE INJECTOR: direct ROM calls (bypassing the broken runtime
        # jump table): OpenDevice=0x6a2, CloseDevice=0x6f0, DoIO=0x718
        # (ROM 0x00F8xxxx mirror), status via io_Error (ioreq+0x1f),
        # NOT the d0 return (OpenDevice ends with a Permit whose d0 is
        # garbage).
        a.install_traphandler()
        a.raw("2c780004")                 # a6 = SysBase
        a.holdbeacon(0x0f00, 2)           # RED 2s: entry
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq
        for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44):
            a.raw("42a8%04x" % off)       # clr.l off(a0)
        a.raw("317c00300012")             # mn_Length = 48
        a.pcrel_w("41fa", "dev")          # a0 = devname
        a.raw("7000")                     # d0 = unit 0
        a.pcrel_w("41fb", "ioreq")        # a1 = ioreq
        a.raw("7200")                     # d1 = flags 0
        a.raw("4eb900f806a2")             # jsr ROM OpenDevice
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq
        a.raw("1028001f")                 # d0.b = io_Error
        a.raw("4a00")                     # tst.b d0 (d0's upper bytes
        a.pcrel_w("6600", "odd_fail")     # hold Permit garbage!)
        a.holdbeacon(0x0fff, 2)           # WHITE 2s: device open!
        # settle, then build events and inject keys
        a.raw("303c%04x" % 0x384)
        a.label("fd_set")
        a.raw("323c2000")
        a.label("fd_s1")
        a.raw("51c9fffe")
        a.pcrel_w("51c8", "fd_set")
        a.pcrel_w("41fb", "events")       # a0 = ev1
        a.raw("43e80010")                 # a1 = ev2
        a.raw("2089")                     # ev1.next = ev2
        a.pcrel_w("41fc", "keys")         # a2 = keys
        a.label("fd_loop")
        a.raw("1442")                     # move.b (a2)+,d2
        a.raw("0c0200ff")                 # cmpi.b #$ff,d2
        a.pcrel_w("6700", "fd_done")      # beq done
        a.pcrel_w("41fd", "events")       # a3 = events
        a.raw("37420006")                 # ev1.ie_Code = d2
        a.raw("00020080")                 # ori.b #$80,d2 (release)
        a.raw("37420016")                 # ev2.ie_Code = d2
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq
        a.raw("317c0003001c")             # io_Command = CMD_WRITE (V33 input.device; 0x409 is V36)
        a.pcrel_w("43fb", "events")       # a1 = events
        a.raw("21490024")                 # io_Data = events
        a.raw("317c00020020")             # io_Length = 2
        # inline DoIO prologue (ROM 0x718-0x72c verbatim): BeginIO with
        # the QUICK flag set, skipping DoIO's wait loop (which calls
        # Wait through the broken runtime table)
        a.raw("2f09")
        a.raw("137c0001001e")
        a.raw("2f0e")
        a.raw("2c690014")
        a.raw("4eb900fe4cb8")         # jsr ROM input.device BeginIO
        a.raw("2c5f")
        a.raw("225f")
        a.pcrel_w("6100", "fd_keydelay")   # bsr.w keydelay
        a.pcrel_b("60", "fd_loop")         # bra loop
        a.label("fd_done")
        a.pcrel_w("41fa", "ioreq")
        a.raw("4eb900f806f0")             # jsr ROM CloseDevice
        a.holdbeacon(0x0ff0, 3)           # YELLOW 3s: done
        a.label("fd_end")
        a.raw("33fc000f00dff180")         # hold BLUE = end
        a.pcrel_b("60", "fd_end")
        a.label("odd_fail")
        a.pcrel_w("41fa", "ioreq")
        a.raw("1028001f")                 # d0.b = io_Error
        a.pcrel_w("6100", "dhold")        # display ~10s
        a.holdbeacon(0x000f, 4)           # BLUE
        a.label("fdf_loop")
        a.raw("33fc000f00dff180")
        a.pcrel_b("60", "fdf_loop")
    elif variant == "oddump":
        # display io_Device ([ioreq+0x14]) and the BeginIO vector
        # ([io_Device-30]) after a successful direct-ROM OpenDevice
        a.install_traphandler()
        a.raw("2c780004")                 # a6 = SysBase
        a.holdbeacon(0x0f00, 2)           # RED 2s: entry
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq
        for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44):
            a.raw("42a8%04x" % off)
        a.raw("317c00300012")             # mn_Length = 48
        a.pcrel_w("41fa", "dev")          # a0 = devname
        a.raw("7000")
        a.pcrel_w("41fb", "ioreq")        # a1 = ioreq
        a.raw("7200")
        a.raw("4eb900f806a2")             # jsr ROM OpenDevice
        a.holdbeacon(0x0fff, 2)           # WHITE 2s: open attempted
        a.pcrel_w("41fa", "ioreq")        # a0 = ioreq
        a.raw("20280014")                 # d0 = io_Device
        a.showphases()
        a.holdbeacon(0x00f0, 2)           # GREEN 2s: vector display
        a.pcrel_w("41fa", "ioreq")
        a.raw("20280014")                 # d0 = io_Device
        a.raw("2040")                     # a0 = io_Device
        a.raw("2028ffe2")                 # d0 = [io_Device-30] BeginIO
        a.showphases()
        a.label("odd_loop")
        a.raw("33fc000f00dff180")
        a.pcrel_b("60", "odd_loop")
    elif variant == "odname":
        # display the first longword at our devname (expect e.g.
        # 0x74696d65 "time") and the DeviceList head pointer at
        # [SysBase+0x15e], as color phases.
        a.install_traphandler()
        a.raw("2c780004")                 # a6 = SysBase
        a.holdbeacon(0x0f00, 3)           # RED 3s: devname display
        a.pcrel_w("41fa", "dev")          # a0 = devname
        a.raw("2010")                     # d0 = first 4 chars
        a.showphases()
        a.holdbeacon(0x00f0, 3)           # GREEN 3s: list head display
        a.raw("41ee015e")                 # lea $15e(a6),a0 (DeviceList)
        a.raw("2010")                     # d0 = lh_Head
        a.showphases()
        a.label("odn_loop")
        a.raw("33fc0ff000dff180")
        a.pcrel_b("60", "odn_loop")
    elif variant == "oddisable":
        # probe: call Disable via the runtime trampoline (jsr -120(a6))
        # from our context. Clean = trampolines work for us; crash =
        # every SysBase-vector call is poison in this context.
        a.install_traphandler()
        a.raw("2c780004")                 # a6 = SysBase
        a.holdbeacon(0x0f00, 3)           # RED 3s: entry
        a.raw("4eaefe88")                 # jsr -120(a6) Disable
        a.holdbeacon(0x00f0, 3)           # GREEN 3s: Disable returned!
        a.raw("4eaefe82")                 # jsr -126(a6) Permit
        a.label("odd2_loop")
        a.raw("33fc0ff000dff180")         # hold YELLOW = done
        a.pcrel_b("60", "odd2_loop")
    elif variant == "lvoprobe":
        # 2026-10-15: the 'poisoned trampoline' results were encoding
        # typos - 4eaefe88 is jsr -376(a6) (mid-table garbage), NOT
        # jsr -120(a6). V34 truth: Enable=-120, Disable=-126,
        # Forbid=-132, Permit=-138; CloseDevice=-450 (-504 is an
        # unrelated stub). This probe calls each with CORRECT
        # encodings, one beacon per step, then opens timer.device and
        # input.device via LVO -444 (io_Error shown as 0x1XX phases).
        a.install_traphandler()
        a.raw("2c780004")                 # a6 = SysBase
        a.holdbeacon(0x0f00, 4)           # RED: entry
        a.raw("4eaeff88")                 # jsr -120(a6) Enable
        a.holdbeacon(0x00f0, 4)           # GREEN: Enable returned
        a.raw("4eaeff82")                 # jsr -126(a6) Disable
        a.holdbeacon(0x0ff0, 4)           # YELLOW: Disable returned
        a.raw("4eaeff7c")                 # jsr -132(a6) Forbid
        a.raw("4eaeff76")                 # jsr -138(a6) Permit
        a.holdbeacon(0x00ff, 4)           # CYAN: Forbid/Permit returned
        # OpenDevice("timer.device") via LVO -444
        a.pcrel_w("41fa", "ioreq")
        for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44):
            a.raw("42a8%04x" % off)
        a.raw("317c00300012")             # mn_Length = 48
        a.pcrel_w("41fa", "dev")          # a0 = timer.device
        a.raw("7000")                     # d0 = unit 0
        a.pcrel_w("41fb", "ioreq")        # a1 = ioreq
        a.raw("7200")                     # d1 = flags 0
        a.raw("4eaefe44")                 # jsr OpenDevice(-444)
        a.pcrel_w("41fa", "ioreq")
        a.raw("4280")                     # clr.l d0
        a.raw("1028001f")                 # d0.b = io_Error
        a.raw("00400100")                 # ori.w #$100,d0 (marker)
        a.showphases()                    # io_Error as 0x1XX phases
        a.raw("4a00")                     # tst.b d0
        a.pcrel_w("6600", "lvp_fail1")
        a.holdbeacon(0x0fff, 4)           # WHITE: timer.device open
        a.pcrel_w("41fa", "ioreq")
        a.raw("4eaefe3e")                 # jsr CloseDevice(-450)
        a.holdbeacon(0x000f, 4)           # BLUE: closed
        a.label("lvp_fail1")
        # OpenDevice("input.device") via LVO -444 (expect graceful
        # failure under the lean startup: io_Error phases, no hang)
        a.pcrel_w("41fa", "ioreq")
        for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44):
            a.raw("42a8%04x" % off)
        a.raw("317c00300012")
        a.pcrel_w("41fa", "dev2")         # a0 = input.device
        a.raw("7000")
        a.pcrel_w("41fb", "ioreq")
        a.raw("7200")
        a.raw("4eaefe44")
        a.pcrel_w("41fa", "ioreq")
        a.raw("4280")
        a.raw("1028001f")
        a.raw("00400100")
        a.showphases()
        a.raw("4a00")
        a.pcrel_w("6600", "lvp_fail2")
        a.holdbeacon(0x0fff, 4)           # WHITE: input.device open!
        a.pcrel_w("41fa", "ioreq")
        a.raw("4eaefe3e")                 # jsr CloseDevice(-450)
        a.label("lvp_fail2")
        a.holdbeacon(0x0f0f, 4)           # MAGENTA: all steps done
        a.label("lvp_loop")
        a.raw("33fc0f0f00dff180")
        a.pcrel_b("60", "lvp_loop")
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
        a.pcrel_w("41fa", "ioreq")    # a0 = ioreq (41fb is a1!)
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
        a.pcrel_w("41fa", "ioreq")    # a0 = ioreq (41fb is a1!)
        a.raw("317c0409001c")         # io_Command = IND_WRITE
        a.pcrel_w("43fb", "events")   # a1 = events
        a.raw("21490024")             # io_Data = events (36)
        a.raw("317c00020020")         # io_Length = 2 (32)
        a.raw("4eaefe38")             # jsr DoIO(-456,a6)
        a.pcrel_b("61", "keydelay")   # bsr keydelay
        a.pcrel_b("60", "loop")       # bra loop

    if variant not in ("stack", "lvoprobe"):
        a.label("done")
        a.pcrel_w("41fa", "ioreq")    # a0 = ioreq (41fb is a1!)
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
        a.label("fd_keydelay")
        a.raw("323c1800")             # d1 = 0x1800
        a.raw("51c9fffe")
        a.raw("4e75")
        a.label("keydelay")
        a.raw("323c1800")             # d1 = 0x1800
        a.raw("51c9fffe")             # dbra d1,*
        a.raw("4e75")
        if trace or variant in ("serloop", "vtest", "oddirect"):
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
        if variant in ("peekvec", "odbase", "odname", "oddump", "lvoprobe"):
            a.emit_showphases()
        if variant in ("dump444", "dumpod", "oddirect", "odsetup", "odclr", "odlea", "odbase", "fulldirect"):
            # dhold: hold color = d0.w for ~10s
            a.label("dhold")
            a.raw("33c000dff180")           # move.w d0,color0
            a.raw("343c1af4")               # d2 = 6900 outer (~10s)
            a.label("dholdo")
            a.raw("33c000dff180")           # rewrite
            a.raw("323c0400")               # d1 = 1024 inner
            a.label("dholds")
            a.raw("51c9fffe")               # dbra d1,inner
            a.pcrel_w("51ca", "dholdo")     # dbra d2,outer
            a.raw("4e75")
    else:
        a.raw("33fc0ff000dff180")     # YELLOW: restored (stack variant)
        a.raw("2e5f")                 # restore SP
        a.raw("7000")
        a.raw("4e75")

    if variant == "lvoprobe":
        a.emit_showphases()

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
    data += b"input.device\x00"                          # dev2
    data += b"\x00" * 36                                # replyport
    data += b"\x00" * STACK_BYTES                       # stack buffer
    while len(data) % 4:
        data += b"\x00"

    # bind the data labels (emitted as pcrel from code) to their offsets
    ioreq = body_len
    events = ioreq + 48
    keys = events + 32
    dev = keys + len(keys_blob)
    dev2 = dev + len(DEVNAME)
    replyport = dev2 + 13
    stacktop = replyport + 36 + STACK_BYTES
    for name, val in (("ioreq", ioreq), ("events", events),
                      ("keys", keys), ("dev", dev), ("dev2", dev2),
                      ("replyport", replyport), ("stacktop", stacktop)):
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
