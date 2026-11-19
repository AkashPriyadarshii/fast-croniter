import sys
import time
import fast_croniter
from . import base

if not hasattr(time, "tzset"):
    time.tzset = lambda: None

sys.modules["croniter"] = fast_croniter
sys.modules["croniter.croniter"] = fast_croniter
sys.modules["croniter.tests"] = sys.modules[__name__]
sys.modules["croniter.tests.base"] = base
