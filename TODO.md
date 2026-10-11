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

## 6. Verification (TO DO — next phase)

- [ ] Set up WinUAE or FS-UAE with Workbench 1.3 environment
- [ ] Run Ed 1.14 live; verify startup, window, status line
- [ ] Verify every extended command against the spec tables
- [ ] Verify save/backup: X writes FROM file and :T/ED-BACKUP
- [ ] Verify edge cases: line >255 on input, tab expansion, binary-file
      rejection, word wrap at right margin, Q confirmation flow
- [ ] Trace DOS packet I/O with the emulator debugger ( breakpoints on
      file reads/writes during startup and save )

## 7. Disassembly pass (TO DO)

- [ ] Import segments into Ghidra (m68k) or run IRA
- [ ] Reconstruct BCPL globvec usage and runtime conventions
- [ ] Locate the extended-command dispatch/table; confirm command letters
      in code (verify no undocumented commands)
- [ ] Check for hidden/undocumented behavior (e.g., extra template
      arguments beyond FROM/SIZE)
- [ ] Annotate findings back into the spec ([BIN] tags)

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
