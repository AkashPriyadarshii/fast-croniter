<p align="center">
  <img src="assets/banner.svg" alt="fast-croniter Banner" width="100%">
</p>

<p align="center">
  <a href="https://github.com/AkashPriyadarshii/fast-croniter/actions/workflows/CI.yml"><img src="https://img.shields.io/github/actions/workflow/status/AkashPriyadarshii/fast-croniter/CI.yml?branch=master&style=flat-square&label=CI" alt="CI Status"></a>
  <a href="https://pypi.org/project/fast-croniter/"><img src="https://img.shields.io/pypi/v/fast-croniter?style=flat-square&color=blue" alt="PyPI Version"></a>
  <a href="https://pypi.org/project/fast-croniter/"><img src="https://img.shields.io/pypi/pyversions/fast-croniter?style=flat-square" alt="Python Versions"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-green?style=flat-square" alt="License"></a>
  <a href="https://akashpriyadarshi.is-a.dev"><img src="https://img.shields.io/badge/author-Akash%20Priyadarshi-orange?style=flat-square" alt="Author"></a>
</p>

---

A high-performance, drop-in Rust replacement for Python's `croniter` with PyO3 bindings and 100% test compatibility with upstream `kiorky/croniter`.

<p align="center">
  <img src="assets/terminal-logo.svg" alt="fast-croniter Terminal Demo" width="100%">
</p>

## Highlights

- **100% Upstream Parity**: Verified against 267/267 reference tests from `kiorky/croniter` (DST gap shifts, Lord Howe transitions, `H` / `R` hashed jitter, Vixie/ISC `implement_cron_bug`, custom timezone handling).
- **Blazing Fast**: **17x to 41x speedup** across recurrence iteration, datetime resolution, and expression validation.
- **True Drop-In**: Identical classes, methods, exceptions, and constants (`croniter`, `croniter_range`, `CroniterBadCronError`, `SECOND_FIELD`, etc.).
- **Zero Runtime Dependencies**: Precompiled native wheels for Linux, macOS, and Windows with abi3 (Python 3.8+).

---

## Performance Benchmarks

Measured over 10,000 iterations on Windows x86_64 comparing `croniter` (pure Python) vs `fast-croniter` (Rust / PyO3):

| Benchmark Scenario | Python `croniter` | Rust `fast-croniter` | Speedup |
|:---|---:|---:|---:|
| Simple Step (`*/5 * * * *`, float) | 38,748 ops/sec | **788,625 ops/sec** | **20.4x faster** |
| Simple Step (`*/5 * * * *`, datetime) | 38,693 ops/sec | **807,683 ops/sec** | **20.9x faster** |
| Hourly with Days (`0 0-23/2 * * mon-fri`) | 24,950 ops/sec | **916,271 ops/sec** | **36.7x faster** |
| Complex Expression (`15 2,4,6 1-15 1,6,12 1-5`) | 9,886 ops/sec | **375,506 ops/sec** | **38.0x faster** |
| Second Precision (`*/10 */2 * * * */15`) | 36,935 ops/sec | **726,760 ops/sec** | **19.7x faster** |
| Expression Validation (`is_valid`) | 7,580 ops/sec | **223,097 ops/sec** | **29.4x faster** |

---

## Quickstart

### 1. Installation

```bash
pip install fast-croniter
```

Or build locally with [Maturin](https://github.com/PyO3/maturin):

```bash
maturin develop --release
```

### 2. Usage

Simply replace your `croniter` import:

```python
# Drop-in replacement:
from fast_croniter import croniter, croniter_range
import datetime

base = datetime.datetime(2024, 1, 1, 0, 0)
iter = croniter("*/5 * * * *", base)

# Get next recurrence as datetime
next_dt = iter.get_next(datetime.datetime)
print(next_dt)  # 2024-01-01 00:05:00

# Get next recurrence as Unix timestamp
next_ts = iter.get_next(float)
print(next_ts)  # 1704067500.0

# Expression match test
is_match = croniter.match("*/5 * * * *", next_dt)
print(is_match)  # True

# Range generator
for dt in croniter_range(datetime.datetime(2024, 1, 1), datetime.datetime(2024, 1, 2), "0 * * * *"):
    print(dt)
```

---

## Local Development & Testing

```bash
# 1. Setup virtualenv
uv venv
.venv\Scripts\activate  # or source .venv/bin/activate on Unix
uv pip install maturin pytest dateutil

# 2. Build Rust extension
maturin develop --release

# 3. Run test suites
cargo test --jobs 2
pytest tests/

# 4. Run benchmarks
python benchmark.py
```

---

## Author

**Akash Priyadarshi** — [akashpriyadarshi.is-a.dev](https://akashpriyadarshi.is-a.dev) · [GitHub](https://github.com/AkashPriyadarshii) · [LinkedIn](https://linkedin.com/in/akashpriyadarshii) · [X](https://x.com/Akash__ydv001)

*Akash Priyadarshi, self-taught developer & open-source builder from Patna, Bihar, India. Rust systems tooling, AI agents, Android internals.*

## License

Dual-licensed under MIT or Apache-2.0.
