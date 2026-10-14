# tools/ — analysis and automation scripts

Two families of tooling live here. Python for binary analysis of the
original Ed 1.14; PowerShell for driving windows (emulator and TUI
testing) with synthetic input and screenshot/OCR verification.

## Binary analysis (Python)

| Script | Purpose |
|---|---|
| `adf_extract.py` | AmigaDOS OFS (`DOS\0`) ADF extractor; on-disk layout verified empirically (hash table at 24, file size at 324) |
| `inspect_adf.py` | Raw block inspector for ADF images |
| `adf_patch.py` | Patch files inside ADF images (used to inject test programs) |
| `hunk_dump.py` | TRIPOS/standard Amiga executable parser + BCPL string dumper |
| `disasm_ed.py` | Capstone m68k disassembler for the Ed binary (full listing at `emulator/ed.asm`) |
| `call_census.py` | Call-site census keyed on `(a2)` globvec offsets |
| `make_gvdump.py` | Hand-assembled globvec memory-dump tool (campaign parked) |
| `make_inject.py` | Build the CIA-based keyboard injector with phase beacons (input probe) |
| `make_inject2.py` | Build the input.device IND_WRITE keyboard injector (InputEvents) |
| `make_odo2.py` | Build the OpenDevice+CloseDevice isolation test (corrected layout) |
| `adf_read.py` | Read a file (or list a directory) out of an ADF image |
| `boot_test.py` | Autonomous boot-test harness: launch WinUAE with a generated config, poll the emulation window with PrintWindow captures, OCR (2x upscaled) + background-color beacon sampling, kill on match/timeout. No host input — works over RDP |

## Window automation (PowerShell)

| Script | Purpose |
|---|---|
| `shot.ps1` / `shotwin.ps1` | Screenshot (screen / window by hwnd) |
| `ocr.ps1` / `cropocr.ps1` / `ocrpos.ps1` | Windows OCR of screenshots |
| `sendkeys.ps1` | SendKeys to a process's main window |
| `sendkeys_fg.ps1` | SendKeys to a window by hwnd, taking foreground robustly and verifying focus before sending |
| `focuswin.ps1` / `raisewin.ps1` | Foreground/raise a window (AttachThreadInput) |
| `listwin.ps1` | List windows by process or title |
| `findoverlay.ps1` | Find top-level windows covering a point (hunting invisible overlays) |
| `guithread.ps1` | Inspect a window's GUI-thread focus/capture/caret state |
| `nofocusshot.ps1` | Screenshot a window without stealing focus |
| `pwshot.ps1` | PrintWindow capture (works even when the window is occluded) |
| `capture_winuae.ps1` | PrintWindow capture of the WinUAE emulation window (largest visible window of the process; no focus steal) |
| `ocrbox.ps1` | OCR with per-word bounding boxes |
| `postkeys.ps1` | PostMessage WM_KEYDOWN to a window |
| `resetkeys.ps1` | Send the WinUAE Amiga-reset combo |
| `install-af.ps1` | Silent install of the Amiga Forever MSI |
| `resizewin.ps1`, `click.ps1`, `sendkeys.ps1`, `bothbuttons.ps1`, `asciiart.ps1`, `catchmsg.ps1` | Emulator-driving helpers used during the live-verification sessions |

## Automation gotchas (all of these bit us)

1. **SendKeys goes to whatever has focus.** `SetForegroundWindow` can
   fail silently (Windows foreground lock), and the keystrokes land
   in an unrelated window — a browser, for instance. Always verify
   `GetForegroundWindow()` equals the target before sending;
   `sendkeys_fg.ps1` does this (AttachThreadInput + restore + verify,
   five attempts).
2. **Console windows do not belong to the console app.** A console
   program's `MainWindowHandle` is typically 0; the visible window is
   owned by the console host (conhost / Windows Terminal). Find it by
   title via `EnumWindows`, not via the app process.
3. **Screenshots are rect captures of the screen.** If the target
   window is minimized or covered, `CopyFromScreen` happily captures
   whatever is there instead. Raise the window first and check what
   you actually captured.
4. **OCR scrambles small console text.** Word order and digits can
   come out wrong or interleaved; use OCR to confirm big visible
   strings (prompts, messages), not for fine-grained assertions.
5. **Give each test window a distinctive title** (e.g.
   `cmd /c title NEOED-LIVE & app.exe`) so enumeration cannot match
   the wrong window.
6. **Piped stdin does not reach a crossterm TUI** — it reads the
   console device (CONIN$) directly, so `app.exe < input` hangs. Drive
   the real window instead (or use a `--script` mode if the app has
   one).
