use std::time::Duration;

use bevy_ecs::system::Resource;
use log::debug;

pub const SECONDS_PER_MINUTE: u8 = 60;
pub const MINUTES_PER_HOUR: u8 = 60;
pub const HOURS_PER_DAY: u8 = 24;
pub const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE as u64 * MINUTES_PER_HOUR as u64;
pub const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR * HOURS_PER_DAY as u64;

pub const TICK_DURATION: Duration = Duration::from_secs(15);
const START_DAY: u64 = 1;
const START_HOUR: u8 = 7;
const START_MINUTE: u8 = 0;
const START_SECOND: u8 = 0;

#[derive(Clone, Debug, PartialEq, Eq, Resource)]
pub struct Time {
    pub day: u64,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Default for Time {
    fn default() -> Self {
        Time::new()
    }
}

impl Time {
    pub fn new() -> Time {
        Time {
            day: START_DAY,
            hour: START_HOUR,
            minute: START_MINUTE,
            second: START_SECOND,
        }
    }

    pub fn tick(&mut self) {
        self.advance(TICK_DURATION);
    }

    fn advance(&mut self, to_add: Duration) {
        debug!("Adding {to_add:?} to current time {self:?}");

        let current_duration = self.to_duration();
        let new_duration = current_duration + to_add;
        self.update_to_match(new_duration);

        debug!("New time: {self:?}");
    }

    fn to_duration(&self) -> Duration {
        let seconds = self.second as u64;
        let seconds_from_minutes: u64 = self.minute as u64 * SECONDS_PER_MINUTE as u64;
        let seconds_from_hours: u64 = self.hour as u64 * SECONDS_PER_HOUR;
        let seconds_from_days: u64 = self.day * SECONDS_PER_DAY;

        Duration::from_secs(seconds + seconds_from_minutes + seconds_from_hours + seconds_from_days)
    }

    fn update_to_match(&mut self, duration: Duration) {
        let days = duration.as_secs() / SECONDS_PER_DAY;
        let hours = (duration.as_secs() / SECONDS_PER_HOUR) % HOURS_PER_DAY as u64;
        let minutes = (duration.as_secs() / SECONDS_PER_MINUTE as u64) % MINUTES_PER_HOUR as u64;
        let seconds = duration.as_secs() % SECONDS_PER_MINUTE as u64;

        // these unwraps are safe because each value has been modulus'd with a u8 value, so they will each fit in a u8
        self.second = seconds.try_into().unwrap();
        self.minute = minutes.try_into().unwrap();
        self.hour = hours.try_into().unwrap();
        self.day = days;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_by_no_time() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
        };

        time.advance(Duration::ZERO);

        let expected = Time {
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_no_rollover() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
        };

        time.advance(Duration::from_secs(59));

        let expected = Time {
            day: 0,
            hour: 0,
            minute: 0,
            second: 59,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_minute_rollover_exact() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 0,
            second: 30,
        };

        time.advance(Duration::from_secs(30));

        let expected = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 0,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_minute_rollover_with_remainder() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 0,
            second: 30,
        };

        time.advance(Duration::from_secs(32));

        let expected = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 2,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_multiple_minute_rollover_exact() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 0,
            second: 30,
        };

        time.advance(Duration::from_secs(90));

        let expected = Time {
            day: 0,
            hour: 0,
            minute: 2,
            second: 0,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_multiple_minute_rollover_with_remainder() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 0,
            second: 30,
        };

        time.advance(Duration::from_secs(95));

        let expected = Time {
            day: 0,
            hour: 0,
            minute: 2,
            second: 5,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_hour_rollover_exact() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(3_510));

        let expected = Time {
            day: 0,
            hour: 1,
            minute: 0,
            second: 0,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_hour_rollover_with_remainder() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(3_695));

        let expected = Time {
            day: 0,
            hour: 1,
            minute: 3,
            second: 5,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_multiple_hour_rollover_exact() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(7_110));

        let expected = Time {
            day: 0,
            hour: 2,
            minute: 0,
            second: 0,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_multiple_hour_rollover_with_remainder() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(7_233));

        let expected = Time {
            day: 0,
            hour: 2,
            minute: 2,
            second: 3,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_day_rollover_exact() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(86_310));

        let expected = Time {
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_day_rollover_with_remainder() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(97_235));

        let expected = Time {
            day: 1,
            hour: 3,
            minute: 2,
            second: 5,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_multiple_day_rollover_exact() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(172_710));

        let expected = Time {
            day: 2,
            hour: 0,
            minute: 0,
            second: 0,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_multiple_day_rollover_with_remainder() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(176_565));

        let expected = Time {
            day: 2,
            hour: 1,
            minute: 4,
            second: 15,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_lots_of_days() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(25_920_000));

        let expected = Time {
            day: 300,
            hour: 0,
            minute: 1,
            second: 30,
        };
        assert_eq!(expected, time);
    }

    #[test]
    fn advance_multiple_times() {
        let mut time = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 30,
        };

        time.advance(Duration::from_secs(5));

        let expected = Time {
            day: 0,
            hour: 0,
            minute: 1,
            second: 35,
        };
        assert_eq!(expected, time);

        time.advance(Duration::from_secs(60));

        let expected = Time {
            day: 0,
            hour: 0,
            minute: 2,
            second: 35,
        };
        assert_eq!(expected, time);

        time.advance(Duration::from_secs(3_600));

        let expected = Time {
            day: 0,
            hour: 1,
            minute: 2,
            second: 35,
        };
        assert_eq!(expected, time);

        time.advance(Duration::from_secs(86_400));

        let expected = Time {
            day: 1,
            hour: 1,
            minute: 2,
            second: 35,
        };
        assert_eq!(expected, time);

        time.advance(Duration::from_secs(30));

        let expected = Time {
            day: 1,
            hour: 1,
            minute: 3,
            second: 5,
        };
        assert_eq!(expected, time);
    }
}
