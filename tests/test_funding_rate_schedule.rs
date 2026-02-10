use ax_exchange_sdk::funding_rate_schedule::{FundingException, FundingRateSchedule, FundingTime};
use ax_exchange_sdk::trading::TimeOfDay;
use ax_exchange_sdk::DaysOfWeek;
use chrono::{DateTime, Duration, NaiveDate, Utc};

fn uk_fx_schedule() -> FundingRateSchedule {
    FundingRateSchedule {
        timezone: chrono_tz::Europe::London,
        times: vec![FundingTime::new(DaysOfWeek::weekdays(), 16, 0, 0)],
        exceptions: vec![],
    }
}

fn crypto_8h_schedule() -> FundingRateSchedule {
    FundingRateSchedule {
        timezone: chrono_tz::UTC,
        times: vec![
            FundingTime::new(DaysOfWeek::all(), 0, 0, 0),
            FundingTime::new(DaysOfWeek::all(), 8, 0, 0),
            FundingTime::new(DaysOfWeek::all(), 16, 0, 0),
        ],
        exceptions: vec![],
    }
}

#[test]
fn test_funding_rate_schedule_invalid_timezone() {
    let json = r#"{
        "timezone": "Invalid/Timezone",
        "times": [
            {
                "days_of_week": [1, 2, 3, 4, 5],
                "time_of_day": {"hours": 16, "minutes": 0}
            }
        ],
        "exceptions": []
    }"#;

    let result: Result<FundingRateSchedule, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_next_funding_time_exactly_at_funding_time() {
    let schedule = uk_fx_schedule();

    // Exactly at 16:00:00 UTC on a Tuesday (Jan 27, 2026 is a Tuesday)
    let now = "2026-01-27T16:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now);

    // Should return NEXT day's 16:00, not today's (because now >= funding_time)
    assert_eq!(next.unwrap().to_rfc3339(), "2026-01-28T16:00:00+00:00");
}

#[test]
fn test_next_funding_time_one_nanosecond_before() {
    let schedule = uk_fx_schedule();

    // 1 nanosecond before 16:00
    let now = "2026-01-27T15:59:59.999999999Z"
        .parse::<DateTime<Utc>>()
        .unwrap();
    let next = schedule.next_funding_time(now);

    // Should return today's 16:00
    assert_eq!(next.unwrap().to_rfc3339(), "2026-01-27T16:00:00+00:00");
}

#[test]
fn test_next_funding_time_one_second_after() {
    let schedule = uk_fx_schedule();

    // 1 second after 16:00
    let now = "2026-01-27T16:00:01Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now);

    // Should return tomorrow's 16:00
    assert_eq!(next.unwrap().to_rfc3339(), "2026-01-28T16:00:00+00:00");
}

#[test]
fn test_next_funding_time_consecutive_holidays() {
    // Test a scenario where multiple consecutive days are holidays
    let schedule = FundingRateSchedule {
        timezone: chrono_tz::Europe::London,
        times: vec![FundingTime::new(DaysOfWeek::weekdays(), 16, 0, 0)],
        exceptions: vec![
            FundingException::holiday(2026, 12, 24, Some("Christmas Eve")),
            FundingException::holiday(2026, 12, 25, Some("Christmas")),
            FundingException::holiday(2026, 12, 26, Some("Boxing Day")),
            FundingException::holiday(2026, 12, 28, Some("Boxing Day (Observed)")),
            FundingException::holiday(2026, 12, 29, Some("Bank Holiday")),
        ],
    };

    // Thursday Dec 24 at 17:00 (after funding)
    let now = "2026-12-24T17:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now);

    // Should skip Dec 24 (holiday), Dec 25 (Christmas), Dec 26 (Sat/Boxing Day),
    // Dec 27 (Sun), Dec 28 (Mon/Boxing Day observed), Dec 29 (Tue/Bank Holiday)
    // Next available is Wednesday Dec 30
    assert_eq!(next.unwrap().to_rfc3339(), "2026-12-30T16:00:00+00:00");
}

