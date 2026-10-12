# MANUAL.md — Neo-Ed User Manual

Neo-Ed is a clean-room reimplementation of the AmigaDOS Ed 1.14
full-screen text editor, written in Rust from a behavior specification
derived by legally reverse engineering the original 1988 binary.

It runs natively on Windows, Linux, and macOS. No emulator, ROM, or
Amiga required.

---

## 0. About this project

Neo-Ed is the product of a **learning and historical preservation
project**: understanding how a real 1980s commercial program worked,
by legally reverse engineering it and reimplementing it from scratch.
It is not a product, a competitor, or a replacement for the original.

**How this was done (and why it's legal):**

- The original Ed 1.14 binary was obtained from the author's own
  licensed copy of Cloanto's *Amiga Forever* (the current rights
  holder for classic AmigaOS). No leaked source code was ever accessed.
- The binary was statically analyzed (disassembly) and dynamically
  observed in an emulator to document its behavior. This is permitted
  in the US (*Sega v. Accolade*, DMCA §1201(f)), the EU (Directive
  2009/24/EC), and the UK (CDPA ss.50A–50BA).
- A **behavior specification** was written in our own words, describing
  only functional behavior — command names, limits, file formats,
  message texts — none of which are copyrightable expression.
- The Rust implementation was then written **from the specification
  alone**, with no reference to the original code, in accordance with
  clean-room methodology.
- No AmigaOS code, ROM images, or proprietary material is included
  in this repository or in the Neo-Ed binary.

The full legal analysis is in [LEGAL.md](LEGAL.md).

**Distribution:** this repository distributes source code and
documentation only. Build from source with `cargo build --release`.
Pre-built binaries may be provided via GitHub Releases in the future. The complete
behavior specification with evidence tags is in
[docs/ed-spec.md](docs/ed-spec.md). The reverse-engineering process
and all BCPL-related findings are documented in [README.md](README.md)
and [BCPL.md](BCPL.md).

**Why preserve Ed?**

Ed is a small, elegant program from the era when software shipped on
a single floppy disk and did one thing well. Its command language
(repeat counts, nested groups, recursive search-and-exchange) is a
masterpiece of economy. By understanding it, we learn not just what
a 1988 editor did, but how software was designed when memory was
scarce, CPUs were slow, and every byte counted. That knowledge is
worth preserving.

---

## 1. Introduction

Neo-Ed is a screen editor. You see your text on screen and edit it
directly with the cursor keys and the characters you type. It has no
mouse support, no menus, and no fonts — just text, a cursor, and a
command line. This is deliberate: it is a faithful clone of a
1980s-era editor that shipped on every Amiga.

What it's good for:

- Source code (maximum line length 255 characters, no tabs inserted)
- Configuration files
- Shell scripts and batch files
- Any text where you want a fast, distraction-free editor

What it's not good for:

- Binary files (it detects and rejects them)
- Files larger than the buffer size you allocate
- Word processing (no styles, fonts, or page layout)

---

## 2. Getting started

### 2.1 Starting Neo-Ed

```
ed myfile.txt
```

If `myfile.txt` exists, it loads into the buffer and the first lines
appear on screen. If it doesn't exist, you get a blank screen with the
message **"Creating new file"** and can start typing.

You can also give a buffer size:

```
ed myfile.txt SIZE 50000
```

or positionally:

```
ed myfile.txt 50000
```

