use core::mem::MaybeUninit;

// ===== Gregorian leap years =====

// leap year have 366 days, instead of 365 in common year
// rules of leap year:
// - divisible by 4           = leap
// - except divisible by 100  = not leap
// - except divisible by 400  = leap again

// the following is total days in X years, with leap years each have 1 extra day

// there are 97 leap years in 400Y
const DAYS_PER_400Y: i64 = DAYS_PER_YEAR * 400 + 97;
// there are 24 leap years in 100Y
const DAYS_PER_100Y: i64 = DAYS_PER_YEAR * 100 + 24;
// there are 1 leap years in 4Y
const DAYS_PER_4Y: i64 = DAYS_PER_YEAR * 4 + 1;

const DAYS_PER_YEAR: i64 = 365;

// ===== Starting date =====

// instead of Epoch, date starts at:
// `Wed, 01 Mar 2000 00:00:00 GMT`

// offset from Epoch to starting date in `day` unit
const DAYS_EPOCH_OFF: i64 = 11017;

// [Mar, .., Feb]
const DAYS_EACH_MONTHS: [i64; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];

// ===== httpdate =====

/// Write [`httpdate`][1] with given seconds.
///
/// Returns `None` if `secs` is negative or resolved date has year more than `9999`.
///
/// `secs` must be seconds since the Epoch, `1970-01-01 00:00:00 +0000 (UTC)`.
///
/// [1]: <https://www.rfc-editor.org/info/rfc9110/#name-date-time-formats>
pub const fn write_date(secs: i64, buf: &mut [MaybeUninit<u8>; 29]) -> Option<&mut [u8; 29]> {
    if secs.is_negative() {
        return None;
    }

    // total days since epoch
    let total_days_epoch = secs / 86400;

    // total days since starting date, maybe negative
    let total_days = total_days_epoch - DAYS_EPOCH_OFF;

    let mut remdays;

    let year;
    let month;

    // ===== gregorian calendar cycles =====
    {
        // is given time less than starting date
        let is_pre_start = total_days.is_negative() as i64;

        // number of 400Y cycles has happened
        let y400_cycles = (total_days / DAYS_PER_400Y) - is_pre_start;
        remdays = (total_days % DAYS_PER_400Y) + (is_pre_start * DAYS_PER_400Y);

        // number of 100Y cycles has happened
        let y100_cycles = {
            let cy = remdays / DAYS_PER_100Y;
            cy - (cy == 4) as i64
        };
        remdays -= y100_cycles * DAYS_PER_100Y;

        // number of 4Y cycles has happened
        let y4_cycles = {
            let cy = remdays / DAYS_PER_4Y;
            cy - (cy == 25) as i64
        };
        remdays -= y4_cycles * DAYS_PER_4Y;

        // number of 1Y cycles has happened
        let y1_cycles = {
            let cy = remdays / 365;
            cy - (cy == 4) as i64
        };
        remdays -= y1_cycles * DAYS_PER_YEAR;

        // ===== remdays calculated =====

        let mut mon = 0;
        month = loop {
            if mon >= DAYS_EACH_MONTHS.len() {
                break mon;
            }
            let mon_len = unsafe { DAYS_EACH_MONTHS.as_ptr().add(mon).read() };
            if remdays < mon_len {
                break mon;
            }
            remdays -= mon_len;
            mon += 1;
        };

        // the starting date month is `Mar`, so `10` and `11` is `Jan` and `Feb` next year
        let year_off = (mon >= 10) as i64;

        year = 2000 + year_off + y1_cycles + 4 * y4_cycles + 100 * y100_cycles + 400 * y400_cycles;
    }

    if !matches!(year, 0..=9999) {
        return None;
    }

    // ===== day-name =====

    // starting date week day is `Wed`
    const WEEK_DAY_TAB: &[u8; 28] = b"\
        Wed,\
        Thu,\
        Fri,\
        Sat,\
        Sun,\
        Mon,\
        Tue,\
    ";

    // make `total_days` positive while maintaining week day
    let week_day = (total_days % 7 + 7) % 7;
    unsafe {
        let week_day = WEEK_DAY_TAB.as_ptr().add((week_day * 4) as usize).cast();
        buf.as_mut_ptr().copy_from_nonoverlapping(week_day, 4);
    }

    // ===== day =====

    // remdays is `0` based, while date is `1` based
    let day = (remdays + 1) as u8;
    buf[4].write(b' ');
    buf[5].write(b'0' + (day / 10));
    buf[6].write(b'0' + (day % 10));
    buf[7].write(b' ');

    // ===== month =====

    // starting date month is `Mar`
    const MONTH_TAB: &[u8; 48] = b"\
        Mar \
        Apr \
        May \
        Jun \
        Jul \
        Aug \
        Sep \
        Oct \
        Nov \
        Dec \
        Jan \
        Feb \
    ";
    unsafe {
        let month = MONTH_TAB.as_ptr().add(month * 4).cast();
        buf.as_mut_ptr().add(8).copy_from_nonoverlapping(month, 4);
    }

    // ===== year =====

    buf[12].write(b'0' + (year / 1000) as u8);
    buf[13].write(b'0' + (year / 100 % 10) as u8);
    buf[14].write(b'0' + (year / 10 % 10) as u8);
    buf[15].write(b'0' + (year % 10) as u8);
    buf[16].write(b' ');

    // ===== time-of-day =====

    // `MaybeUninit` unaware of array
    write_time_inner(secs, buf.as_mut_ptr().cast());
    unsafe { Some(&mut *(buf as *mut [_; 29] as *mut [u8; 29])) }
}

