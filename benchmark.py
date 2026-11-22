import datetime
import gc
import sys
import timeit

# Upstream Python croniter
sys.path.insert(0, "upstream-croniter/src")
import croniter as py_croniter_mod
from croniter import croniter as py_croniter, croniter_range as py_croniter_range

# Fast Rust croniter
import fast_croniter as rust_croniter_mod
from fast_croniter import croniter as rust_croniter, croniter_range as rust_croniter_range

N = 10000

print(f"=====================================================================")
print(f"  BENCHMARK: croniter (Pure Python) vs fast_croniter (Rust / PyO3)   ")
print(f"  Single-Step Iterations: {N} | Platform: {sys.platform}            ")
print(f"=====================================================================\n")

scenarios = [
    ("Simple Step (float)", "*/5 * * * *", float, True),
    ("Simple Step (datetime)", "*/5 * * * *", datetime.datetime, True),
    ("Hourly with Days (float)", "0 0-23/2 * * mon-fri", float, True),
    ("Day OR Union (datetime)", "0 0 10 * wed", datetime.datetime, True),
    ("Complex Multi-field (datetime)", "15 2,4,6 1-15 1,6,12 1-5", datetime.datetime, True),
    ("Second-level Precision (float)", "*/10 */2 * * * */15", float, True),
]

base_dt = datetime.datetime(2024, 1, 1, 0, 0, 0)

for name, expr, ret_type, day_or in scenarios:
    print(f"--- Benchmark: {name} ['{expr}'] ---")

    # Warmup
    c_py_warm = py_croniter(expr, base_dt, ret_type=ret_type, day_or=day_or)
    c_rust_warm = rust_croniter(expr, base_dt, ret_type=ret_type, day_or=day_or)
    for _ in range(100):
        c_py_warm.get_next(ret_type)
        c_rust_warm.get_next(ret_type)

    def run_py():
        c = py_croniter(expr, base_dt, ret_type=ret_type, day_or=day_or)
        for _ in range(N):
            c.get_next(ret_type)

    def run_rust():
        c = rust_croniter(expr, base_dt, ret_type=ret_type, day_or=day_or)
        for _ in range(N):
            c.get_next(ret_type)

    gc.disable()
    t_py = min(timeit.repeat(run_py, number=1, repeat=3))
    t_rust = min(timeit.repeat(run_rust, number=1, repeat=3))
    gc.enable()

    speedup = t_py / t_rust if t_rust > 0 else float("inf")
    print(f"  Python croniter:   {t_py*1000:7.2f} ms ({N/t_py:10,.0f} ops/sec)")
    print(f"  Rust fast-croniter:{t_rust*1000:7.2f} ms ({N/t_rust:10,.0f} ops/sec)")
    print(f"  >>> Speedup: {speedup:6.2f}x faster <<<\n")

# Benchmark Range Generation
print(f"--- Benchmark: croniter_range Generation (10,000 steps) ['*/5 * * * *'] ---")
start_range = datetime.datetime(2024, 1, 1)
stop_range = datetime.datetime(2024, 2, 5)

def run_py_range():
    list(py_croniter_range(start_range, stop_range, "*/5 * * * *", ret_type=float))

def run_rust_range():
    list(rust_croniter_range(start_range, stop_range, "*/5 * * * *", ret_type=float))

gc.disable()
t_py_rng = min(timeit.repeat(run_py_range, number=1, repeat=3))
t_rust_rng = min(timeit.repeat(run_rust_range, number=1, repeat=3))
gc.enable()

speedup_rng = t_py_rng / t_rust_rng if t_rust_rng > 0 else float("inf")
count = 10080
print(f"  Python croniter:   {t_py_rng*1000:7.2f} ms ({count/t_py_rng:10,.0f} items/sec)")
print(f"  Rust fast-croniter:{t_rust_rng*1000:7.2f} ms ({count/t_rust_rng:10,.0f} items/sec)")
print(f"  >>> Speedup: {speedup_rng:6.2f}x faster <<<\n")

# Benchmark Validation / Parsing
print(f"--- Benchmark: Expression Parsing & Validation (is_valid) [{N} calls] ---")
test_expr = "15 2,4,6 1-15 1,6,12 mon-fri"

gc.disable()
t_py_val = min(timeit.repeat(lambda: py_croniter.is_valid(test_expr), number=N, repeat=3))
t_rust_val = min(timeit.repeat(lambda: rust_croniter.is_valid(test_expr), number=N, repeat=3))
gc.enable()

speedup_val = t_py_val / t_rust_val if t_rust_val > 0 else float("inf")
print(f"  Python croniter:   {t_py_val*1000:7.2f} ms ({N/t_py_val:10,.0f} ops/sec)")
print(f"  Rust fast-croniter:{t_rust_val*1000:7.2f} ms ({N/t_rust_val:10,.0f} ops/sec)")
print(f"  >>> Speedup: {speedup_val:6.2f}x faster <<<\n")
