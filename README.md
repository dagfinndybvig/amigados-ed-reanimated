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
3. **(Optional) Reimplement** — a clean-room port built only from the
   spec, differentially tested against the original in an emulator.

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
LEGAL.md      the legal argument and hard rules (see LEGAL.md)
TODO.md       progress tracker — what is done and what is next
docs/
  ed-spec.md  the behavior specification of Ed 1.14 (main deliverable)
tools/
  adf_extract.py   AmigaDOS OFS (DOS\0) ADF extractor, layout-verified
  inspect_adf.py   raw block inspector for ADF images
  hunk_dump.py     TRIPOS/standard Amiga executable parser + string dump
ref/          research copies of the AmigaDOS manual text (copyrighted:
              local only, never redistribute)
materials/    files extracted from the Workbench 1.3 disk image
              (copyrighted: local only, never redistribute)
af-extract/   administrative (no-install) extract of the Amiga Forever MSI
```

## Current state

- Ed 1.14 extracted and triaged; load format identified as the old
  TRIPOS/BCPL format with nine code hunks.
- Complete command set, screen model, buffer limits, and save/backup
  semantics documented in `docs/ed-spec.md`, cross-checked against the
  binary's string table (all error messages, prompts, and the
  `:T/ED-BACKUP` backup path confirmed).

## Roadmap

- [x] Legal framework and project setup
- [x] Binary extraction and triage
- [x] Behavior spec draft
- [ ] Dynamic verification in an emulator
- [ ] Disassembly pass: command-letter table, undocumented behavior
- [ ] Version comparison (Ed 1.10 / 1.14 / 2.x / 3.1)
- [ ] (Optional) clean-room reimplementation + differential testing

## License

The original work in this repository (tools, documentation, specification)
is licensed under the MIT License — see [LICENSE](LICENSE). This covers only
material authored by this project. Ed, AmigaOS, and the manuals referenced
remain the property of their rights holders (see [LEGAL.md](LEGAL.md));
no Commodore/Cloanto material is included in this repository.
