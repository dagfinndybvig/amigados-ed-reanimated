# TODO.md — Project Progress Tracker

Status legend: [x] done · [ ] to do · [~] in progress · [-] optional / deferred

## 1. Setup and legal

- [x] Project directory and README.md (overview, goals, methodology)
- [x] LEGAL.md — full legal argument, per jurisdiction, with hard rules
- [x] .gitignore excluding copyrighted material (materials/, ref/, af-extract/)
- [ ] Copy LEGAL.md rules into a short CONTRIBUTING note if the project
      ever goes public

## 2. Acquisition and extraction (DONE)

- [x] Locate Amiga Forever materials (uninstalled
      `AmigaForever10Plus.msi` + Workbench 3.1 adf/rom in Amiga_setup)
- [x] Administrative install (msiexec /a) of the MSI into af-extract/
- [x] Identify Workbench 1.3 disk image (amiga-os-134-workbench.adf)
- [x] Write OFS extractor (tools/adf_extract.py); on-disk layout verified
      empirically (hash table at 24, file size at 324, next-chain reads)
- [x] Extract all 166 files from Workbench 1.3 to materials/workbench13/
- [x] Extract Ed (c/Ed, 19,564 bytes)

## 3. Triage (DONE)

- [x] Write hunk/triage dumper (tools/hunk_dump.py) supporting both
      standard and old TRIPOS load formats
- [x] Identify load format: old TRIPOS/BCPL, 9 code hunks, 19,400 bytes,
      0x3E9/0x3F2 segment markers
- [x] Identify version: Ed 1.14 (strings "Ed 1.14", "Ed (1.14)")
- [x] Dump full BCPL string table (commands' prompts, errors, window spec
      RAW:0/0/639/199/Ed 1.14, :t/ed-backup, SH status labels)

## 4. Documentation research (DONE)

- [x] COMPUTE! issue 73 (June 1986) Ed 1.10 article — immediate and
      extended command tables
- [x] AmigaDOS Manual 2nd edition (1987) — complete Ed 1.x chapter
      (commands, margins, blocks, undo, save/backup semantics)
- [x] AmigaDOS Manual 3rd edition (1991) — consulted only to identify
      OS 2.0-era additions (mouse, WITH, DF, EM) not present in 1.14

## 5. Behavior specification (DRAFT DONE)

- [x] docs/ed-spec.md v0.1 — full command tables, startup template,
      screen/buffer model, block and undo semantics, save/backup,
      complete observed error-string set; claims tagged
      [BIN]/[DOC]/[VERIFY]
- [ ] Convert remaining [VERIFY] tags to [BIN]/[DOC] (see section 6)

## 6. Verification (LIVE SESSION 2026-10-11 — mostly done)

Environment ended up as: Amiga Forever player (installed) running the real
Workbench 1.3 environment with genuine Kickstart 1.3; Ed driven via
keyboard automation + screenshot/OCR. Note: standalone WinUAE could not
use Cloanto's encrypted (AMIROMTYPE1) ROMs; the AF player decrypts at
runtime.

