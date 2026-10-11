# AmigaDOS Ed 1.14 — Behavior Specification (draft)

Status: draft, v0.2. Derived from (a) static analysis of the Ed 1.14 binary
extracted from the Workbench 1.3 disk image, (b) documented behavior in the
*AmigaDOS Manual, 2nd edition* (Bantam, 1987) and COMPUTE! issue 73 (June
1986, covering Ed 1.10), and (c) a live verification session (2026-10-11)
running the real Ed 1.14 under Kickstart 1.3 in an emulator. Findings are
tagged: **[BIN]** verified in the binary, **[DOC]** documented in the
manuals, **[LIVE]** verified against the running editor,
**[VERIFY]** still needs confirmation. Where [LIVE] and [DOC] disagree,
[LIVE] wins and the conflict is noted.

## 1. Program identification

- File: `Ed`, 19,564 bytes, located in `c/` on the Workbench 1.3 disk.
- Version string: `Ed 1.14` **[BIN]**; VSTRING-style string `Ed (1.14)` **[BIN]**.
- Load format: old TRIPOS/BCPL load file (magic `0x000003F3`, followed by
  a hunk directory without the standard `HUNK_HEADER` id; nine code hunks
  delimited by `0x3E9`/`0x3F2` markers; 19,400 bytes of m68k code, no
  data/BSS hunks — globals are supplied by the BCPL runtime globvec) **[BIN]**.
- Ed is a BCPL program of TRIPOS heritage, run via the AmigaDOS CLI.

## 2. Invocation

Template: `ED FROM/A,SIZE` **[BIN]** — the binary's ReadArgs template string is
exactly `FROM/A,SIZE` (no `/K` modifier), so the SIZE value may be given
either as a keyword (`ED file SIZE 45000`, as the manual shows) or
positionally (`ED file 45000`) **[BIN]**; the 2nd-ed. manual's OCR shows a
garbled `PBOM/A`, which is `FROM/A`.

- `ED filename` — opens `filename`; if it exists it is loaded and the first
  lines displayed, otherwise a blank screen is presented ("Creating new file"
  **[BIN]**).
