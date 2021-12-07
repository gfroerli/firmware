//! RTC related helper functions.
use stm32l0xx_hal::rtc::{Datelike, NaiveDateTime, Timelike};

/// Takes a `datetime` and returns the seconds of uptime.
///
/// This could also be implemented by doing `(dt2 - dt1).num_seconds()`, but
/// that would involve 64 bit arithmetic in Chrono which does not perform well
/// on a 32 bit microcontroller. Instead, this implementation fully works with
/// 32 bit integer arithmetic, because we only need to support a certain year
/// range, and need no subsecond precision.
///
/// Note: The RTC is initialized to 2001-01-01 00:00:00.
pub fn datetime_to_uptime(dt: NaiveDateTime) -> u32 {
    let h = 3_600;
    let d = 24 * h;
    let y = 365 * d;

    // We need to get the number of full leap years since 2001. (Partial leap
    // years can be ignored because we use `.ordinal0()` which considers leap
    // years.) The first leap year after 2001 is 2004, thus we can use the
    // following calculation:
    //
    //     ceil((year - 2004) / 4)
    //
    // Unfortunately Rust doesn't have stable ceiling division yet. To avoid
    // floating point operations, add 4-1 to the year before dividing by 4.
    //
    // Note: This assumes that every 4th year is a leap year. This will work
    // for all years between 2001 and 2099.
    let full_leap_years = ((dt.year() as u32).saturating_sub(2004) + 3) / 4;

    let full_year_seconds = (dt.year() as u32 - 2001) * y + full_leap_years * d;
    let full_day_seconds = dt.ordinal0() * d;
    let current_day_seconds = dt.num_seconds_from_midnight();

    full_year_seconds + full_day_seconds + current_day_seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;
    use stm32l0xx_hal::rtc::NaiveDate;

    #[rstest]
    #[case(NaiveDate::from_ymd(2001, 1, 1).and_hms(0, 0, 0), 0)]
    #[case(NaiveDate::from_ymd(2001, 1, 1).and_hms(0, 0, 3), 3)]
    #[case(NaiveDate::from_ymd(2001, 1, 1).and_hms(20, 10, 59), 72_659)]
    #[case(NaiveDate::from_ymd(2003, 2, 3).and_hms(7, 0, 5), 65_948_405)]
    fn test_datetime_to_uptime_predefined(
        #[case] datetime: NaiveDateTime,
        #[case] expected_seconds: u32,
    ) {
        assert_eq!(datetime_to_uptime(datetime), expected_seconds);
    }

    #[test]
    fn test_datetime_to_uptime_vs_builtin() {
        let reference = NaiveDate::from_ymd(2001, 1, 1).and_hms(0, 0, 0);
        let datetime = NaiveDate::from_ymd(2098, 11, 28).and_hms(13, 14, 15);
        let chrono_builtin = (datetime - reference).num_seconds();
        let gfroerli_firmware = datetime_to_uptime(datetime);
        assert_eq!(gfroerli_firmware as i64, chrono_builtin);
    }
}
