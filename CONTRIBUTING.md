# CONTRIBUTING.md

This is a research and historical-preservation project: a legal,
clean-room reverse engineering of AmigaDOS Ed 1.14 (1988), producing a
behavior specification and a Rust reimplementation ("Neo-Ed"). Keep
that framing in mind for every change.

## Hard rules (see LEGAL.md for the full argument)

1. **Never touch the leaked AmigaOS source tree** — not to read, not
   to compare, not to link. All analysis comes from the binary
   (obtained under the author's Cloanto Amiga Forever license) and
   from published manuals.
2. **No proprietary material in this repository.** ROM images, ADF
   disk images, and extracted Workbench files stay in gitignored
   directories (`materials/`, `ref/`, `af-extract/`, `emulator/`).
   Check `git status` before committing if in doubt.
3. **Respect the clean-room split.** The spec (`docs/ed-spec.md`) is
   written from binary analysis and live observation; the Rust code is
   written from the spec only. If you have read the disassembly, you
   are on the spec side; do not write `ed-rs/` code in the same
   session (LEGAL.md §6).
4. **Tag every spec claim** with its evidence: `[BIN]` (from the
   binary), `[LIVE]` (observed in the emulator), `[DOC]` (published
   source), or `[VERIFY]` (still to be confirmed).

## Working on the Rust clone

- Build and test inside `ed-rs/`:

  ```
  cargo build --release
  cargo test
  ```

- The TUI state machine (`screen.rs::process_key`) is unit-tested
  without a terminal; keep new event logic testable there instead of
  inside `run_loop`, which does the I/O.
- `--script` mode (`headless.rs`) is the harness for scripted and
  future differential tests.
- Keep `TODO.md` current after each work phase — this project's
  memory lives in its docs.

## Gotchas that have bitten us

- **BCPL gotchas** (load format, calling convention, word pointers,
  string packing) are collected in [BCPL.md](BCPL.md) — read it
  before doing any binary analysis.
- **User-facing gotchas** (piped stdin hangs the TUI, `--script`
  cannot ask prompts, CRLF handling, backup path, margin defaults)
  are in [MANUAL.md](MANUAL.md) section 9.
- **Automation gotchas** for the emulator/TUI-driving PowerShell
  scripts are in [tools/README.md](tools/README.md).
- **Line endings:** working-tree files are a mix of LF and CRLF (git
  `autocrlf` normalizes on commit). Match the dominant style of the
  file you are editing; do not reformat whole files.
- **Rust toolchain** (if installed via rustup on Windows): ensure
  `~/.cargo/bin` is on PATH before `cargo` commands.
