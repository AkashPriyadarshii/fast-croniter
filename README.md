# fast-croniter

A high-performance, drop-in Rust replacement for Python's `croniter` with PyO3 bindings and 100% test compatibility with upstream `kiorky/croniter`.

## Features

- **100% Drop-in Compatibility**: Passes 100% of upstream test suites (267/267 tests) including DST transitions, `H` / `R` hash expressions, range iteration, custom timezones (zoneinfo and pytz), and 7-field second/year expressions.
- **Blazing Fast**: **17x - 41x speedup** across parsing, matching, and iteration compared to pure-Python `croniter`.
- **Zero Configuration**: Exposes identical classes, methods, exceptions, and constants (`croniter`, `croniter_range`, `CroniterBadCronError`, etc.).

## Performance Benchmarks

Measured over 10,000 iterations comparing `croniter` (pure Python) vs `fast-croniter` (Rust / PyO3):

| Benchmark Scenario | Python `croniter` | Rust `fast-croniter` | Speedup |
|:-------------------|------------------:|---------------------:|--------:|
| Simple Step (`*/5 * * * *`, float) | 38,463 ops/sec | **748,167 ops/sec** | **19.5x** |
| Simple Step (`*/5 * * * *`, datetime) | 37,898 ops/sec | **669,452 ops/sec** | **17.7x** |
| Hourly with Days (`0 0-23/2 * * mon-fri`) | 24,417 ops/sec | **935,576 ops/sec** | **38.3x** |
| Complex Expression (`15 2,4,6 1-15 1,6,12 1-5`) | 9,574 ops/sec | **393,343 ops/sec** | **41.1x** |
| Second Precision (`*/10 */2 * * * */15`) | 34,407 ops/sec | **963,930 ops/sec** | **28.0x** |
| Expression Validation (`is_valid`) | 7,213 ops/sec | **213,848 ops/sec** | **29.7x** |

## Installation

```bash
pip install fast-croniter
```

Or build locally with Maturin:

```bash
maturin develop --release
```

## Quickstart

```python
from fast_croniter import croniter, croniter_range
import datetime

base = datetime.datetime(2024, 1, 1, 0, 0)
iter = croniter("*/5 * * * *", base)

# Get next match as datetime
next_dt = iter.get_next(datetime.datetime)
print(next_dt)  # 2024-01-01 00:05:00

# Get next match as float timestamp
next_ts = iter.get_next(float)
print(next_ts)

# Match testing
is_match = croniter.match("*/5 * * * *", next_dt)
print(is_match)  # True

# Range generator
for dt in croniter_range(datetime.datetime(2024, 1, 1), datetime.datetime(2024, 1, 2), "0 * * * *"):
    print(dt)
```

## License

MIT / Apache-2.0
