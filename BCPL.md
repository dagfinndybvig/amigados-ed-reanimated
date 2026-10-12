# BCPL.md — Everything We Know About BCPL, and Why It Matters Here

A working reference for the language of AmigaDOS 1.x and the Gotchas it
has caused in this project. Findings are tagged: **[BIN]** from code
analysis, **[LIVE]** from emulator observation, **[DOC]** from published
sources, **[GOTCHA]** where it bit us.

## 1. What BCPL is

BCPL (Basic Combined Programming Language) is a typeless, procedural
language designed by Martin Richards in 1966 at Cambridge and London
Universities, derived from CPL. It compiles to native code (via an
intermediate Ocode) and has no type declarations: every value is a
machine word, pointers are integers, and the language has no
compile-time type checking.

BCPL is the direct ancestor of B (and therefore C). Its influence on C is
visible everywhere: `++`/`--`, `if`/`while`/`for` block structure,
pointer arithmetic, separate compilation with a linker, and the
standard-library style (`writef`, `getvec`, etc.).

## 2. Why it matters to this project

AmigaDOS 1.x (the CLI, Shell, and all the C: commands including **Ed**)
was written in **BCPL** by MetaComCo, derived from their TRIPOS
operating system. Commodore contracted MetaComCo to port TRIPOS to the
Amiga; the result became the `dos.library` and command set of
AmigaDOS 1.0 through 1.3.

From AmigaOS 2.0 onward, AmigaDOS was rewritten in C and assembly,
retaining 1.x BCPL compatibility through a translation layer. This is
the era boundary that defines our target: **Ed 1.14 is a BCPL program**,
and its runtime conventions are BCPL conventions.

## 3. BCPL on the Amiga — the specific facts we have established

### 3.1 Executable format

