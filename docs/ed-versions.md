# Ed version history — 1.10 → 1.14 → 2.00

Derived by extracting `c/Ed` from every Workbench ADF in the Amiga Forever
extraction (`af-extract/.../Shared/adf/`) with `tools/adf_extract.py`, and
from Cloanto's OS 3.x hard-disk install tree
(`af-extract/.../Shared/dir/System/C/Ed`). Binaries live in
`materials/ed-versions/` (gitignored). Analysis tool: `tools/hunk_dump.py`
(extended 2026-10-15 to parse data hunks `0x3EA` and `HUNK_RELOC32`
blocks in the old TRIPOS container).

## 1. The binaries

| Version | Shipped with | Size (bytes) | MD5 | Notes |
|---|---|---|---|---|
| Ed 1.10 | WB 1.0, WB 1.1 | 18,804 | `78dd2cf0df313f57dd8dc62d84fb9264` | WB 1.0 and 1.1 binaries are byte-identical |
| Ed 1.14 | WB 1.2, WB 1.3 | 19,564 | `28f5d85c93dc8250fdccec7e6228cbcb` | WB 1.2 and 1.3 binaries are byte-identical |
| Ed 2.00 | OS 2.x–3.x install (C/Ed) | 25,044 | `1ad3371290be4a855684d24c3535f197` | `$VER: ed 37.11 (13.5.91)` |

Key facts **[BIN]**:

- **Ed 1.14 debuted in Workbench 1.2**, not 1.3 — the WB 1.2 and WB 1.3
  binaries are byte-for-byte identical. Likewise Ed 1.10 is identical in
  WB 1.0 and 1.1. There are exactly three distinct 1.x-era binaries in
  the set, not four.
- **WB 1.0/1.1/1.2 also ship a second editor, `c/Edit`** (19,068 bytes in
  1.0/1.1, 18,836 in 1.2). **WB 1.3 drops Edit** and keeps only Ed.
- **The Workbench 3.1 floppy contains no Ed binary at all** (only the
  script `S/Ed-startup`). In OS 2.x/3.x Ed is installed from the install
  tree into `C:`; the binary Cloanto ships for that whole era is the
  single Ed 2.00 / V37.11 above. So "the 3.1 Ed" and "the 2.0 Ed" are
  the same program; there is no separate 3.x Ed in this set.
- The 2.x/3.x floppies are FFS (`DOS\1`/`DOS\3`); the 1.x–2.04 ones are
  OFS (`DOS\0`). The OFS extractor handles both directory layouts since
  the hash-table structure is the same; only the 1.x disks were needed
  for extraction here.

## 2. 1.10 → 1.14 (same BCPL program, small delta)

Both are old-TRIPOS BCPL load files with nine code hunks. Segment sizes
(1.10 → 1.14): seg0 316 = 316; seg1 1724 → 1676; seg2 1588 → 1592;
seg3 1648 → 1632; seg4 2944 → 3016; seg5 2356 → 2320; seg6 3116 → 3192;
seg7 4076 → 4804; seg8 872 → 852. The largest growth is seg7 (+728).

String-table delta **[BIN]** (all other strings identical):

- Added: `Error - Disk Full`, `Tabs in file expanded`
  (1.10 only has `Tabs in input file expanded`).
- Changed: version strings `Ed (1.10)` → `Ed (1.14)`,
  `RAW:0/0/639/199/Ed 1.10` → `RAW:0/0/639/199/Ed 1.14`.

So 1.14 = 1.10 + disk-full handling + a second tab-expansion message +
miscellaneous fixes (seg7, the largest segment, holds the bulk of it).
The command set, prompts, and status labels are unchanged.

## 3. Ed 2.00 — a different program

Ed 2.00 is not a BCPL/TRIPOS program anymore:

- Load format: same outer container (magic `0x3F3`, hunk directory,
  `0x3E9`/`0x3F2` markers) but with **one 24,948-byte code hunk**, an
  8-byte **data hunk** (`0x3EA`, contents `0x1000, 0x0000`, referenced
  via a `HUNK_RELOC32` block that patches code offset 0xA — the
  `movea.l abs.l,a4` globvec-table load), and an empty third hunk.
- C-era strings: `$VER: ed 37.11 (13.5.91)`, printf-style `%s`,
  `Ed requires Version 2.0 to run`, `Unable to open needed library`.
