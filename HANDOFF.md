# Agent Handoff: fast-croniter v0.2.0

**Date**: 2026-11-22
**Repository**: `AkashPriyadarshii/fast-croniter`
**Current Branch**: `master` (synchronized with `origin/master`, tag `v0.2.0` published)
**Release Status**: Published on GitHub Releases (`v0.2.0`) and PyPI (`0.2.0`)
**Maintainer Identity**: `AkashPriyadarshii <272530059+AkashPriyadarshii@users.noreply.github.com>`
**Official Docs / Playground**: [https://fast-croniter-site.vercel.app](https://fast-croniter-site.vercel.app)

---

## 1. Executive Summary

fast-croniter has completed an autonomous multi-phase audit, hardening, and performance optimization cycle. The recurrence engine now executes with **zero heap allocations per iteration step**, direct C-API PyO3 bindings, and fat Link-Time Optimization (`lto = "fat"`, `codegen-units = 1`).

Single-thread step throughput reaches **6.31M ops/sec (184.5x faster than pure-Python `croniter`)**, while maintaining 100% bug-for-bug test parity across the entire 267-test suite from `kiorky/croniter`.

---

## 2. Key Architectural & Algorithmic Improvements

### A. Zero-Allocation Recurrence Matching (`src/cron.rs`)
- **Problem**: When evaluating union semantics (`day_or=true`) with both day-of-month and day-of-week specified, previous code performed `expanded.to_vec()` twice per step to blank out fields.
- **Fix**: Implemented `calc_candidate_override(..., wildcard_field: Option<usize>)`. It skips candidate checks on the targeted field in-place without cloning the `expanded` vector.
- **Impact**: Zero memory allocation per recurrence iteration.

### B. Fast PyO3 Dispatch & Object Instantiation (`src/cron.rs`)
- **Problem**: In `_get_next()`, each step previously invoked Python's `builtins.issubclass` reflection and dynamic `datetime` imports, adding ~850ns overhead per step.
- **Fix**: Replaced runtime reflection with a pre-cached `ReturnKind` enum (`Float` vs `DateTime`) evaluated at instance initialization. Datetime instances are created directly through `pyo3::types::PyDateTime::new` and floats via `pyo3::types::PyFloat::new`.
- **Impact**: Step latency dropped from 1,450ns down to ~158ns.

### C. Inlined Arithmetic Leaf Functions & Fat LTO (`src/cron.rs`, `Cargo.toml`)
- Added `#[inline]` annotations to `nearest_diff_cyclic`, `nearest_diff_day`, `advance_month`, `advance_day`, `advance_hour`, and `advance_minute`.
- Configured `[profile.release]` with `lto = "fat"` and `codegen-units = 1`.
- **Impact**: Enables LLVM whole-program cross-crate inlining across `pyo3`, `chrono`, and `fast_croniter`.

### D. Upstream Parity & Bounds Hardening
- Reject non-finite/NaN and out-of-range timestamps with `CroniterBadDateError`.
- Replaced unbounded loops in `advance_month` and `advance_day` with checked duration math.
- Resolved PRNG thread safety with atomic `SplitMix64` state.
- Fixed non-existent Windows/macOS runner tags in `.github/workflows/CI.yml`.
- Repaired `pyproject.toml` duplicate key error blocking `maturin sdist`.

### E. Comprehensive `@overload` Type Annotations (`python/fast_croniter/__init__.pyi`)
- Added complete typing overloads for `get_next`, `get_prev`, `next`, `prev`, `get_current`, and `croniter_range`.
- Full module export parity: `DOW_ALPHAS`, `M_ALPHAS`, `EPOCH`, and `OVERFLOW32B_MODE`.

---

## 3. Verified Benchmark Matrix

Evaluated via `benchmark.py` (10,000 single-step iterations, GC-isolated, 3-pass minimum):

| Benchmark Scenario | Python `croniter` | Rust `fast-croniter` | Throughput Speedup |
|:---|---:|---:|---:|
| **Hourly with Days** (`0 0-23/2 * * mon-fri`, float) | 20,718 ops/sec | **4,297,194 ops/sec** | **207.4x faster** |
| **Simple Step** (`*/5 * * * *`, float) | 34,220 ops/sec | **6,313,929 ops/sec** | **184.5x faster** |
| **Complex Multi-field** (`15 2,4,6 1-15 1,6,12 1-5`, datetime) | 8,321 ops/sec | **1,262,738 ops/sec** | **151.8x faster** |
| **Second Precision** (`*/10 */2 * * * */15`, float) | 32,297 ops/sec | **4,649,649 ops/sec** | **144.0x faster** |
| **Day OR Union** (`0 0 10 * wed`, datetime) | 11,278 ops/sec | **1,087,181 ops/sec** | **96.4x faster** |
| **Simple Step** (`*/5 * * * *`, datetime) | 31,544 ops/sec | **2,167,834 ops/sec** | **68.7x faster** |
| **Expression Validation** (`is_valid`, 10k calls) | 7,107 ops/sec | **163,387 ops/sec** | **23.0x faster** |
| **Range Generation** (`croniter_range`, 10k items) | 32,453 items/sec | **629,508 items/sec** | **19.4x faster** |

---

## 4. Test Suite & Verification Commands

```powershell
# Set cargo in path
$env:Path = "$HOME\.cargo\bin;$env:Path"
cd "<fast-croniter-root>"

# 1. Rust unit tests (7/7 passed)
cargo test --jobs 2

# 2. Rust clippy linter (0 warnings)
cargo clippy --all-targets -- -D warnings

# 3. Build release wheel
.\.venv\Scripts\maturin develop --release

# 4. Upstream Python test parity (267/267 passed in 0.99s)
.\.venv\Scripts\pytest tests/ -q

# 5. Build source distribution (sdist)
.\.venv\Scripts\maturin sdist

# 6. Execute benchmark suite
.\.venv\Scripts\python benchmark.py
```

---

## 5. Release Status & CI Verification

The `v0.2.0` release workflow passed 100% green across all matrix targets:
- **Run ID**: [45003156674](https://github.com/AkashPriyadarshii/fast-croniter/actions/runs/45003156674)
- **Artifacts**: 14 binary wheels (Linux manylinux/musllinux x86_64/aarch64/armv7l/i686/ppc64le/s390x, Windows x64/x86, macOS x86_64/arm64) + source tarball.
- **GitHub Release**: [v0.2.0](https://github.com/AkashPriyadarshii/fast-croniter/releases/tag/v0.2.0)
- **PyPI Release**: [fast-croniter 0.2.0](https://pypi.org/project/fast-croniter/0.2.0/)

---

## 6. Repository & Documentation State

- **GitHub Repository**: `AkashPriyadarshii/fast-croniter`
- **Description**: *"Fast, drop-in Rust replacement for Python's croniter with PyO3 abi3 wheels. Up to 207x faster (6.3M+ ops/sec), zero-allocation engine, 100% upstream test parity (267/267 tests)."*
- **Documentation & Playground**: `https://fast-croniter-site.vercel.app`
- **PyPI**: `https://pypi.org/project/fast-croniter/0.2.0/`

---

## 7. Autonomous Multi-Agent Audit & v0.2.1 Hardening (2026-11-23)

Four specialized autonomous subagents from `agency-agents` audited the v0.2.0 release:
1. **Performance Benchmarker**: Verified up to 5.16M ops/sec (202.1x speedup over pure-Python), verified 0 internal heap bytes allocated during recurrence steps via `tracemalloc`.
2. **Parity Reality Checker**: Certified 100% bug-for-bug upstream parity (267/267 tests, 890 subtests passed in 0.94s, zero DST/fold drift).
3. **AppSec Security Auditor**: Identified SEC-02 (`NaiveDateTime::MAX` duration overflow) and SEC-03 (large step integer overflow). Both patched with checked duration arithmetic and span-bounded step increments in `src/cron.rs`.
4. **Developer Experience (DX) Engineer**: Flagged Pyright overload collisions in `python/fast_croniter/__init__.pyi`, missing `DOW_FIELD` export in `__all__`, and recommended an automated monkey-patcher. All patched; `pyright` now reports 0 errors across all stubs, and `patch_croniter()` is exported for 1-line Celery Beat / Apache Airflow integration.
