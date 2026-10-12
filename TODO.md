# TODO.md — Project Progress Tracker

Status legend: [x] done · [ ] to do · [~] in progress · [-] optional / deferred

## 1. Setup and legal

- [x] Project directory and README.md (overview, goals, methodology)
- [x] LEGAL.md — full legal argument, per jurisdiction, with hard rules
- [x] .gitignore excluding copyrighted material (materials/, ref/, af-extract/)
- [x] CONTRIBUTING.md — clean-room rules summary + dev gotchas
      (created 2026-10-12 now that the repo is public)

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

## 6b. Emulator input crisis (2026-10-12 evening — OPEN)

Interactive verification is currently BLOCKED by an emulator input
problem on this machine. Everything below was established by experiment:

**Symptom matrix:**

| Path | Result |
|---|---|
| Amiga boots, renders, blinks, runs our patched startup | works (all builds) |
| Screenshot/OCR automation (nofocusshot + cropocr/ocrbox) | works |
| User physical keyboard → emulation | dead (Ed AND the 1> CLI) |
| User physical mouse → Amiga (clicks on icons) | dead ("mouse integration not working well" in AF too) |
| Synthetic SendInput keyboard (sendkeys_fg) | dead (SendKeys itself proven OK elsewhere) |
| PostMessage WM_KEYDOWN (postkeys.ps1) | reaches the GUI, NOT the emulation (raw-input path) |
| Host-level mouse (title bar drag, GUI) | works |
| WinUAE F12 settings GUI | opens (at least once) |

Tested on BOTH the AF-bundled winuae64.exe and the official
open-source WinUAE 6.0.3 (downloaded to emulator/winuae-oss/). Not a
WinUAE-config problem (inactive_pause etc. were set and verified),
not an Amiga problem (boots fully with blinking cursor).

**ROOT CAUSE FOUND: the machine is accessed via Remote Desktop.**
RDP is the classic killer of emulator input: WinUAE reads keyboard
via raw input and mouse in relative capture, both of which break or
behave erratically under RDP input tunneling. This also explains
"mouse integration not working well in Amiga Forever". Yesterday
(Oct 11) everything worked — presumably a locally-attended session.

**Fix for the next session:** do interactive verification while at
the physical machine (or via a non-RDP connection). Optional aids
if RDP must be used: WinUAE Game ports → "Windows mouse"/mousehack
(absolute pointer, no capture); test whether the RDP client's
keyboard reaches the emulation — synthetic SendInput from tool
processes does NOT (confirmed tonight even with mousehack=yes).

Other findings from the session (valuable, keep):

- ed-verify.uae's kick.rom is Kickstart 3.1 (header 0x1114, 512KB),
  NOT 1.3 — that config is the "Ed crashes under KS 3.1" experiment;
  Ed 1.14 fails there with error #87000004. Never use it for live
  Ed work. KS 1.3 = emulator/kick13.rom.
- Patched-boot-disk pipeline VERIFIED LIVE: adf_patch.py edits hold
  (marker text "MARKERZZ 775" appeared in the boot CLI; List s:
  output rendered). Patched disks: edboot-exist*.adf, edboot-marker*.
- edboot-shell.adf drops to an INTERACTIVE CLI (EndCLI removed) —
  the right base for any future interactive session.
- AmigaDOS lock gotcha: Ed cannot open the startup-sequence file
  while it is executing (exclusive locks) → "Creating new file".
  Edit a copy or an unused file instead.
- OSS WinUAE configs: atest3.uae (KS 1.3 + sound off + pause
  options) is the known-good boot config; start-ed.bat launches it.

**Windows App update (later same evening):** synthetic input (keys,
clicks, even the Ctrl+Win+Win reset combo) into the emulation is dead
under Windows App as well, on both the AF player (launched
successfully from our end via its UI: AmigaForever.exe → select
Workbench 1.3 → ENTER) and the OSS build. Host-level UI (player
dialogs, menus via ALT accelerators) IS drivable. A fully
autonomous, input-free pipeline is the way forward:

1. [x] Autonomous boot pipeline proven: patched ADF → boot →
      screenshot/OCR verification → kill → ADF diff/extract
      (marker "MARKERZZ 775" rendered; ADF write-check showed only
      our own patched blocks)
2. [x] Ruled out: AmigaDOS 1.3 has NO `<file` input redirection
      (2.0+ feature) — `C:ED RAM:a <s/CLI-Startup` never opened Ed
      (ReadArgs "Bad arguments", swallowed by Failat 100). Test disk
      edboot-atest.adf kept for reference
