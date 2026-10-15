# WinUAE injection sub-project (local only)

A patched WinUAE build that lives entirely under `emulator/winuae-inject/`
(gitignored — WinUAE is GPL, so no source or binaries from it may enter this
clean-room repository). Everything below is reproduced from local state only.

## Layout

```
emulator/winuae-inject/
├── src/            shallow clone of github.com/tonioni/WinUAE (master)
├── bin/winuae64.exe   built injection-capable binary (Release|x64)
├── tools/          nasm, zlib, libpng, Wintab headers, D3DX (nuget)
├── build.bat       msbuild wrapper (VS 2019 BuildTools, vcvars64 + nasm in PATH)
├── build-zlib.bat  builds tools/zlib/lib/{zlibstatic.lib,png.lib} with /MT
├── reconfig.py     reapplies all local patches to a pristine vcxproj (idempotent;
│                   run after `git checkout -- .../winuae_msvc.vcxproj`)
├── fixsysconfig.py local feature undefs + stubs in src/od-win32/sysconfig.h
├── fixdinput.py / fixdinput2.py / fixd3d11.py   small source guards (CATWEASEL etc.)
├── genstubs.py     regenerates src/stubs.cpp from linker output (syms2.txt)
└── prune.py        (superseded by reconfig.py)
```

## Rebuild from scratch

```
cd emulator/winuae-inject/src
git checkout -- od-win32/winuae_msvc15/winuae_msvc.vcxproj od-win32/dinput.cpp od-win32/direct3d11.cpp
cd ..
python reconfig.py && python fixsysconfig.py && python fixdinput.py && python fixdinput2.py && python fixd3d11.py
python -c "<re-add options.h include to direct3d11.cpp d3d_select>"   # see git log of this file
cmd //c build-zlib.bat && cmd //c build-png.bat
cmd //c build.bat
-> bin/winuae64.exe
```

## What was changed vs upstream (and why)

Build environment: VS 2019 BuildTools (v142 toolset override), NASM added to
PATH, `stdclatest` → `stdc17` (VS2019 does not know it), SDK 10.0.19041.

Third-party libs are not bundled in the git tree, so these features are
switched off in `sysconfig.h` (undef block at end of file): CHD, Lua, MIDI
emulator, libmpeg2, OpenAL, PortAudio, Direct3D11, Logitech LCD, tablet
library, Catweasel, CAPS/SCP, Pro-Wizard, serial ENET, drive sounds, RAR/7z.
Vendored instead: zlib + libpng (built with /MT into `tools/zlib/lib`),
Wintab SDK headers (MIT, for dinput.cpp), d3dx9 (Microsoft.DXSDK.D3DX nuget;
d3dx9_43.dll already in System32, delay-loaded).

Removed from the Release|x64 project: blkdev_cdimage.cpp (FLAC), parser.cpp
(Ghostscript), win32_uaenet.cpp (pcap), win32_videograb.cpp (ATL/DShow),
ioport.cpp (TVicPort). `src/stubs.cpp` (generated) provides no-op
definitions for the symbols those files used to export, plus the D3D_*
display-function-pointer storage (upstream defines it inside
direct3d11.cpp's WITH_DIRECT3D11 guard).

Small source guards: catweasel call sites in dinput.cpp and win32gui.cpp
(stub macros in sysconfig.h), `d3d_select` tail of direct3d11.cpp
(D3D9/GDI path kept), `DWMWA_USE_IMMERSIVE_DARK_MODE` define for the older
SDK. Output redirected from upstream's hardcoded `d:\amiga\` to `bin/`.

## Verification

`python tools/boot_test.py <adf> <tag> --exe emulator/winuae-inject/bin/winuae64.exe`
boots KS1.3 identically to the stock OSS binary (same color timeline, same
OCR, 50 FPS).

## The injection hook (implemented)

No new keycode plumbing was needed: upstream `keybuf.cpp` already has
`keybuf_inject(const uae_char *txt)` — the same entry point the clipboard
paste-to-keyboard path uses. It filters ASCII through `kbtable`
(`\n` = Return 0x44, `\t`, letters, digits, punctuation) and queues the
text; `get_next_key()` drains it as press/release pairs paced by
`can_inject()` (~6 chars/sec at 50 FPS).

Added `src/od-win32/inject_server.cpp`: a TCP listener on
`127.0.0.1:7765` (loopback only, one connection at a time). Protocol:
send one line (bytes up to `\n` or EOF); the line is passed to
`keybuf_inject()` with a Return appended, the server replies `OK\n` and
closes. The thread starts from WM_CREATE in win32.cpp, next to
`clipboard_init`. Two more local patches: `patch_win32.py` (win32.cpp
hook, CRLF-safe) and `fixbeta.py` (disables the Public Beta Disclaimer
dialog that otherwise blocks headless startup).

Rebuild chain additions: `python patch_win32.py`, `python fixbeta.py`
(both idempotent, run after `reconfig.py`; `reconfig.py` also adds
inject_server.cpp to the vcxproj).

Harness: `python tools/boot_test.py <adf> <tag> --exe
emulator/winuae-inject/bin/winuae64.exe --inject "25:Echo HELLO\n"`
— injects `Echo HELLO` + Return 25 seconds after launch (bare text
defaults to 15s; literal `\n` becomes Enter; repeatable, `SECONDS:TEXT`
per entry). Verified end-to-end on `emulator/scratch/edboot-injectcli.adf`
(edboot-fulldirect + `Echo INJECT-READY` + `Shell` appended to its
startup-sequence): injected text appears at the 1.3 CLI prompt.

Known capture quirk: PrintWindow shots of the beta build sometimes come
out as blended frames (odd mode colors); OCR of a blended frame can lag
one screen behind. Live observation confirms typing is correct.
