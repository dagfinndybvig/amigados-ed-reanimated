# LEGAL.md — Legal Basis and Ground Rules for This Project

This document states the legal argument underpinning this reverse-engineering
project, and the rules every participant must follow. It is a reasoned
position statement, **not legal advice**; it reflects copyright law as
commonly understood in the US, EU, and UK as of its writing.

## 1. What is being analyzed, and who owns it

**Subject:** `Ed` version 1.14, the full-screen text editor shipped on the
Workbench 1.3 disk of AmigaOS 1.3 (1988), a BCPL program of TRIPOS
heritage, originally written for Commodore-Amiga.

**Copyright chain:** Ed was authored under contract to Commodore
(AmigaDOS 1.x derives from TRIPOS, written by MetaComCo). Commodore's
assets passed through ESCOM and subsequent holders; the rights to classic
AmigaOS are currently held by **Cloanto** (acquired via Amiga Corporation,
2022), which licenses the ROM and OS images through its *Amiga Forever*
product line.

Consequences:

- Ed is a **proprietary, copyrighted program**. It is not abandonware in
  any legal sense, regardless of how long ago it was sold.
- The rights holder is identifiable and actively licensing; there is a
  fully lawful channel to obtain the software.

## 2. Lawful acquisition (the precondition for everything)

All analysis in this project is performed on a copy obtained from the
project owner's **Amiga Forever** license, purchased from Cloanto. This is
a sanctioned distribution by the current rightsholder — the strongest
possible provenance. (Original Workbench disks owned since the 1980s would
be an equivalent lawful source.)

**Rule 1 — Provenance first.** No file in `materials/` may be analyzed
unless its provenance is the licensed Amiga Forever package or original
retail media. Copies downloaded from the internet ("abandonware" sites,
torrent ROM packs) are not used, because their provenance cannot be
established.

## 3. The legal argument for reverse engineering

The activity of this project is (a) static analysis of a lawfully owned
copy, (b) dynamic analysis in an emulator, and (c) writing an independent
description of behavior. Each step is lawful in the relevant
jurisdictions:

### 3.1 United States

- **Intermediate copying for study is fair use.** In *Sega Enterprises v.
  Accolade* (9th Cir. 1992) and *Sony Computer Entertainment v. Connectix*
  (9th Cir. 2000), courts held that disassembling a lawfully obtained
  copy to study unprotected ideas (functional behavior, interfaces) is
  fair use, even though the copy itself is infringing in form. This
  project's spec writing is exactly that protected activity.
- **Interoperability analysis is expressly permitted.** 17 U.S.C.
  §1201(f) authorizes circumvention and reverse engineering of
  technological measures where necessary to achieve interoperability of an
  independently created program.
- **Archival copies are permitted.** 17 U.S.C. §117 allows the owner of a
  copy to make copies "as an essential step in the utilization" of the
  program and for archival purposes. Our disk-image extraction into
  `materials/` is within this.
- **Idea/expression dichotomy.** 17 U.S.C. §102(b): copyright protects
  expression, not "any idea, procedure, process, system, method of
  operation." A command set, a key binding table, an on-disk format, an
  algorithm's behavior — these are methods of operation and facts. The
  spec (`docs/ed-spec.md`) is our own expression describing them.

### 3.2 European Union

Directive 2009/24/EC on the legal protection of computer programs:

- **Art. 5(1–2):** the lawful acquirer may perform acts of loading,
  running, and error correction without authorization, and **Art. 5(2)**
  permits making a back-up copy. **Art. 5(5)** states these rights
  **may not be overridden by contract**.
- **Art. 6:** decompilation is permitted where indispensable to obtain
  information to achieve interoperability, subject to conditions we
  satisfy: lawful copy, no other source of the information (there is no
  source available for 1.x-era Ed other than analysis and the manuals),
  and use limited to the interoperable parts. **Art. 6(1)** likewise
  applies "notwithstanding any contrary agreement."

### 3.3 United Kingdom

