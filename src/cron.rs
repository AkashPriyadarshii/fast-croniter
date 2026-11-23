#![allow(clippy::too_many_arguments, clippy::collapsible_if)]

use std::collections::{HashMap, HashSet};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyInt, PyList, PySet, PyTuple};

use crate::exceptions::{
    CroniterBadCronError, CroniterBadDateError, CroniterError,
    CroniterNotAlphaError, CroniterUnsupportedSyntaxError,
};

pub type PyObject = Py<PyAny>;

pub const MINUTE_FIELD: usize = 0;
pub const HOUR_FIELD: usize = 1;
pub const DAY_FIELD: usize = 2;
pub const MONTH_FIELD: usize = 3;
pub const DOW_FIELD: usize = 4;
pub const SECOND_FIELD: usize = 5;
pub const YEAR_FIELD: usize = 6;

pub const UNIX_FIELDS: [usize; 5] = [0, 1, 2, 3, 4];
pub const SECOND_FIELDS: [usize; 6] = [0, 1, 2, 3, 4, 5];
pub const YEAR_FIELDS: [usize; 7] = [0, 1, 2, 3, 4, 5, 6];

pub const RANGES: [(u32, u32); 7] = [
    (0, 59),
    (0, 23),
    (1, 31),
    (1, 12),
    (0, 6),
    (0, 59),
    (1970, 2099),
];

