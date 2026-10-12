# Reverse Engineering AmigaDOS Ed

A legal reverse-engineering project studying **Ed**, the full-screen text
editor shipped with classic AmigaOS, with the goal of producing a complete,
publishable behavior specification — and optionally a clean-room
reimplementation.

Target: **Ed 1.14** from **Workbench 1.3** (AmigaOS 1.3, 1988), the classic
BCPL-era editor.

## What is Ed?

Ed is the screen editor that shipped with every classic AmigaOS release
(1.0 through 3.1). Unlike the Workbench Notepad, it has no icon: it is an
AmigaDOS command (`ED <file> [SIZE n]`) run from the CLI. It is a
programmer's editor — no fonts, no styles, no mouse in the 1.x versions —
but with block operations, search/exchange, command grouping, and repeat
counts.

Technically it is an interesting specimen:

- The 1.x Ed is a **BCPL program** descended from TRIPOS, the operating
  system AmigaDOS was built from (written by MetaComCo). It uses the BCPL
  runtime, the globals vector ("globvec"), and DOS packets for file I/O.
- It ships in the **old TRIPOS load format** (magic `0x000003F3` without a
  standard `HUNK_HEADER`), not the regular Amiga executable format —
  the version analyzed here consists of nine `0x3E9`-delimited code hunks,
  19,400 bytes of 68000 machine code, and no data hunks.
- From AmigaOS 2.0 onward, AmigaDOS was rewritten in C and the shipped Ed
  gained new features (mouse selection, command files, menus) — this
  project targets the earlier 1.x lineage.

## Goals

1. **Understand** — how Ed works: startup, buffer model, command language,
   file handling, save/backup behavior.
2. **Document** — publish a behavior specification of Ed 1.14 written in
   our own words, derived from analysis of a lawfully owned binary plus
   the publicly documented (manual) behavior.
3. **(Optional) Reimplement** — a clean-room port in Rust built only from
   the spec, differentially tested against the original in an emulator.

## Legal approach

This project is set up to stay on the legal side of the line. The full
argument — copyright status, statutory basis per jurisdiction, and the
hard rules — is stated in **[LEGAL.md](LEGAL.md)**. In brief:

**Lawful source.** The binary under analysis comes from the user's own
**Amiga Forever** license (Cloanto, the current rightsholder of classic
AmigaOS). Reverse engineering a lawfully owned copy for study and
interoperability is broadly permitted (US: *Sega v. Accolade*, DMCA
§1201(f), 17 U.S.C. §117; EU: Directive 2009/24/EC Arts. 5–6; UK: CDPA
s.50B).

**Rules followed throughout:**

1. **Never touch the leaked AmigaOS sources** circulating online. Using
   them would taint the project and turn analysis into infringement.
2. **No redistribution** of the Ed binary, disk images, ROMs, or derived
   listings. Everything in `materials/`, `ref/`, and `af-extract/` stays
   local.
3. **Publish facts, not code.** Command sets, limits, file formats, and
   algorithms described in our own words are not copyrightable; the spec
   and docs are written to be publishable. Disassembled source is not
   published.
4. **Clean-room discipline** for any reimplementation: the spec is written
   by the analysis side; the implementation side sees only the spec.

## How the work is done

1. **Acquire** the target from the licensed Amiga Forever package
   (Workbench 1.3 disk image).
2. **Extract** the disk contents with our own OFS filesystem reader
   (`tools/adf_extract.py`), whose on-disk layout was verified empirically
   against the real image.
3. **Triage** the binary: load-format parsing, segment map, BCPL string
   dump (`tools/hunk_dump.py`).
4. **Cross-reference** the findings with the published AmigaDOS
   manuals (1987 2nd edition documents this era of Ed; later editions
   describe the newer 2.x Ed and were used only to delimit differences).
5. **Write the spec** (`docs/ed-spec.md`), tagging every claim as
   `[BIN]` (verified in binary), `[DOC]` (documented), or `[VERIFY]`.
6. **Verify dynamically** in WinUAE/FS-UAE and with a disassembly pass
   (planned).

## Project layout

```
README.md     this file — project overview and ground rules
LEGAL.md      the legal argument and hard rules
BCPL.md       BCPL language reference and all project Gotchas
TODO.md       progress tracker — what is done and what is next
docs/
  ed-spec.md  the behavior specification of Ed 1.14 (main deliverable)
ed-rs/        the clean-room Rust reimplementation (the clone)
  src/
    buffer.rs    line-buffer model (255-char lines, block marks, wrap)
    cmdline.rs   extended command parser + executor (38 commands)
    screen.rs    full-screen TUI (crossterm)
    headless.rs  --script mode for non-interactive testing
    main.rs      CLI entry, FROM/A,SIZE template
tools/
  adf_extract.py   AmigaDOS OFS (DOS\0) ADF extractor, layout-verified
  inspect_adf.py   raw block inspector for ADF images
  hunk_dump.py     TRIPOS/standard Amiga executable parser + string dump
  disasm_ed.py     Capstone m68k disassembler for the Ed binary
  call_census.py   runtime call-site census
  make_gvdump.py   hand-assembled memory-dump tool builder
  *.ps1            emulator automation harness (screenshot/OCR/keyboard)
ref/          research copies of the AmigaDOS manual text (copyrighted:
              local only, never redistribute)
materials/    files extracted from the Workbench 1.3 disk image
              (copyrighted: local only, never redistribute)
af-extract/   administrative (no-install) extract of the Amiga Forever MSI
emulator/     emulator, ROM, disk images, configs, screenshots
              (copyrighted: local only, never redistribute)
```

