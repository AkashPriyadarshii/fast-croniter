import datetime
from typing import (
    Any,
    Generator,
    Iterator,
    Optional,
    Set,
    Tuple,
    Type,
    TypeVar,
    Union,
)

_T = TypeVar("_T", float, datetime.datetime)

MINUTE_FIELD: int
HOUR_FIELD: int
DAY_FIELD: int
MONTH_FIELD: int
DOW_FIELD: int
SECOND_FIELD: int
YEAR_FIELD: int

UNIX_FIELDS: Tuple[int, ...]
SECOND_FIELDS: Tuple[int, ...]
YEAR_FIELDS: Tuple[int, ...]
VALID_LEN_EXPRESSION: Set[int]

UTC_DT: datetime.timezone
EPOCH: datetime.datetime
OVERFLOW32B_MODE: bool

cron_m: Any

class CroniterError(ValueError): ...
class CroniterBadTypeRangeError(TypeError): ...
class CroniterBadCronError(CroniterError): ...
class CroniterUnsupportedSyntaxError(CroniterBadCronError): ...
class CroniterBadDateError(CroniterError): ...
class CroniterNotAlphaError(CroniterBadCronError): ...

class CroniterIterator(Iterator[_T]):
    def __iter__(self) -> CroniterIterator[_T]: ...
    def __next__(self) -> _T: ...

class croniter:
    MONTHS_IN_YEAR: int
    RANGES: Tuple[Tuple[int, int], ...]
    ALPHACONV: Tuple[dict[str, Union[int, str]], ...]
    LOWMAP: Tuple[dict[int, int], ...]
    LEN_MEANS_ALL: Tuple[int, ...]

    expressions: list[str]
    expanded: list[list[Union[int, str]]]
    nth_weekday_of_month: dict[int, set[Union[int, str]]]
    nearest_weekday: set[int]
    fields: Tuple[int, ...]
    cur: float
    start_time: float
    dst_start_time: float
    day_or: bool
    second_at_beginning: bool
    is_prev: bool
    ret_type: type
    tzinfo: Optional[datetime.tzinfo]

    def __init__(
        self,
        expr_format: str,
        start_time: Optional[Union[float, int, datetime.datetime]] = None,
        ret_type: Optional[Type[_T]] = None,
        day_or: bool = True,
        max_years_between_matches: Optional[int] = None,
        is_prev: bool = False,
        hash_id: Optional[Union[str, bytes]] = None,
        implement_cron_bug: bool = False,
        second_at_beginning: bool = False,
        expand_from_start_time: bool = False,
    ) -> None: ...

    def get_next(
        self,
        ret_type: Optional[Type[_T]] = None,
        start_time: Optional[Union[float, int, datetime.datetime]] = None,
        update_current: bool = True,
    ) -> _T: ...

    def get_prev(
        self,
        ret_type: Optional[Type[_T]] = None,
        start_time: Optional[Union[float, int, datetime.datetime]] = None,
        update_current: bool = True,
    ) -> _T: ...

    def next(
        self,
        ret_type: Optional[Type[_T]] = None,
        start_time: Optional[Union[float, int, datetime.datetime]] = None,
    ) -> _T: ...

    def prev(
        self,
        ret_type: Optional[Type[_T]] = None,
        start_time: Optional[Union[float, int, datetime.datetime]] = None,
    ) -> _T: ...

    def get_current(self, ret_type: Optional[Type[_T]] = None) -> _T: ...
    def set_current(
        self,
        start_time: Optional[Union[float, int, datetime.datetime]] = None,
        force: bool = True,
    ) -> float: ...

    def all_next(
        self,
        ret_type: Optional[Type[_T]] = None,
        start_time: Optional[Union[float, int, datetime.datetime]] = None,
        update_current: bool = True,
    ) -> CroniterIterator[_T]: ...

    def all_prev(
        self,
        ret_type: Optional[Type[_T]] = None,
        start_time: Optional[Union[float, int, datetime.datetime]] = None,
        update_current: bool = True,
    ) -> CroniterIterator[_T]: ...

    def iter(self, *args: Any, **kwargs: Any) -> Any: ...
    def __iter__(self) -> croniter: ...
    def __next__(self) -> Any: ...

    @staticmethod
    def datetime_to_timestamp(d: datetime.datetime) -> float: ...
    def timestamp_to_datetime(
        self, timestamp: float, tzinfo: Optional[datetime.tzinfo] = ...
    ) -> datetime.datetime: ...

    @classmethod
    def is_valid(
        cls,
        expression: str,
        hash_id: Optional[Union[str, bytes]] = None,
        encoding: str = "UTF-8",
        second_at_beginning: bool = False,
        strict: bool = False,
        strict_year: Optional[Union[int, list[int]]] = None,
    ) -> bool: ...

    @classmethod
    def match(
        cls,
        expression: str,
        testdate: Union[float, int, datetime.datetime],
        day_or: bool = True,
        second_at_beginning: bool = False,
        precision_in_seconds: Optional[float] = None,
    ) -> bool: ...

    @classmethod
    def match_range(
        cls,
        expression: str,
        from_datetime: Union[float, int, datetime.datetime],
        to_datetime: Union[float, int, datetime.datetime],
        day_or: bool = True,
        second_at_beginning: bool = False,
        precision_in_seconds: Optional[float] = None,
    ) -> bool: ...

    @classmethod
    def expand(
        cls,
        expression: str,
        hash_id: Optional[Union[str, bytes]] = None,
        second_at_beginning: bool = False,
        from_timestamp: Optional[float] = None,
        from_timestamp_tz: Optional[datetime.tzinfo] = None,
        strict: bool = False,
        strict_year: Optional[Union[int, list[int]]] = None,
    ) -> Tuple[list[list[Union[int, str]]], dict[int, set[Union[int, str]]]]: ...

def datetime_to_timestamp(d: datetime.datetime) -> float: ...

def croniter_range(
    start: Union[float, int, datetime.datetime],
    stop: Union[float, int, datetime.datetime],
    expr_format: str,
    ret_type: Optional[Type[_T]] = None,
    day_or: bool = True,
    exclude_ends: bool = False,
    _croniter: Optional[Type[croniter]] = None,
    second_at_beginning: bool = False,
    expand_from_start_time: bool = False,
) -> Generator[_T, None, None]: ...
