# AmigaDOS Ed 1.14 — Behavior Specification (draft)

Status: draft, v0.1. Derived from (a) static analysis of the Ed 1.14 binary
extracted from the Workbench 1.3 disk image, and (b) documented behavior in
the *AmigaDOS Manual, 2nd edition* (Bantam, 1987) and COMPUTE! issue 73
(June 1986, covering Ed 1.10). Static findings are marked **[BIN]**; manual
findings are marked **[DOC]**. Items still needing verification against the
binary or live behavior are marked **[VERIFY]**.

## 1. Program identification

- File: `Ed`, 19,564 bytes, located in `c/` on the Workbench 1.3 disk.
- Version string: `Ed 1.14` **[BIN]**; VSTRING-style string `Ed (1.14)` **[BIN]**.
- Load format: old TRIPOS/BCPL load file (magic `0x000003F3`, followed by
  a hunk directory without the standard `HUNK_HEADER` id; nine code hunks
  delimited by `0x3E9`/`0x3F2` markers; 19,400 bytes of m68k code, no
  data/BSS hunks — globals are supplied by the BCPL runtime globvec) **[BIN]**.
- Ed is a BCPL program of TRIPOS heritage, run via the AmigaDOS CLI.

## 2. Invocation

Template: `ED <FROM/A>,SIZE/K` **[DOC]** (the 2nd-ed. manual's OCR shows a
garbled `PBOM/A`; the argument is the FROM file).

- `ED filename` — opens `filename`; if it exists it is loaded and the first
  lines displayed, otherwise a blank screen is presented ("Creating new file"
  **[BIN]**).
- `SIZE n` — text buffer size in bytes. Default: max(40000, file size)
  **[DOC]**; errors: "Invalid number given for SIZE", "SIZE of %N too small"
  **[BIN]**.
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
- Right margin default column 79; word wrap applies while typing at end of
  line **[DOC]**. `EX` disables the right margin for the current line.
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
| `EQ /s/t/` | Exchange with query — prompts "Exchange? "; Y applies, N skips |
| `EX` | Extend: ignore right margin on current line until cursor moves |
| `F /s/` | Find `s` forward from one place past cursor |
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
  `IB` then `DB` works **[DOC]**.
- Single-line block: `BS;BE` **[DOC]**.
- Marks used without a marked block: "No block marked", bad order:
  "Block incorrectly specified" **[BIN]**; `SH` reports block state
  ("Unset" when undefined) **[BIN]**.

### 5.3 Undo semantics

`U` restores the original version of the current line only. Ed keeps a copy
of the current line and commits it when the cursor leaves the line or the
screen scrolls; it does not undo line deletion **[DOC]**.

## 6. Files and saving

- `X`: writes buffer to the FROM file, then also writes the buffer to
  `:T/ED-BACKUP`; the backup persists until the next exit overwrites it
  **[DOC]**, corroborated by the binary string `:t/ed-backup` **[BIN]**.
- "Writing to file" message during save; "Error - Disk Full" on write
  failure **[BIN]**.
- `SA` may save to a different filename; `SA` then `Q` equals `X` **[DOC]**.
- `Q` prompts for confirmation only if there are unsaved edits **[DOC]**.
- `IF` inserts any readable file after the current line; `WB` writes the
  marked block to a file, overwriting an existing file of that name
  **[DOC]**.

## 7. Status display (SH) fields

Observed labels in the binary **[BIN]**: `Editing file`, `Tab distance`,
`Left margin`, `Block start`, `Buffer usage`, `Unset`.

## 8. Error and message strings (complete observed set)

From the binary **[BIN]**: Last line deleted; No room in buffer; Creating
new file; Input lines truncated; Top of file; Line too long; Unknown
command; Commands abandoned; Syntax error; Unable to open file; No block
marked; Block incorrectly specified; Tabs in input file expanded; File
contains binary; File too big; Bad arguments; Invalid number given for
SIZE; SIZE of %N too small; Unable to open window %S; Not enough free
store; Invalid file name; Edits will be lost - type Y to confirm: ;
Exchange? ; Writing to file; Error - Disk Full; Type any character to
continue ; Unable to obtain information about %S; %S is a directory and
cannot be edited.

## 9. Differences from later ED versions

The AmigaOS 2.0+ ED (documented in the 1991 3rd-edition manual) adds mouse
selection, `WITH` command files, function-key display (`DF`), menus (`EM`),
`WIDTH`/`HEIGHT`/`WINDOW`/`TABS` template arguments, and extra commands
(`DL`, `DW`, `EL`, `EP`, `FC`). None of these apply to Ed 1.14
**[VERIFY against disassembly]**.

## 10. Open items / next steps

1. Dynamic verification in WinUAE/FS-UAE (breakpoints on DOS packet I/O) to
   confirm startup sequence and save/backup ordering.
2. Disassembly pass (Ghidra or IRA) to confirm the extended-command letter
   table in code and check for undocumented commands.
3. Confirm whether Ed 1.14 accepts any template arguments beyond FROM and
   SIZE (manual documents only those two).
4. Diff against Ed 1.10 (WB 1.2) and Ed 2.x/3.1 binaries for the version
   history section.

## 11. Sources

- `materials/workbench13/c/Ed` — Ed 1.14 binary (user's Amiga Forever
  license; do not redistribute).
- *The AmigaDOS Manual*, 2nd edition (Bantam Books, 1987), chapter "ED — The
  Screen Editor" (Internet Archive scan, used for research).
- *The AmigaDOS Manual*, 3rd edition (1991) — consulted to identify OS 2.0
  ED feature additions (not applicable to 1.14).
- COMPUTE! issue 73 (June 1986), "Meet ED: The AmigaDOS Editor" (covers Ed
  1.10; atarimagazines.com).
- Binary string/hunk analysis: `tools/hunk_dump.py` over the raw segments.
