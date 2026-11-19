import copy
import datetime
import math
import sys
from dateutil.tz import datetime_exists, tzutc

from . import fast_croniter as _native_m
from .fast_croniter import (
    CroniterBadCronError,
    CroniterBadDateError,
    CroniterBadTypeRangeError,
    CroniterError,
    CroniterIterator,
    CroniterNotAlphaError,
    CroniterUnsupportedSyntaxError,
    croniter as _NativeCroniter,
)

cron_m = sys.modules[__name__]

MINUTE_FIELD = 0
HOUR_FIELD = 1
DAY_FIELD = 2
MONTH_FIELD = 3
DOW_FIELD = 4
SECOND_FIELD = 5
YEAR_FIELD = 6

UNIX_FIELDS = (MINUTE_FIELD, HOUR_FIELD, DAY_FIELD, MONTH_FIELD, DOW_FIELD)
SECOND_FIELDS = (MINUTE_FIELD, HOUR_FIELD, DAY_FIELD, MONTH_FIELD, DOW_FIELD, SECOND_FIELD)
YEAR_FIELDS = (
    MINUTE_FIELD,
    HOUR_FIELD,
    DAY_FIELD,
    MONTH_FIELD,
    DOW_FIELD,
    SECOND_FIELD,
    YEAR_FIELD,
)

OVERFLOW32B_MODE = False
UTC_DT = datetime.timezone.utc
EPOCH = datetime.datetime.fromtimestamp(0, UTC_DT)

M_ALPHAS = {
    "jan": 1,
    "feb": 2,
    "mar": 3,
    "apr": 4,
    "may": 5,
    "jun": 6,
    "jul": 7,
    "aug": 8,
    "sep": 9,
    "oct": 10,
    "nov": 11,
    "dec": 12,
}
DOW_ALPHAS = {
    "sun": 0,
    "mon": 1,
    "tue": 2,
    "wed": 3,
    "thu": 4,
    "fri": 5,
    "sat": 6,
}

VALID_LEN_EXPRESSION = {5, 6, 7}
MARKER = object()


def datetime_to_timestamp(d):
    return _NativeCroniter.datetime_to_timestamp(d)


def _is_successor(date, previous_date, is_prev):
    if is_prev:
        return date.astimezone(UTC_DT) < previous_date.astimezone(UTC_DT)
    return date.astimezone(UTC_DT) > previous_date.astimezone(UTC_DT)


def _timezone_delta(date1, date2):
    offset1 = date1.utcoffset()
    offset2 = date2.utcoffset()
    assert offset1 is not None
    assert offset2 is not None
    return offset2 - offset1


def _add_tzinfo(date, previous_date, is_prev):
    localize = getattr(previous_date.tzinfo, "localize", None)
    if localize is not None:
        import pytz

        try:
            result = localize(date, is_dst=None)
        except pytz.NonExistentTimeError:
            while True:
                date += datetime.timedelta(minutes=1)
                try:
                    result = localize(date, is_dst=None)
                except pytz.NonExistentTimeError:
                    continue
                break
            return result, False
        except pytz.AmbiguousTimeError:
            candidates = [
                localize(date, is_dst=True),
                localize(date, is_dst=False),
            ]
            candidates.sort(key=lambda value: value.astimezone(UTC_DT))
            if is_prev:
                candidates.reverse()
            for candidate in candidates:
                if _is_successor(candidate, previous_date, is_prev):
                    return candidate, True
            return candidates[-1], True
        return result, True

    result = date.replace(fold=1 if is_prev else 0, tzinfo=previous_date.tzinfo)
    if not datetime_exists(result):
        while not datetime_exists(result):
            result += datetime.timedelta(minutes=1)
        return result, False

    farther = date.replace(fold=0 if is_prev else 1, tzinfo=previous_date.tzinfo)
    if result.utcoffset() != farther.utcoffset():
        if not _is_successor(result, previous_date, is_prev):
            result = farther
    return result, True