#[test]
fn test_next_funding_time_empty_schedule_returns_none() {
    let schedule = FundingRateSchedule::default();

    assert_eq!(schedule.timezone, chrono_tz::UTC);
    assert!(schedule.times.is_empty());
    assert!(schedule.exceptions.is_empty());

    let now = "2026-01-27T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
    assert!(schedule.next_funding_time(now).is_none());
}

#[test]
fn test_next_funding_time_far_future_lookup() {
    // Verify the 14-day lookahead window limitation
    let schedule = FundingRateSchedule {
        timezone: chrono_tz::UTC,
        times: vec![FundingTime::new(
            DaysOfWeek::new(vec![1]).unwrap(),
            16,
            0,
            0,
        )],
        exceptions: vec![
            // Skip next 3 Mondays (all within 14 days)
            FundingException::holiday(2026, 1, 26, None),
            FundingException::holiday(2026, 2, 2, None),
            FundingException::holiday(2026, 2, 9, None),
        ],
    };

    // Start on a Monday
    let now = "2026-01-26T10:00:00Z".parse::<DateTime<Utc>>().unwrap();

    // Should find the 4th Monday (Feb 16) which is 21 days away
    // BUT it's outside the 14-day window!
    let next = schedule.next_funding_time(now);

    // This exposes that the 14-day limit might need adjustment for extreme exception lists
    assert!(next.is_none()); // Expected: None due to 14-day limit
}

#[test]
fn test_next_funding_time_weekday() {
    // Test normal weekday case: Monday 10:00 AM London time
    let schedule = uk_fx_schedule();

    // Monday, Jan 27, 2026 at 10:00 UTC (10:00 London time in winter)
    let now = "2026-01-27T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // Should be same day at 16:00 London time = 16:00 UTC (winter)
    assert_eq!(next.to_rfc3339(), "2026-01-27T16:00:00+00:00");
}

#[test]
fn test_next_funding_time_after_days_funding() {
    // Test when current time is after today's funding time
    let schedule = uk_fx_schedule();

    // Monday at 17:00 UTC (after 16:00 funding)
    let now = "2026-01-27T17:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // Should be next day (Tuesday) at 16:00 UTC
    assert_eq!(next.to_rfc3339(), "2026-01-28T16:00:00+00:00");
}

#[test]
fn test_next_funding_time_weekend_skip() {
    // Test that weekends are skipped
    let schedule = uk_fx_schedule();

    // Sunday, Jan 25, 2026 at 10:00 UTC
    let now = "2026-01-25T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // Should skip to Monday, Jan 26 at 16:00 UTC
    assert_eq!(next.to_rfc3339(), "2026-01-26T16:00:00+00:00");
}

#[test]
fn test_next_funding_time_holiday_exception() {
    // Test that holiday exceptions (empty times) are skipped
    let schedule = FundingRateSchedule {
        timezone: chrono_tz::Europe::London,
        times: vec![FundingTime::new(DaysOfWeek::weekdays(), 16, 0, 0)],
        exceptions: vec![FundingException::holiday(
            2026,
            12,
            25,
            Some("Christmas Day"),
        )],
    };

    // Thursday, Dec 24, 2026 at 17:00 UTC (after funding)
    let now = "2026-12-24T17:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // Should skip Friday (Christmas) and go to Monday, Dec 28
    assert_eq!(next.to_rfc3339(), "2026-12-28T16:00:00+00:00");
}

#[test]
fn test_next_funding_time_half_day_exception() {
    // Test early close (half-day) exception
    let schedule = FundingRateSchedule {
        timezone: chrono_tz::America::New_York,
        times: vec![FundingTime::new(DaysOfWeek::weekdays(), 16, 0, 0)],
        exceptions: vec![FundingException {
            date: NaiveDate::from_ymd_opt(2026, 11, 27).unwrap(), // Friday after Thanksgiving
            times: vec![TimeOfDay {
                hours: 13,
                minutes: 0,
                seconds: 0,
            }],
            reason: Some("Day after Thanksgiving - early close".to_string()),
        }],
    };

    // Friday, Nov 27, 2026 at 12:00 EST (17:00 UTC)
    let now = "2026-11-27T17:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // Should be same day at 13:00 EST = 18:00 UTC (EST is UTC-5)
    assert_eq!(next.to_rfc3339(), "2026-11-27T18:00:00+00:00");
}

