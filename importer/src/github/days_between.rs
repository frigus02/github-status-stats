use chrono::{Date, DateTime, FixedOffset};

pub struct DaysBetween {
    curr: Date<FixedOffset>,
    until: Date<FixedOffset>,
}

impl Iterator for DaysBetween {
    type Item = Date<FixedOffset>;

    fn next(&mut self) -> Option<Date<FixedOffset>> {
        if self.curr <= self.until {
            let result = self.curr;
            self.curr = self.curr.succ();
            Some(result)
        } else {
            None
        }
    }
}

pub fn days_between(since: &str, until: &str) -> Result<DaysBetween, chrono::ParseError> {
    let since = DateTime::parse_from_rfc3339(since)?.date();
    let until = DateTime::parse_from_rfc3339(until)?.date();
    Ok(DaysBetween { curr: since, until })
}