**[BIN]** Ed 1.14 ships in the 1.x-era Amiga hunk format documented in
the AmigaDOS Technical Reference Manual (ATRM, "Amiga Binary File
Structure"). This format is *not* the modern Amiga hunk format:

| Field | 1.x-era (ATRM) | Modern (post-2.0) |
|---|---|---|
| Header block | `0x3F3` (1011, hunk_header) | `0x3F3` magic + `0x3F4` HUNK_HEADER |
| Code block | `0x3E9` (1001, hunk_code) | `0x3EA` (1002, HUNK_CODE) |
| End block | `0x3F2` (1010, hunk_end) | `0x3F3` (1011, HUNK_END) |
| Data block | `0x3EA` (1002, hunk_data) | `0x3EB` (1003, HUNK_DATA) |

The 1.x header contains a resident-library name list (zero-terminated)
followed by the hunk table size, first/last hunk slots, and per-hunk
sizes in longwords. The modern format wraps the same idea differently
and is **rejected by the 1.3 loader**.

**[GOTCHA]** Our first hand-assembled test program used the modern
format; the 1.3 Shell silently failed to load it. The fix was to use
the ATRM numbering.

### 3.2 Load and entry

**[BIN]** The file contains a hunk table (sizes in longs) followed by
code hunks delimited by `0x3E9` (code) and `0x3F2` (end). Ed has nine
code hunks, 19,400 bytes total. There are no data or BSS hunks — all
data lives in the globals vector (see below).

**[GOTCHA]** The entry point is not documented in any manual we found.
We assumed the C-ABI convention (`A6 = DOSBase`) because that is how
the ROM Kernel Manual describes `CreateProc`. This was **wrong**: the
1.3 Shell enters commands with the BCPL convention instead, and a test
program using `jsr -30(a6)` crashed immediately ("Software error —
task held").

### 3.3 BCPL calling convention (the critical discovery)

**[BIN, cross-verified against the Type binary]** This is the single
most important Gotcha:

- `jsr (a5)` does **not** dispatch through a slot table. It is the
  BCPL **CALLPROC** instruction: it calls the procedure whose pointer
  is in register `a4`.
- The `a4` value is loaded from `off(a2)` — a2 is the program's
  globals area (the global vector), and the offsets are fixed per
  program.
- `d0`–`d3` hold the first four scalar arguments.
- `a5` points at the runtime's call-thunk block (small helpers at
  fixed A5-relative offsets: `jsr $10(a5)`, `jsr $12(a5)`).
- `a6` holds the **return address** (BCPL procedures return with
  `jmp (a6)`).

**[GOTCHA]** Our first attempt to model this ("19 distinct function
numbers in d0") was completely wrong: the d0 values are just common
first arguments (12, 16, 20, etc.), not function indices. The function
identity is encoded in the `(a2)` global offset from which a4 is
loaded. This invalidated a full session's census interpretation.

### 3.4 Global vector (globvec)

**[BIN]** Each BCPL process has a global vector — an array of 32-bit
words. Some entries hold data (the program's global variables), others
hold procedure pointers. In AmigaDOS, `DosLibrary.dl_GV` (at offset
0x26 from the library base) points to the **shared global vector**
used by all internal DOS calls.

Ed's segment 0 is its globvec initializer: a fill loop writes
`0x474C0003, 0x474C0005, ...` — the "GL" magic + sequential glue
index — into the allocated vector. This is the standard AmigaDOS BCPL
glue mechanism where dos.library resolves each reference lazily.

**[GOTCHA]** The byte pair `0x474C` appears **nowhere** in the ROM, so
the GL references are not resolved by a simple ROM-side magic compare.
Either the resolver is in a RAM-resident overlay of dos.library, or the
`0x474Cxxxx` values must be re-read under the corrected CALLPROC
convention. This is the open question that parked the slot-naming
campaign.

### 3.5 String format

**[BIN]** BCPL strings are length-prefixed (one byte of length followed
by up to 255 characters), padded to even length. This is not the C
convention of null-terminated.

The AmigaDOS BCPL routines pass strings as **word pointers** (byte
address >> 2), not byte pointers. This is visible in compiled code as
`lsr.l #2, d1` before passing a string to a DOS routine.

**[GOTCHA]** Our string scanner (`tools/hunk_dump.py`) initially had a
word-padding bug: after a string of odd byte-length, the scan
skipped one byte too many and missed subsequent strings. This cost us
six real message strings ("Search failed", "Unmatched ()", "Number
expected", "Cursor inside block", "Buffer corrupt", "Right margin") —
all of which turned out to be genuine Ed messages that should have been
in the spec from the start.

### 3.6 Memory and pointers

**[BIN]** BCPL on the Amiga uses **word addressing**: pointers are
(byte address >> 2). A pointer of value 3 means byte offset 12. This
is visible everywhere in Ed's compiled code:

```m68k
movea.l a3, d1
lsr.l   #$2, d1      ; convert byte pointer to BCPL word pointer
moveq   #$1c, d0     ; pass to a routine
jsr     (a5)
```

**[GOTCHA]** Attempting to call dos.library routines with byte
pointers (the C convention) will silently pass garbage. You must
`lsr.l #2` first.

### 3.7 The 1.x dos.library ABI

**[GOTCHA — THREE ITERATIONS]** The biggest single time sink in the
dump-tool campaign. We tried three calling conventions for
dos.library's LVO entries (`Open`, `Write`, `Close` at LVOs -30, -48,
-36):

1. **Standard C stack-args (pea/pea/jsr)** — crash. The 1.x
   dos.library does not read arguments from the stack the way modern
   libraries do.
2. **Reversed stack order** — crash. The documented "1.x reversed
   stack args" quirk alone doesn't help because...
3. **Register args (D1-D3)** — also crashed, but for a different
   reason: we were passing byte pointers instead of word pointers.

The correct combination is likely register args with word pointers,
but we have not yet isolated it definitively. The campaign is parked;
resume by disassembling the 1.3 ROM's dos.library LVO entry code to
read the actual ABI.

**[LIVE]** What we *did* prove works:
- `movea.l $4.w, a6` → `jsr -$228(a6)` (OpenLibrary) works; exec.library
  uses standard conventions even in 1.3.
- A pure `moveq #0, d0; rts` program loads, runs, and returns cleanly.
- The 1.x ATRM load-file format is correct (the 1.3 Shell loads and
  runs it).

## 4. BCPL language features relevant to Ed

Ed is a BCPL program, so its design reflects BCPL idioms:

- **No structs**: BCPL has no user-defined types. Data is addressed as
  integer offsets from a base pointer. Ed's buffer state lives at
  fixed offsets in its global vector ($260–$2C8 for the line-array
  base, count, capacity, gap pointers).
- **Vector-of-lines**: Ed's line buffer is a simple array of
  word-pointers to line data, manipulated with explicit
  move/shift/copy loops — no linked lists, no realloc.
- **Recursive descent**: The command-line parser (groups, RP, repeat
  counts) is a small recursive parser, not a table-driven one. The
  command dispatch in segment 6 is a binary compare tree over the
  32-bit command word.
- **The 255-char line limit**: BCPL strings are length-byte +
  max-255-char; Ed inherits this as its maximum line length.

## 5. Known BCPL implementations for the Amiga

**[DOC]** Based on our research:

1. **Original MetaComCo BCPL toolchain** — the compiler + ALINK that
   built Ed itself. Produces the 1.x-era load format. Not publicly
   available outside the leaked AmigaOS source tree (**off-limits**,
   LEGAL.md §4).
2. **Arnor BCPL Compiler v1.0 (1986)** — downloaded and inspected;
   it is an **Amstrad CPC/PCW** product (CPCEMU `.dsk` image, AMSDOS),
   not Amiga software. Cannot produce Amiga binaries.
3. **Martin Richards' BCPL** (cl.cam.ac.uk/~mr10, free, source
   available; includes Cintpos, the TRIPOS descendant) — clean
   provenance, but targets Cintcode on modern hosts; producing
   Amiga m68k binaries means porting a code generator.

**[GOTCHA]** There is no readily available, legal, Amiga-targeting
BCPL compiler. The original toolchain is effectively unobtainable
without touching the leaked tree, and no third-party ever released
one for the Amiga m68k. For idiom comparison during disassembly, the
only practical route is to compile small test programs with a
different-era BCPL (e.g., Martin Richards' modern distribution) and
compare code patterns qualitatively.

## 6. Summary of Gotchas

| # | Gotcha | Cost | Lesson |
|---|---|---|---|
| 1 | Wrong load-file format (modern vs 1.x hunk numbering) | One boot cycle | Always check the era-appropriate manual (ATRM for 1.x) |
| 2 | Wrong entry convention (assumed C-ABI `A6=DOSBase`) | Two boot cycles + a guru | The 1.3 Shell enters commands with BCPL conventions |
| 3 | Wrong calling convention model (d0 = function number) | A full session's census | `jsr (a5)` = CALLPROC via `a4`; d0-d3 are args |
| 4 | Wrong dos.library ABI (stack args / reversed / registers) | Three boot cycles | 1.x dos.library has a non-standard ABI; read the ROM |
| 5 | Byte pointers vs BCPL word pointers | One boot cycle | Always `lsr.l #2` before passing pointers |
| 6 | String scanner word-padding bug | Six missed message strings | `(1 + n) & 1`, not `n & 1` |
| 7 | Missing SH label "Right margin" | Spec completeness | Same scanner bug as above |
| 8 | No legal Amiga BCPL compiler available | Blocked idiom comparison | Documented; alternative routes listed |
| 9 | CLI window closes on EndCLI, hiding all error output | A whole evening's misdiagnosis | Keep the startup CLI open when testing custom commands |
| 10 | d0 at rts = the CLI's return code (garbage d0 prints "failed 65535") | Several wrong "crash" theories | Always `moveq #0,d0` before rts in custom commands |
| 11 | Bcc/BSR word-form and DBcc displacements are relative to the extension word (instruction+2), not the instruction end | Two gurus in hand-built programs | Word-branch ref = instr+2; DBcc self-loop = FFFE |
| 12 | MOVE encoding: source mode in bits [5-3], source register in bits [2-0] | Several bad encodings | Verify hand encodings against ROM bytes + capstone |

### 6.1 Single-hunk ATRM programs: proven working (2026-10-13)

Small hand-assembled programs in the 1.x-era single-hunk format
(`0x3F3` header, one `0x3E9` code hunk, `0x3F2` end) load and run
fine under the 1.3 Shell, entered at hunk 0 offset 0, returning
with `rts` (exit code = d0). This corrects the parked gvdump
conclusion that the 1.x program ABI was unresolved — those runs
were misled by gotchas 9 and 10 above. The remaining open problem
(see TODO 6b item 3) is a Software Failure inside an
OpenDevice-based injector, not the load format or entry
convention.

## 7. Sources

- AmigaDOS Technical Reference Manual (1985, MetaComCo/Commodore) — the
  hunk format, calling conventions, and DosLibrary structure.
- AmigaDOS Manual, 2nd edition (1987, Bantam) — BCPL and TRIPOS
  background, Ed chapter.
- AmigaOS Wikipedia article — BCPL → C rewrite timeline.
- Martin Richards' BCPL page (cl.cam.ac.uk/~mr10/BCPL.html) — language
  history, free implementations, Cintpos.
- Retrocomputing Stack Exchange — dos.library BCPL internals.
- Our own binary analysis (tools/disasm_ed.py, tools/call_census.py,
  tools/hunk_dump.py) — the concrete findings tagged [BIN] above.