- Window spec: `RAW:0/0/639/199/Ed 2.00/CLOSE` — same geometry as 1.14
  plus the **CLOSE gadget**, with a new exit path:
  `File has been changed - type Y to save and exit: ` (alongside the
  unchanged `Edits will be lost - type Y to confirm: ` and
  `:t/ed-backup`).

### Template (answers spec open question for 2.x)

`FROM/A,SIZE/N,WITH/K,WINDOW/K,TABS/N,WIDTH=COLS/N,HEIGHT=ROWS/N` **[BIN]**

— confirms the 3rd-edition manual's 2.0 additions: `WITH`, `WINDOW`,
`TABS`, `WIDTH`/`COLS`, `HEIGHT`/`ROWS`, all keyword-only except
positional `SIZE` (now `/N`).

### Menu table (SI strings embedded in the binary)

The binary carries a complete menu/key table (format:
`SI<n> 2'<label>''<command>'`), which doubles as a command list for
2.00 **[BIN]**:

| Menu item | Shortcut | Command emitted |
|---|---|---|
| New | ESC nw | `NW` |
| Open... | ESC op | `OP ?/Open File:/` |
| Insert File... | ESC if | `IF ?/Insert File:/` |
| Write Block... | ESC wb | `WB ?/Write File:/` |
| Save | ESC sa | `SA` |
| Save As... | ESC sa | `SA ?/Save As File:/` |
| Save & Exit | ESC x | `X` |
| About | ESC sh | `SH` |
| Quit | ESC q | `Q` |
| Undo Line | ESC u | `U` |
| Start Block | ESC bs | `BS` |
| End Block | ESC be | `BE` |
| Show Block | ESC sb | `SB` |
| Insert Block | ESC ib | `IB` |
| Delete Block | ESC db | `DB` |
| Top | ESC t | `T` |
| Bottom | ESC b | `B` |
| Go to Line.. | ESC m | `M ?/Goto Line:/` |
| Find... | ESC s | `F ?/String:/` |
| Reverse Find... | ESC bf | `BF ?/String:/` |
| Replace... | ESC e | `E ?/Search:/ ?/Replace:/` |
| Global Replace... | ESC rp e | `E ?/Search:/ ?/Replace:/;RP E` |
| Query Replace... | ESC eq | `EQ ?/Search:/ ?/Replace:/` |
| Global Q-Replace... | ESC rp eq | `EQ ?/Search:/ ?/Replace:/;RP EQ` |
| Set FN Key... | ESC sf | `SF ?/Key:/ ?/Command:/` |
| Show FN Key... | ESC df | `DF ?/Key:/` |
| Reset Keys | ESC rk | `RK` |
| Right Margin... | ESC sr | `SR ?/Right Margin:/` |
| Left Margin... | ESC sl | `SL ?/Left Margin:/` |
| Ignore Case | ESC uc | `UC` |
| Case Sensitive | ESC lc | `LC` |
| Extended Command... | ESC cm | `CM` |
| Repeat Last | ESC re | `RE` |
| Run File... | ESC rf | `RF ?/Command File:/` |
| AREXX Command... | ESC rx | `RX ?/Rexx CMD:/` |
| Redisplay | ESC vw | `VW` |

Commands in 2.00 not present in 1.14: `NW`, `OP`, `RK`, `RE`, `VW`,
`RX`, plus the menu-driven forms of everything else. The 1.14 command
set (38 commands) is essentially the subset of this table minus the
2.0 additions — consistent with the 3rd-edition manual's list.

## 4. Summary timeline

- **WB 1.0 / 1.1 (1985–86):** Ed 1.10 (+ a separate `Edit`).
- **WB 1.2 (1987?):** Ed 1.14 — disk-full error, tab-expansion message,
  seg7 work; `Edit` still present but shrinking.
- **WB 1.3 (1988):** same Ed 1.14 binary; `Edit` dropped.
- **OS 2.0 (1990) → 3.1 (1993):** Ed 2.00 (V37.11, 13.5.1991) — a
  single C-era binary with menus, CLOSE gadget, AREXX, function keys,
  and the extended template. Unchanged across 2.x and 3.x in this set.

## 5. Method notes

- Extraction: `python tools/adf_extract.py <adf> <outdir>` (OFS disks);
  the Ed 2.00 binary was copied directly from the Cloanto install tree.
- The TODO's "user's amiga.adf" (WB 3.1) was not needed: the WB 3.1
  floppy has no Ed; the install-tree binary above is the 3.x Ed.
- String dumps: `python tools/hunk_dump.py <file> --strings`; diffs
  computed on the (offset, string) sets per version.
