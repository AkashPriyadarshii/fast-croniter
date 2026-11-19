import sys
import timeit
import datetime

# Upstream Python croniter
sys.path.insert(0, "upstream-croniter/src")
import croniter as py_croniter_mod
from croniter import croniter as py_croniter

# Fast Rust croniter
from fast_croniter import croniter as rust_croniter

N = 10000

print(f"=== Benchmarking croniter (pure Python) vs fast_croniter (Rust/PyO3) ===")
print(f"Iterations: {N}\n")

expressions = [
    ("Simple Step (float)", "*/5 * * * *", float),
    ("Simple Step (datetime)", "*/5 * * * *", datetime.datetime),
    ("Hourly with Days (float)", "0 0-23/2 * * mon-fri", float),
    ("Complex Expression (datetime)", "15 2,4,6 1-15 1,6,12 1-5", datetime.datetime),
    ("Second precision (float)", "*/10 */2 * * * */15", float),
]

base_dt = datetime.datetime(2024, 1, 1, 0, 0, 0)

for name, expr, ret_type in expressions:
    print(f"--- Benchmark: {name} ['{expr}'] ---")
    
    # 1. get_next loop
    def run_py():
        c = py_croniter(expr, base_dt, ret_type=ret_type)
        for _ in range(N):
            c.get_next(ret_type)

    def run_rust():
        c = rust_croniter(expr, base_dt, ret_type=ret_type)
        for _ in range(N):
            c.get_next(ret_type)

    t_py = timeit.timeit(run_py, number=1)
    t_rust = timeit.timeit(run_rust, number=1)
    speedup = t_py / t_rust if t_rust > 0 else float("inf")

    print(f"  Python croniter: {t_py:.4f}s ({N/t_py:,.0f} ops/sec)")
    print(f"  Rust fast-croniter: {t_rust:.4f}s ({N/t_rust:,.0f} ops/sec)")
    print(f"  Speedup: {speedup:.2f}x faster\n")
# Benchmark parsing / is_valid
print(f"--- Benchmark: Expression Parsing & Validation (is_valid) [{N} calls] ---")
test_expr = "15 2,4,6 1-15 1,6,12 mon-fri"
t_py_val = timeit.timeit(lambda: py_croniter.is_valid(test_expr), number=N)
t_rust_val = timeit.timeit(lambda: rust_croniter.is_valid(test_expr), number=N)
speedup_val = t_py_val / t_rust_val if t_rust_val > 0 else float("inf")
print(f"  Python croniter.is_valid: {t_py_val:.4f}s ({N/t_py_val:,.0f} ops/sec)")
print(f"  Rust fast_croniter.is_valid: {t_rust_val:.4f}s ({N/t_rust_val:,.0f} ops/sec)")
print(f"  Speedup: {speedup_val:.2f}x faster\n")
