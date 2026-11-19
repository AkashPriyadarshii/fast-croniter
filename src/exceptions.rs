use pyo3::create_exception;
use pyo3::exceptions::{PyTypeError, PyValueError};

create_exception!(fast_croniter, CroniterError, PyValueError);
create_exception!(fast_croniter, CroniterBadTypeRangeError, PyTypeError);
create_exception!(fast_croniter, CroniterBadCronError, CroniterError);
create_exception!(fast_croniter, CroniterUnsupportedSyntaxError, CroniterBadCronError);
create_exception!(fast_croniter, CroniterBadDateError, CroniterError);
create_exception!(fast_croniter, CroniterNotAlphaError, CroniterBadCronError);