- [x] Emulation environment with real Kickstart 1.3
- [x] Run Ed 1.14 live; window title "Ed 1.14" confirmed
- [x] SH status display: full field list, defaults (tab 003, left margin
      001, right margin 077 — conflicts with manual's 79), block fields
      show line text / Unset, "Type any character to continue"
- [x] Failed find message: "search failed" (not in static string dump!)
- [x] EQ prompt "Exchange? " + Y applies the exchange
- [x] Block lifecycle: BS;BE / IB / DB, and "No block marked" after DB→IB
- [x] ST/SL/SR with SH readback (111/010/060); semicolon command lines;
      "command space number" syntax
- [x] Word wrap live test (SR 20 → wrapped short lines)
- [x] Q confirmation: "Edits will be lost - type Y to confirm: "; N stays
- [x] X save order + backup path correction: FROM file written first, then
      t/ed-backup relative to the CURRENT DIRECTORY (leading-colon syntax,
      not the T: volume); failed backup on protected volume → DOS
      requester → Cancel → Ed exits with FROM file already saved
- [x] BONUS FINDING: Ed 1.14 fails under Kickstart 3.1 (Software Failure
      #87000004) while other 1.3 BCPL commands run — needs disassembly
      follow-up
- [ ] Remaining live tests: M n, repeat counts, groups/RP, UC/LC, U undo,
      IF/WB file ops, tab-distance behavior, buffer-usage growth, positive
      t/ed-backup path in a writable directory
- [ ] Trace DOS packet I/O with the emulator debugger (breakpoints on
      file reads/writes during startup and save)

## 7. Disassembly pass (IN PROGRESS)

Tooling: Capstone m68k via tools/disasm_ed.py (chosen over Ghidra/IRA for
scriptable pattern analysis; full listing at emulator/ed.asm, 6417 lines).

- [x] Disassembly tooling working (linear disasm + SKIPDATA + immediates
      scan)
- [x] Extended-command dispatch located (seg6 0x07D6-0x0988): binary
      compare tree, 15 single letters + ( ) + 23 two-letter commands —
      exactly matches the spec table, NO undocumented commands; BF shares
      F's handler, EQ shares E's; unknowns fall to common branch
- [x] Template string found in binary: "FROM/A,SIZE" (no /K — SIZE accepts
      positional form)
- [x] String-table scan bug fixed (word-padding); new messages: End of
      file, Unmatched (), Number expected, Cursor inside block,
      Search failed, Buffer corrupt
- [x] BCPL calling convention reconstructed: jsr (a5) = globvec dispatch,
      d0 = byte offset of slot, args in d1-d3/a4; seg0 = globvec init
      filling 0x474Cxxxx GL-glue references (AmigaDOS BCPL glue mechanism)
- [x] Call census (tools/call_census.py): 545 jsr (a5) sites, 19 distinct
      slot offsets (12..120 = globvec entries 3..30) — Ed's complete
      runtime API surface
- [ ] Name the 19 globvec slots: need KS 1.3 dos.library glue table
      (GL-index order). Options: live observation in emulator debugger
      with known DOS calls as anchors; TRIPOS BCPL function list
- [ ] Immediate/CTRL-key dispatch (seg8): letters via moveq + globvec
      call 28; tabulate the full key map
- [ ] Right-margin default: SH live readback says 077; no 0x4f (79)
      immediate anywhere in the code; locate the init (data table?) to
      close the manual conflict
- [ ] Annotate findings back into the spec ([BIN] tags) — partly done
- [-] OPTIONAL AID — BCPL compiler for idiom comparison (researched
      2026-10-11; clearly BCPL-related, hence recorded here):
      compile small test programs with a period-correct BCPL compiler to
      learn compiler idioms (globvec JSR (A5) call patterns, prologues,
      BCPL string layout), then pattern-match against Ed's segments.
      Toolchain options, by provenance:
      1. Original MetaComCo BCPL compiler + ALINK (the tool that built
         Ed, emitting the TRIPOS load format we detect): only cleanly
         available inside the leaked AmigaOS tree — OFF-LIMITS
         (LEGAL.md §4). The 1985 Commodore dev kit shipped Macro
         Assembler/ALINK/C, not the BCPL compiler. Treat as unavailable.
      2. Arnor BCPL Compiler v1.0 (1986, Amiga): runnable disk image on
         archive.org (BCPL_Compiler_v1.0_1986_Arnor, .dsk 194KB, runs in
         WinUAE). Commercial product, murky rights status — local private
         study only, never redistribute; weaker provenance than our
         Amiga Forever baseline.
      3. Martin Richards' BCPL (cl.cam.ac.uk/~mr10, free, source
         available; includes Cintpos, TRIPOS's descendant): clean
         provenance, but targets Cintcode on modern hosts — producing
         Amiga m68k binaries means porting a code generator.
      Not needed for the spec or the Rust port (clean-room from spec
      only, LEGAL.md §6); pure disassembly aid.

## 8. Version comparison (TO DO)

- [ ] Extract Ed from Workbench 1.2 (amiga-os-120-workbench.adf) and diff
- [ ] Extract Ed from Workbench 3.1 (user's amiga.adf) and diff the
      command set against the 1991 manual's list
- [ ] Write version-history section (1.10 → 1.14 → 2.x → 3.1)

## 9. Clean-room reimplementation in Rust (OPTIONAL — do not start before
      sections 6–7 are complete)

- [ ] Target language: Rust (decided); portable console clone
- [ ] Freeze the spec as the clean-room input
- [ ] Implement from the spec only (see LEGAL.md §6)
- [ ] Differential testing against original in emulator: same inputs,
      compare outputs/behavior

## 10. Open questions

- [ ] Does Ed 1.14 accept any template arguments beyond FROM and SIZE?
      (spec §9) — resolve via disassembly or live testing
- [ ] Meaning of the "12;0;0" string in segment 2 — likely console
      control; confirm during dynamic analysis
- [ ] Confirm whether any WB 1.3-era Ed revision differences exist vs
      the 1987 manual (manual covers 1.2-era Ed; binary is 1.14)