pub const LEN_MEANS_ALL: [usize; 7] = [60, 24, 31, 12, 7, 60, 130];
pub const DAYS_IN_MONTH: [u32; 13] = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpandedVal {
    All,
    Num(u32),
    Last, // 'l'
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpandedField {
    All,
    Values(Vec<ExpandedVal>),
}

impl ExpandedField {
    pub fn is_all(&self) -> bool {
        match self {
            ExpandedField::All => true,
            ExpandedField::Values(vals) => vals.len() == 1 && matches!(vals[0], ExpandedVal::All),
        }
    }

    pub fn to_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        match self {
            ExpandedField::All => {
                let list = PyList::new(py, ["*"])?;
                Ok(list.into())
            }
            ExpandedField::Values(vals) => {
                let items: Vec<PyObject> = vals
                    .iter()
                    .map(|v| match v {
                        ExpandedVal::All => "*".into_pyobject(py).unwrap().into_any().unbind(),
                        ExpandedVal::Num(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
                        ExpandedVal::Last => "l".into_pyobject(py).unwrap().into_any().unbind(),
                    })
                    .collect();
                let list = PyList::new(py, items)?;
                Ok(list.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NthValue {
    Nth(u32),
    Last,
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 400 == 0) || (year % 4 == 0 && year % 100 != 0)
}

pub fn last_day_of_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

pub fn day_of_week(year: i32, month: u32, day: u32) -> u32 {
    // 0 = Sunday, 1 = Monday, ..., 6 = Saturday
    if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
        match date.weekday() {
            chrono::Weekday::Sun => 0,
            chrono::Weekday::Mon => 1,
            chrono::Weekday::Tue => 2,
            chrono::Weekday::Wed => 3,
            chrono::Weekday::Thu => 4,
            chrono::Weekday::Fri => 5,
            chrono::Weekday::Sat => 6,
        }
    } else {
        0
    }
}

pub fn get_nth_weekday_of_month(year: i32, month: u32, dow: u32) -> Vec<u32> {
    let days = last_day_of_month(year, month);
    let mut res = Vec::new();
    for d in 1..=days {
        if day_of_week(year, month, d) == dow {
            res.push(d);
        }
    }
    res
}

pub fn get_nearest_weekday(year: i32, month: u32, day: u32) -> u32 {
    let last_day = last_day_of_month(year, month);
    let day = day.min(last_day);
    let dow = day_of_week(year, month, day);
    // python weekday: 0=Mon, 4=Fri, 5=Sat, 6=Sun
    let py_w = match dow {
        0 => 6,
        n => n - 1,
    };
    if py_w < 5 {
        day
    } else if py_w == 5 {
        if day > 1 {
            day - 1
        } else {
            day + 2
        }
    } else if day < last_day {
        day + 1
    } else {
        day - 2
    }
}

pub fn get_low_from_current_date_number(field_index: usize, step: u32, dt: NaiveDateTime) -> u32 {
    match field_index {
        MINUTE_FIELD => dt.minute() % step,
        HOUR_FIELD => dt.hour() % step,
        DAY_FIELD => ((dt.day() - 1) % step) + 1,
        MONTH_FIELD => ((dt.month() - 1) % step) + 1,
        DOW_FIELD => {
            let iso = dt.weekday().number_from_monday(); // Mon=1..Sun=7
            (iso % 7) % step
        }
        SECOND_FIELD => dt.second() % step,
        YEAR_FIELD => {
            let year_start = RANGES[YEAR_FIELD].0 as i32;
            let diff = (dt.year() - year_start).rem_euclid(step as i32);
            (year_start + diff) as u32
        }
        _ => 0,
    }
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB88320 & mask);
        }
    }
    !crc
}

fn month_alpha(s: &str) -> Option<u32> {
    match s.to_ascii_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

fn dow_alpha(s: &str) -> Option<u32> {
    match s.to_ascii_lowercase().as_str() {
        "sun" => Some(0),
        "mon" => Some(1),
        "tue" => Some(2),
        "wed" => Some(3),
        "thu" => Some(4),
        "fri" => Some(5),
        "sat" => Some(6),
        _ => None,
    }
}

fn alphaconv(field_index: usize, key: &str, expr_format: &str) -> Result<u32, PyErr> {
    if field_index == MONTH_FIELD {
        if let Some(m) = month_alpha(key) {
            return Ok(m);
        }
    } else if field_index == DOW_FIELD {
        if let Some(d) = dow_alpha(key) {
            return Ok(d);
        }
    }
    Err(CroniterNotAlphaError::new_err(format!(
        "[{}] is not acceptable",
        expr_format
    )))
}

fn map_value_alias(field_index: usize, val: u32) -> u32 {
    if field_index == DOW_FIELD && val == 7 {
        0
    } else {
        val
    }
}

#[derive(Debug, Clone)]
pub struct ParsedCron {
    pub expressions: Vec<String>,
    pub expanded: Vec<ExpandedField>,
    pub nth_weekday_of_month: HashMap<u32, Vec<NthValue>>,
    pub nearest_weekday: Vec<u32>,
}

pub fn expand_expression(
    expr_format: &str,
    hash_id: Option<&[u8]>,
    second_at_beginning: bool,
    from_dt: Option<NaiveDateTime>,
    strict: bool,
    strict_years: Option<Vec<i32>>,
) -> Result<ParsedCron, PyErr> {
    let mut efl = expr_format.to_ascii_lowercase();

    let aliases = [
        ("@midnight", "0 0 * * *", "h h(0-2) * * * h"),
        ("@hourly", "0 * * * *", "h * * * * h"),
        ("@daily", "0 0 * * *", "h h * * * h"),
        ("@weekly", "0 0 * * 0", "h h * * h h"),
        ("@monthly", "0 0 1 * *", "h h h * * h"),
        ("@yearly", "0 0 1 1 *", "h h h h * h"),
        ("@annually", "0 0 1 1 *", "h h h h * h"),
    ];

    for &(alias, plain, hashed) in &aliases {
        if efl == alias {
            efl = if hash_id.is_some() { hashed.to_string() } else { plain.to_string() };
            break;
        }
    }

    let mut expressions: Vec<String> = Vec::new();
    let mut cur_tok = String::new();
    for c in efl.chars() {
        if c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\x0b' || c == '\x0c' {
            if !cur_tok.is_empty() {
                expressions.push(cur_tok);
                cur_tok = String::new();
            }
        } else {
            cur_tok.push(c);
        }
    }
    if !cur_tok.is_empty() {
        expressions.push(cur_tok);
    }

    if expressions.len() < 5 || expressions.len() > 7 {
        return Err(CroniterBadCronError::new_err(
            "Exactly 5, 6 or 7 columns has to be specified for iterator expression.",
        ));
    }

    if expressions.len() > 5 && second_at_beginning {
        let sec = expressions.remove(0);
        expressions.insert(SECOND_FIELD, sec);
    }

    let mut expanded = Vec::new();
    let mut nth_weekday_of_month: HashMap<u32, Vec<NthValue>> = HashMap::new();
    let mut nearest_weekday: Vec<u32> = Vec::new();

    for (field_index, expr_str) in expressions.iter().enumerate() {
        let mut cur_expr = expr_str.clone();

        // 1. Expand hash or random
        cur_expr = expand_hash_or_random(field_index, &cur_expr, hash_id, from_dt.is_some(), expr_format)?;

        // 2. Check ?
        if cur_expr.contains('?') {
            if cur_expr != "?" {
                return Err(CroniterBadCronError::new_err(format!(
                    "[{}] is not acceptable. Question mark can not used with other characters",
                    expr_format
                )));
            }
            if field_index != DAY_FIELD && field_index != DOW_FIELD {
                return Err(CroniterBadCronError::new_err(format!(
                    "[{}] is not acceptable. Question mark can only used in day_of_month or day_of_week",
                    expr_format
                )));
            }
            cur_expr = "*".to_string();
        }

        // 3. Comma-separated parts
        let parts: Vec<&str> = cur_expr.split(',').collect();
        let mut res: Vec<ExpandedVal> = Vec::new();
        let mut seen_nums: HashSet<u32> = HashSet::new();

        for part in parts {
            let p = part;
            if p.is_empty() {
                continue;
            }

            // Check DOW special: <dow>#<nth> or l<dow> / L<dow>
            let mut dow_nth: Option<NthValue> = None;
            let mut cleaned_part = p;

            if field_index == DOW_FIELD {
                if let Some(hash_pos) = p.find('#') {
                    let he = &p[..hash_pos];
                    let last_str = &p[hash_pos + 1..];
                    let nth = match last_str.parse::<u32>() {
                        Ok(n) if (1..=5).contains(&n) => n,
                        _ => {
                            return Err(CroniterBadCronError::new_err(format!(
                                "[{}] is not acceptable. Invalid day_of_week value: '{}'",
                                expr_format, last_str
                            )));
                        }
                    };
                    dow_nth = Some(NthValue::Nth(nth));
                    cleaned_part = he;
                } else if p.starts_with('l') || p.starts_with('L') {
                    let last_str = &p[1..];
                    if last_str.parse::<u32>().is_ok() {
                        dow_nth = Some(NthValue::Last);
                        cleaned_part = last_str;
                    }
                }
            }

            // Check DAY special: (\d+)w or w(\d+)
            if field_index == DAY_FIELD && (cleaned_part.ends_with('w') || cleaned_part.ends_with('W') || cleaned_part.starts_with('w') || cleaned_part.starts_with('W')) {
                let num_str = cleaned_part.trim_matches(|c| c == 'w' || c == 'W');
                let w_day = match num_str.parse::<u32>() {
                    Ok(d) if (1..=31).contains(&d) => d,
                    _ => {
                        return Err(CroniterBadCronError::new_err(format!(
                            "[{}] is not acceptable, nearest weekday day value '{}' out of range",
                            expr_format, num_str
                        )));
                    }
                };
                if cur_expr.contains(',') {
                    return Err(CroniterBadCronError::new_err(format!(
                        "[{}] is not acceptable. 'W' can only be used with a single day value, not in a list or range",
                        expr_format
                    )));
                }
                nearest_weekday.push(w_day);
                res.push(ExpandedVal::Num(w_day));
                continue;
            }

            // Range or step: <low>-<high>/<step> or <low>-<high> or */<step> or <start>/<step>
            let mut norm = cleaned_part.to_string();
            if norm.starts_with("*/") {
                norm = format!("{}-{}/{}", RANGES[field_index].0, RANGES[field_index].1, &norm[2..]);
            }

            if norm.contains('/') || norm.contains('-') {
                let (range_str, step) = if let Some(slash_pos) = norm.find('/') {
                    let r = &norm[..slash_pos];
                    let s_str = &norm[slash_pos + 1..];
                    let s = match s_str.parse::<u32>() {
                        Ok(n) if n > 0 => n,
                        _ => {
                            return Err(CroniterBadCronError::new_err(format!(
                                "[{}] step '{}' in field {} is not acceptable",
                                expr_format, s_str, field_index
                            )));
                        }
                    };
                    (r, s)
                } else {
                    (norm.as_str(), 1)
                };

                let (low_str, high_str, start_with_step) = if let Some(dash_pos) = range_str.find('-') {
                    (&range_str[..dash_pos], &range_str[dash_pos + 1..], false)
                } else {
                    (range_str, "", true)
                };

                let low = if let Ok(n) = low_str.parse::<u32>() {
                    map_value_alias(field_index, n)
                } else {
                    alphaconv(field_index, low_str, expr_format)?
                };

                let high = if start_with_step {
                    RANGES[field_index].1
                } else if high_str == "l" || high_str == "L" {
                    if field_index == DAY_FIELD { 31 } else {
                        return Err(CroniterBadCronError::new_err(format!("{expr_format} is out of bands")));
                    }
                } else if let Ok(n) = high_str.parse::<u32>() {
                    map_value_alias(field_index, n)
                } else {
                    alphaconv(field_index, high_str, expr_format)?
                };

                let (min_range, max_range) = RANGES[field_index];
                if low < min_range || low > max_range || high < min_range || high > max_range {
                    return Err(CroniterBadCronError::new_err(format!("{expr_format} is out of bands")));
                }

                let field_span = max_range - min_range + 1;
                let step = step.min(field_span);

                let start_at_field_max = start_with_step && (low == high);
                let (mut final_low, final_high) = if low == high && !start_with_step {
                    (min_range, max_range)
                } else {
                    (low, high)
                };

                if let Some(dt_start) = from_dt {
                    if !start_at_field_max {
                        let phase = get_low_from_current_date_number(field_index, step, dt_start);
                        let mut p = phase;
                        if p < final_low {
                            p += (final_low - p).div_ceil(step) * step;
                        }
                        if final_low <= p && p <= final_high {
                            final_low = p;
                        }
                    }
                }

                let mut range_vals = Vec::new();
                if start_at_field_max {
                    range_vals.push(final_low);
                } else if final_low > final_high {
                    let mut curr = final_low;
                    while curr <= max_range {
                        range_vals.push(curr);
                        curr = match curr.checked_add(step) {
                            Some(next) => next,
                            None => break,
                        };
                    }
                    let to_skip = if let Some(&last_val) = range_vals.last() {
                        let already_skipped = max_range.saturating_sub(last_val);
                        let curpos = last_val.saturating_sub(min_range);
                        let len = max_range - min_range + 1;
                        if curpos + step > len && already_skipped < step {
                            step - already_skipped
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                    let mut curr2 = min_range + to_skip;
                    while curr2 <= final_high {
                        range_vals.push(curr2);
                        curr2 = match curr2.checked_add(step) {
                            Some(next) => next,
                            None => break,
                        };
                    }
                } else {
                    let mut curr = final_low;
                    while curr <= final_high {
                        range_vals.push(curr);
                        curr = match curr.checked_add(step) {
                            Some(next) => next,
                            None => break,
                        };
                    }
                }

                for v in range_vals {
                    if let Some(nth) = &dow_nth {
                        nth_weekday_of_month.entry(v).or_default().push(nth.clone());
                    }
                    if seen_nums.insert(v) {
                        res.push(ExpandedVal::Num(v));
                    }
                }
            } else {
                if cleaned_part == "*" {
                    res.push(ExpandedVal::All);
                } else if cleaned_part == "l" || cleaned_part == "L" {
                    if field_index != DAY_FIELD {
                        return Err(CroniterBadCronError::new_err(format!(
                            "[{}] is not acceptable",
                            expr_format
                        )));
                    }
                    res.push(ExpandedVal::Last);
                } else if cleaned_part.starts_with('-') {
                    return Err(CroniterBadCronError::new_err(format!(
                        "[{}] is not acceptable, negative numbers not allowed",
                        expr_format
                    )));
                } else {
                    let val = if let Ok(n) = cleaned_part.parse::<u32>() {
                        map_value_alias(field_index, n)
                    } else {
                        alphaconv(field_index, cleaned_part, expr_format)?
                    };

                    let (min_range, max_range) = RANGES[field_index];
                    if val < min_range || val > max_range {
                        return Err(CroniterBadCronError::new_err(format!(
                            "[{}] is not acceptable, out of range",
                            expr_format
                        )));
                    }

                    if let Some(nth) = &dow_nth {
                        nth_weekday_of_month.entry(val).or_default().push(nth.clone());
                    }
                    if seen_nums.insert(val) {
                        res.push(ExpandedVal::Num(val));
                    }
                }
            }
        }

        res.sort_by(|a, b| match (a, b) {
            (ExpandedVal::All, ExpandedVal::All) => std::cmp::Ordering::Equal,
            (ExpandedVal::All, _) => std::cmp::Ordering::Less,
            (_, ExpandedVal::All) => std::cmp::Ordering::Greater,
            (ExpandedVal::Num(x), ExpandedVal::Num(y)) => x.cmp(y),
            (ExpandedVal::Num(_), ExpandedVal::Last) => std::cmp::Ordering::Less,
            (ExpandedVal::Last, ExpandedVal::Num(_)) => std::cmp::Ordering::Greater,
            (ExpandedVal::Last, ExpandedVal::Last) => std::cmp::Ordering::Equal,
        });

        if res.len() == 1 && matches!(res[0], ExpandedVal::All) {
            expanded.push(ExpandedField::All);
        } else if res.len() == LEN_MEANS_ALL[field_index] {
            let day_has_star = expressions.get(DAY_FIELD).map(|s| s.contains('*')).unwrap_or(true);
            let dow_has_star = expressions.get(DOW_FIELD).map(|s| s.contains('*')).unwrap_or(true);
            if (field_index == DAY_FIELD && !dow_has_star) || (field_index == DOW_FIELD && !day_has_star) {
                expanded.push(ExpandedField::Values(res));
            } else {
                expanded.push(ExpandedField::All);
            }
        } else {
            expanded.push(ExpandedField::Values(res));
        }
    }

    if !nth_weekday_of_month.is_empty() {
        if let ExpandedField::Values(vals) = &expanded[DOW_FIELD] {
            let mut remaining: HashSet<u32> = HashSet::new();
            for v in vals {
                if let ExpandedVal::Num(n) = v {
                    if !nth_weekday_of_month.contains_key(n) {
                        remaining.insert(*n);
                    }
                }
            }
            if !remaining.is_empty() && vals.len() != LEN_MEANS_ALL[DOW_FIELD] {
                return Err(CroniterUnsupportedSyntaxError::new_err(format!(
                    "day-of-week field does not support mixing literal values and nth day of week syntax. Cron: '{}'",
                    expr_format
                )));
            }
        }
    }

    if strict {
        let days_field = &expanded[DAY_FIELD];
        let months_field = &expanded[MONTH_FIELD];
        if let (ExpandedField::Values(d_vals), ExpandedField::Values(m_vals)) = (days_field, months_field) {
            let mut int_days = Vec::new();
            for d in d_vals {
                if let ExpandedVal::Num(n) = d {
                    int_days.push(*n);
                }
            }
            let mut int_months = Vec::new();
            for m in m_vals {
                if let ExpandedVal::Num(n) = m {
                    int_months.push(*n);
                }
            }
            if !int_days.is_empty() && !int_months.is_empty() {
                let mut days_in_month = DAYS_IN_MONTH;
                if int_months.contains(&2) {
                    let has_leap_year = if let Some(years) = &strict_years {
                        years.iter().any(|&y| is_leap_year(y))
                    } else if expanded.len() > YEAR_FIELD {
                        if let ExpandedField::Values(y_vals) = &expanded[YEAR_FIELD] {
                            let int_years: Vec<i32> = y_vals.iter().filter_map(|v| match v {
                                ExpandedVal::Num(n) => Some(*n as i32),
                                _ => None,
                            }).collect();
                            if !int_years.is_empty() {
                                int_years.iter().any(|&y| is_leap_year(y))
                            } else {
                                true
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    };
                    if has_leap_year {
                        days_in_month[2] = 29;
                    }
                }
                let min_day = int_days.iter().copied().min().unwrap();
                let max_possible = int_months.iter().map(|&m| days_in_month[m as usize]).max().unwrap();
                if min_day > max_possible {
                    return Err(CroniterBadCronError::new_err(format!(
                        "[{}] is not acceptable. Day(s) {:?} can never occur in month(s) {:?}",
                        expr_format, int_days, int_months
                    )));
                }
            }
        }
    }

    Ok(ParsedCron {
        expressions,
        expanded,
        nth_weekday_of_month,
        nearest_weekday,
    })
}

fn expand_hash_or_random(
    field_index: usize,
    expr: &str,
    hash_id: Option<&[u8]>,
    has_from_dt: bool,
    _expr_format: &str,
) -> Result<String, PyErr> {
    let lower = expr.to_ascii_lowercase();
    if !lower.starts_with('h') && !lower.starts_with('r') {
        return Ok(expr.to_string());
    }

    let is_hash = lower.starts_with('h');
    let is_rand = lower.starts_with('r');
    if !is_hash && !is_rand {
        return Ok(expr.to_string());
    }

    if is_hash && hash_id.is_none() {
        return Err(CroniterBadCronError::new_err("Hashed definitions must include hash_id"));
    }

    let rest = &lower[1..];
    let (range_opt, div_opt) = if let Some(slash_idx) = rest.find('/') {
        let (r_part, div_part) = rest.split_at(slash_idx);
        (r_part, Some(&div_part[1..]))
    } else {
        (rest, None)
    };

    let (range_begin_opt, range_end_opt) = if range_opt.starts_with('(') && range_opt.ends_with(')') {
        let inner = &range_opt[1..range_opt.len() - 1];
        if let Some(dash_idx) = inner.find('-') {
            let b_str = &inner[..dash_idx];
            let e_str = &inner[dash_idx + 1..];
            let b: u32 = b_str.parse().map_err(|_| CroniterBadCronError::new_err("Bad expression"))?;
            let e: u32 = e_str.parse().map_err(|_| CroniterBadCronError::new_err("Bad expression"))?;
            (Some(b), Some(e))
        } else {
            return Ok(expr.to_string());
        }
    } else if range_opt.is_empty() {
        (None, None)
    } else {
        return Ok(expr.to_string());
    };

    let (min_range, max_range) = RANGES[field_index];
    if let (Some(b), Some(e)) = (range_begin_opt, range_end_opt) {
        if b >= e {
            return Err(CroniterBadCronError::new_err("Range end must be greater than range begin"));
        }
        let b_val = map_value_alias(field_index, b);
        let e_val = map_value_alias(field_index, e);
        if b_val < min_range || b_val > max_range || e_val < min_range || e_val > max_range {
            return Err(CroniterBadCronError::new_err(format!("[{expr}] is not acceptable, out of range")));
        }
    }

    let crc = if is_rand {
        rand_u32()
    } else if let Some(hid) = hash_id {
        crc32(hid)
    } else {
        return Err(CroniterBadCronError::new_err(format!("Hash expression requires hash_id: {expr}")));
    };

    let do_hash = |rb: u32, re: u32| -> u32 {
        ((crc >> field_index) % (re - rb + 1)) + rb
    };

    let divisor = if let Some(d_str) = div_opt {
        let d: u32 = d_str.parse().map_err(|_| CroniterBadCronError::new_err("Bad expression"))?;
        if d == 0 {
            return Err(CroniterBadCronError::new_err(format!("Bad expression: {expr}")));
        }
        Some(d)
    } else {
        None
    };

    match (range_begin_opt, range_end_opt, divisor) {
        (Some(b), Some(e), Some(d)) => {
            if has_from_dt {
                Ok(format!("{b}-{e}/{d}"))
            } else {
                let draw_end = (b + d - 1).min(e);
                let drawn = do_hash(b, draw_end);
                if drawn == e {
                    Ok(drawn.to_string())
                } else {
                    Ok(format!("{drawn}-{e}/{d}"))
                }
            }
        }
        (Some(b), Some(e), None) => {
            let drawn = do_hash(b, e);
            Ok(drawn.to_string())
        }
        (None, None, Some(d)) => {
            let b = min_range;
            let e = max_range;
            if has_from_dt {
                Ok(format!("{b}-{e}/{d}"))
            } else {
                let draw_end = (b + d - 1).min(e);
                let drawn = do_hash(b, draw_end);
                if drawn == e {
                    Ok(drawn.to_string())
                } else {
                    Ok(format!("{drawn}-{e}/{d}"))
                }
            }
        }
        (None, None, None) => {
            let drawn = do_hash(min_range, max_range);
            Ok(drawn.to_string())
        }
        _ => Ok(expr.to_string()),
    }
}

fn rand_u32() -> u32 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0x853c49e6748fea9b);
    let mut s = SEED.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
    s = (s ^ (s >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    s = (s ^ (s >> 27)).wrapping_mul(0x94d049bb133111eb);
    (s ^ (s >> 31)) as u32
}

#[inline]
fn nearest_diff_cyclic(x: i32, vals: &[ExpandedVal], range_val: i32, is_prev: bool) -> i32 {
    if vals.iter().any(|v| matches!(v, ExpandedVal::All)) {
        return 0;
    }
    if !is_prev {
        for v in vals {
            if let ExpandedVal::Num(n) = v {
                let n = *n as i32;
                if n >= x {
                    return n - x;
                }
            }
        }
        if let Some(ExpandedVal::Num(first)) = vals.first() {
            (*first as i32) - x + range_val
        } else {
            0
        }
    } else {
        for v in vals.iter().rev() {
            if let ExpandedVal::Num(n) = v {
                let n = *n as i32;
                if n <= x {
                    return n - x;
                }
            }
        }
        let mut candidate = if let Some(ExpandedVal::Num(last)) = vals.last() {
            *last as i32
        } else {
            0
        };
        for v in vals.iter().rev() {
            if let ExpandedVal::Num(n) = v {
                let n = *n as i32;
                if n <= range_val {
                    candidate = n;
                    break;
                }
            }
        }
        if candidate > range_val {
            -range_val
        } else {
            candidate - x - range_val
        }
    }
}

#[inline]
fn nearest_diff_year(x: i32, vals: &[ExpandedVal], is_prev: bool) -> Option<i32> {
    if vals.iter().any(|v| matches!(v, ExpandedVal::All)) {
        return Some(0);
    }
    if !is_prev {
        for v in vals {
            if let ExpandedVal::Num(n) = v {
                let n = *n as i32;
                if n >= x {
                    return Some(n - x);
                }
            }
        }
        None
    } else {
        for v in vals.iter().rev() {
            if let ExpandedVal::Num(n) = v {
                let n = *n as i32;
                if n <= x {
                    return Some(n - x);
                }
            }
        }
        None
    }
}

#[inline]
fn nearest_diff_day(x: i32, vals: &[ExpandedVal], range_val: i32, is_prev: bool) -> i32 {
    if vals.iter().any(|v| matches!(v, ExpandedVal::All)) {
        return 0;
    }
    if !is_prev {
        for v in vals {
            let n = match v {
                ExpandedVal::Num(n) => *n as i32,
                ExpandedVal::Last => range_val,
                ExpandedVal::All => return 0,
            };
            if n > range_val {
                continue;
            }
            if n >= x {
                return n - x;
            }
        }
        let first = match vals.first() {
            Some(ExpandedVal::Num(n)) => *n as i32,
            Some(ExpandedVal::Last) => range_val,
            _ => 1,
        };
        first - x + range_val
    } else {
        for v in vals.iter().rev() {
            if *v != ExpandedVal::Last {
                if let ExpandedVal::Num(n) = v {
                    let n = *n as i32;
                    if n <= x {
                        return n - x;
                    }
                }
            }
        }
        if vals.contains(&ExpandedVal::Last) {
            return -x;
        }
        let mut candidate = match vals.last() {
            Some(ExpandedVal::Num(n)) => *n as i32,
            Some(ExpandedVal::Last) => range_val,
            _ => 1,
        };
        for v in vals.iter().rev() {
            let c = match v {
                ExpandedVal::Num(n) => *n as i32,
                ExpandedVal::Last => range_val,
                _ => continue,
            };
            if c <= range_val {
                candidate = c;
                break;
            }
        }
        if candidate > range_val {
            -range_val
        } else {
            candidate - x - range_val
        }
    }
}

#[inline]
fn advance_month(dt: NaiveDateTime, diff: i32, is_prev: bool) -> Option<NaiveDateTime> {
    let total_months = (dt.year() as i64) * 12 + (dt.month() as i64 - 1) + (diff as i64);
    let new_year_i64 = total_months.div_euclid(12);
    if !(-262143..=262142).contains(&new_year_i64) {
        return None;
    }
    let new_year = new_year_i64 as i32;
    let new_month = (total_months.rem_euclid(12) + 1) as u32;
    if is_prev {
        let new_day = last_day_of_month(new_year, new_month);
        NaiveDate::from_ymd_opt(new_year, new_month, new_day)
            .and_then(|d| d.and_hms_opt(23, 59, 59))
    } else {
        NaiveDate::from_ymd_opt(new_year, new_month, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
    }
}

#[inline]
fn advance_day(dt: NaiveDateTime, diff: i32, is_prev: bool) -> Option<NaiveDateTime> {
    let new_date = dt.date().checked_add_signed(chrono::Duration::days(diff as i64))?;
    if is_prev {
        new_date.and_hms_opt(23, 59, 59)
    } else {
        new_date.and_hms_opt(0, 0, 0)
    }
}

#[inline]
fn advance_hour(dt: NaiveDateTime, diff: i32, is_prev: bool) -> Option<NaiveDateTime> {
    let new_dt = dt.checked_add_signed(chrono::Duration::hours(diff as i64))?;
    if is_prev {
        new_dt.with_minute(59)?.with_second(59)
    } else {
        new_dt.with_minute(0)?.with_second(0)
    }
}

#[inline]
fn advance_minute(dt: NaiveDateTime, diff: i32, is_prev: bool) -> Option<NaiveDateTime> {
    let new_dt = dt.checked_add_signed(chrono::Duration::minutes(diff as i64))?;
    if is_prev {
        new_dt.with_second(59)
    } else {
        new_dt.with_second(0)
    }
}

#[inline]
pub fn calc_candidate(
    now: NaiveDateTime,
    expanded: &[ExpandedField],
    nth_weekday_of_month: &HashMap<u32, Vec<NthValue>>,
    nearest_weekday: &[u32],
    is_prev: bool,
    max_years: i64,
) -> Result<NaiveDateTime, PyErr> {
    calc_candidate_override(now, expanded, nth_weekday_of_month, nearest_weekday, is_prev, max_years, None)
}

#[inline]
pub fn calc_candidate_override(
    now: NaiveDateTime,
    expanded: &[ExpandedField],
    nth_weekday_of_month: &HashMap<u32, Vec<NthValue>>,
    nearest_weekday: &[u32],
    is_prev: bool,
    max_years: i64,
    wildcard_field: Option<usize>,
) -> Result<NaiveDateTime, PyErr> {
    let mut dt = if is_prev {
        now.checked_sub_signed(chrono::Duration::microseconds(1))
            .ok_or_else(|| CroniterBadDateError::new_err("Date underflow while initializing candidate"))?
    } else if expanded.len() > 5 {
        now.checked_add_signed(chrono::Duration::seconds(1))
            .and_then(|d| d.with_nanosecond(0))
            .ok_or_else(|| CroniterBadDateError::new_err("Date overflow while initializing candidate"))?
    } else {
        now.checked_add_signed(chrono::Duration::minutes(1))
            .and_then(|d| d.with_second(0))
            .and_then(|d| d.with_nanosecond(0))
            .ok_or_else(|| CroniterBadDateError::new_err("Date overflow while initializing candidate"))?
    };

    if expanded.len() > 5 {
        dt = dt.with_nanosecond(0).unwrap_or(dt);
    } else {
        dt = dt.with_second(0).and_then(|d| d.with_nanosecond(0)).unwrap_or(dt);
    }

    let current_year = dt.year();

    while ((dt.year() - current_year).abs() as i64) <= max_years {
        // Proc year
        if expanded.len() == 7 && !expanded[YEAR_FIELD].is_all() && wildcard_field != Some(YEAR_FIELD) {
            if let ExpandedField::Values(vals) = &expanded[YEAR_FIELD] {
                let diff = nearest_diff_year(dt.year(), vals, is_prev);
                if diff.is_none() {
                    break;
                }
                let diff_y = diff.unwrap();
                if diff_y != 0 {
                    let new_year = dt.year() + diff_y;
                    dt = if is_prev {
                        NaiveDate::from_ymd_opt(new_year, 12, 31)
                            .and_then(|d| d.and_hms_opt(23, 59, 59))
                            .ok_or_else(|| CroniterBadDateError::new_err("Invalid date representation in year step"))?
                    } else {
                        NaiveDate::from_ymd_opt(new_year, 1, 1)
                            .and_then(|d| d.and_hms_opt(0, 0, 0))
                            .ok_or_else(|| CroniterBadDateError::new_err("Invalid date representation in year step"))?
                    };
                    continue;
                }
            }
        }

        // Proc month
        if !expanded[MONTH_FIELD].is_all() && wildcard_field != Some(MONTH_FIELD) {
            if let ExpandedField::Values(vals) = &expanded[MONTH_FIELD] {
                let diff = nearest_diff_cyclic(dt.month() as i32, vals, 12, is_prev);
                if diff != 0 {
                    dt = advance_month(dt, diff, is_prev).ok_or_else(|| {
                        CroniterBadDateError::new_err("Date overflow while advancing month")
                    })?;
                    continue;
                }
            }
        }

        // Proc day
        if !nearest_weekday.is_empty() {
            let mut candidates = Vec::new();
            for &w_day in nearest_weekday {
                let cand = get_nearest_weekday(dt.year(), dt.month(), w_day);
                if (is_prev && cand <= dt.day()) || (!is_prev && dt.day() <= cand) {
                    candidates.push(cand);
                }
            }
            if candidates.is_empty() {
                dt = if is_prev {
                    let first = NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1).ok_or_else(|| {
                        CroniterBadDateError::new_err("Invalid date representation")
                    })?;
                    let prev_last = first.checked_sub_signed(chrono::Duration::days(1)).ok_or_else(|| {
                        CroniterBadDateError::new_err("Date overflow while decrementing day")
                    })?;
                    prev_last.and_hms_opt(23, 59, 59).ok_or_else(|| {
                        CroniterBadDateError::new_err("Invalid time representation")
                    })?
                } else {
                    let days = last_day_of_month(dt.year(), dt.month());
                    let to_add = days - dt.day() + 1;
                    dt.date().checked_add_signed(chrono::Duration::days(to_add as i64)).and_then(|d| d.and_hms_opt(0, 0, 0)).ok_or_else(|| {
                        CroniterBadDateError::new_err("Date overflow while advancing day")
                    })?
                };
                continue;
            }
            candidates.sort();
            let target_day = if is_prev { *candidates.last().unwrap() } else { candidates[0] };
            let diff_day = target_day as i32 - dt.day() as i32;
            if diff_day != 0 {
                dt = advance_day(dt, diff_day, is_prev).ok_or_else(|| {
                    CroniterBadDateError::new_err("Date overflow while advancing day")
                })?;
                continue;
            }
        } else if !expanded[DAY_FIELD].is_all() && wildcard_field != Some(DAY_FIELD) {
            if let ExpandedField::Values(vals) = &expanded[DAY_FIELD] {
                let days_in_month = last_day_of_month(dt.year(), dt.month());
                let has_last = vals.contains(&ExpandedVal::Last);
                if has_last && days_in_month == dt.day() {
                    // matches last day
                } else {
                    let range_val = if is_prev {
                        let (py, pm) = if dt.month() == 1 {
                            (dt.year() - 1, 12)
                        } else {
                            (dt.year(), dt.month() - 1)
                        };
                        last_day_of_month(py, pm)
                    } else {
                        days_in_month
                    };
                    let diff = nearest_diff_day(dt.day() as i32, vals, range_val as i32, is_prev);
                    if diff != 0 {
                        dt = advance_day(dt, diff, is_prev).ok_or_else(|| {
                            CroniterBadDateError::new_err("Date overflow while advancing day")
                        })?;
                        continue;
                    }
                }
            }
        }

        // Proc DOW
        if !nth_weekday_of_month.is_empty() {
            let mut candidates = Vec::new();
            for (&wday, nths) in nth_weekday_of_month {
                let occs = get_nth_weekday_of_month(dt.year(), dt.month(), wday);
                for nth in nths {
                    let cand = match nth {
                        NthValue::Last => occs.last().copied(),
                        NthValue::Nth(n) => {
                            if (1..=occs.len() as u32).contains(n) {
                                Some(occs[(*n - 1) as usize])
                            } else {
                                None
                            }
                        }
                    };
                    if let Some(c) = cand {
                        if (is_prev && c <= dt.day()) || (!is_prev && dt.day() <= c) {
                            candidates.push(c);
                        }
                    }
                }
            }
            if candidates.is_empty() {
                dt = if is_prev {
                    let first = NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1).ok_or_else(|| {
                        CroniterBadDateError::new_err("Invalid date representation")
                    })?;
                    let prev_last = first.checked_sub_signed(chrono::Duration::days(1)).ok_or_else(|| {
                        CroniterBadDateError::new_err("Date overflow while decrementing day")
                    })?;
                    prev_last.and_hms_opt(23, 59, 59).ok_or_else(|| {
                        CroniterBadDateError::new_err("Invalid time representation")
                    })?
                } else {
                    let days = last_day_of_month(dt.year(), dt.month());
                    let to_add = days - dt.day() + 1;
                    dt.date().checked_add_signed(chrono::Duration::days(to_add as i64)).and_then(|d| d.and_hms_opt(0, 0, 0)).ok_or_else(|| {
                        CroniterBadDateError::new_err("Date overflow while advancing day")
                    })?
                };
                continue;
            }
            candidates.sort();
            let target_day = if is_prev { *candidates.last().unwrap() } else { candidates[0] };
            let diff_day = target_day as i32 - dt.day() as i32;
            if diff_day != 0 {
                dt = advance_day(dt, diff_day, is_prev).ok_or_else(|| {
                    CroniterBadDateError::new_err("Date overflow while advancing day")
                })?;
                continue;
            }
        } else if !expanded[DOW_FIELD].is_all() && wildcard_field != Some(DOW_FIELD) {
            if let ExpandedField::Values(vals) = &expanded[DOW_FIELD] {
                let cur_dow = day_of_week(dt.year(), dt.month(), dt.day());
                let diff = nearest_diff_cyclic(cur_dow as i32, vals, 7, is_prev);
                if diff != 0 {
                    dt = advance_day(dt, diff, is_prev).ok_or_else(|| {
                        CroniterBadDateError::new_err("Date overflow while advancing day")
                    })?;
                    continue;
                }
            }
        }

        // Proc hour
        if !expanded[HOUR_FIELD].is_all() && wildcard_field != Some(HOUR_FIELD) {
            if let ExpandedField::Values(vals) = &expanded[HOUR_FIELD] {
                let diff = nearest_diff_cyclic(dt.hour() as i32, vals, 24, is_prev);
                if diff != 0 {
                    dt = advance_hour(dt, diff, is_prev).ok_or_else(|| {
                        CroniterBadDateError::new_err("Date overflow while advancing hour")
                    })?;
                    continue;
                }
            }
        }

        // Proc minute
        if !expanded[MINUTE_FIELD].is_all() && wildcard_field != Some(MINUTE_FIELD) {
            if let ExpandedField::Values(vals) = &expanded[MINUTE_FIELD] {
                let diff = nearest_diff_cyclic(dt.minute() as i32, vals, 60, is_prev);
                if diff != 0 {
                    dt = advance_minute(dt, diff, is_prev).ok_or_else(|| {
                        CroniterBadDateError::new_err("Date overflow while advancing minute")
                    })?;
                    continue;
                }
            }
        }

        // Proc second
        if expanded.len() > 5 {
            if !expanded[SECOND_FIELD].is_all() && wildcard_field != Some(SECOND_FIELD) {
                if let ExpandedField::Values(vals) = &expanded[SECOND_FIELD] {
                    let diff = nearest_diff_cyclic(dt.second() as i32, vals, 60, is_prev);
                    if diff != 0 {
                        dt = dt.checked_add_signed(chrono::Duration::seconds(diff as i64)).ok_or_else(|| {
                            CroniterBadDateError::new_err("Date overflow while advancing second")
                        })?;
                        continue;
                    }
                }
            }
        } else {
            dt = dt.with_second(0).unwrap_or(dt);
        }

        return Ok(dt);
    }

    Err(CroniterBadDateError::new_err(if is_prev {
        "failed to find prev date"
    } else {
        "failed to find next date"
    }))
}

#[inline]
pub fn calc_next(
    now: NaiveDateTime,
    expanded: &[ExpandedField],
    nth_weekday_of_month: &HashMap<u32, Vec<NthValue>>,
    nearest_weekday: &[u32],
    is_prev: bool,
    day_or: bool,
    max_years: i64,
    expressions: &[String],
    implement_cron_bug: bool,
) -> Result<NaiveDateTime, PyErr> {
    if !expanded[DAY_FIELD].is_all() && !expanded[DOW_FIELD].is_all() && day_or {
        if implement_cron_bug
            && (expressions
                .get(DAY_FIELD)
                .map(|s| s.starts_with('*'))
                .unwrap_or(false)
                || expressions
                    .get(DOW_FIELD)
                    .map(|s| s.starts_with('*'))
                    .unwrap_or(false))
        {
            // Under implement_cron_bug, bypass union and do intersection
            return calc_candidate(
                now,
                expanded,
                nth_weekday_of_month,
                nearest_weekday,
                is_prev,
                max_years,
            );
        }

        let clean_split = nth_weekday_of_month.is_empty() && nearest_weekday.is_empty();

        let t1 = calc_candidate_override(
            now,
            expanded,
            nth_weekday_of_month,
            nearest_weekday,
            is_prev,
            max_years,
            Some(DOW_FIELD),
        );
        let t1_opt = match t1 {
            Ok(dt) => Some(dt),
            Err(e) => {
                if !clean_split {
                    return Err(e);
                }
                None
            }
        };

        let t2 = calc_candidate_override(
            now,
            expanded,
            nth_weekday_of_month,
            nearest_weekday,
            is_prev,
            max_years,
            Some(DAY_FIELD),
        );
        let t2_opt = match t2 {
            Ok(dt) => Some(dt),
            Err(e) => {
                if !clean_split {
                    return Err(e);
                }
                None
            }
        };

        match (t1_opt, t2_opt) {
            (Some(a), Some(b)) => {
                if is_prev {
                    Ok(if a > b { a } else { b })
                } else {
                    Ok(if a < b { a } else { b })
                }
            }
            (Some(a), None) => Ok(a),
            (None, Some(b)) => Ok(b),
            (None, None) => Err(CroniterBadDateError::new_err(if is_prev {
                "failed to find prev date"
            } else {
                "failed to find next date"
            })),
        }
    } else {
        calc_candidate(
            now,
            expanded,
            nth_weekday_of_month,
            nearest_weekday,
            is_prev,
            max_years,
        )
    }
}

#[pyclass(name = "CroniterIterator")]
pub struct CroniterIterator {
    cron: Py<Croniter>,
    ret_type: Option<PyObject>,
    is_prev: bool,
    update_current: bool,
}

#[pymethods]
impl CroniterIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<PyObject>> {
        let is_prev = slf.is_prev;
        let update_current = slf.update_current;
        let ret_type = slf.ret_type.as_ref().map(|o| o.clone_ref(py));
        let cron_cell = slf.cron.clone_ref(py);
        let mut cron_ref = cron_cell.borrow_mut(py);

        let res = if is_prev {
            cron_ref.get_prev(py, ret_type.as_ref().map(|o| o.bind(py)), None, Some(update_current))
        } else {
            cron_ref.get_next(py, ret_type.as_ref().map(|o| o.bind(py)), None, Some(update_current))
        };

        match res {
            Ok(obj) => Ok(Some(obj)),
            Err(e) => {
                if e.is_instance_of::<CroniterBadDateError>(py) {
                    if cron_ref.max_years_explicitly_set {
                        Ok(None)
                    } else {
                        Err(e)
                    }
                } else {
                    Err(e)
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnKind {
    Float,
    DateTime,
}

#[pyclass(name = "croniter", subclass)]
pub struct Croniter {
    pub expr_format: String,
    pub parsed: ParsedCron,
    pub day_or: bool,
    pub max_years_between_matches: i64,
    pub max_years_explicitly_set: bool,
    pub is_prev: bool,
    pub second_at_beginning: bool,
    pub implement_cron_bug: bool,
    pub expand_from_start_time: bool,
    pub start_time: f64,
    pub dst_start_time: f64,
    pub cur: f64,
    pub tzinfo: Option<PyObject>,
    pub ret_type: PyObject,
    pub ret_type_kind: ReturnKind,
}

#[pymethods]
impl Croniter {
    #[classattr]
    #[pyo3(name = "RANGES")]
    fn class_ranges(py: Python<'_>) -> PyResult<PyObject> {
        let mut items = Vec::with_capacity(RANGES.len());
        for &(a, b) in &RANGES {
            items.push(PyTuple::new(py, [a, b])?.into_any().unbind());
        }
        let t = PyTuple::new(py, items)?;
        Ok(t.into())
    }

    #[classattr]
    #[pyo3(name = "DAYS_IN_MONTH")]
    fn class_days_in_month(py: Python<'_>) -> PyResult<PyObject> {
        let d = PyDict::new(py);
        d.set_item(1, 31)?;
        d.set_item(2, 28)?;
        d.set_item(3, 31)?;
        d.set_item(4, 30)?;
        d.set_item(5, 31)?;
        d.set_item(6, 30)?;
        d.set_item(7, 31)?;
        d.set_item(8, 31)?;
        d.set_item(9, 30)?;
        d.set_item(10, 31)?;
        d.set_item(11, 30)?;
        d.set_item(12, 31)?;
        Ok(d.into())
    }

    #[classattr]
    const MONTHS_IN_YEAR: u32 = 12;

    #[classattr]
    #[pyo3(name = "LEN_MEANS_ALL")]
    fn class_len_means_all(py: Python<'_>) -> PyResult<PyObject> {
        let t = PyTuple::new(py, LEN_MEANS_ALL)?;
        Ok(t.into())
    }

    #[classattr]
    #[pyo3(name = "LOWMAP")]
    fn class_lowmap(py: Python<'_>) -> PyResult<PyObject> {
        let mut list = Vec::new();
        for i in 0..7 {
            let d = PyDict::new(py);
            if i == 2 || i == 3 {
                d.set_item(0, 1)?;
            } else if i == 4 {
                d.set_item(7, 0)?;
            }
            list.push(d.into_any().unbind());
        }
        let t = PyTuple::new(py, list)?;
        Ok(t.into())
    }

    #[new]
    #[pyo3(signature = (
        expr_format,
        start_time=None,
        ret_type=None,
        day_or=true,
        max_years_between_matches=None,
        is_prev=false,
        hash_id=None,
        implement_cron_bug=false,
        second_at_beginning=false,
        expand_from_start_time=false
    ))]
    pub fn new(
        py: Python<'_>,
        expr_format: &str,
        start_time: Option<&Bound<'_, PyAny>>,
        ret_type: Option<&Bound<'_, PyAny>>,
        day_or: Option<bool>,
        max_years_between_matches: Option<&Bound<'_, PyAny>>,
        is_prev: Option<bool>,
        hash_id: Option<&Bound<'_, PyAny>>,
        implement_cron_bug: Option<bool>,
        second_at_beginning: Option<bool>,
        expand_from_start_time: Option<bool>,
    ) -> PyResult<Self> {
        let implement_cron_bug = implement_cron_bug.unwrap_or(false);
        let expand_from_start_time = expand_from_start_time.unwrap_or(false);
        let day_or = day_or.unwrap_or(true);
        let is_prev = is_prev.unwrap_or(false);
        let second_at_beginning = second_at_beginning.unwrap_or(false);

        let (max_years, explicitly_set) = if let Some(obj) = max_years_between_matches {
            if obj.is_exact_instance_of::<pyo3::types::PyBool>() || !obj.is_instance_of::<PyInt>() {
                let type_name = obj.get_type().name()?.to_string();
                let repr = obj.repr()?.to_string();
                return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                    "max_years_between_matches must be an int >= 1 or None, not {} (got {})",
                    type_name, repr
                )));
            }
            let val: i64 = obj.extract()?;
            if val < 1 {
                let repr = obj.repr()?.to_string();
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "max_years_between_matches must be an int >= 1 or None (got {})",
                    repr
                )));
            }
            (val, true)
        } else {
            (50, false)
        };

        let hash_bytes: Option<Vec<u8>> = if let Some(h) = hash_id {
            if let Ok(b) = h.extract::<Vec<u8>>() {
                Some(b)
            } else if let Ok(s) = h.extract::<String>() {
                Some(s.into_bytes())
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "hash_id must be bytes or UTF-8 string",
                ));
            }
        } else {
            None
        };

        let (ret_type_obj, ret_type_kind) = if let Some(rt) = ret_type {
            let dt_type = py.get_type::<pyo3::types::PyDateTime>();
            let fl_type = py.get_type::<PyFloat>();
            let kind = if rt.is(&dt_type) {
                ReturnKind::DateTime
            } else if rt.is(&fl_type) {
                ReturnKind::Float
            } else if let Ok(py_type) = rt.cast::<pyo3::types::PyType>() {
                if py_type.is_subclass(&dt_type).unwrap_or(false) {
                    ReturnKind::DateTime
                } else {
                    ReturnKind::Float
                }
            } else {
                ReturnKind::Float
            };
            (rt.clone().unbind(), kind)
        } else {
            (py.get_type::<PyFloat>().into_any().unbind(), ReturnKind::Float)
        };

        let mut instance = Croniter {
            expr_format: expr_format.to_string(),
            parsed: ParsedCron {
                expressions: vec![],
                expanded: vec![],
                nth_weekday_of_month: HashMap::new(),
                nearest_weekday: vec![],
            },
            day_or,
            max_years_between_matches: max_years,
            max_years_explicitly_set: explicitly_set,
            is_prev,
            second_at_beginning,
            implement_cron_bug,
            expand_from_start_time,
            start_time: 0.0,
            dst_start_time: 0.0,
            cur: 0.0,
            tzinfo: None,
            ret_type: ret_type_obj,
            ret_type_kind,
        };

        if let Some(st) = start_time {
            instance.set_current(py, Some(st), Some(true))?;
        } else {
            let time_mod = py.import("time")?;
            let now_ts: f64 = time_mod.call_method0("time")?.extract()?;
            instance.start_time = now_ts;
            instance.dst_start_time = now_ts;
            instance.cur = now_ts;
        }

        let from_dt: Option<NaiveDateTime> = if instance.expand_from_start_time {
            if let Some(ref tz_obj) = instance.tzinfo {
                let dt_cls = py.import("datetime")?.getattr("datetime")?;
                let tz_utc = py.import("datetime")?.getattr("timezone")?.getattr("utc")?;
                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("tz", tz_utc)?;
                let utc_dt = dt_cls.call_method("fromtimestamp", (instance.dst_start_time,), Some(&kwargs))?;
                let aware = utc_dt.call_method1("astimezone", (tz_obj.bind(py),))?;
                let y: i32 = aware.getattr("year")?.extract()?;
                let m: u32 = aware.getattr("month")?.extract()?;
                let d: u32 = aware.getattr("day")?.extract()?;
                let h: u32 = aware.getattr("hour")?.extract()?;
                let min: u32 = aware.getattr("minute")?.extract()?;
                let s: u32 = aware.getattr("second")?.extract()?;
                NaiveDate::from_ymd_opt(y, m, d).and_then(|date| date.and_hms_opt(h, min, s))
            } else {
                let sec = instance.dst_start_time.floor() as i64;
                chrono::DateTime::from_timestamp(sec, 0).map(|dt| dt.naive_utc())
            }
        } else {
            None
        };

        instance.parsed = expand_expression(
            expr_format,
            hash_bytes.as_deref(),
            second_at_beginning,
            from_dt,
            false,
            None,
        )?;

        Ok(instance)
    }

    #[getter]
    pub fn cur(&self) -> f64 {
        self.cur
    }

    #[setter]
    pub fn set_cur(&mut self, val: f64) -> PyResult<()> {
        if !val.is_finite() {
            return Err(CroniterBadDateError::new_err("Invalid non-finite timestamp"));
        }
        self.cur = val;
        Ok(())
    }

    #[getter]
    pub fn start_time(&self) -> f64 {
        self.start_time
    }

    #[setter]
    pub fn set_start_time(&mut self, val: f64) -> PyResult<()> {
        if !val.is_finite() {
            return Err(CroniterBadDateError::new_err("Invalid non-finite timestamp"));
        }
        self.start_time = val;
        Ok(())
    }

    #[getter]
    pub fn dst_start_time(&self) -> f64 {
        self.dst_start_time
    }

    #[setter]
    pub fn set_dst_start_time(&mut self, val: f64) -> PyResult<()> {
        if !val.is_finite() {
            return Err(CroniterBadDateError::new_err("Invalid non-finite timestamp"));
        }
        self.dst_start_time = val;
        Ok(())
    }

    #[getter]
    pub fn tzinfo(&self, py: Python<'_>) -> Option<PyObject> {
        self.tzinfo.as_ref().map(|o| o.clone_ref(py))
    }

    #[setter]
    pub fn set_tzinfo(&mut self, _py: Python<'_>, val: Option<&Bound<'_, PyAny>>) {
        self.tzinfo = val.map(|v| v.clone().unbind());
    }

    #[getter]
    pub fn _max_years_between_matches(&self) -> i64 {
        self.max_years_between_matches
    }

    #[setter(_max_years_between_matches)]
    pub fn set_max_years_between_matches(&mut self, val: i64) {
        self.max_years_between_matches = val;
    }

    #[getter]
    pub fn _max_years_btw_matches_explicitly_set(&self) -> bool {
        self.max_years_explicitly_set
    }

    #[setter(_max_years_btw_matches_explicitly_set)]
    pub fn set_max_years_btw_matches_explicitly_set(&mut self, val: bool) {
        self.max_years_explicitly_set = val;
    }

    #[getter]
    pub fn _implement_cron_bug(&self) -> bool {
        self.implement_cron_bug
    }

    #[setter(_implement_cron_bug)]
    pub fn set_implement_cron_bug(&mut self, val: bool) {
        self.implement_cron_bug = val;
    }

    #[getter]
    pub fn _expand_from_start_time(&self) -> bool {
        self.expand_from_start_time
    }

    #[setter(_expand_from_start_time)]
    pub fn set_expand_from_start_time(&mut self, val: bool) {
        self.expand_from_start_time = val;
    }

    #[getter]
    pub fn _day_or(&self) -> bool {
        self.day_or
    }

    #[setter(_day_or)]
    pub fn set_day_or_internal(&mut self, val: bool) {
        self.day_or = val;
    }

    #[getter]
    pub fn day_or(&self) -> bool {
        self.day_or
    }

    #[setter]
    pub fn set_day_or(&mut self, val: bool) {
        self.day_or = val;
    }

    #[getter]
    pub fn second_at_beginning(&self) -> bool {
        self.second_at_beginning
    }

    #[setter]
    pub fn set_second_at_beginning(&mut self, val: bool) {
        self.second_at_beginning = val;
    }

    #[getter]
    pub fn _ret_type(&self, py: Python<'_>) -> PyObject {
        self.ret_type.clone_ref(py)
    }

    #[setter(_ret_type)]
    pub fn set_ret_type_internal(&mut self, _py: Python<'_>, val: &Bound<'_, PyAny>) {
        self.ret_type = val.clone().unbind();
    }

    #[getter]
    pub fn ret_type(&self, py: Python<'_>) -> PyObject {
        self.ret_type.clone_ref(py)
    }

    #[setter]
    pub fn set_ret_type(&mut self, _py: Python<'_>, val: &Bound<'_, PyAny>) {
        self.ret_type = val.clone().unbind();
    }

    #[getter]
    pub fn expanded(&self, py: Python<'_>) -> PyResult<PyObject> {
        let items: PyResult<Vec<PyObject>> = self
            .parsed
            .expanded
            .iter()
            .map(|f| f.to_py(py))
            .collect();
        let list = PyList::new(py, items?)?;
        Ok(list.into())
    }

    #[getter]
    pub fn nth_weekday_of_month(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (&wday, nths) in &self.parsed.nth_weekday_of_month {
            let set = PySet::empty(py)?;
            for nth in nths {
                match nth {
                    NthValue::Nth(n) => set.add(n)?,
                    NthValue::Last => set.add("l")?,
                }
            }
            dict.set_item(wday, set)?;
        }
        Ok(dict.into())
    }

    #[getter]
    pub fn nearest_weekday(&self, py: Python<'_>) -> PyResult<PyObject> {
        let set = PySet::empty(py)?;
        for &w in &self.parsed.nearest_weekday {
            set.add(w)?;
        }
        Ok(set.into())
    }

    #[getter]
    pub fn expressions(&self, py: Python<'_>) -> PyResult<PyObject> {
        let list = PyList::new(py, &self.parsed.expressions)?;
        Ok(list.into())
    }

    #[getter]
    pub fn fields(&self, py: Python<'_>) -> PyResult<PyObject> {
        let len = self.parsed.expanded.len();
        let indices: Vec<usize> = (0..len).collect();
        let t = PyTuple::new(py, indices)?;
        Ok(t.into())
    }

    #[pyo3(signature = (start_time=None, force=true))]
    pub fn set_current(
        &mut self,
        py: Python<'_>,
        start_time: Option<&Bound<'_, PyAny>>,
        force: Option<bool>,
    ) -> PyResult<f64> {
        let force = force.unwrap_or(true);
        if force {
            if let Some(st) = start_time {
            let datetime_cls = py.import("datetime")?.getattr("datetime")?;

            if st.is_instance(&datetime_cls)? {
                let tz = st.getattr("tzinfo")?;
                self.tzinfo = if tz.is_none() {
                    None
                } else {
                    Some(tz.into())
                };

                let ts: f64 = Self::datetime_to_timestamp(st)?;
                self.start_time = ts;
                self.dst_start_time = ts;
                self.cur = ts;
            } else if let Ok(f) = st.extract::<f64>() {
                self.tzinfo = None;
                self.start_time = f;
                self.dst_start_time = f;
                self.cur = f;
            } else if let Ok(i) = st.extract::<i64>() {
                self.tzinfo = None;
                let f = i as f64;
                self.start_time = f;
                self.dst_start_time = f;
                self.cur = f;
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "Invalid start_time type",
                ));
            }
        }
        }
        Ok(self.cur)
    }

    #[pyo3(signature = (ret_type=None))]
    pub fn get_current(
        &self,
        py: Python<'_>,
        ret_type: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let rt = ret_type.map(|t| t.clone().unbind()).unwrap_or_else(|| self.ret_type.clone_ref(py));
        let datetime_cls = py.import("datetime")?.getattr("datetime")?;
        let is_dt: bool = py.import("builtins")?.getattr("issubclass")?.call1((rt.bind(py), &datetime_cls))?.extract()?;

        if is_dt {
            self.timestamp_to_datetime(py, self.cur, None)
        } else {
            Ok(self.cur.into_pyobject(py)?.into_any().unbind())
        }
    }

    #[pyo3(signature = (ret_type=None, start_time=None, update_current=true))]
    pub fn get_next(
        &mut self,
        py: Python<'_>,
        ret_type: Option<&Bound<'_, PyAny>>,
        start_time: Option<&Bound<'_, PyAny>>,
        update_current: Option<bool>,
    ) -> PyResult<PyObject> {
        if start_time.is_some() && self.expand_from_start_time {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "start_time is not supported when using expand_from_start_time = True.",
            ));
        }
        self._get_next(py, ret_type, start_time, false, update_current.unwrap_or(true))
    }

    #[pyo3(signature = (ret_type=None, start_time=None, update_current=true))]
    pub fn get_prev(
        &mut self,
        py: Python<'_>,
        ret_type: Option<&Bound<'_, PyAny>>,
        start_time: Option<&Bound<'_, PyAny>>,
        update_current: Option<bool>,
    ) -> PyResult<PyObject> {
        if start_time.is_some() && self.expand_from_start_time {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "start_time is not supported when using expand_from_start_time = True.",
            ));
        }
        self._get_next(py, ret_type, start_time, true, update_current.unwrap_or(true))
    }

    #[pyo3(signature = (ret_type=None, start_time=None))]
    pub fn next(
        &mut self,
        py: Python<'_>,
        ret_type: Option<&Bound<'_, PyAny>>,
        start_time: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        self.get_next(py, ret_type, start_time, Some(true))
    }

    #[pyo3(signature = (ret_type=None, start_time=None))]
    pub fn prev(
        &mut self,
        py: Python<'_>,
        ret_type: Option<&Bound<'_, PyAny>>,
        start_time: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        self.get_prev(py, ret_type, start_time, Some(true))
    }

    #[pyo3(signature = (ret_type=None, start_time=None, update_current=true))]
    pub fn all_next(
        slf: Py<Self>,
        py: Python<'_>,
        ret_type: Option<&Bound<'_, PyAny>>,
        start_time: Option<&Bound<'_, PyAny>>,
        update_current: Option<bool>,
    ) -> PyResult<CroniterIterator> {
        let rt = ret_type.map(|t| t.clone().unbind());
        if let Some(st) = start_time {
            slf.borrow_mut(py).set_current(py, Some(st), Some(true))?;
        }

        Ok(CroniterIterator {
            cron: slf,
            ret_type: rt,
            is_prev: false,
            update_current: update_current.unwrap_or(true),
        })
    }

    #[pyo3(signature = (ret_type=None, start_time=None, update_current=true))]
    pub fn all_prev(
        slf: Py<Self>,
        py: Python<'_>,
        ret_type: Option<&Bound<'_, PyAny>>,
        start_time: Option<&Bound<'_, PyAny>>,
        update_current: Option<bool>,
    ) -> PyResult<CroniterIterator> {
        let rt = ret_type.map(|t| t.clone().unbind());
        if let Some(st) = start_time {
            slf.borrow_mut(py).set_current(py, Some(st), Some(true))?;
        }

        Ok(CroniterIterator {
            cron: slf,
            ret_type: rt,
            is_prev: true,
            update_current: update_current.unwrap_or(true),
        })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<PyObject> {
        let is_prev = slf.is_prev;
        slf._get_next(py, None, None, is_prev, true)
    }

    #[classmethod]
    #[pyo3(signature = (
        expression,
        hash_id=None,
        encoding="UTF-8",
        second_at_beginning=false,
        strict=false,
        strict_year=None
    ))]
    pub fn is_valid(
        _cls: &Bound<'_, pyo3::types::PyType>,
        expression: &str,
        hash_id: Option<&Bound<'_, PyAny>>,
        encoding: Option<&str>,
        second_at_beginning: Option<bool>,
        strict: Option<bool>,
        strict_year: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let _ = encoding;
        let hash_bytes: Option<Vec<u8>> = if let Some(h) = hash_id {
            if let Ok(b) = h.extract::<Vec<u8>>() {
                Some(b)
            } else if let Ok(s) = h.extract::<String>() {
                Some(s.into_bytes())
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "hash_id must be bytes or UTF-8 string",
                ));
            }
        } else {
            None
        };

        let strict_years: Option<Vec<i32>> = match strict_year {
            Some(sy) if !sy.is_none() => {
                if let Ok(val) = sy.extract::<i32>() {
                    Some(vec![val])
                } else {
                    sy.extract::<Vec<i32>>().ok()
                }
            }
            _ => None,
        };

        match expand_expression(
            expression,
            hash_bytes.as_deref(),
            second_at_beginning.unwrap_or(false),
            None,
            strict.unwrap_or(false),
            strict_years,
        ) {
            Ok(_) => Ok(true),
            Err(e) => {
                let py = _cls.py();
                if e.is_instance_of::<CroniterError>(py) {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    #[classmethod]
    #[pyo3(signature = (
        cron_expression,
        hash_id=None,
        second_at_beginning=false,
        from_timestamp=None,
        from_timestamp_tz=None,
        strict=false,
        strict_year=None
    ))]
    pub fn expand(
        _cls: &Bound<'_, pyo3::types::PyType>,
        py: Python<'_>,
        cron_expression: &str,
        hash_id: Option<&Bound<'_, PyAny>>,
        second_at_beginning: Option<bool>,
        from_timestamp: Option<f64>,
        from_timestamp_tz: Option<&Bound<'_, PyAny>>,
        strict: Option<bool>,
        strict_year: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let expression = cron_expression;
        let hash_bytes: Option<Vec<u8>> = if let Some(h) = hash_id {
            if let Ok(b) = h.extract::<Vec<u8>>() {
                Some(b)
            } else if let Ok(s) = h.extract::<String>() {
                Some(s.into_bytes())
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "hash_id must be bytes or UTF-8 string",
                ));
            }
        } else {
            None
        };

        let from_dt: Option<NaiveDateTime> = if let Some(ts) = from_timestamp {
            if let Some(tz_obj) = from_timestamp_tz {
                if !tz_obj.is_none() {
                    let dt_cls = py.import("datetime")?.getattr("datetime")?;
                    let tz_utc = py.import("datetime")?.getattr("timezone")?.getattr("utc")?;
                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("tz", tz_utc)?;
                    let utc_dt = dt_cls.call_method("fromtimestamp", (ts,), Some(&kwargs))?;
                    let aware = utc_dt.call_method1("astimezone", (tz_obj,))?;
                    let y: i32 = aware.getattr("year")?.extract()?;
                    let m: u32 = aware.getattr("month")?.extract()?;
                    let d: u32 = aware.getattr("day")?.extract()?;
                    let h: u32 = aware.getattr("hour")?.extract()?;
                    let min: u32 = aware.getattr("minute")?.extract()?;
                    let s: u32 = aware.getattr("second")?.extract()?;
                    NaiveDate::from_ymd_opt(y, m, d).and_then(|date| date.and_hms_opt(h, min, s))
                } else {
                    let sec = ts.floor() as i64;
                    chrono::DateTime::from_timestamp(sec, 0).map(|dt| dt.naive_utc())
                }
            } else {
                let sec = ts.floor() as i64;
                chrono::DateTime::from_timestamp(sec, 0).map(|dt| dt.naive_utc())
            }
        } else {
            None
        };

        let strict_years: Option<Vec<i32>> = match strict_year {
            Some(sy) if !sy.is_none() => {
                if let Ok(val) = sy.extract::<i32>() {
                    Some(vec![val])
                } else {
                    sy.extract::<Vec<i32>>().ok()
                }
            }
            _ => None,
        };

        let parsed = expand_expression(
            expression,
            hash_bytes.as_deref(),
            second_at_beginning.unwrap_or(false),
            from_dt,
            strict.unwrap_or(false),
            strict_years,
        )?;

        let expanded_items: PyResult<Vec<PyObject>> = parsed
            .expanded
            .iter()
            .map(|f| f.to_py(py))
            .collect();
        let exp_list = PyList::new(py, expanded_items?)?;

        let dict = PyDict::new(py);
        for (&wday, nths) in &parsed.nth_weekday_of_month {
            let set = PySet::empty(py)?;
            for nth in nths {
                match nth {
                    NthValue::Nth(n) => set.add(n)?,
                    NthValue::Last => set.add("l")?,
                }
            }
            dict.set_item(wday, set)?;
        }

        let res = PyTuple::new(py, [exp_list.into_any().unbind(), dict.into_any().unbind()])?;
        Ok(res.into())
    }

    #[classmethod]
    pub fn _get_nth_weekday_of_month(
        _cls: &Bound<'_, pyo3::types::PyType>,
        py: Python<'_>,
        year: i32,
        month: u32,
        day_of_week: u32,
    ) -> PyResult<PyObject> {
        let days = get_nth_weekday_of_month(year, month, day_of_week);
        let t = PyTuple::new(py, days)?;
        Ok(t.into())
    }

    #[classmethod]
    #[pyo3(name = "match", signature = (
        cron_expression,
        testdate,
        day_or=true,
        second_at_beginning=false,
        precision_in_seconds=None
    ))]
    pub fn match_fn(
        cls: &Bound<'_, pyo3::types::PyType>,
        py: Python<'_>,
        cron_expression: &str,
        testdate: &Bound<'_, PyAny>,
        day_or: Option<bool>,
        second_at_beginning: Option<bool>,
        precision_in_seconds: Option<f64>,
    ) -> PyResult<bool> {
        Self::match_range(cls, py, cron_expression, testdate, testdate, day_or, second_at_beginning, precision_in_seconds)
    }

    #[classmethod]
    #[pyo3(signature = (
        cron_expression,
        from_datetime,
        to_datetime,
        day_or=true,
        second_at_beginning=false,
        precision_in_seconds=None
    ))]
    pub fn match_range(
        _cls: &Bound<'_, pyo3::types::PyType>,
        py: Python<'_>,
        cron_expression: &str,
        from_datetime: &Bound<'_, PyAny>,
        to_datetime: &Bound<'_, PyAny>,
        day_or: Option<bool>,
        second_at_beginning: Option<bool>,
        precision_in_seconds: Option<f64>,
    ) -> PyResult<bool> {
        let expression = cron_expression;
        let dt_cls = py.import("datetime")?.getattr("datetime")?;
        let mut cron = Croniter::new(
            py,
            expression,
            Some(to_datetime),
            Some(&dt_cls),
            day_or,
            None,
            Some(false),
            None,
            None,
            second_at_beginning,
            None,
        )?;

        let tdp_obj = cron.get_current(py, Some(&dt_cls))?;
        let tdp = tdp_obj.bind(py);

        let micro: u32 = tdp.getattr("microsecond")?.extract()?;
        let tdp_adjusted = if micro == 0 {
            let td = py.import("datetime")?.getattr("timedelta")?.call1((0, 0, 1))?;
            tdp.call_method1("__add__", (td,))?
        } else {
            tdp.clone()
        };

        cron.set_current(py, Some(&tdp_adjusted), Some(true))?;

        let tdt_obj = match cron.get_prev(py, Some(&dt_cls), None, Some(true)) {
            Ok(o) => o,
            Err(_) => return Ok(false),
        };
        let tdt = tdt_obj.bind(py);

        let precision = precision_in_seconds.unwrap_or(if cron.parsed.expanded.len() > 5 {
            1.0
        } else {
            60.0
        });

        let diff_span: f64 = to_datetime
            .call_method1("__sub__", (from_datetime,))?
            .call_method0("total_seconds")?
            .extract()?;
        let duration = diff_span + precision;

        let diff_dt: f64 = if tdp_adjusted.gt(tdt)? {
            tdp_adjusted.call_method1("__sub__", (tdt,))?.call_method0("total_seconds")?.extract()?
        } else {
            tdt.call_method1("__sub__", (&tdp_adjusted,))?.call_method0("total_seconds")?.extract()?
        };

        Ok(diff_dt < duration)
    }

    #[staticmethod]
    pub fn datetime_to_timestamp(d: &Bound<'_, PyAny>) -> PyResult<f64> {
        let tz = d.getattr("tzinfo")?;
        if tz.is_none() {
            let dt_cls = d.py().import("datetime")?.getattr("datetime")?;
            let epoch = dt_cls.call1((1970, 1, 1))?;
            let diff = d.call_method1("__sub__", (epoch,))?;
            let secs: f64 = diff.call_method0("total_seconds")?.extract()?;
            Ok(secs)
        } else {
            let utcoffset = d.call_method0("utcoffset")?;
            let dt_cls = d.py().import("datetime")?.getattr("datetime")?;
            let kwargs = pyo3::types::PyDict::new(d.py());
            kwargs.set_item("tzinfo", d.py().None())?;
            let naive = d.call_method("replace", (), Some(&kwargs))?;
            let naive_sub = naive.call_method1("__sub__", (utcoffset,))?;
            let epoch = dt_cls.call1((1970, 1, 1))?;
            let diff = naive_sub.call_method1("__sub__", (epoch,))?;
            let secs: f64 = diff.call_method0("total_seconds")?.extract()?;
            Ok(secs)
        }
    }

    #[staticmethod]
    pub fn _datetime_to_timestamp(d: &Bound<'_, PyAny>) -> PyResult<f64> {
        Self::datetime_to_timestamp(d)
    }

    // Direct Rust calculation helper for timezone bridge
    pub fn _calc_naive(&self, cur_ts: f64, is_prev: bool) -> PyResult<f64> {
        if !cur_ts.is_finite() {
            return Err(CroniterBadDateError::new_err("Timestamp must be finite"));
        }
        let base_sec = cur_ts.floor() as i64;
        let usec = ((cur_ts - cur_ts.floor()) * 1e6).round() as u32;
        let (sec, nsec) = if usec >= 1_000_000 {
            (base_sec.saturating_add(1), 0)
        } else {
            (base_sec, usec * 1000)
        };
        let naive_now = chrono::DateTime::from_timestamp(sec, nsec)
            .map(|dt| dt.naive_utc())
            .ok_or_else(|| CroniterBadDateError::new_err("Timestamp out of representable date range"))?;

        let res = calc_next(
            naive_now,
            &self.parsed.expanded,
            &self.parsed.nth_weekday_of_month,
            &self.parsed.nearest_weekday,
            is_prev,
            self.day_or,
            self.max_years_between_matches,
            &self.parsed.expressions,
            self.implement_cron_bug,
        )?;

        let ts = res.and_utc().timestamp() as f64;
        let subsec = (res.and_utc().timestamp_subsec_micros() as f64) / 1e6;
        Ok(ts + subsec)
    }

    #[pyo3(signature = (timestamp, tzinfo=None))]
    pub fn timestamp_to_datetime(
        &self,
        py: Python<'_>,
        timestamp: f64,
        tzinfo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let dt_cls = py.import("datetime")?.getattr("datetime")?;
        let tz_utc = py.import("datetime")?.getattr("timezone")?.getattr("utc")?;

        let tz = if let Some(t) = tzinfo {
            if t.is_none() {
                None
            } else {
                Some(t.clone().unbind())
            }
        } else {
            self.tzinfo.as_ref().map(|o| o.clone_ref(py))
        };

        let kwargs = pyo3::types::PyDict::new(py);
        kwargs.set_item("tz", &tz_utc)?;
        let utc_dt = dt_cls.call_method("fromtimestamp", (timestamp,), Some(&kwargs))?;
        let repl_kwargs = pyo3::types::PyDict::new(py);
        repl_kwargs.set_item("tzinfo", py.None())?;
        let result = utc_dt.call_method("replace", (), Some(&repl_kwargs))?;

        if let Some(tz_obj) = tz {
            let repl_utc = pyo3::types::PyDict::new(py);
            repl_utc.set_item("tzinfo", &tz_utc)?;
            let dt_with_utc = result.call_method("replace", (), Some(&repl_utc))?;
            let aware = dt_with_utc.call_method1("astimezone", (tz_obj.bind(py),))?;
            Ok(aware.into())
        } else {
            Ok(result.into())
        }
    }

    #[pyo3(signature = (timestamp, tzinfo=None))]
    pub fn _timestamp_to_datetime(
        &self,
        py: Python<'_>,
        timestamp: f64,
        tzinfo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        self.timestamp_to_datetime(py, timestamp, tzinfo)
    }

    #[getter]
    pub fn is_prev(&self) -> bool {
        self.is_prev
    }

    #[getter]
    pub fn _is_prev(&self) -> bool {
        self.is_prev
    }

    #[setter]
    #[allow(non_snake_case)]
    pub fn set__is_prev(&mut self, val: bool) {
        self.is_prev = val;
    }

    #[setter]
    pub fn set_is_prev(&mut self, val: bool) {
        self.is_prev = val;
    }
}

