# Contributing to Treeport

Thanks for taking the time to contribute! Treeport is a small, focused,
zero-dependency CLI, and contributions of any size — a typo fix, a bug
report, a new flag — are welcome.

## Table of contents

- [Ground rules](#ground-rules)
- [Getting set up](#getting-set-up)
- [Making a change](#making-a-change)
- [Commit messages](#commit-messages)
- [Before you open a PR](#before-you-open-a-pr)
- [Pull request process](#pull-request-process)
- [Reporting bugs](#reporting-bugs)
- [Proposing a feature](#proposing-a-feature)
- [Project conventions](#project-conventions)
- [Release process](#release-process)

## Ground rules

- Be respectful and constructive in issues and reviews.
- **Keep it dependency-free.** Treeport builds with `rustc`/Cargo std only —
  no external crates. This is a deliberate design goal (see the README), not
  an oversight, so PRs adding a crate dependency will generally be declined
  unless there's a very strong reason (and even then, expect discussion
  first — please open an issue before writing the code).
- Keep the CLI's Unicode/RTL correctness intact. If your change touches
  string handling, tree rendering, or export formatting, test it against a
  path/filename containing Persian, Arabic, or Hebrew text, not just ASCII.

## Getting set up

You need only the Rust toolchain — no other dependencies to install.

```bash
git clone https://github.com/amirk1998/treeport
cd treeport
cargo build
cargo run -- . -q          # quick sanity check
```

Or, since Treeport has zero external dependencies, a single-file build also
works without Cargo:

```bash
rustc -O src/main.rs -o treeport
```

## Making a change

1. **Open an issue first for anything non-trivial** (new flags, output
   format changes, behavioral changes) so we can agree on the approach
   before you invest time in it. Typo fixes and small bug fixes can skip
   straight to a PR.
2. **Branch off `main`**, named by intent:
   - `feat/<short-description>` — new functionality
   - `fix/<short-description>` — bug fix
   - `docs/<short-description>` — documentation only
   - `refactor/<short-description>` — internal change, no behavior change
   - `chore/<short-description>` — tooling, CI, dependencies (of the build,
     not the crate)
3. Make focused commits. It's fine to have several commits in a PR — they
   get squashed on merge — but each one should represent a coherent step,
   not "wip" / "fix typo" / "actually fix it".

## Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <short summary>

[optional longer body explaining the why, not just the what]
```

Common types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`.

Examples:

```
feat: add --min-depth to complement --max-depth
fix: correct wasted-space total when duplicates span multiple depths
docs: clarify .treeportignore glob matching in README
```

The PR title (which becomes the squash-merge commit on `main`) should
follow the same format — that's what feeds GitHub's auto-generated release
notes.

## Before you open a PR

Run the same checks CI will run, so you're not waiting on a red build to
find out:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
cargo test
```

If you touched:

- **Any flag or CLI option** → update the `USAGE` string in `main.rs`, the
  module doc comment at the top of the file, and the relevant section of
  `README.md`. These three tend to drift out of sync — check all three.
- **Export format (Markdown/JSON/CSV)** → run a real export and sanity check
  it opens correctly (JSON: `jq . report.json`; CSV: open in a spreadsheet
  app; Markdown: preview on GitHub).
- **Anything Unicode/RTL-related** → test with a Persian/Arabic/Hebrew path
  or filename, both in the tree view and in an export, with and without
  `--ascii`.
- **User-facing behavior** → add a line under `## [Unreleased]` in
  `CHANGELOG.md` (see [Keep a Changelog](https://keepachangelog.com/) for
  the `Added` / `Changed` / `Fixed` / `Removed` categories).

## Pull request process

1. Open the PR against `main` with a clear description of **what** changed
   and **why** — link the issue it resolves if there is one.
2. CI (`ci.yml`) must pass: fmt, clippy, build, and tests on Linux, macOS,
   and Windows.
3. Expect review feedback — this is a small project, so turnaround is
   usually quick, but please be patient.
4. PRs are **squash-merged**, so don't worry about tidying up intermediate
   commits — just make sure the PR title/description is what you'd want as
   the permanent commit message.
5. Your branch gets deleted automatically on merge.

## Reporting bugs

Open an issue with:

- Your OS and Treeport version (`treeport --help` shows usage; include the
  release tag or commit you built from).
- The exact command you ran.
- What you expected vs. what happened.
- If it's rendering-related (tree layout, RTL text, colors), a screenshot or
  copy-pasted terminal output helps a lot — these bugs are often invisible
  in a plain text description.

## Proposing a feature

Open an issue describing the use case before writing code — especially for
anything that would add a new flag, change default behavior, or touch the
export formats. This avoids a finished PR getting stuck on a design
disagreement that could've been settled up front.

## Project conventions

- **Branching model:** GitHub Flow — short-lived branches off `main`, PR +
  CI-gated merge, no long-lived `develop` branch. See the README's
  [Versioning](README.md#versioning) section for how releases are tagged.
- **Formatting/linting:** `cargo fmt` (default settings, no `rustfmt.toml`
  overrides) and `cargo clippy` with warnings denied. CI enforces both.
- **No `unsafe` unless justified.** The Windows console/FFI code is the one
  deliberate exception — if you add more, explain why in a comment at the
  `unsafe` block, not just in the PR description.
- **Cross-platform by default.** Before assuming a path separator, line
  ending, or console behavior, check how the existing code branches on
  `cfg!(windows)` / `cfg!(unix)` and follow the same pattern.

## Release process

Releases are cut by the maintainer via git tags (`v*.*.*`), which trigger
`release.yml` to cross-compile and publish binaries automatically. As a
contributor you don't need to do anything release-related — just make sure
your `CHANGELOG.md` entry under `## [Unreleased]` accurately describes your
change, and it'll be included in the next version bump.

---

If anything in this guide is unclear or out of date, that's a bug too —
feel free to open a PR fixing it.