The Copyright, Designs and Patents Act 1988, as amended, provides
statutory exceptions mirroring the EU regime: **s.50A** (lawful user may
make back-up copies), **s.50B** (decompilation where necessary for
interoperability), **s.50BA** (observation and study of program function
while performing normal use). These cannot be excluded by contract.

### 3.4 On EULA anti-reverse-engineering clauses

Some licenses purport to prohibit reverse engineering. The project takes
the following position: the Amiga Forever content is distributed as data
files (disk images) rather than under a shrink-wrapped software EULA
binding the analysis activity; and in the EU/UK the statutory
interoperability and back-up rights are non-overridable. In the US,
outcomes on contract-based restrictions vary by circuit, which is one more
reason this project's publishable output is a **behavioral description**,
not disassembled code (see §5).

## 4. The leaked-source rule (the one irreversible mistake)

Source code of AmigaOS (including early BCPL components) has leaked onto
the internet over the years. This is the single largest legal risk to a
project like this, because copyright infringement can be *inadvertent and
unfixable*: once leaked code has informed a "reimplementation," the work is
tainted and cannot be cleansed by rewriting.

**Rule 2 — Never access leaked AmigaOS source code**, in whole or in
part, from any source. This includes: the 3.x-era source trees circulated
since the 2000s, excerpts, "reference" snippets from forums or wikis
reproducing that code, and any derivative repositories. If such material
surfaces during research, close it immediately and record what happened in
the project notes.

This project's sources are exclusively:

1. The licensed **binary** (Amiga Forever disk image), and
2. **Published manuals and magazine documentation** of Ed's behavior
   (commercial publications that document the command set for users).

## 5. What may and may not be published

| May be published (this project's output) | Must never be published |
|---|---|
| Behavior specifications written in our own words | Any portion of Ed's binary or any disassembly/decompilation of it |
| Command tables, key bindings, limits, defaults | The extracted Workbench disk contents (`materials/`) |
| On-disk format descriptions (OFS layout, TRIPOS load format) | ROM images, disk images (`af-extract/`, any `.adf`/`.rom`) |
| Our own tooling (`tools/`) | Scans/text of the AmigaDOS manuals (`ref/`) |
| Findings about algorithms, described textually | Code derived from Ed, BCPL runtime, or any Commodore source |

The legal basis for the left column is the idea/expression dichotomy
(§102(b) in the US; Art. 1(2)–(3) of Directive 2009/24/EC in the EU):
functional descriptions are not the protected expression.

## 6. Clean-room discipline (for the reimplementation phase)

If the optional reimplementation proceeds:

1. **Analysis side** reads the binary and manuals and produces only
   `docs/ed-spec.md` (and further specs).
2. **Implementation side** works exclusively from the spec. Members who
   have studied the disassembly do not write implementation code.
3. **Differential testing** compares behavior only — inputs and outputs
   of the original and the clone — never code.
4. The implementation carries its own original code and documentation,
   with no Commodore-derived text, comments, strings, or data.

## 7. Standing rules of the repository

1. Nothing in `materials/`, `ref/`, or `af-extract/` is ever committed to a
   public repository, uploaded, or shared. These directories hold
   third-party copyrighted material used locally for licensed private
   study. A `.gitignore` excluding them should precede any `git init`.
2. Tools in `tools/` are original code and may be published.
3. The spec in `docs/` is written to be publishable and may be released.
4. If in doubt about a source or an output, stop and re-read §4 and §5.

## 8. Summary of the argument

Ed 1.14 is proprietary software owned by Cloanto. The project obtained it
through the rightsholder's own retail channel (Amiga Forever), giving
lawful possession. Studying a lawfully obtained copy — statically,
dynamically, in an emulator — to document how it behaves is a permitted
act in the US (fair use, §117, §1201(f), idea/expression dichotomy), the
EU (Directive 2009/24/EC arts. 5–6, non-overridable by contract), and the
UK (CDPA ss.50A–50BA). The project never touches leaked sources, never
publishes copyrighted material, and confines its output to independently
written descriptions of uncopyrightable functional behavior. A future
reimplementation, if any, is performed clean-room from those descriptions.