## Current state

- Ed 1.14 extracted and triaged; load format identified as the 1.x-era
  Amiga hunk format (ATRM "Amiga Binary File Structure") with nine code
  hunks.
- Behavior spec substantially **live-verified** under genuine Kickstart
  1.3: SH display format, parameter commands, block lifecycle, Q/EQ
  prompts, word wrap, and a corrected backup path (`t/ed-backup`
  relative to the current directory, not the T: volume).
- Disassembly pass (Capstone): the complete extended-command dispatch
  found and verified (38 commands, none undocumented); the BCPL calling
  convention reconstructed (`jsr (a5)` = CALLPROC with the procedure
  pointer in `a4`); segment roles mapped; template corrected to
  `FROM/A,SIZE`; two manual-vs-binary discrepancies documented (right
  margin 077 not 79; SIZE floor 10000 not 40000).
- **Rust MVP (ed-rs/)**: clean-room command engine with the full 38-command
  set, buffer model, block semantics, SH display, save/backup, verified
  message set, and a headless `--script` mode for testing. Command engine
  proven by scripted smoke tests; TUI compiles, interactive testing pending.
- Runtime slot naming parked (not needed for the MVP; documented in TODO
  section 7 with resume instructions).

## Running the clone

**Prerequisites:** [Rust](https://rustup.rs) (any recent stable; the
`crossterm` crate is the only dependency). No Amiga ROMs or emulator
needed — the clone is a native Windows/Linux/macOS binary.

**Build:**

```
cd ed-rs
cargo build --release
```

The binary is at `ed-rs/target/release/ed` (`.exe` on Windows).

**Edit a file (interactive):**

```
ed-rs/target/release/ed myfile.txt          # Linux/macOS
ed-rs	argetelease\ed.exe myfile.txt     # Windows
```

**Optional buffer size** (keyword or positional; minimum 10,000 bytes,
matching the binary's `0x2710` clamp, not the manual's 40,000):

```
ed.exe myfile.txt SIZE 50000
ed.exe myfile.txt 50000
```

**Run a script of Ed commands headlessly** (for testing and CI):

```
ed.exe --script commands.ed myfile.txt
```

**In the editor:**

| Key | Action |
|---|---|
| type | insert at cursor (word wrap at column 77) |
| arrows | move cursor |
| BACKSPACE / DEL | delete left / at cursor |
| RETURN | split line |
| TAB | next tab stop (distance 3, inserts nothing) |
| CTRL-A | insert blank line after |
| CTRL-B | delete line |
| CTRL-D / CTRL-U | scroll 12 lines toward start / end |
| CTRL-E | top of screen (or bottom if already there) |
| CTRL-F | flip case of letter at cursor |
| CTRL-G | repeat last extended command |
| CTRL-O | delete word or spaces |
| CTRL-R / CTRL-T | previous / next word |
| CTRL-V | redraw |
| CTRL-Y | delete to end of line |
| **ESC** | enter extended command mode |
| ESC + RETURN | execute command, back to typing |
| ESC + ESC | execute command, stay in command mode |

Extended commands: A, B, BE, BF, BS, CE, CL, CR, CS, D, DB, DC, E, EQ,
EX, F, I, IB, IF, J, LC, M, N, P, Q, RP, S, SA, SB, SH, SL, SR, ST, T,
U, UC, WB, X, plus `(` `)` for groups and numbers for repeat counts.
See `docs/ed-spec.md` section 5 for the full reference.

**Note on X:** saving with X writes the FROM file and also a backup
to `t/ed-backup` relative to the current directory (this is the
corrected backup path from the binary analysis).

## Roadmap

- [x] Legal framework and project setup
- [x] Binary extraction and triage
- [x] Behavior spec draft
- [x] Dynamic verification in an emulator (genuine KS 1.3)
- [x] Disassembly pass: command dispatch verified, calling convention reconstructed
- [x] Clean-room Rust MVP: command engine + buffer + save/backup (TUI interactive testing pending)
- [ ] Differential testing: Rust clone vs original in emulator
- [ ] Version comparison (Ed 1.10 / 1.14 / 2.x / 3.1)

## License

The original work in this repository (tools, documentation, specification)
is licensed under the MIT License — see [LICENSE](LICENSE). This covers only
material authored by this project. Ed, AmigaOS, and the manuals referenced
remain the property of their rights holders (see [LEGAL.md](LEGAL.md));
no Commodore/Cloanto material is included in this repository.
