# Reservations for the Cloud Agent

Work reserved for the Cloud Agent, which contributes by pull request from a separate cloud session, so the local waves never duplicate it. The plan, budget and briefs are in [the Cloud Agent plan](../research/cloud-agent-plan.md).

**Rules:**
- A reserved item or file is not a local agent's. Wave 5 briefs list these under "not yours; reserved for the Cloud Agent", and their scopes drop the reserved items.
- **Before each Wave 5 sub-wave,** the orchestrator runs `gh pr list --state all --search "head:cloud/"` and updates the status lines below:
  - **merged:** the items are done, and struck from the plan;
  - **open:** still reserved;
  - **abandoned,** after seven days with no commit or comment: released back to Wave 5, only with the owner's OK.
- **This file is append-only:** add status lines; never rewrite history.
- **The Cloud Agent touches only its reserved files.** If it needs another file, it stops and says so in its pull request.

## Task 1: fuzz targets for the parsers that have none

- **Branch:** `cloud/fuzz-parsers`.
- **Pull request:** not opened yet.
- **Covers:** W5m's deliverables 1 to 3 (sub-wave 5a) in `docs/research/wave5-plan.md`. The fuzz-coverage items in `docs/research/whats-left.md` (quality).
- **New fuzz targets:** `latex_math`, `asciimath`, `bibtex`, `ris`, `csl_json`, `theme`, `lexicon`, `vault_import` and `rpc`, plus a `cargo xtask fuzz-seed` command and the nightly matrix lines.
- **Owns:**
  - `fuzz/Cargo.toml` (appended), `fuzz/src/lib.rs`, `fuzz/README.md`, and new files under `fuzz/fuzz_targets/`;
  - the new `xtask/src/fuzz_seed.rs`, with one `mod` line and one match arm in `xtask/src/main.rs`;
  - `.github/workflows/nightly.yml`: the fuzz matrix, and the "Seed the corpus" step;
  - new files under `fixtures/cloud/`;
  - **only for a crash or unbounded walk a target finds:** a small cap with a regression test in `crates/textweaver-math/src/` (the parser), `crates/textweaver-cite/src/{bibtex,ris,csljson}.rs`, `crates/textweaver-theme/src/file.rs`, `crates/textweaver-lexicon/src/data.rs`, `crates/textweaver-vault/src/import.rs` and `crates/textweaver-app/src/rpc.rs`.
- **Budget:** expected $32, high $58. Stop point $44.
- **Status:**
  - Sunday, September 27, 2026, 9:59 PM: reserved. The owner has started a Cloud Agent session.
  - Sunday, September 27, 2026, 10:03 PM: the owner passed these reservations to the Cloud Agent session, which is recalibrating its work to them.

## Task 2: the generated settings reference and the docs consistency check

- **Branch:** `cloud/docs-checks`.
- **Pull request:** not opened yet.
- **Covers:** W5p's deliverables 2 and 3, and quick wins 1, 2, 4 and 5 (sub-wave 5c), in `docs/research/wave5-plan.md`. The docs-drift items in `docs/research/whats-left.md` (documentation).
- **Owns:**
  - the new `docs/settings-reference.md`, which is generated;
  - the new `crates/textweaver-app/tests/settings_reference.rs`;
  - the new `xtask/src/docs_check.rs`, with its `mod` line and match arms in `xtask/src/main.rs`;
  - two steps in the `docs` job of `.github/workflows/ci.yml`;
  - the drift fixes the check finds in `docs/README.md` (the Decisions list, crate count and Roadmap line), `docs/adr/README.md` (index lines only), `docs/roadmap.md` (the status block) and `docs/dev/architecture.md` (the crate count only);
  - two lines in `docs/dev/testing.md`.
- **Not its:** `docs/settings.md` and `settings_schema.rs`.
- **Budget:** expected $22, high $40. Stop point $32.
- **Status:**
  - Sunday, September 27, 2026, 9:59 PM: reserved. Recommended to run during the pause after Wave 4.

## Task 3 (optional): epubcheck and veraPDF on the writers' output

- **Branch:** `cloud/second-tool-checks`.
- **Pull request:** not opened yet.
- **Covers:** part of W6t (Wave 6): the EPUB and PDF second-tool checks. liblouis stays with W6t.
- **Owns:** the new `.github/workflows/second-tool.yml`, and its warnings allowlist file.
- **Budget:** expected $18, high $32. Stop point $24. **Runs only if tasks 1 and 2 cost $80 or less.**
- **Status:**
  - Sunday, September 27, 2026, 9:59 PM: reserved, conditional. Not started.

## See also

- [The Cloud Agent plan](../research/cloud-agent-plan.md)
- [Tasks and agent briefs](tasks.md)
- [Roadmap to the final alpha](../research/roadmap-2026.md)
