# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

This is the **lecture/courseware** repo for a Chinese-language, hands-on course that teaches RocksDB by building a simplified LSM storage engine (`mini-LSM`) from scratch in Rust. It contains **only teaching material** — self-contained HTML lessons — not engine code. The learner is the user (`chuangliu0815`); Claude acts as the **teacher** and lesson author.

The mission arc is: **使用 RocksDB → 理解架构 → 改源码 → 自建引擎** (use → architecture → modify source → build your own). This repo is the artifacts for the final "自建引擎" stage (阶段 4). See `MISSION.md` for the full mission, `NOTES.md` for the running teaching log + learned preferences, `RESOURCES.md` for curated sources, and `roadmap.html` for the live phase plan.

## The two repos (critical)

- **This repo** (`rocksdb-mini-lectures/`): HTML lessons. Claude writes these.
- **Sibling engine repo** (`../rocksdb-mini/`): the actual Rust engine the user **hand-writes** (Rust edition 2024; deps: `bytes`, `crossbeam-skiplist`, `anyhow`, `crc32fast`, `tempfile`, `lz4_flex`). Modules: `kv`, `memtable`, `wal`, `sst`, `sst_flat`, `block`, `iterator`, `merge`, `nocache`, `engine`.

> NOTE: `NOTES.md` records macOS paths (`/Users/chuang/...`) from the user's machine. In this environment the repos live under `/root/dev/rocksdb-mini/`. Trust the actual filesystem, not the logged paths.

## Teaching discipline (non-negotiable — see NOTES.md ★ lines)

- **The user writes all engine code by hand.** Claude shows code + explains *why*, but does **not** write into `../rocksdb-mini/src/`. Only deviate if the user explicitly says "you write it."
- **The user runs all commands.** Claude may verify that shown code compiles by spinning up a throwaway cargo project under `$CLAUDE_JOB_DIR/tmp` (never touching the user's engine src). Verification claims must be backed by an actually-run test; performance claims must be measured and have their boundaries labeled honestly (MVP simplifications stated explicitly).
- Claude *may*: compile/run to verify, fix scaffolding/CMake/CI glue, and review the user's code.
- **Communicate and teach in Chinese.**

## Lesson format (every lesson HTML must contain)

Each phase is a numbered directory (`000N-phase-X-name/`) with a hub `index.html` linking sub-lessons (e.g. `a1-...`, `a2-...`). Every lesson carries **five blocks**:

1. 🎯 **Goal + Done-when** — explicit goal and a concrete completion test.
2. 🧪 **Harness** — teach a *reusable harness system*, not just "write a test." The course accumulates named harness types (table-driven, model-diff oracle, invariant, fault-injection, crash-recovery, format/round-trip, cross-persistence round-trip, layered-lookup diff, …).
3. 🧰 **Coding style** — a transferable programming idiom.
4. 🧠 **System mindset** — cross-domain systems thinking (Linux/storage/network/algorithms/HPC), mapped back to a RocksDB source anchor (cite real file:line in the RocksDB source when used).
5. 🔄 **CI/CD** — incrementally build a real pipeline lesson-by-lesson.

Also required per lesson: numbered **steps** (what code to write / what commands to run), the **Rust to hand-type** (shown, user types), and **core questions left for the user to self-answer** (no answers given), plus a first-hand source + follow-up reminder. The styling is a self-contained `<style>` block (serif body, monospace kickers, warm paper palette) — match the existing lessons rather than introducing a framework.

Keep all content inside the courseware; if a lesson gets long, split into multiple HTMLs organized under the hub `index.html`.

## Progress & roadmap

`roadmap.html` is the **living document** — ~36 phases / 6 tiers / 6 milestones (M1 MVP … M6 "your own RocksDB"). Phase status (`done`/`now`/`todo`) is tracked inline via `class="st ..."` spans and updated as work proceeds. Completed so far: Phases A–K (dirs `0002`–`0012`), reaching scan/snapshot. Each lesson logs its MVP simplifications and the "deliberate gaps" that a later phase fills (e.g. flat full-index SST in Phase C → block format in Phase E → block cache in Phase S).

When picking up teaching: read `NOTES.md` (latest dated entries hold current state + the next planned lesson), check `roadmap.html` for the `now` marker, then continue authoring the next lesson in the established five-block format.

## Working notes

- There are no build/test/lint commands **for this repo** — it is static HTML. Open the `.html` files in a browser to preview.
- Engine build/test happens in the sibling repo and is run by the user: `cargo test --all`, `cargo fmt --check`, `cargo clippy -- -D warnings` (the CI gate this course teaches). `0004-phase-c-sst/ci-reference.yml` is the reference GitHub Actions workflow.
