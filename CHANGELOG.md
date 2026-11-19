# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
