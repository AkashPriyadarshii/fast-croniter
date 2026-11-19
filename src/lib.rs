pub mod cron;
pub mod exceptions;

use pyo3::prelude::*;

#[pymodule]
mod fast_croniter {

    #[pymodule_export]
    use super::cron::Croniter as croniter;

    #[pymodule_export]
    use super::cron::CroniterIterator;

    #[pymodule_export]
    use super::exceptions::CroniterBadCronError;

    #[pymodule_export]
    use super::exceptions::CroniterBadDateError;

    #[pymodule_export]
    use super::exceptions::CroniterBadTypeRangeError;

    #[pymodule_export]
    use super::exceptions::CroniterError;

    #[pymodule_export]
    use super::exceptions::CroniterNotAlphaError;

    #[pymodule_export]
    use super::exceptions::CroniterUnsupportedSyntaxError;
}

