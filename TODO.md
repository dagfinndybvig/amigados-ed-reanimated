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
- [x] Remaining live tests DONE (2026-10-13, by hand on the host
      machine): M n is 1-based (M2 -> line 2); repeat counts work
      (3 A /xx/ -> three lines in reading order); RP groups
      terminate and insert per match; UC/LC toggle case
      sensitivity; U undoes only until the cursor leaves the line;
      IF and WB require delimited strings (bare argument ->
      "Syntax error" - matches the clone's parser); ST readback
      zero-padded (ST 5 -> 005); t/ed-backup is written under the
      cwd and a write-protected cwd fails the backup AFTER the
      FROM file is saved (second live confirmation)
- [ ] Still open from that list: buffer-usage growth readback (does
      SH show nonzero usage with content?), positive t/ed-backup
      content check in a writable directory
- [ ] Trace DOS packet I/O with the emulator debugger (breakpoints on
      file reads/writes during startup and save)

## 6b. Side quest: programmatic emulator control (2026-10-12 -> OPEN)

**This section is a SIDE QUEST, not a project deliverable.** The main
quest - behavior spec + clean-room Rust clone - reached its MVP
independently of everything below. What follows is the campaign to
gain *programmatic control over the emulator* (patched boot disks,
synthetic input, autonomous verification), which became worth pursuing
only because interactive emulator input is impossible over remote
sessions (RDP and Windows App both break raw-input keyboard and
relative mouse capture). Its payoff is infrastructure: once it works,
the remaining spec [VERIFY] items (A-command, M n, groups/RP, U undo,
IF/WB) and the differential test suite can run unattended, hands-off.
Treat it as a solved-when-convenient engineering track; the clone and
the spec are the product.

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
3. [~] InputEvent injector (make_inject2.py) � EXTENSIVE PROGRESS
      2026-10-12/13, one bug from working. State:

      PROVEN LIVE on KS 1.3 (each by boot experiment):
      - 1.x ATRM single-hunk load files load and run fine (v0,
        v0pad40/82, redtest, stacktest all clean). The parked
        campaign conclusion "ABI unresolved" was partly an artifact:
        the CLI window closes on EndCLI and hides error messages.
      - Entry = hunk 0 offset 0; rts return works; d0 at rts is the
        CLI return code (must be 0 or the Shell prints
        "C:X failed <code>"; 65535 = garbage d0, NOT a crash).
      - Stack at entry is valid (push/pop test clean).
      - Phase color beacons (color reg 0) work but a Guru requester
        repaints the palette, erasing them.

      ENCODING CONVENTIONS SETTLED (verified vs ROM + docs):
      - Bcc/BSR word form: displacement relative to extension word
        (instruction+2). DBcc: same (self-loop = FFFE). Byte forms
        relative to instruction end (+2).
      - MOVE field order: size[15-12], dest reg[11-9], dest mode
        [8-6], src mode[5-3], src reg[2-0].
      - Key encodings: lea (d16,pc),aN = 41FA..41FD; move.b
        (a2)+,d2 = 144A; move.l a1,(a0) = 2089; move.w #imm,disp(a0)
        = 317C+imm+disp; move.w d2,disp(a3) = 3742+disp; clr.l
        disp(a0) = 42A8+disp; movea.l $4.w,a6 = 2C78 0004.
      - jsr -LVO(a6): OpenDevice=-444 (4EAE FE44), DoIO=-456
        (FE38), CloseDevice=-504 (FE08), OpenLibrary=-552 (FE...228).
        -444 confirmed by 12 call sites in kick13.rom itself.
        V33 corrections (2026-10-14 session, verified vs the ROM
        vector table at 0x1a7c): CloseDevice = -450 NOT -504;
        DoIO = -456 ok; OpenLibrary = -552 ok. ROM linked at
        0x00FC0000 (entry jmp $00FC00D2); ROM functions (file
        offset): OpenDevice 0x6a2, CloseDevice 0x6f0, DoIO 0x718,
        MakeLibrary 0x1528, Disable 0x1428, Permit 0x1436,
        AddDevice 0x690, FindName 0x1696, input.device BeginIO
        0x24cb8, its command table 0x25c9e (17 cmds) + support
        table 0x25ce2, its init 0x25ad0 (vecTable 0x25c8e,
        dataTable 0x25bf4), romtag 0x24c26.
      - OpenDevice registers: a0=devName, d0=unit, a1=ioreq,
        d1=flags (autodoc-confirmed).
      - IOStdReq V33: io_Command=28, io_Flags/Actual=30,
        io_Length=32, io_Data=36, io_Offset=40 (48-byte total OK);
        Message.mn_Length at offset 18 (set to 48 before open).

      SOLVED 2026-10-14: THE ORIGINAL REMAINING BUG WAS AN
      ENCODING TYPO, not a stack overflow. 'lea d16(pc),a0' is 41FA;
      41FB is a1. The ioreq-setup leas were commented 'a0 = ioreq'
      but emitted 41FB, so the 12 clr.l off(a0) writes ran against
      stale a0 (the entry's stacktop pointer), zeroing 48 bytes PAST
      the segment. With 41FA the setup is clean (odclr bisect boot
      passes, machine stable). The private stack is fine.

      2026-10-14 state of the injector (make_inject3.py, diagnostic
      ladder odsetup/odclr/odlea/odbase/odname/oddump/peekvec/
      dump444/dumpod/oddisable/fulldirect):
      - Direct ROM calls WORK from the command context where the
        runtime SysBase trampolines crash: jsr -120(a6) (Disable)
        crashes reproducibly from our context (oddisable probe),
        which poisons every library-vector call including BeginIO's
        reply path (Disable + PutMsg + Permit via the table).
        ROM OpenDevice at 0x00FC06A2 opens timer.device cleanly
        (io_Error = 0; test via io_Error at ioreq+0x1f, NOT d0 -
        the ROM ends with a Permit whose d0 is garbage).
      - Under the LEAN startup input.device is NOT in the device
        list (its own init calls MakeLibrary/OpenDevice through
        the poisoned table); under the FULL AF startup it IS
        (WHITE open-beacon seen in full1-full6 boots).
      - V33 input.device command map: CMD_WRITE(3) dispatches to
        the invalid stub (io_Error=-3); the real event injection
        goes through the ASYNC path (cmds 9/10/11 queue via
        0x24d10, handlers 0x5d02) - all of which reply through
        the poisoned table calls.
      - WinUAE serial TCP bridge (serial_port=TCP://0.0.0.0:1234)
        never opened a listener in any config tried (open
        question); the color-phase beacon channel (custom reg 0
        as a 12-bit display) is the working readback instead.
        Single custom-register writes are repainted by the
        display within a frame - only rewrite spin loops are
        visible to screenshots. ROM/serial port-enable bytes
        copied verbatim from devs/serial.device 0x316.
      - NEXT (two attack paths): (1) bypass the reply entirely:
        inline Forbid/Permit (the ROM's own idiom: addq.b #1,
        $127(a6) at ROM 0x6b4 / subq) + enqueue InputEvents
        directly into input.device's event list (the node's
        internal queue set up by the lib-init at 0x25aea, list at
        node+0x48 per the NEWLIST idiom at 0x25af6); (2) find out
        WHY the trampolines crash in the CLI-command context (dump
        [-120..-115] and its target like dump444 did for -444) -
        note OpenLibrary(-552) reportedly worked in the old
        session, so some trampolines may be intact.

      PREVIOUS NOTE (2026-10-13, superseded):
      HEADLINE: the exec V33 LVO numbering is now DEFINITIVELY
      correct - the vector table was extracted from the ROM (105
      entries at ROM offset 0x1a7c: word offsets relative to the
      table, FFFF-terminated, built by the inline init code) and
      Supervisor(-30), OpenDevice(-444), DoIO(-456), CloseDevice
      (-504), OpenLibrary(-552) all code-verified as their V36+
      counterparts. OpenLibrary("dos.library") runs CLEAN from a
      startup-CLI command (live proof). input.device's Open (-6 at
      file 0x25d16) and Close (-12 at 0x24e52) vectors are each
      4-6 trivial instructions (set io_Unit, opencount+-1) - they
      cannot crash or hang. The FindName helper (0xf81696) is a
      plain list walk. YET OpenDevice("input.device") from the same
      context hangs (no requester, no List output = machine stuck)
      or crashes with task-held.

      LIVE HYPOTHESIS: the BCPL Shell gives commands a tiny stack;
      OpenDevice's call chain pushes ~24 bytes more than
      OpenLibrary's (movem d0-d1 + a2 + a6 saves + nested jsr) and
      overflows it. Fix: switch to a private stack buffer in the
      command's own segment before the exec calls, restore before
      rts. Encodings still to verify from ROM context: move.l
      a7,(a1) (candidates 0x228F/0x224F family - capstone has
      rendering quirks here; cross-check against ROM code), and
      the restore path. The v0/stacktest pea+addq passed but only
      used 4 bytes of stack.

      Also fixed this session: 144A was an illegal encoding
      (move.b (a2)+,d2 is 0x1442 - field order src mode[5-3],
      src reg[2-0]); test-build layout overlap bug (IOREQ label
      inside code region - data must follow code, patch points
      found programmatically); odo3-style beq.s+4 off-by-2.

      Boot matrix tonight (all on edboot.adf + CLI-open startup):
      v0/v0pad40/v0pad82/redtest/stacktest = clean; openlibtest
      (OpenLibrary) = clean with full List output; odo4
      (OpenDevice+CloseDevice+LED) = hang; testA/onekey/inject2 =
      task-held requester. Exec LVO map + input.device vector
      analysis in tools/ notes below.

      INFRASTRUCTURE READY: edboot-inject2.adf (injector in c/Eval,
      CLI-stays-open startup), make_inject2.py (all encodings
      verified), pwshot.ps1 (occluded-window capture), ocrbox.ps1
      (position OCR), adf_patch/adf_extract verified end-to-end.

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

## 8. Version comparison (DONE 2026-10-15)

- [x] Extract Ed from Workbench 1.2 (amiga-os-120-workbench.adf) and diff
      (2026-10-15): WB 1.2's Ed is BYTE-IDENTICAL to WB 1.3's (both
      "Ed 1.14", md5 28f5d85c...); WB 1.0/1.1 ship Ed 1.10
      (byte-identical to each other, md5 78dd2cf0...), which differs
      from 1.14 only by "Error - Disk Full", "Tabs in file expanded"
      and seg7 work. WB 1.0/1.1/1.2 also ship c/Edit; WB 1.3 drops it
- [x] Extract Ed from the 3.1-era install and diff the command set
      against the 1991 manual's list (2026-10-15): the WB 3.1 floppy
      has NO Ed binary (only S/Ed-startup); the OS 2.x-3.x Ed is one
      binary, Ed 2.00 / $VER: ed 37.11 (13.5.91), 25,044 bytes, from
      Cloanto's install tree (Shared/dir/System/C/Ed) - C-era, single
      24,948-byte code hunk + 8-byte data hunk + reloc, template
      FROM/A,SIZE/N,WITH/K,WINDOW/K,TABS/N,WIDTH=COLS/N,HEIGHT=ROWS/N,
      embedded menu table (adds NW OP RK RE VW RX SF/DF). Full report:
      docs/ed-versions.md; binaries in materials/ed-versions/
- [x] Write version-history section (1.10 -> 1.14 -> 2.x -> 3.1)
      (2026-10-15, docs/ed-versions.md)

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
      21 unit tests cover immediate keys, extended command entry, EQ
      confirm Y/N, Q confirm, SH display, CTRL-G repeat, X/Q/Ctrl-C
      exit; live session in a real Windows console (driven via
      sendkeys + screenshot/OCR) verified rendering, typing, SH
      display, Q confirm prompt with N/Y, Ctrl-C, and X saving FROM +
      t/ed-backup
- [x] Bug fixed by the TUI pass: Q with unsaved edits exited without
      asking (quit flag and confirm message were both set); now the
      QuitConfirm prompt is shown first, per spec
- [x] A-command cursor behavior VERIFIED against the original
      (2026-10-13, by hand on the host machine): the cursor moves to
      the newly inserted line at column 0; consecutive A's insert in
      reading order (A /one/; A /two/ from end of line 2 -> one then
      two, cursor on the "t" of "two"). The clone's insert_after
      model was already correct; spec tagged [LIVE]
- [x] Fixed-window TUI (2026-10-13, from live user testing): Neo-Ed
      renders a 12-row window inside the terminal, like the original
      Ed's own 640x200 console window - title bar " Neo-Ed"
      (white-on-black, like the "Ed 1.14" title), 10 text lines,
      status line at the window bottom; CTRL-D/U now scroll one
      window (10 lines), CTRL-E bottom-of-window uses window rows;
      MANUAL 2.2 and 3.5 updated; 21 tests pass
- [x] Window pass 2 (2026-10-13, live feedback): cursor was hidden
      for the whole session (Hide at init, only shown on cleanup) -
      now visible throughout; status line and the SH continue prompt
      are reverse video like the title bar, so the bottom edge of
      the window is clearly visible
- [x] Native window rewrite (2026-10-13): replaced the in-terminal
      crossterm TUI with an eframe/egui native window sized 640x200
      (the original's RAW:0/0/639/199 screen), titled "Neo-Ed", that
      closes when the editor quits. The testable process_key state
      machine and --script headless mode are unchanged; 21 tests pass
- [x] GUI polish pass (2026-10-13): the window close button now
      prompts to save (Y/N) when there are unsaved edits instead of
      silently discarding them; the cursor blinks; line spacing uses
      the font row height (not the glyph height) and the font size
      is chosen to fit the 640x200 window; window is non-resizable
      like the original. 21 tests pass
- [x] GUI bugfix (2026-10-13): status messages (e.g. Unknown command)
      stuck on the status line forever - the GUI never cleared
      ed.last_message the way the old crossterm loop did. Now every
      key press clears it (message persists until the next command,
      spec 3) and mouse-move events no longer clear messages.
      Verified live: ESC zz ENTER shows the message, RIGHT clears it.
- [x] GUI bugfix 2 (2026-10-13): scrolling was lost in the native
      window rewrite (the crossterm Screen::scroll_follow was
      deleted) - the cursor walked off-screen past row 10 and
      b.left was never updated. Ported scroll_follow into gui.rs
      (vertical one line at a time, horizontal ten characters,
      spec 3) with 4 new unit tests; 25 total. Verified live:
      DOWN x14 keeps the cursor visible on the last row.
- [x] Practical default (2026-10-13): buffer SIZE now defaults
      to 40,000 bytes (the 1987 manual's figure) instead of the
      original binary's 10,000; the 10,000 floor is unchanged,
      so explicit SIZE handling stays faithful. README,
      MANUAL sections 2 and 7 updated
- [x] 20-row window (2026-10-13): TEXT_ROWS 10 -> 20; window height
      now derived from the row count (title + rows + status at
      16px rows = 640x360) instead of the original's 640x200
      ten-row screen - a deliberate readability deviation
      (MANUAL 7 item 1); CTRL-D/U scroll a 20-line window.
      25 tests pass; verified live (window height, scroll)
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

- [x] Does Ed 1.14 accept any template arguments beyond FROM and
      SIZE? RESOLVED (2026-10-15): no - 1.14's template is exactly
      FROM/A,SIZE [BIN]; Ed 2.00's is
      FROM/A,SIZE/N,WITH/K,WINDOW/K,TABS/N,WIDTH=COLS/N,HEIGHT=ROWS/N
- [ ] Meaning of the "12;0;0" string in segment 2 — likely console
      control; confirm during dynamic analysis
- [ ] Confirm whether any WB 1.3-era Ed revision differences exist vs
      the 1987 manual (manual covers 1.2-era Ed; binary is 1.14)