#[test]
fn test_next_funding_time_multi_session() {
    // Test multiple funding times per day (crypto-style)
    let schedule = crypto_8h_schedule();

    // Monday at 09:00 UTC (after 08:00, before 16:00)
    let now = "2026-01-27T09:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // Should be same day at 16:00 UTC
    assert_eq!(next.to_rfc3339(), "2026-01-27T16:00:00+00:00");
}

#[test]
fn test_next_funding_time_dst_transition() {
    // Test behavior around DST transition
    // In 2026, US DST starts on March 8 (spring forward at 2:00 AM)
    let schedule = FundingRateSchedule {
        timezone: chrono_tz::America::New_York,
        times: vec![FundingTime::new(DaysOfWeek::all(), 16, 0, 0)],
        exceptions: vec![],
    };

    // Saturday, March 7, 2026 at 20:00 UTC (before DST)
    // 16:00 EST = 21:00 UTC
    let now = "2026-03-07T20:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // Should be same day at 16:00 EST = 21:00 UTC
    assert_eq!(next.to_rfc3339(), "2026-03-07T21:00:00+00:00");

    // Sunday, March 8, 2026 at 20:00 UTC (after DST transition)
    // 16:00 EDT = 20:00 UTC (EDT is UTC-4)
    let now = "2026-03-08T19:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // Should be same day at 16:00 EDT = 20:00 UTC
    assert_eq!(next.to_rfc3339(), "2026-03-08T20:00:00+00:00");
}

#[test]
fn test_next_funding_time_dst_fall_back() {
    // In 2026, US DST ends on November 1 (fall back at 2:00 AM)
    // At 1:00 AM EDT, clocks fall back to 1:00 AM EST, so 1:00 AM occurs twice.
    let schedule = FundingRateSchedule {
        timezone: chrono_tz::America::New_York,
        times: vec![FundingTime::new(DaysOfWeek::all(), 1, 30, 0)],
        exceptions: vec![],
    };

    // Before the ambiguous window: Saturday Oct 31 at 23:00 UTC
    // (which is 7:00 PM EDT on Oct 31)
    let now = "2026-10-31T23:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // 1:30 AM EDT on Nov 1 = 05:30 UTC (the earliest interpretation)
    assert_eq!(next.to_rfc3339(), "2026-11-01T05:30:00+00:00");
}

#[test]
fn test_next_funding_time_dst_spring_forward_gap() {
    // In 2026, US DST starts on March 8 (spring forward at 2:00 AM)
    // 2:30 AM doesn't exist on this day.
    let schedule = FundingRateSchedule {
        timezone: chrono_tz::America::New_York,
        times: vec![FundingTime::new(DaysOfWeek::all(), 2, 30, 0)],
        exceptions: vec![],
    };

    // Before the gap: Saturday March 7 at 06:00 UTC
    let now = "2026-03-07T06:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // March 7 is before DST, so 2:30 AM EST = 07:30 UTC
    assert_eq!(next.to_rfc3339(), "2026-03-07T07:30:00+00:00");

    // Now query from after March 7's funding but before March 8's gap
    let now = "2026-03-07T08:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let next = schedule.next_funding_time(now).unwrap();

    // March 8 2:30 AM doesn't exist (spring forward), so skip to March 9
    // March 9 2:30 AM EDT = 06:30 UTC
    assert_eq!(next.to_rfc3339(), "2026-03-09T06:30:00+00:00");
}

#[test]
fn test_funding_rate_schedule_with_seconds() {
    use insta::assert_json_snapshot;

    // Test that seconds field works when non-zero
    let schedule = FundingRateSchedule {
        timezone: chrono_tz::UTC,
        times: vec![FundingTime::new(
            DaysOfWeek::new(vec![1]).unwrap(),
            16,
            30,
            45,
        )],
        exceptions: vec![],
    };

    assert_json_snapshot!(schedule, @r#"
    {
      "timezone": "UTC",
      "times": [
        {
          "days_of_week": [
            1
          ],
          "time_of_day": {
            "hours": 16,
            "minutes": 30,
            "seconds": 45
          }
        }
      ],
      "exceptions": []
    }
    "#);

    // Verify it can be deserialized back
    let json = serde_json::to_string(&schedule).unwrap();
    let deserialized: FundingRateSchedule = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.times[0].time_of_day.seconds, 45);
}

