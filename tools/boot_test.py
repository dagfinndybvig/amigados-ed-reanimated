#!/usr/bin/env python3
"""Autonomous boot-test harness: boot an ADF under the OSS WinUAE,
poll the emulation window with PrintWindow captures, OCR the text,
watch background-color beacons, then kill the emulator.

Everything here is hands-off: no keyboard/mouse input to the emulation
(the only kind that works over RDP). Output: PNG shots + OCR text +
beacon color timeline.

Usage:
  python boot_test.py <image.adf> <tag> [--timeout 180] [--interval 6]
      [--wait-for <regex>]   stop early when OCR text matches
      [--keep-all]           keep every poll shot, not just the last
      [--base-config <uae>]  default emulator/atest5.uae

Prints a JSON-ish summary at the end: shots taken, OCR lines of the
final shot, background color hex per shot, and whether wait-for hit.
"""
import argparse
import os
import re
import subprocess
import sys
import time

REPO = os.path.dirname(os.path.abspath(__file__))
EMUDIR = os.path.join(REPO, "emulator")
SCRATCH = os.path.join(EMUDIR, "scratch")
WINUAE = os.path.join(EMUDIR, "winuae-oss", "winuae64.exe")
CAPTURE = os.path.join(REPO, "tools", "capture_winuae.ps1")
OCRBOX = os.path.join(REPO, "tools", "ocrbox.ps1")


def make_config(adf, tag, base):
    with open(base, "r") as f:
        cfg = f.read()
    cfg = re.sub(r"(?m)^floppy0=.*$", lambda m: "floppy0=" + os.path.abspath(adf), cfg)
    path = os.path.join(SCRATCH, "boot-%s.uae" % tag)
    with open(path, "w") as f:
        f.write(cfg)
    return path


def capture(png):
    p = subprocess.run(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
         CAPTURE, "-OutPng", png],
        capture_output=True, text=True, timeout=60)
    return "CAPTURED" in p.stdout


def ocr(png):
    # upscale 2x first: the 8x8 Amiga font OCRs much more reliably
    scaled = os.path.join(SCRATCH, "shots", "_scaled.png")
    from PIL import Image
    im = Image.open(png).convert("RGB")
    im = im.resize((im.width * 2, im.height * 2), Image.NEAREST)
    im.save(scaled)
    p = subprocess.run(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
         OCRBOX, "-Png", scaled, "-Scale", "2"],
        capture_output=True, text=True, errors="replace", timeout=120)
    words = []
    for line in p.stdout.splitlines():
        m = re.match(r"\s*\d+,\s*\d+\s+\d+x\s*\d+\s+(.*)$", line)
        if m:
            words.append(m.group(1))
    return words


def bg_color(png):
    """Mode color of the capture = custom register 0 (screen background)."""
    from PIL import Image
    im = Image.open(png).convert("RGB")
    counts = {}
    for c in im.getdata():
        counts[c] = counts.get(c, 0) + 1
    return max(counts, key=counts.get), counts[max(counts, key=lambda c: counts[c])]


def to_hex(c):
    return "%02x%02x%02x" % c


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("adf")
    ap.add_argument("tag")
    ap.add_argument("--timeout", type=int, default=180)
    ap.add_argument("--interval", type=int, default=6)
    ap.add_argument("--wait-for", default=None)
    ap.add_argument("--keep-all", action="store_true")
    ap.add_argument("--base-config", default=os.path.join(EMUDIR, "atest5.uae"))
    ap.add_argument("--ocr-every", type=int, default=2, help="OCR every Nth poll")
    args = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

    adf = os.path.abspath(args.adf)
    cfg = make_config(adf, args.tag, args.base_config)
    shots = os.path.join(SCRATCH, "shots")
    os.makedirs(shots, exist_ok=True)

    proc = subprocess.Popen([WINUAE, "-f", cfg])
    print("launched pid=%d cfg=%s" % (proc.pid, cfg))

    deadline = time.time() + args.timeout
    i, ocr_words, colors = 0, [], []
    hit = False
    last_png = None
    try:
        while time.time() < deadline:
            time.sleep(args.interval)
            png = os.path.join(shots, "%s-%02d.png" % (args.tag, i))
            if not capture(png):
                print("poll %d: no window yet" % i)
                i += 1
                continue
            last_png = png
            col, n = bg_color(png)
            colors.append((to_hex(col), n))
            do_ocr = (i % args.ocr_every == 0) or time.time() > deadline - args.interval
            if do_ocr:
                ocr_words = ocr(png)
                print("poll %d: bg=%s ocr=%s" % (i, colors[-1][0],
                                                 " ".join(ocr_words)[:120]))
            else:
                print("poll %d: bg=%s" % (i, colors[-1][0]))
            if not args.keep_all and last_png != png:
                pass
            if args.wait_for and ocr_words and \
                    re.search(args.wait_for, " ".join(ocr_words), re.I):
                hit = True
                break
            i += 1
    finally:
        import shutil
        if last_png and os.path.exists(last_png):
            keep = os.path.join(shots, "%s-final.png" % args.tag)
            shutil.copyfile(last_png, keep)
        subprocess.run(["taskkill", "/PID", str(proc.pid), "/T", "/F"],
                       capture_output=True)
        time.sleep(2)

    print("---")
    print("shots: %d, wait_for hit: %s" % (i, hit))
    print("colors:", " ".join(c for c, _ in colors))
    if ocr_words:
        print("final OCR: " + " ".join(ocr_words))
    print("final shot: %s" % last_png)
    return 0 if (not args.wait_for or hit) else 1


if __name__ == "__main__":
    sys.exit(main())