def _calc_next_tz(cron, is_prev):
    now = cron.timestamp_to_datetime(cron.cur)
    unaware_now = now.replace(tzinfo=None)
    epoch = datetime.datetime(1970, 1, 1)
    unaware_cur_ts = (unaware_now - epoch).total_seconds()
    unaware_ts = cron._calc_naive(unaware_cur_ts, is_prev)
    unaware_time = epoch + datetime.timedelta(seconds=unaware_ts)

    aware_time, exists = _add_tzinfo(unaware_time, now, is_prev)
    if not exists and (
        not _is_successor(aware_time, now, is_prev) or "*" in cron.expressions[HOUR_FIELD]
    ):
        while not exists:
            cur_ts = (unaware_time - epoch).total_seconds()
            unaware_ts = cron._calc_naive(cur_ts, is_prev)
            unaware_time = epoch + datetime.timedelta(seconds=unaware_ts)
            aware_time, exists = _add_tzinfo(unaware_time, now, is_prev)

    offset_delta = _timezone_delta(now, aware_time)
    if not offset_delta:
        return aware_time

    alternative_unaware_time = unaware_now + offset_delta
    alt_ts = (alternative_unaware_time - epoch).total_seconds()
    alt_calc_ts = cron._calc_naive(alt_ts, is_prev)
    alt_unaware = epoch + datetime.timedelta(seconds=alt_calc_ts)
    alternative_aware_time, exists = _add_tzinfo(alt_unaware, now, is_prev)

    if not exists:
        return aware_time

    if not _is_successor(alternative_aware_time, now, is_prev):
        return aware_time

    if _is_successor(aware_time, alternative_aware_time, is_prev):
        return alternative_aware_time

    return aware_time


class croniter(_NativeCroniter):
    MONTHS_IN_YEAR = 12
    RANGES = ((0, 59), (0, 23), (1, 31), (1, 12), (0, 6), (0, 59), (1970, 2099))
    ALPHACONV = (
        {},
        {},
        {"l": "l"},
        copy.deepcopy(M_ALPHAS),
        copy.deepcopy(DOW_ALPHAS),
        {},
        {},
    )
    LOWMAP = ({}, {}, {0: 1}, {0: 1}, {7: 0}, {}, {})
    LEN_MEANS_ALL = (60, 24, 31, 12, 7, 60, 130)

    def timestamp_to_datetime(self, timestamp: float, tzinfo=MARKER) -> datetime.datetime:
        if tzinfo is MARKER:
            tzinfo = self.tzinfo
        if OVERFLOW32B_MODE:
            result = EPOCH.replace(tzinfo=None) + datetime.timedelta(seconds=timestamp)
        else:
            result = datetime.datetime.fromtimestamp(timestamp, tz=tzutc()).replace(tzinfo=None)
        if tzinfo:
            result = result.replace(tzinfo=UTC_DT).astimezone(tzinfo)
        return result

    _timestamp_to_datetime = timestamp_to_datetime

    def iter(self, *args, **kwargs):
        return self.all_prev if self.is_prev else self.all_next


def croniter_range(
    start,
    stop,
    expr_format,
    ret_type=None,
    day_or=True,
    exclude_ends=False,
    _croniter=None,
    second_at_beginning=False,
    expand_from_start_time=False,
):
    _croniter = _croniter or croniter
    auto_rt = datetime.datetime
    if type(start) is not type(stop) and not (
        isinstance(start, type(stop)) or isinstance(stop, type(start))
    ):
        raise CroniterBadTypeRangeError(
            f"The start and stop must be same type. {type(start)} != {type(stop)}"
        )
    if isinstance(start, (float, int)):
        start, stop = (
            datetime.datetime.fromtimestamp(t, tzutc()).replace(tzinfo=None) for t in (start, stop)
        )
        auto_rt = float
    if ret_type is None:
        ret_type = auto_rt
    if not exclude_ends:
        ms1 = datetime.timedelta(microseconds=1)
        if start < stop:
            start -= ms1
            stop += ms1
        else:
            start += ms1
            stop -= ms1
    year_span = math.floor(abs(stop.year - start.year)) + 1
    ic = _croniter(
        expr_format,
        start,
        ret_type=datetime.datetime,
        day_or=day_or,
        max_years_between_matches=year_span,
        second_at_beginning=second_at_beginning,
        expand_from_start_time=expand_from_start_time,
    )
    if start < stop:
        def cont(v):
            return v < stop
        step = ic.get_next
    else:
        def cont(v):
            return v > stop
        step = ic.get_prev
    try:
        dt = step()
        while cont(dt):
            if ret_type is float:
                yield ic.get_current(float)
            else:
                yield dt
            dt = step()
    except CroniterBadDateError:
        return


__all__ = [
    "DAY_FIELD",
    "HOUR_FIELD",
    "MINUTE_FIELD",
    "MONTH_FIELD",
    "OVERFLOW32B_MODE",
    "SECOND_FIELD",
    "UTC_DT",
    "YEAR_FIELD",
    "CroniterBadCronError",
    "CroniterBadDateError",
    "CroniterBadTypeRangeError",
    "CroniterError",
    "CroniterNotAlphaError",
    "CroniterUnsupportedSyntaxError",
    "cron_m",
    "croniter",
    "croniter_range",
    "datetime_to_timestamp",
]
