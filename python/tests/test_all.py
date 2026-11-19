import pytest
import fast_croniter


def test_sum_as_string():
    assert fast_croniter.sum_as_string(1, 1) == "2"
