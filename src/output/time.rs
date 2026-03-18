//! Timestamp formatting.

use std::sync::LazyLock;
use std::time::SystemTime;

use chrono::{DateTime, Datelike, Local, Timelike};
use unicode_width::UnicodeWidthStr;


/// Every timestamp in lx needs to be rendered by a **time format**.
/// Formatting times is tricky, because how a timestamp is rendered can
/// depend on one or more of the following:
///
/// - The user's locale, for printing the month name as "Feb", or as "fév",
///   or as "2月";
/// - The current year, because certain formats will be less precise when
///   dealing with dates far in the past;
/// - The formatting style that the user asked for on the command-line.
///
/// Currently lx does not support *custom* styles, where the user enters a
/// format string in an environment variable or something. Just these four.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TimeFormat {

    /// The **default format** uses the user's locale to print month names,
    /// and specifies the timestamp down to the minute for recent times, and
    /// day for older times.
    DefaultFormat,

    /// Use the **ISO format**, which specifies the timestamp down to the
    /// minute for recent times, and day for older times. It uses a number
    /// for the month so it doesn't use the locale.
    ISOFormat,

    /// Use the **long ISO format**, which specifies the timestamp down to the
    /// minute using only numbers, without needing the locale or year.
    LongISO,

    /// Use the **full ISO format**, which specifies the timestamp down to the
    /// millisecond and includes its offset down to the minute. This too uses
    /// only numbers so doesn't require any special consideration.
    FullISO,
}

impl TimeFormat {
    pub fn format(self, time: SystemTime) -> String {
        let dt: DateTime<Local> = time.into();
        match self {
            Self::DefaultFormat => default(dt),
            Self::ISOFormat     => iso(dt),
            Self::LongISO       => long(dt),
            Self::FullISO       => full(dt),
        }
    }
}


fn default(date: DateTime<Local>) -> String {
    let month_name = LOCALE.short_month_name(date.month0() as usize);

    if is_recent(&date) {
        match *MAXIMUM_MONTH_WIDTH {
            4 => format!("{:>2} {:<4} {:02}:{:02}",
                         date.day(), month_name,
                         date.hour(), date.minute()),
            5 => format!("{:>2} {:<5} {:02}:{:02}",
                         date.day(), month_name,
                         date.hour(), date.minute()),
            _ => format!("{:>2} {} {:02}:{:02}",
                         date.day(), month_name,
                         date.hour(), date.minute()),
        }
    } else {
        match *MAXIMUM_MONTH_WIDTH {
            4 => format!("{:>2} {:<4} {:>5}",
                         date.day(), month_name, date.year()),
            5 => format!("{:>2} {:<5} {:>5}",
                         date.day(), month_name, date.year()),
            _ => format!("{:>2} {} {:>5}",
                         date.day(), month_name, date.year()),
        }
    }
}

fn long(date: DateTime<Local>) -> String {
    format!("{:04}-{:02}-{:02} {:02}:{:02}",
            date.year(), date.month(), date.day(),
            date.hour(), date.minute())
}

fn full(date: DateTime<Local>) -> String {
    let offset = date.offset().local_minus_utc();
    let offset_hours = offset / 3600;
    let offset_minutes = (offset % 3600).abs() / 60;
    let nanos = date.timestamp_subsec_nanos();
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09} {:+03}{:02}",
            date.year(), date.month(), date.day(),
            date.hour(), date.minute(), date.second(), nanos,
            offset_hours, offset_minutes)
}

fn iso(date: DateTime<Local>) -> String {
    if is_recent(&date) {
        format!("{:02}-{:02} {:02}:{:02}",
                date.month(), date.day(),
                date.hour(), date.minute())
    }
    else {
        format!("{:04}-{:02}-{:02}",
                date.year(), date.month(), date.day())
    }
}

fn is_recent(date: &DateTime<Local>) -> bool {
    date.year() == *CURRENT_YEAR
}


static CURRENT_YEAR: LazyLock<i32> = LazyLock::new(|| Local::now().year());

static LOCALE: LazyLock<locale::Time> = LazyLock::new(|| {
    locale::Time::load_user_locale()
        .unwrap_or_else(|_| locale::Time::english())
});

static MAXIMUM_MONTH_WIDTH: LazyLock<usize> = LazyLock::new(|| {
    // Some locales use a three-character wide month name (Jan to Dec);
    // others vary between three to four (1月 to 12月, juil.). We check each month width
    // to detect the longest and set the output format accordingly.
    let mut maximum_month_width = 0;
    for i in 0..12 {
        let current_month_width = UnicodeWidthStr::width(&*LOCALE.short_month_name(i));
        maximum_month_width = std::cmp::max(maximum_month_width, current_month_width);
    }
    maximum_month_width
});