impl Croniter {
    fn _get_next(
        &mut self,
        py: Python<'_>,
        ret_type: Option<&Bound<'_, PyAny>>,
        start_time: Option<&Bound<'_, PyAny>>,
        is_prev: bool,
        update_current: bool,
    ) -> PyResult<PyObject> {
        if let Some(st) = start_time {
            self.set_current(py, Some(st), Some(true))?;
        }
        self.is_prev = is_prev;

        let is_dt = match ret_type {
            None => self.ret_type_kind == ReturnKind::DateTime,
            Some(rt) => {
                let dt_type = py.get_type::<pyo3::types::PyDateTime>();
                let fl_type = py.get_type::<PyFloat>();
                if rt.is(&fl_type) {
                    false
                } else if rt.is(&dt_type) {
                    true
                } else if let Ok(py_type) = rt.cast::<pyo3::types::PyType>() {
                    let is_dt_sub = py_type.is_subclass(&dt_type).unwrap_or(false);
                    let is_fl_sub = py_type.is_subclass(&fl_type).unwrap_or(false);
                    if !is_dt_sub && !is_fl_sub {
                        return Err(pyo3::exceptions::PyTypeError::new_err(
                            "Invalid ret_type, only 'float' or 'datetime' is acceptable.",
                        ));
                    }
                    is_dt_sub
                } else {
                    return Err(pyo3::exceptions::PyTypeError::new_err(
                        "Invalid ret_type, only 'float' or 'datetime' is acceptable.",
                    ));
                }
            }
        };

        // Timezone aware path
        if self.tzinfo.is_some() {
            let fast_croniter_mod = py.import("fast_croniter")?;
            let calc_tz_fn = fast_croniter_mod.getattr("_calc_next_tz")?;
            let res_dt = calc_tz_fn.call1((self.clone_ref_py(py)?, is_prev))?;
            let ts: f64 = Self::datetime_to_timestamp(&res_dt)?;
            if update_current {
                self.cur = ts;
            }
            if is_dt {
                Ok(res_dt.into())
            } else {
                Ok(ts.into_pyobject(py)?.into_any().unbind())
            }
        } else {
            // Pure native Rust path
            if !self.cur.is_finite() {
                return Err(CroniterBadDateError::new_err("Timestamp must be finite"));
            }
            let cur_floor = self.cur.floor();
            let cur_sec = cur_floor as i64;
            let fract = self.cur - cur_floor;
            let cur_nsec = if fract == 0.0 {
                0
            } else {
                ((fract * 1e6).round() as u32) * 1000
            };
            let naive_now = chrono::DateTime::from_timestamp(cur_sec, cur_nsec)
                .map(|dt| dt.naive_utc())
                .ok_or_else(|| CroniterBadDateError::new_err("Timestamp out of representable date range"))?;

            let result_naive = calc_next(
                naive_now,
                &self.parsed.expanded,
                &self.parsed.nth_weekday_of_month,
                &self.parsed.nearest_weekday,
                is_prev,
                self.day_or,
                self.max_years_between_matches,
                &self.parsed.expressions,
                self.implement_cron_bug,
            )?;

            let ts = result_naive.and_utc().timestamp() as f64;
            if update_current {
                self.cur = ts;
            }

            if is_dt {
                let py_dt = pyo3::types::PyDateTime::new(
                    py,
                    result_naive.year(),
                    result_naive.month() as u8,
                    result_naive.day() as u8,
                    result_naive.hour() as u8,
                    result_naive.minute() as u8,
                    result_naive.second() as u8,
                    0,
                    None,
                )?;
                Ok(py_dt.into_any().unbind())
            } else {
                let py_fl = pyo3::types::PyFloat::new(py, ts);
                Ok(py_fl.into_any().unbind())
            }
        }
    }

