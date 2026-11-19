from datetime import datetime, tzinfo
from typing import Any, Callable, Generator, Iterator, List, Optional, Type, TypeVar, Union

T = TypeVar("T", float, datetime)

MINUTE_FIELD: int
HOUR_FIELD: int
DAY_FIELD: int
MONTH_FIELD: int
DOW_FIELD: int
SECOND_FIELD: int
YEAR_FIELD: int
VALID_LEN_EXPRESSION: List[int]
UTC_DT: tzinfo
OVERFLOW32B_MODE: bool

class CroniterError(ValueError): ...
class CroniterBadCronError(CroniterError): ...
class CroniterBadDateError(CroniterError): ...
class CroniterNotAlphaError(CroniterError): ...
class CroniterUnsupportedSyntaxError(CroniterError): ...
class CroniterBadTypeRangeError(CroniterError): ...

class croniter:
    expr: str
    cur: float
    start_time: float
    tzinfo: Optional[tzinfo]

    def __init__(
        self,
        expr_format: str,
        start_time: Optional[Union[float, int, datetime]] = None,
        ret_type: Optional[Type[T]] = None,
        day_or: Optional[bool] = True,
        max_years_between_matches: Optional[int] = None,
        is_prev: Optional[bool] = False,
        hash_id: Optional[Union[str, bytes]] = None,
        implement_cron_bug: Optional[bool] = False,
        second_at_beginning: Optional[bool] = None,
        expand_from_start_time: Optional[bool] = False,
    ) -> None: ...

    def get_next(self, ret_type: Optional[Type[T]] = None) -> T: ...
    def get_prev(self, ret_type: Optional[Type[T]] = None) -> T: ...
    def get_current(self, ret_type: Optional[Type[T]] = None) -> T: ...
    def set_current(self, start_time: Union[float, int, datetime], force: bool = True) -> float: ...
    def all_next(self, ret_type: Optional[Type[T]] = None) -> Generator[T, None, None]: ...
    def all_prev(self, ret_type: Optional[Type[T]] = None) -> Generator[T, None, None]: ...
    def iter(self, ret_type: Optional[Type[T]] = None) -> Iterator[T]: ...
    def __iter__(self) -> Iterator[Any]: ...
    def __next__(self) -> Any: ...

    @classmethod
    def is_valid(
        cls,
        expression: str,
        hash_id: Optional[Union[str, bytes]] = None,
        encoding: str = "UTF-8",
        second_at_beginning: Optional[bool] = None,
    ) -> bool: ...

    @classmethod
    def match(
        cls,
        expression: str,
        dt: datetime,
        second_at_beginning: Optional[bool] = None,
    ) -> bool: ...

def croniter_range(
    start: Union[float, datetime],
    stop: Union[float, datetime],
    expr_format: str,
    ret_type: Optional[Type[T]] = None,
    day_or: bool = True,
    exclude_ends: bool = False,
    _is_prev: bool = False,
    second_at_beginning: Optional[bool] = None,
    expand_from_start_time: bool = False,
) -> Generator[T, None, None]: ...