The minimum buffer size is 10,000 bytes. If you don't specify one, the
buffer is set to 10,000 bytes (the original Ed clamped at `0x2710`;
the manual's claim of 40,000 was wrong).

If the file contains binary data (null bytes), you'll see **"File
contains binary"** and the editor won't start. If it's larger than your
buffer, you'll see **"File too big"**.

### 2.2 The screen (the window)

Like the original Ed, which opened its own small console window titled "Ed 1.14", Neo-Ed opens its own native window (640x200, matching the original's RAW:0/0/639/199 screen):

```text
+------------------------------------------+
| Neo-Ed                                   |  title bar
| first line of your text                  |
| ...                                      |
| tenth line of your text                  |
| status line                              |
+------------------------------------------+
```

The bottom row of the window is the **status line**. It shows:

- Error messages (which stay until your next command)
- The command line when you press ESC
- Prompts like **"Exchange? "** or **"Edits will be lost - type Y to
  confirm: "**

The window shows 10 lines of text between the title bar and the status line. The title bar and the status line are both drawn highlighted (white on black), like the original's Amiga window furniture, so the top and bottom edges of the window are always clearly visible. The cursor is shown in the text area at your editing position. CTRL-D and CTRL-U scroll the text by one window (10 lines), and CTRL-E jumps between the top and bottom of the window.

### 2.3 Leaving Neo-Ed

| Command | Action |
|---|---|
| ESC then `X` then RETURN | Save and exit |
| ESC then `Q` then RETURN | Exit without saving (asks for confirmation if you've made changes) |
| CTRL-C | Force quit (terminal-level) |

`X` is the normal way to leave. It writes your text to the file you
named when you started, then also writes a backup copy to
`t/ed-backup` in the current directory. The backup persists until the
next time you exit, at which point it's overwritten.

`Q` discards your changes. If you've edited anything since loading,
you'll see:

```
Edits will be lost - type Y to confirm:
```

Press **Y** to quit, or anything else to continue editing.

---

## 3. Immediate mode (typing and editing)

When Neo-Ed starts, you're in **immediate mode**. Anything you type is
inserted at the cursor position. There is no overwrite mode.

### 3.1 Moving the cursor

| Key | Action |
|---|---|
| Up/Down/Left/Right arrows | Move one position in that direction |
| CTRL-R | Cursor to the space after the previous word |
| CTRL-T | Cursor to the first character of the next word |
| CTRL-] | Cursor to the right edge of the current line (or left if already there) |
| CTRL-E | Cursor to the top of the screen (or bottom if already there) |
| TAB | Cursor to the next tab position (multiples of the tab distance; inserts nothing) |

If you try to move past the top of the file, you'll see **"Top of
file"**. Past the bottom: **"End of file"**.

When the cursor reaches the right edge of the screen, the text
scrolls horizontally in steps of 10 characters. Vertically, it scrolls
one line at a time.

### 3.2 Inserting text

Type any character to insert it at the cursor. If the line would
exceed 255 characters, you'll see **"Line too long"** and the character
is refused.

Press **RETURN** to split the current line at the cursor. Press
**CTRL-A** to insert a blank line after the current line without
splitting.

### 3.3 Word wrap

When you're typing at the end of a line and the cursor reaches the
right margin (column 77 by default), Neo-Ed automatically moves the
half-typed word down to a new line. This is word wrap. It only happens
while you're typing at the end of a line — if you insert text in the
middle of a line, the line just gets longer.

The right margin starts at 77 (the original's default, not the 79 the
manuals claimed). You can change it with the SR command (section 4.5).

### 3.4 Deleting text

| Key | Deletes |
|---|---|
| BACKSPACE | Character to the left of the cursor |
| DEL | Character at the cursor |
| CTRL-B | The entire current line |
| CTRL-O | From cursor to end of word (or all spaces to next word) |
| CTRL-Y | From cursor to end of line |

If you delete the buffer's last line, you'll see **"Last line
deleted"**.

### 3.5 Scrolling

| Key | Scrolls |
|---|---|
| CTRL-D | One window (10 lines) toward the beginning |
| CTRL-U | One window (10 lines) toward the end |
| CTRL-V | Redraw the entire screen |

### 3.6 Changing case

Press **CTRL-F** to flip the case of the letter at the cursor (upper
to lower, or lower to upper). The cursor advances to the next
character. Non-letters are skipped but the cursor still advances.

### 3.7 Repeating commands

**CTRL-G** repeats the last extended command line you typed. For
example, if you searched for something with `F /hello/` and want to
find the next occurrence, just press CTRL-G.

---

## 4. Extended mode (the command line)

Press **ESC** to enter extended mode. The cursor moves to the status
line and you can type commands. When you press **RETURN**, the
commands execute and you return to immediate mode. If you press
**ESC** instead of RETURN, the commands execute but you stay in
extended mode (so you can type another command immediately).

### 4.1 Command syntax

Extended commands are one or two letters, case-insensitive. You can
give multiple commands on one line, separated by semicolons:

```
T; F /hello/; 3 A /world/
```

This moves to the top, finds "hello", and inserts "world" three
times after that line.

A **number** before a command repeats it that many times:

```
5 D          (delete 5 lines)
4 E /old/new/  (exchange the next 4 occurrences)
```

A **group** is enclosed in parentheses and can nest:

```
RP ( F /TODO/; 2 A /  / )
```

This finds every "TODO" in the file and inserts two blank lines after
each one.

**RP** (Repeat) runs a command or group until an error occurs (like
reaching the end of the file on a search).

Strings are introduced and terminated by a **delimiter** — any
character that isn't a letter, digit, space, semicolon, or bracket.
Common choices: `/text/`, `!text!`, `:text:`. If your text contains a
`/`, use a different delimiter.

You can't nest the same delimiter inside a string. Empty strings are
legal: `A ::` inserts a blank line.

### 4.2 Movement commands

| Command | Action |
|---|---|
| `T` | Move to top of file |
| `B` | Move to bottom of file |
| `N` | Move to start of next line |
| `P` | Move to start of previous line |
| `M n` | Move to line n (1-based; `M503` goes to line 503) |
| `CE` | Cursor to end of current line |
| `CS` | Cursor to start of current line |
| `CL` | Cursor one position left |
| `CR` | Cursor one position right |

### 4.3 Text commands

| Command | Action |
|---|---|
| `A /s/` | Insert a line containing `s` after the current line |
| `I /s/` | Insert a line containing `s` before the current line |
| `S` | Split the current line at the cursor (same as RETURN) |
| `J` | Join the current line with the next line |
| `D` | Delete the current line |
| `DC` | Delete the character at the cursor |
| `U` | Undo changes made to the current line only |

`A` moves the cursor to the newly inserted line, so consecutive `A`
commands insert in reading order.

### 4.4 Search and exchange

| Command | Action |
|---|---|
| `F /s/` | Find the next occurrence of `s` (searches forward from one past the cursor) |
| `BF /s/` | Find the previous occurrence of `s` (searches backward) |
| `E /s/t/` | Exchange the next occurrence of `s` for `t` (searches from the cursor position) |
| `EQ /s/t/` | Exchange with query — asks **"Exchange? "** before each replacement; Y applies, N skips |
| `UC` | Ignore case in searches (until LC) |
| `LC` | Distinguish case in searches (default) |

If a search fails, you'll see **"search failed"** on the status line.

**Exchanges and margins:** Neo-Ed ignores margin settings while
exchanging text. If the second string is empty (`E /old//`), the
found text is deleted. If the first string is empty (`E //new/`),
the second string is inserted at the cursor.

To change all occurrences in the file:

```
T; RP E /old/new/
```

Move to the top first, then repeat the exchange until no more
matches are found.

### 4.5 Margins and tabs

| Command | Action | Default |
|---|---|---|
| `SL n` | Set left margin to column n | 1 |
| `SR n` | Set right margin to column n | 77 |
| `ST n` | Set tab distance to every n columns | 3 |
| `EX` | Disable the right margin for the current line (until cursor moves) |

The left margin is where RETURN and CTRL-A place the cursor on a new
line. The right margin is where word wrap kicks in. The tab distance
determines where TAB stops (at multiples of the distance); TAB never
inserts tab characters into the file.

### 4.6 Block commands

A **block** is a range of whole lines marked with BS (Block Start)
and BE (Block End).

| Command | Action |
|---|---|
| `BS` | Mark the block start at the cursor's line |
| `BE` | Mark the block end at the cursor's line |
| `DB` | Delete the block |
| `IB` | Insert a copy of the block after the current line (block stays marked) |
| `SB` | Scroll the screen so the block's first line is at the top |
| `WB /file/` | Write the block to a file (overwriting any existing file) |

**Block rules:**

1. Blocks are always whole lines — you can't start or end a block in
   the middle of a line. Split the line first if you need to.
2. Any change to the text **invalidates the block marks** (except
   `IB`, which keeps them defined).
3. After `DB`, the block is undefined, so you can't `DB` then `IB`.
   But you can `IB` then `DB` — the block stays marked after the
   insert.
4. If the cursor is inside the marked block, `DB` refuses with
   **"Cursor inside block"**.
5. To mark a single line as a block: move to that line and type
   `BS;BE` on one command line.

### 4.7 File commands

| Command | Action |
|---|---|
| `X` | Save the file and exit |
| `SA` | Save the file (without exiting) |
| `SA /file/` | Save to a different file |
| `IF /file/` | Insert a file after the current line |
| `WB /file/` | Write the marked block to a file |
| `Q` | Quit without saving (confirms if edits exist) |

`SA` followed by `Q` is equivalent to `X`.

`X` writes the buffer to your FROM file, then writes the same buffer
to `t/ed-backup` in the current directory. The backup file persists
until the next time you exit.

### 4.8 Status display

`SH` (SHow) replaces the screen with the editor's state:

```
Editing file   : myfile.txt
Tab distance  : 003
Left margin   : 001
Right margin  : 077
Block start   : Unset
Block end     : Unset
Buffer usage  : 000%
```

Press any character to return to your text. If a block is defined,
the **Block start** and **Block end** fields show the text of the
marked lines (truncated). If no block is defined, they show **Unset**.

---

## 5. Error messages

| Message | Meaning |
|---|---|
| End of file | You tried to move past the last line |
| Top of file | You tried to move above the first line |
| Line too long | The line would exceed 255 characters |
| Last line deleted | The buffer's last line was deleted by D |
| Creating new file | The FROM file didn't exist; you're starting fresh |
| search failed | F or BF found no match |
| Unknown command | You typed a command that doesn't exist |
| Syntax error | The command line couldn't be parsed |
| Unmatched () | You opened a group without closing it |
| Number expected | M, SL, SR, or ST needs a number argument |
| No block marked | You used a block command without marking a block |
| Cursor inside block | You tried to DB while the cursor is inside the block |
| Edits will be lost - type Y to confirm: | Q with unsaved changes |
| Exchange? | EQ is asking you to confirm a replacement |
| Writing to file | The file is being saved |
| Unable to open file | The file couldn't be written or read |
| File contains binary | The file has null bytes and can't be edited |
| File too big | The file exceeds the buffer size |
| Bad arguments | The command-line arguments were wrong |
| Invalid number given for SIZE | The SIZE argument wasn't a valid number |
| SIZE of %N too small | The SIZE argument was below 10,000 |
| Commands abandoned | A running command sequence was interrupted |

Messages stay on the status line until your next command.

---

## 6. Command-line arguments

```
ed FROM [SIZE n]
ed --script SCRIPT FROM
```

`FROM` is the file to edit (required). `SIZE` is the buffer size in
bytes (optional, keyword or positional, minimum 10,000).

`--script` runs a file of extended-mode commands (one per line)
against the buffer without the interactive screen. Nothing is saved
unless a command in the script (usually `X`) saves and exits. Two
prompts cannot be asked in script mode: `Q` exits immediately even
with unsaved edits (no confirmation), and `EQ` moves the cursor to
each match but never applies the exchange — use `E` in scripts. Use
`--script` for automated testing:

```
ed --script test.ed input.txt
```

---

## 7. Differences from the original Ed 1.14

Neo-Ed is a clean-room clone: it was written from a behavior
specification derived by legal reverse engineering, not from the
original source code. It matches the original's behavior everywhere
we could verify, with these deliberate differences:

1. **Native window**, not an Amiga RAW: console window. It opens its
   own 640x200 window (matching the original's RAW:0/0/639/199 screen)
   and closes when you quit.
2. **No Workbench integration** — no icons, no Intuition screens.
3. **Cross-platform** — Windows, Linux, macOS.
4. **Word-pointer transparency** — the original's BCPL word-pointer
   addressing (byte >> 2) is invisible to the user.
5. **CTRL-C** — force-quit is handled by the terminal layer, not by
   AmigaDOS signal processing.

Everything else — the command language, the buffer model, the block
semantics, the margins, the search and exchange behavior, the save
and backup mechanism, and all the messages — matches the original as
documented in `docs/ed-spec.md`.

---

## 8. Technical notes

- **Maximum line length**: 255 characters (BCPL string limit).
- **Buffer minimum**: 10,000 bytes (the binary's `0x2710` clamp).
- **Tab distance**: 3 columns (matching the original's default).
- **Right margin**: 77 (from the SH display of the original binary;
  the 1987 manual's "79" was incorrect).
- **File I/O**: files are always written with a trailing linefeed,
  even if the original didn't end with one.
- **Backup path**: `t/ed-backup` relative to the current directory
  (from the binary's `:t/ed-backup` string; the leading colon means
  current-directory-relative, not the T: volume).

For the complete behavior specification with evidence tags for every
claim, see `docs/ed-spec.md`. For the reverse-engineering process
itself, see `README.md`.

---

## 9. Gotchas and quirks

Things that surprise people. Most are inherited from the original Ed
1.14; a few are consequences of running a 1988 terminal program on a
modern host.

1. **The editor opens its own native window.** Running `ed` with stdin
   or stdout redirected (a pipe, `ed file < input`, `echo | ed file`)
   does not work: the window needs a real display. For non-interactive
   use, use `--script` (section 6).
2. **`--script` cannot ask questions.** `Q` exits without the
   confirmation prompt, and `EQ` never applies its exchanges (see
   section 6).
3. **Q asks, X never asks.** `X` always saves and exits; `Q` asks for
   confirmation first if anything has changed.
4. **The backup goes to the current directory.** `t/ed-backup` is
   relative to where you started the editor — not next to the edited
   file, and not an Amiga `T:` ram-disk volume.
5. **Word wrap only fires while typing at the end of a line.**
   Inserting text in the middle of a long line never wraps it.
6. **The right margin is 77, not 79.** The 1987 manual says 79, but
   the original binary's SH display says 077.
7. **The minimum buffer is 10,000 bytes**, not the 40,000 the manual
   claimed.
8. **CRLF files keep their carriage returns.** The editor treats
   `\r` as an ordinary character, so files saved by Windows programs
   carry a stray `\r` at the end of every line and may render oddly.
   The original Amiga convention (LF line endings) works best.
9. **Any edit invalidates block marks**, except `IB` (section 4.6).
   `DB` then `IB` fails with "No block marked"; `IB` then `DB` works.
10. **A saved file always ends with a linefeed**, even if the file
    you loaded did not.
11. **Buffer usage reads 000%** — the size accounting is not
    implemented yet; the other SH fields are accurate.
12. **Messages persist on the status line** until your next command.
    A stale "search failed" from minutes ago is normal.
13. **`U` undoes only the current line, only until you leave it.**
    Ed keeps a copy of the line you are on; the moment the cursor
    leaves the line (or the line is deleted, split, or joined), the
    changes are committed and `U` restores nothing. `U` never
    resurrects a deleted line.