    fn clone_ref_py(&self, py: Python<'_>) -> PyResult<Py<Croniter>> {
        Py::new(py, Croniter {
            expr_format: self.expr_format.clone(),
            parsed: self.parsed.clone(),
            day_or: self.day_or,
            max_years_between_matches: self.max_years_between_matches,
            max_years_explicitly_set: self.max_years_explicitly_set,
            is_prev: self.is_prev,
            second_at_beginning: self.second_at_beginning,
            implement_cron_bug: self.implement_cron_bug,
            expand_from_start_time: self.expand_from_start_time,
            start_time: self.start_time,
            dst_start_time: self.dst_start_time,
            cur: self.cur,
            tzinfo: self.tzinfo.as_ref().map(|o| o.clone_ref(py)),
            ret_type: self.ret_type.clone_ref(py),
            ret_type_kind: self.ret_type_kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_val() {
        assert_eq!(crc32(b"hello"), 907060870);
    }

    #[test]
    fn test_leap_years() {
        assert!(is_leap_year(1996));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
    }

    #[test]
    fn test_last_day_of_months() {
        assert_eq!(last_day_of_month(2024, 2), 29);
        assert_eq!(last_day_of_month(2023, 2), 28);
        assert_eq!(last_day_of_month(2024, 4), 30);
        assert_eq!(last_day_of_month(2024, 12), 31);
    }

    #[test]
    fn test_expand_wildcard() {
        let p = expand_expression("* * * * *", None, false, None, false, None).unwrap();
        assert_eq!(p.expanded.len(), 5);
        for f in &p.expanded {
            assert!(f.is_all());
        }
    }

    #[test]
    fn test_expand_steps() {
        let p = expand_expression("*/15 0-12/3 * * *", None, false, None, false, None).unwrap();
        match &p.expanded[0] {
            ExpandedField::Values(vals) => {
                assert_eq!(
                    vals,
                    &[
                        ExpandedVal::Num(0),
                        ExpandedVal::Num(15),
                        ExpandedVal::Num(30),
                        ExpandedVal::Num(45)
                    ]
                );
            }
            _ => panic!("Expected values for minute"),
        }
    }

    #[test]
    fn test_calc_next_simple() {
        let p = expand_expression("*/5 * * * *", None, false, None, false, None).unwrap();
        let base = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(12, 1, 0).unwrap();
        let next = calc_next(
            base,
            &p.expanded,
            &p.nth_weekday_of_month,
            &p.nearest_weekday,
            false,
            true,
            50,
            &p.expressions,
            false,
        )
        .unwrap();
        assert_eq!(next, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(12, 5, 0).unwrap());
    }

    #[test]
    fn test_calc_prev_simple() {
        let p = expand_expression("*/5 * * * *", None, false, None, false, None).unwrap();
        let base = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(12, 6, 0).unwrap();
        let prev = calc_next(
            base,
            &p.expanded,
            &p.nth_weekday_of_month,
            &p.nearest_weekday,
            true,
            true,
            50,
            &p.expressions,
            false,
        )
        .unwrap();
        assert_eq!(prev, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(12, 5, 0).unwrap());
    }
}
