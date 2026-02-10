use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use super::days_of_week::DaysOfWeek;
use super::trading::TimeOfDay;

/// Machine-readable funding rate schedule for perpetual contracts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FundingRateSchedule {
    /// Timezone for all times (chrono_tz::Tz, serializes as IANA string)
    #[cfg_attr(feature = "utoipa", schema(value_type = String, example = "Europe/London"))]
    pub timezone: Tz,

    /// Recurring funding times
    pub times: Vec<FundingTime>,

    /// Dates with modified schedules (e.g., half-days, special times, or holidays).
    ///
    /// Note: Exception dates are interpreted in the schedule's timezone (the benchmark timezone),
    /// not UTC. For example, if the timezone is "America/New_York" and an exception date is
    /// "2025-12-25", it refers to December 25th in New York time.
    pub exceptions: Vec<FundingException>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FundingTime {
    /// Days of week (1=Monday, 7=Sunday)
    pub days_of_week: DaysOfWeek,

    /// Funding time
    pub time_of_day: TimeOfDay,
}

impl FundingTime {
    pub fn new(days_of_week: DaysOfWeek, hours: u8, minutes: u8, seconds: u8) -> Self {
        Self {
            days_of_week,
            time_of_day: TimeOfDay {
                hours,
                minutes,
                seconds,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FundingException {
    /// The date this exception applies to
    pub date: NaiveDate,

    /// Replacement times for this date
    pub times: Vec<TimeOfDay>,

    /// Human-readable reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl FundingException {
    pub fn holiday(year: i32, month: u32, day: u32, reason: Option<&str>) -> Self {
        Self {
            date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            times: vec![],
            reason: reason.map(String::from),
        }
    }
}

impl Default for FundingRateSchedule {
    /// Creates a default funding rate schedule with no funding times.
    ///
    /// This is useful as a fallback for instruments that don't have an explicit schedule yet.
    /// A schedule with no times will always return `None` from `next_funding_time()`,
    /// indicating no funding events are scheduled.
    fn default() -> Self {
        Self {
            timezone: chrono_tz::UTC,
            times: vec![],
            exceptions: vec![],
        }
    }
}

impl FundingRateSchedule {
    pub fn next_funding_time(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let now_tz = now.with_timezone(&self.timezone);

        // Check today and the next 14 days.
        // Dates are computed in the schedule's timezone (benchmark timezone),
        // so exception dates are matched in that timezone, not UTC.
        for day_offset in 0..15 {
            let date = now_tz.date_naive() + chrono::TimeDelta::days(day_offset);

            let exception = self.exceptions.iter().find(|e| e.date == date);
            let times: Vec<TimeOfDay> = if let Some(e) = exception {
                e.times.clone()
            } else {
                let dow = date.weekday().number_from_monday() as u8;
                self.times
                    .iter()
                    .filter(|t| t.days_of_week.contains(dow))
                    .map(|t| t.time_of_day)
                    .collect()
            };

            let earliest = times
                .into_iter()
                .filter_map(|t| {
                    self.timezone
                        .from_local_datetime(&date.and_hms_opt(
                            t.hours as u32,
                            t.minutes as u32,
                            t.seconds as u32,
                        )?)
                        .earliest()
                        .map(|dt| dt.with_timezone(&Utc))
                })
                .filter(|dt| *dt > now)
                .min();

            if earliest.is_some() {
                return earliest;
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_funding_rate_schedule_serde_round_trip() {
        let json = r#"{
            "timezone": "Europe/London",
            "times": [
                {
                    "days_of_week": [1, 2, 3, 4, 5],
                    "time_of_day": {"hours": 16, "minutes": 0, "seconds": 0}
                }
            ],
            "exceptions": [
                {"date": "2026-12-25", "times": [], "reason": "Christmas Day"}
            ]
        }"#;

        let schedule: FundingRateSchedule = serde_json::from_str(json).unwrap();
        let round_tripped: FundingRateSchedule =
            serde_json::from_str(&serde_json::to_string(&schedule).unwrap()).unwrap();
        assert_eq!(schedule, round_tripped);
    }
}
