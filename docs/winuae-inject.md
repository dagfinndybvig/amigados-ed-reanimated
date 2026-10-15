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

## Next: the injection hook

Plan: hook the host-keyboard → Amiga keycode path (cia.c / keybuf.c — the
function that queues a raw Amiga keycode) and add a tiny listener (TCP
socket or watched drop-file, ~100-300 lines C) that feeds keycodes in.
Then boot_test.py gains an inject step and the [VERIFY] items in
docs/ed-spec.md become mechanically testable.
