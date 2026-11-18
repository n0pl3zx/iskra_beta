//! Локальное время и человекочитаемые подписи дат на языке интерфейса.

use crate::i18n;
use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Local {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
}

/// Переводит unix-время в миллисекундах в локальные часы/минуты.
pub fn local(ts_ms: i64) -> Local {
    if ts_ms <= 0 {
        return Local::default();
    }
    // FILETIME считается от 1601-01-01, unix — от 1970-01-01.
    let ft100ns = (ts_ms as u64).saturating_add(116_444_736_000_000) * 10_000;
    let ft = FILETIME {
        dwLowDateTime: (ft100ns & 0xFFFF_FFFF) as u32,
        dwHighDateTime: (ft100ns >> 32) as u32,
    };
    let mut utc: SYSTEMTIME = unsafe { std::mem::zeroed() };
    let mut loc: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe {
        if FileTimeToSystemTime(&ft, &mut utc) == 0 {
            return Local::default();
        }
        if SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut loc) == 0 {
            return Local {
                year: utc.wYear,
                month: utc.wMonth,
                day: utc.wDay,
                hour: utc.wHour,
                minute: utc.wMinute,
            };
        }
    }
    Local {
        year: loc.wYear,
        month: loc.wMonth,
        day: loc.wDay,
        hour: loc.wHour,
        minute: loc.wMinute,
    }
}

pub fn hhmm(ts_ms: i64) -> String {
    let l = local(ts_ms);
    format!("{:02}:{:02}", l.hour, l.minute)
}

fn same_day(a: &Local, b: &Local) -> bool {
    a.year == b.year && a.month == b.month && a.day == b.day
}

/// Подпись даты для разделителя в переписке.
pub fn date_label(ts_ms: i64) -> String {
    let now = crate::protocol::now_ms();
    let today = local(now);
    let day = local(ts_ms);
    if day.year == 0 {
        return String::new();
    }
    if same_day(&day, &today) {
        return i18n::s().today.into();
    }
    let yesterday = local(now - 86_400_000);
    if same_day(&day, &yesterday) {
        return i18n::s().yesterday.into();
    }
    let month = i18n::s().months[(day.month as usize).clamp(1, 12) - 1];
    if day.year == today.year {
        format!("{} {}", day.day, month)
    } else {
        format!("{} {} {}", day.day, month, day.year)
    }
}

/// Время в списке чатов.
pub fn list_time(ts_ms: i64) -> String {
    if ts_ms <= 0 {
        return String::new();
    }
    let now = crate::protocol::now_ms();
    let today = local(now);
    let day = local(ts_ms);
    if same_day(&day, &today) {
        hhmm(ts_ms)
    } else {
        let yesterday = local(now - 86_400_000);
        if same_day(&day, &yesterday) {
            i18n::s().yesterday_lower.into()
        } else if day.year == today.year {
            format!("{:02}.{:02}", day.day, day.month)
        } else {
            format!("{:02}.{:02}.{:02}", day.day, day.month, (day.year % 100) as u8)
        }
    }
}

pub fn last_seen(ts_ms: i64) -> String {
    let s = i18n::s();
    if ts_ms <= 0 {
        return s.last_seen_recently.into();
    }
    let now = crate::protocol::now_ms();
    let today = local(now);
    let day = local(ts_ms);
    if same_day(&day, &today) {
        s.last_seen_at.replace("{}", &hhmm(ts_ms))
    } else {
        let yesterday = local(now - 86_400_000);
        if same_day(&day, &yesterday) {
            s.last_seen_yesterday.into()
        } else {
            let month = s.months[(day.month as usize).clamp(1, 12) - 1];
            s.last_seen_on
                .replace("{}", &format!("{} {}", day.day, month))
        }
    }
}
