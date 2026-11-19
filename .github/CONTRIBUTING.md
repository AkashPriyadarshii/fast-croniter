# Contributing to fast-croniter

Thank you for your interest in improving `fast-croniter`! We welcome bug fixes, performance improvements, and documentation polish.

## Quick Start (Local Setup)

1. Fork and clone the repository.
2. Setup Python virtual environment & Rust toolchain:
   ```bash
   uv venv
   .venv\Scripts\activate  # Windows, or source .venv/bin/activate on Unix
   uv pip install maturin pytest python-dateutil pytz
   ```
3. Build the native extension in release mode:
   ```bash
   maturin develop --release
   ```
4. Run the full test suite:
   ```bash
   cargo test --jobs 2
   pytest tests/
   ```

## Contribution Rules

- **Upstream Parity First**: `fast-croniter` is a drop-in replacement for `kiorky/croniter`. Any behavior deviation from upstream must be documented or fixed.
- **Commit Style**: Use Conventional Commits (`feat:`, `fix:`, `perf:`, `docs:`, `test:`, `ci:`).
- **Tests Required**: Every bugfix must include a reproducing test; every feature must include unit coverage.
- **Linters Green**: Zero Clippy warnings (`cargo clippy --all-targets -- -D warnings`).
- **No AI Slop**: PRs containing unverified bulk-generated code, fake statistics, or redundant narrative comments will be rejected.

## Pull Request Process

1. Create a feature branch (`git checkout -b fix/issue-description`).
2. Verify all linters and tests pass locally.
3. Open a PR using our standard template and link any relevant issues.
