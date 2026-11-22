# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-22

### Added
- Comprehensive `@overload` type stubs in `fast_croniter/__init__.pyi` for `get_next`, `get_prev`, `get_current`, and `croniter_range`.
- Full module exports parity: `DOW_ALPHAS`, `M_ALPHAS`, `EPOCH`, and dynamic `OVERFLOW32B_MODE`.
- Python 3.8 backwards compatibility with conditional `backports.zoneinfo` dependency.
- Modern 8-scenario benchmark suite in `benchmark.py` with GC isolation and warmup loops.

### Changed
- Replaced dynamic Python reflection and module imports in `_get_next()` with pre-cached `ReturnKind` enum and direct C-API `PyDateTime::new` / `PyFloat::new`.
- Implemented non-allocating `calc_candidate_override` to eliminate all heap allocations (`expanded.to_vec()`) during `day_or` matching.
- Inlined leaf arithmetic and date advancement helper functions.
- Enabled fat Link-Time Optimization (`lto = "fat"`, `codegen-units = 1`) in release builds, boosting single-step throughput to **6.31M ops/sec** (over **184x to 207x faster** than pure Python `croniter`).

### Fixed
- Hardened boundary checks on non-finite and out-of-range timestamps with `CroniterBadDateError`.
- Replaced potential overflow loops in `advance_month` and `advance_day` with checked duration math.
- Resolved race conditions in PRNG using atomic `SplitMix64` state.
- Fixed non-existent Windows and macOS runner targets in GitHub Actions CI matrix.

## [0.1.0] - 2026-11-19

### Added
- Core Rust cron parser and evaluation engine with PyO3 bindings.
- 100% test compatibility with upstream `kiorky/croniter` test fixtures (267/267 tests passing).
- Support for 5-field (standard Unix), 6-field (second precision), and 7-field (year precision) expressions.
- Native recurrence iteration methods: `get_next`, `get_prev`, `get_current`, `set_current`, `all_next`, `all_prev`, and Python iterator protocols.
- Class methods `croniter.is_valid()`, `croniter.match()`, `croniter.match_range()`, and `croniter.expand()`.
- Hashed cron expressions (`H` / `R`) and keyword aliases (`@daily`, `@midnight`, `@hourly`, etc.).
- Complete upstream exception hierarchy (`CroniterError`, `CroniterBadCronError`, `CroniterBadDateError`, `CroniterNotAlphaError`, `CroniterUnsupportedSyntaxError`, `CroniterBadTypeRangeError`).
- Generator function `croniter_range()`.
- Benchmarks demonstrating 17x to 41x speedup over pure-Python `croniter`.
- GitHub Actions CI matrix for multi-platform wheel builds (Linux, macOS, Windows).
- Vector OpenGraph banner (`assets/banner.svg`) and animated terminal preview (`assets/terminal-logo.svg`).
