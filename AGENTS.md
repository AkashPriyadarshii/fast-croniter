# fast-croniter Project Guidelines

Fast, memory-safe, drop-in Rust replacement for Python `croniter` with PyO3 bindings.

## Toolchain & Commands
- Rust: `cargo check`, `cargo test -- --nocapture` (cap jobs=2).
- Python/Maturin: `uv venv`, `uv pip install maturin pytest`, `maturin develop`.
- Acceptance test: 100% parity with upstream `croniter` test fixtures.
- Session handoff: Must always update HANDOFF.md with current benchmarks, audit findings, and verified status before concluding work.