/// Write the `time-of-day` part of the [`httpdate`][1] with given seconds.
///
/// Returns the passed `buf`. Returns `None` if `secs` is negative.
///
/// `secs` must be seconds since the Epoch, `1970-01-01 00:00:00 +0000 (UTC)`.
///
/// [1]: <https://www.rfc-editor.org/info/rfc9110/#name-date-time-formats>
#[inline]
pub const fn write_time(secs: i64, buf: &mut [u8; 29]) -> Option<&mut [u8; 29]> {
    if secs.is_negative() {
        return None;
    }
    write_time_inner(secs, buf.as_mut_ptr());
    Some(buf)
}

const fn write_time_inner(secs: i64, buf: *mut u8) {
    let secs_of_day = secs % 86400;
    let hour = (secs_of_day / 3600) as u8;
    let min = ((secs_of_day % 3600) / 60) as u8;
    let sec = (secs_of_day % 60) as u8;
    unsafe {
        buf.add(17).write(b'0' + (hour / 10));
        buf.add(18).write(b'0' + (hour % 10));
        buf.add(19).write(b':');
        buf.add(20).write(b'0' + (min / 10));
        buf.add(21).write(b'0' + (min % 10));
        buf.add(22).write(b':');
        buf.add(23).write(b'0' + (sec / 10));
        buf.add(24).write(b'0' + (sec % 10));
        buf.add(25).copy_from_nonoverlapping(b" GMT".as_ptr(), 4)
    }
}

// ===== tests =====

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::write_date;

    macro_rules! test_me {
        ($sec:expr, $exp:literal) => {
            let mut b = [MaybeUninit::uninit(); _];
            let date = write_date($sec, &mut b).unwrap();
            assert_eq!(str::from_utf8(date), Ok($exp));
        };
    }

    #[test]
    fn test_httpdate() {
        test_me!(0, "Thu, 01 Jan 1970 00:00:00 GMT");
        test_me!(784111777, "Sun, 06 Nov 1994 08:49:37 GMT");
        test_me!(946684800, "Sat, 01 Jan 2000 00:00:00 GMT");
        test_me!(951782400, "Tue, 29 Feb 2000 00:00:00 GMT");

        test_me!(951782399, "Mon, 28 Feb 2000 23:59:59 GMT");
        test_me!(951782400, "Tue, 29 Feb 2000 00:00:00 GMT");
        test_me!(951868799, "Tue, 29 Feb 2000 23:59:59 GMT");
        test_me!(951868800, "Wed, 01 Mar 2000 00:00:00 GMT");

        test_me!(1475419451, "Sun, 02 Oct 2016 14:44:11 GMT");
        test_me!(1577836800, "Wed, 01 Jan 2020 00:00:00 GMT");
        test_me!(1582934400, "Sat, 29 Feb 2020 00:00:00 GMT");
        test_me!(1609459200, "Fri, 01 Jan 2021 00:00:00 GMT");
        test_me!(1767225600, "Thu, 01 Jan 2026 00:00:00 GMT");
        test_me!(1790510400, "Sun, 27 Sep 2026 12:00:00 GMT");

        test_me!(4107542399, "Sun, 28 Feb 2100 23:59:59 GMT");
        test_me!(4107542400, "Mon, 01 Mar 2100 00:00:00 GMT");

        test_me!(13574534399, "Mon, 28 Feb 2400 15:59:59 GMT");
        test_me!(13574534400, "Mon, 28 Feb 2400 16:00:00 GMT");
    }
}