3. [ ] NEXT: InputEvent injector — a tiny hand-assembled m68k program
      (model: tools/make_gvdump.py, which already builds and boots
      custom segments) that builds IECLASS_RAWKEY InputEvent chains
      and writes them to input.device via IND_WRITE. Startup runs it
      right before Ed; Ed then receives our byte stream as real
      keystrokes. Needs the rawkey code table (A=0x20 ... Z=0x2C,
      ENTER=0x44, ESC=0x45, SPACE=0x40, digits, slash, semicolon)
      and press/release event pairs per key. This unlocks the
      A-command test AND the whole differential suite with zero
      interactive input.

**Next steps for interactive verification (physical machine):**

1. Next live session: work from the physical machine (no RDP), boot
   edboot-shell.adf via start-ed.bat (drops to an interactive CLI),
   then type `Ed RAM:atest.txt` and run the A-command test:
   LINE1, RETURN, LINE2, ESC, `A /one/; A /two/`, RETURN — the
   order of "one"/"two" in the buffer answers the A-cursor question.
2. The pending live items (A-command cursor, M n, groups/RP, U undo
   live check, IF/WB, differential testing) all depend on this.

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
- [~] Runtime slot naming — PARKED (not needed for the MVP; see 9).
      Last state: gvdump tool v0 (return) and v3 (OpenLibrary) run clean;
      v4 (stack args), v5 (reversed stack), v6 (register args) all crash.
      Next step if ever resumed: read the actual 1.3 dos.library LVO
      entry code from kick13.rom to settle the ABI, instead of guessing.
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
      2. Arnor BCPL Compiler v1.0 (1986): downloaded and inspected — it
         is an AMSTRAD CPC/PCW product (CPCEMU .dsk image, AMSDOS,
         40-track), NOT Amiga software. Contains real BCPL library
         material (MANIFEST, writef/wrch family) useful only as language
         reference. Cannot produce Amiga binaries; would need a CPC
         emulator (WinAPE/Caprice) merely to run. Gray provenance —
         local study only.
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

## 9. Clean-room reimplementation in Rust (IN PROGRESS — started
      2026-10-12, after decision that the MVP does not need the parked
      runtime-internals work)

- [x] Target language: Rust (decided); portable console clone
- [x] Spec is the sole input (LEGAL.md §6 clean-room rule); spec is
      behavior-complete enough for an MVP (commands, screen model,
      buffer limits, margins, block semantics, save/backup, messages)
- [x] ed-rs MVP command engine: all 38 commands, semicolons, repeat
      counts, nested groups, RP loops, block semantics, SH display,
      X/Q/SA/IF/WB, t/ed-backup relative to cwd, verified message set
- [x] Headless --script mode for testing (proven by scripted smoke
      tests: E exchange, RP groups, SH readback, X save + backup)
- [x] TUI interactive testing (2026-10-12): run_loop refactored into a
      testable process_key state machine (no terminal I/O in the logic);
      14 unit tests cover immediate keys, extended command entry, EQ
      confirm Y/N, Q confirm, SH display, CTRL-G repeat, X/Q/Ctrl-C
      exit; live session in a real Windows console (driven via
      sendkeys + screenshot/OCR) verified rendering, typing, SH
      display, Q confirm prompt with N/Y, Ctrl-C, and X saving FROM +
      t/ed-backup
- [x] Bug fixed by the TUI pass: Q with unsaved edits exited without
      asking (quit flag and confirm message were both set); now the
      QuitConfirm prompt is shown first, per spec
- [ ] Verify A-command cursor behavior: current implementation leaves
      cursor on the current line (consecutive A's insert in LIFO);
      needs live verification against the original
- [ ] Differential testing against original in emulator (deferred
      until MVP exists)
- [ ] SH display: Buffer usage field always reads 000% (size
      accounting not implemented; documented in MANUAL.md §9)
- [-] CRLF files: `\r` is kept as an ordinary character (Amiga
      convention is LF); documented in MANUAL.md §9 — decide whether
      load should normalize CRLF
- [x] U (undo current line) implemented from spec 5.3 — entry-baseline
      model: the baseline resets whenever the cursor enters a line,
      so leaving the line commits the edit; no resurrection of
      deleted/split/joined lines. 4 new tests; headless --script
      verification passed. With U the clone's "38 commands" claim is
      now literally true (U was the one missing command)
- [x] Known --script limitations documented (Q skips confirmation,
      EQ never applies; MANUAL.md §6/§9)

## 10. Open questions

- [ ] Does Ed 1.14 accept any template arguments beyond FROM and SIZE?
      (spec §9) — resolve via disassembly or live testing
- [ ] Meaning of the "12;0;0" string in segment 2 — likely console
      control; confirm during dynamic analysis
- [ ] Confirm whether any WB 1.3-era Ed revision differences exist vs
      the 1987 manual (manual covers 1.2-era Ed; binary is 1.14)
