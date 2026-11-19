# fast-croniter Quality & Correctness Constraints

## Non-Negotiable Gates
1. **Reference Parity**: 100% test compatibility with upstream `kiorky/croniter` test fixtures. No skipping or silencing failing upstream assertions.
2. **Deterministic Behavior**: Zero undefined behavior or panics on malformed or extreme inputs. All errors must return explicit typed exceptions (`CroniterBadCronError`, etc.).
3. **No Unsafe Code**: `#![forbid(unsafe_code)]` or `#![deny(unsafe_code)]` in the Rust core unless strictly required by PyO3 FFI boundaries.
4. **Performance Gate**: Must execute at least 20x faster than pure-Python `croniter` on standard recurrence iterations.
5. **Linting & Hygiene**: Zero Clippy warnings (`cargo clippy --all-targets -- -D warnings`); zero Ruff warnings on Python wrappers.