#[test]
fn test_funding_times_over_full_week() {
    let schedule = uk_fx_schedule();
    let start: DateTime<Utc> = "2026-01-26T00:00:00Z".parse().unwrap(); // Monday

    let mut next_times = vec![];
    for hour in 0..(7 * 24) {
        let now = start + Duration::hours(hour);
        if let Some(next) = schedule.next_funding_time(now) {
            if !next_times.contains(&next) {
                next_times.push(next);
            }
        }
    }

    // Mon Jan 26, Tue Jan 27, Wed Jan 28, Thu Jan 29, Fri Jan 30, Mon Feb 2
    assert_eq!(next_times.len(), 6);
}

#[test]
fn test_multiple_daily_sessions_stepping() {
    let schedule = crypto_8h_schedule();

    // At midnight exactly, next should be 08:00 (current time is not "next")
    let now: DateTime<Utc> = "2026-01-26T00:00:00Z".parse().unwrap();
    let next = schedule.next_funding_time(now).unwrap();
    assert_eq!(next.to_rfc3339(), "2026-01-26T08:00:00+00:00");

    // At 08:01, next should be 16:00
    let now = now + Duration::hours(8) + Duration::minutes(1);
    let next = schedule.next_funding_time(now).unwrap();
    assert_eq!(next.to_rfc3339(), "2026-01-26T16:00:00+00:00");

    // At 16:01, next should be next day's 00:00
    let now = now + Duration::hours(8);
    let next = schedule.next_funding_time(now).unwrap();
    assert_eq!(next.to_rfc3339(), "2026-01-27T00:00:00+00:00");
}

#[test]
fn test_timezone_day_boundary() {
    // Midnight in UTC+9: the local "Monday" funding time falls on Sunday in UTC
    let schedule_tokyo = FundingRateSchedule {
        timezone: chrono_tz::Asia::Tokyo,
        times: vec![FundingTime::new(DaysOfWeek::weekdays(), 0, 0, 0)],
        exceptions: vec![],
    };

    // Sunday 14:30 UTC = Sunday 23:30 JST
    let now: DateTime<Utc> = "2026-01-25T14:30:00Z".parse().unwrap();
    // Next: Monday 00:00 JST = Sunday 15:00 UTC
    let next = schedule_tokyo.next_funding_time(now).unwrap();
    assert_eq!(next.to_rfc3339(), "2026-01-25T15:00:00+00:00");

    // After crossing: next Tuesday 00:00 JST = Monday 15:00 UTC
    let now: DateTime<Utc> = "2026-01-25T15:01:00Z".parse().unwrap();
    let next = schedule_tokyo.next_funding_time(now).unwrap();
    assert_eq!(next.to_rfc3339(), "2026-01-26T15:00:00+00:00");

    // Late evening in UTC-8: local "Monday 23:30" falls on Tuesday in UTC
    let schedule_pacific = FundingRateSchedule {
        timezone: chrono_tz::America::Los_Angeles,
        times: vec![FundingTime::new(DaysOfWeek::weekdays(), 23, 30, 0)],
        exceptions: vec![],
    };

    // Tuesday 06:00 UTC = Monday 22:00 PST
    let now: DateTime<Utc> = "2026-01-27T06:00:00Z".parse().unwrap();
    // Next: Monday 23:30 PST = Tuesday 07:30 UTC
    let next = schedule_pacific.next_funding_time(now).unwrap();
    assert_eq!(next.to_rfc3339(), "2026-01-27T07:30:00+00:00");

    // After crossing: next is Tuesday 23:30 PST = Wednesday 07:30 UTC
    let now = now + Duration::hours(2);
    let next = schedule_pacific.next_funding_time(now).unwrap();
    assert_eq!(next.to_rfc3339(), "2026-01-28T07:30:00+00:00");
}