- `SIZE n` — text buffer size in bytes. Default: max(40000, file size)
  **[DOC]**; errors: "Invalid number given for SIZE", "SIZE of %N too small"
  **[BIN]**. **Correction from code** **[BIN, code]**: the only size-floor
  constant anywhere in the binary is `0x2710` = 10,000 (three sites in
  seg4; the clamp at seg4+0x010E reads "if computed size >= 10000 use it,
  else 10000"). No 40,000 (`0x9C40`) constant exists — the 40,000 minimum
  in the 1987 manual and the "40K" default in the 1986 COMPUTE! article
  (Ed 1.10) do not match the Ed 1.14 binary; possibly a 1.10-to-1.14
  change **[VERIFY context]**.
- Rejects: "Bad arguments", "Invalid file name", "Unable to obtain
  information about %S", "%S is a directory and cannot be edited" **[BIN]**.
- Opens its own console window via `RAW:0/0/639/199/Ed 1.14` (640x200,
  title "Ed 1.14") **[BIN]**; failure gives "Unable to open window %S"
  **[BIN]**.
- Allocation failure: "Not enough free store" **[BIN]**.

## 3. Screen and buffer model

- Full-screen editor on its own RAW: console window; bottom line is the
  message/command line. Error messages remain until the next command **[DOC]**.
- Vertical scrolling: one line at a time; horizontal: ten characters at a
  time **[DOC]**.
- Line length: maximum 255 characters; longer inserts give "Line too long"
  **[DOC][BIN]**. Input lines longer than the limit are cut on load
  ("Input lines truncated" **[BIN]**).
- Tabs: the TAB key moves to the next tab stop (multiples of the tab
  distance, default 3); no TAB characters are inserted **[DOC]**. TAB
  characters found in an input file are expanded to spaces ("Tabs in input
  file expanded", "Tabs in input file" **[BIN]**).
- The buffer must hold the whole file; "File too big" and "No room in
  buffer" **[BIN]**.
- Files containing binary are rejected: "File contains binary" **[BIN][DOC]**.
- Writing always terminates the file with a linefeed even if the original
  had none **[DOC]**.
- Right margin default is **077** per the SH display **[LIVE]** — the 1987
  manual states 79 **[DOC]**; the SH readback is the program's own state
  display and is treated as authoritative pending disassembly confirmation.
  Word wrap while typing at the end of a line is confirmed live (with
  `SR 20` set, typed text wrapped into successive lines) **[LIVE]**. `EX`
  disables the right margin for the current line.
- Insert-only editing (no overwrite mode) **[DOC]**.

## 4. Immediate commands (special keys)

| Key | Action |
|---|---|
| BACKSPACE / CTRL-H | Delete character left of cursor |
| DEL | Delete character at cursor |
| RETURN / CTRL-M | Split line at cursor; new line starts at left margin |
| CTRL-A | Insert blank line after current line |
| CTRL-B | Delete current line ("Last line deleted" when last line is removed **[BIN]**) |
| CTRL-D | Scroll text down 12 lines (toward top) |
| CTRL-U | Scroll text up 12 lines (toward end) |
| CTRL-E | Cursor to top of screen; if already there, to bottom of screen |
| CTRL-] | Cursor to right edge of current line; if already there, to left edge |
| CTRL-T | Cursor to start of next word |
| CTRL-R | Cursor to space after previous word |
| TAB / CTRL-I | Cursor to next tab position (inserts nothing) |
| CTRL-F | Flip case of character at cursor (non-letters untouched), then advance |
| CTRL-O | Delete to end of word at cursor, or all spaces up to next word |
| CTRL-Y | Delete from cursor to end of line |
| CTRL-V | Redisplay (verify) screen |
| CTRL-G | Repeat the last extended command line |
| ESC / CTRL-[ | Enter extended command mode |
| Arrows | Move cursor one position, scrolling as needed |

## 5. Extended command mode

- Entered with ESC; input appears on the command line (bottom line).
- Command line terminated by RETURN (back to immediate mode) or ESC (stay in
  extended mode) **[DOC]**.
- Commands are one or two letters, case-insensitive; multiple commands on a
  line are separated by `;` **[DOC]**.
- Strings are delimited by any character that is not a letter, digit, space,
  semicolon, or bracket, e.g. `/text/`, `!text!` **[DOC]**.
- A number before a command is a repeat count, e.g. `4 E /a/b/` **[DOC]**.
- Command groups in parentheses may nest: `RP ( P /x/; 3 A/ / )` **[DOC]**.
- `RP` repeats a command/group until an error occurs **[DOC]**.
- Any repeated/grouped sequence aborts if you type a character, or on error
  ("Commands abandoned" **[BIN]**).
- Unknown input: "Unknown command", "Syntax error" **[BIN]**.

### 5.1 Command list

The complete command set is verified from the dispatch code **[BIN, code]**
(see section 5.4): 15 single-letter commands, `(` and `)` group syntax, and
23 two-letter commands — no undocumented commands exist.

| Command | Action |
|---|---|
| `A /s/` | Insert line with `s` after current line |
| `B` | Move to bottom of file |
| `BE` | Set block end mark at cursor |
| `BS` | Set block start mark at cursor |
| `BF /s/` | Find `s` searching backwards from cursor |
| `CE` | Cursor to end of current line |
| `CL` | Cursor one position left |
| `CR` | Cursor one position right |
| `CS` | Cursor to start of current line |
| `D` | Delete current line |
| `DB` | Delete marked block |
| `DC` | Delete character at cursor |
| `E /s/t/` | Exchange `s` into `t` (next occurrence; use with RP/count) |
| `EQ /s/t/` | Exchange with query — prompts "Exchange? " **[LIVE]**; Y applies, N skips |
| `F /s/` | Find `s` forward from one place past cursor; failure shows "search failed" **[LIVE]** |
| `EX` | Extend: ignore right margin on current line until cursor moves |
| `I /s/` | Insert line with `s` before current line |
| `IB` | Insert copy of marked block after current line |
| `IF /s/` | Insert file named `s` after current line |
| `J` | Join current line with next line |
| `LC` | Distinguish upper/lowercase in searches (default) |
| `UC` | Ignore case distinction in searches |
| `M n` | Move cursor to line number n (e.g. `M503`) |
| `N` | Move to start of next line |
| `P` | Move to start of previous line |
| `Q` | Quit without saving; if edits exist prompts "Edits will be lost - type Y to confirm: " |
| `RP cmd` | Repeat cmd/group until error |
| `S` | Split current line at cursor (like RETURN) |
| `SA [/s/]` | Save buffer to current file, or to filename `s` |
| `SB` | Show block: scroll so block's first line is top of screen |
| `SH` | Show state: file name, tab distance, margins, block marks, buffer usage |
| `SL n` | Set left margin to column n |
| `SR n` | Set right margin to column n |
| `ST n` | Set tab distance to n (default 3) |
| `T` | Move to top of file |
| `U` | Undo changes made to the current line only |
| `WB /s/` | Write marked block to file named `s` |
| `X` | Exit: write buffer to file, then quit |

Empty strings in `E` are legal: empty search string inserts the second
string at the cursor; empty second string deletes the found text **[DOC]**.
Margins are ignored while exchanging **[DOC]**.

### 5.2 Block semantics

- `BS`/`BE` mark whole lines (a block cannot start or end mid-line; split
  the line first) **[DOC]**.
- Any change to the text undefines the block marks, except `IB` **[DOC]**.
- After `DB` the block is undefined, so `DB` then `IB` is impossible, but
  `IB` then `DB` works **[DOC]** — verified live: `IB` inserted the copy,
  `DB` deleted it, and a following `IB` failed **[LIVE]**.
- Single-line block: `BS;BE` **[DOC]**, confirmed live (SH then showed both
  block fields holding the line's text) **[LIVE]**.
- Marks used without a marked block: "No block marked" **[BIN]**, observed
  live after the `DB`-then-`IB` sequence **[LIVE]**; bad order:
  "Block incorrectly specified" **[BIN]**; `SH` reports block state
  ("Unset" when undefined) **[BIN][LIVE]**.

### 5.3 Undo semantics

`U` restores the original version of the current line only. Ed keeps a copy
of the current line and commits it when the cursor leaves the line or the
screen scrolls; it does not undo line deletion **[DOC]**.

### 5.4 Command dispatch (code analysis)

Located in segment 6 at 0x07D6–0x0988 **[BIN, code]**
(`tools/disasm_ed.py`, Capstone m68k; census by `tools/call_census.py`):

- The command word is compared as a 32-bit value in `d1` through a binary
  compare tree: single letters as ASCII longs (`'A'`=0x41 ... `'X'=0x58),
  two-letter commands as packed words (`"BS"`=0x4253, `"EQ"`=0x4551, ...).
- The full set found in the tree matches section 5.1 exactly: `(`, `)`,
  A, B, D, E, F, I, J, M, N, P, Q, S, T, U, X (single) and BE, BF, BS, CE,
  CL, CR, CS, DB, DC, EQ, EX, IB, IF, LC, RP, SA, SB, SH, SL, SR, ST, UC,
  WB (two-letter).
- `BF` jumps to the same handler as `F` (0x3BC), and `EQ` to the same as
  `E` (0x370): the one-letter handlers consume an optional second letter
  from the command line.
- All non-matching values fall through to a common path (0x25E),
  the "Unknown command" branch.
- The immediate (CTRL-key) command dispatch sits in segment 8
  (letters loaded via `moveq`, calling globvec entry 28 through
  `jsr (a5)`); full key mapping still to be tabulated **[VERIFY]**.

### 5.5 Runtime interface (BCPL globvec) — code analysis

**Calling convention** **[BIN, code]**: every external call is
`jsr (a5)` with `d0` = byte offset (multiple of 4) of the target slot in
the process global vector, scalar arguments in `d1`–`d3`, and pointer
arguments pre-loaded into `a4` from `off(a2)` (Ed's globals, also
globvec-relative).

**Segment 0 is the globvec initialization** **[BIN, code]**: a fill loop
writes `0x474C0003, 0x474C0005, ...` ("GL" magic + sequential glue
index) into the allocated vector — the standard AmigaDOS BCPL glue
mechanism, where dos.library resolves each "GL" reference to the actual
function on first use.

**Call census** **[BIN, code]**: 545 `jsr (a5)` sites across all nine
segments, using exactly **19 distinct slot offsets**: 0, 12, 16, 20, 24,
28, 32, 36, 40, 44, 48, 52, 60, 64, 76, 80, 108, 112, 120 — i.e. globvec
entries 3–30. Heaviest users: slot 32 (78 sites), slot 28 (69), slot 16
(91), slot 12 (58). Segments specialize: seg4 (startup/CLI) uses 112/120,
seg7 (file I/O) uses 60/64/76/80, seg8 (keyboard) uses 108/112.

**Open**: naming each slot requires the Kickstart 1.3 dos.library glue
table (the GL-index → function order). Candidate routes for a later
pass: observing calls live in the emulator debugger with known DOS
operations as anchors, or correlating with the published TRIPOS BCPL
function list. The 19-slot census is Ed's complete runtime API surface
either way **[VERIFY for names]**.

**Segment roles** **[BIN, code]**: seg0 = globvec initialization; seg1 =
line-buffer engine (word-pointer-indexed line array accessed via `a0`,
buffer state in globals $260–$2C8: base, line count, capacity, gap
pointers; insert/delete/shift loops match the documented line editing
behavior); seg4 = startup/CLI (ReadArgs, size validation, window open);
seg6 = extended-command dispatch and handlers; seg7 = file I/O and the
SH display; seg8 = immediate/CTRL-key handling. Two direct helper call
offsets also exist — `jsr $10(a5)` (4 sites) and `jsr $12(a5)` (9
sites) — small runtime helpers (likely pointer/string utilities), exact
semantics pending the debugger pass **[VERIFY]**.

## 6. Files and saving

- `X` performs two writes, in order **[LIVE]**:
  1. the buffer to the FROM file (observed: the FROM file exists on disk
     after a failed step 2);
  2. the buffer to the backup file.
- **Backup path correction** **[LIVE]**: the backup is written to
  `t/ed-backup` *relative to the current directory* — the binary string
  `:t/ed-backup` uses AmigaDOS leading-colon current-directory syntax, not
  the `T:` volume (an earlier draft of this spec misread the 1987 manual's
  ":T/ED-BACKUP" as a volume reference; the leading colon makes it a
  relative path, which is why the Workbench disks carry a `t` drawer).
- If the backup write hits a write-protected volume, AmigaDOS raises the
  system requester ("Volume ... is write protected. Retry/Cancel");
  choosing Cancel leaves the FROM file saved (step 1 already completed) and
  Ed terminates **[LIVE]**.
- The backup persists until the next exit overwrites it **[DOC]**.
- "Writing to file" message during save; "Error - Disk Full" on write
  failure **[BIN]**.
- `SA` may save to a different filename; `SA` then `Q` equals `X` **[DOC]**.
- `Q` prompts for confirmation only if there are unsaved edits; the prompt
  is "Edits will be lost - type Y to confirm: " **[BIN]**, confirmed live —
  answering N returns to editing **[LIVE]**.
- `IF` inserts any readable file after the current line; `WB` writes the
  marked block to a file, overwriting an existing file of that name
  **[DOC]**.

## 7. Status display (SH)

`SH` replaces the screen with a status panel **[LIVE]**; press any character
to return ("Type any character to continue" **[BIN]**). Observed format
**[LIVE]**:

- Labels: `Editing file`, `Tab distance`, `Left margin`, `Right margin`,
  `Block start`, `Block end`, `Buffer usage` (all seven confirmed as
  **[BIN]** strings; the first scan missed `Right margin` due to the
  padding bug noted in section 8).
- Values are zero-padded three-digit numbers: tab distance default `003`,
  left margin default `001`, right margin default `077`, buffer usage
  `000%`.
- `Block start`/`Block end` show the *text* of the marked first/last line
  (truncated to fit), or `Unset` when no block is defined.
- All fields read back correctly after parameter changes: `ST 8` → `008`,
  `ST 111` / `SL 10` / `SR 60` → `111 010 060` **[LIVE]**, which also
  confirms multi-command lines separated by `;` and "command + space +
  number" argument syntax.

## 8. Error and message strings (complete observed set)

Complete string table (corrected 2026-10-11; the first pass had a
word-padding bug in the scanner — `tools/hunk_dump.py` — that skipped
strings following odd-length strings) **[BIN]**:

Startup/CLI (seg4): Unable to obtain information about %S; %S is a
directory and cannot be edited; RAW:0/0/639/199/Ed 1.14; FROM/A,SIZE;
Bad arguments; Invalid number given for SIZE; SIZE of %N too small;
Unable to open window %S; Not enough free store; Invalid file name;
Ed (1.14); File contains binary; File too big.

Editor messages (seg3): Last line deleted; No room in buffer; Creating
new file; Input lines truncated; Top of file; **End of file**; Line too
long; Unknown command; **Unmatched ()**; Commands abandoned; Syntax
error; Unable to open file; **Number expected**; No block marked;
**Cursor inside block**; Block incorrectly specified; **Search failed**
(confirmed live on failed `F`); Tabs in input file expanded; Error.

Save/backup (seg6/seg7): Edits will be lost - type Y to confirm: ;
:t/ed-backup; Writing to file; **Buffer corrupt**; Error - Disk Full;
File contains binary; No room in buffer; Tabs in file expanded; Input
lines truncated; Exchange? ; Unset.

SH display (seg7): Editing file; Tab distance; Left margin;
Right margin; Block start; Block end; Buffer usage; Type any character
to continue.

Newly identified after the scan fix, all relevant to spec behavior:
- "End of file" — the bottom-of-file counterpart of "Top of file"
- "Unmatched ()" — command-group parenthesis mismatch
- "Number expected" — numeric argument missing for M/SL/SR/ST
- "Cursor inside block" — block operation rejected with the cursor
  inside the marked block
- "Search failed" — live-confirmed on failed find (see section 9)

## 9. Live verification notes (2026-10-11 session)

Environment: genuine Kickstart 1.3 ROM (licensed via Amiga Forever), real
Workbench 1.3 disk in DF0:, Ed 1.14 started from the CLI as
`ED RAM:edtest.txt`.

- Ed opens its own console window titled "Ed 1.14" **[LIVE]** (matches the
  binary's `RAW:0/0/639/199/Ed 1.14` string **[BIN]**).
- Ed 1.14 fails to start under Kickstart 3.1 (A1200, 68020) with a
  Software Failure alert ("Program failed (error #87000004)"), while other
  1.3 BCPL commands (SetPatch, Assign, Mount, Resident, LoadWB) run fine
  in the same environment **[LIVE]**. Ed 1.14 requires its contemporary
  Kickstart; the failure under 3.1 is worth pinpointing during disassembly
  (likely a globvec/compat-layer entry).
- The status line sits on the last line of Ed's window; messages remain
  until the next command **[DOC][LIVE]**.
- `ZZ` (unknown command) → "Unknown command" **[LIVE]**.

## 10. Differences from later ED versions

The AmigaOS 2.0+ ED (documented in the 1991 3rd-edition manual) adds mouse
selection, `WITH` command files, function-key display (`DF`), menus (`EM`),
`WIDTH`/`HEIGHT`/`WINDOW`/`TABS` template arguments, and extra commands
(`DL`, `DW`, `EL`, `EP`, `FC`). None of these apply to Ed 1.14
**[VERIFY against disassembly]**.

## 11. Open items / next steps

1. Disassembly pass (Ghidra or IRA) to: confirm the extended-command
   letter table, locate the "search failed" string, and pin down the
   default right-margin constant (077 vs the manual's 79).
2. Pinpoint why Ed 1.14 fails under Kickstart 3.1's BCPL compat layer.
3. Remaining live tests: `M n` navigation, repeat counts (`4 E /a/b/`),
   command groups and `RP`, `UC`/`LC` case handling, `U` undo, `IF`/`WB`
   file operations, tab-distance behavior, buffer-usage display growth,
   and the positive path of the `t/ed-backup` write (in a writable
   current directory).
4. Diff against Ed 1.10 (WB 1.2) and Ed 2.x/3.1 binaries for the version
   history section.

## 12. Sources

- `materials/workbench13/c/Ed` — Ed 1.14 binary (user's Amiga Forever
  license; do not redistribute).
- *The AmigaDOS Manual*, 2nd edition (Bantam Books, 1987), chapter "ED — The
  Screen Editor" (Internet Archive scan, used for research).
- *The AmigaDOS Manual*, 3rd edition (1991) — consulted to identify OS 2.0
  ED feature additions (not applicable to 1.14).
- COMPUTE! issue 73 (June 1986), "Meet ED: The AmigaDOS Editor" (covers Ed
  1.10; atarimagazines.com).
- Binary string/hunk analysis: `tools/hunk_dump.py` over the raw segments.
