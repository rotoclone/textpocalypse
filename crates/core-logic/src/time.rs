use log::debug;

const SECONDS_PER_MINUTE: u8 = 60;
const MINUTES_PER_HOUR: u8 = 60;
const HOURS_PER_DAY: u8 = 24;

const TICK_SECONDS: u8 = 15;
const START_DAY: usize = 1;
const START_HOUR: u8 = 7;
const START_MINUTE: u8 = 0;
const START_SECOND: u8 = 0;

#[derive(Copy, Clone, Debug)]
pub struct Time {
    pub day: usize,
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
        self.add_seconds(TICK_SECONDS);
    }

    fn add_seconds(&mut self, seconds_to_add: u8) {
        debug!("Adding {seconds_to_add} seconds to current time {self:?}");

        //TODO if self.second + seconds_to_add is > 2^8, will this overflow?
        let seconds_remaining_in_minute = SECONDS_PER_MINUTE - self.second;
        let new_second = (self.second + seconds_to_add) % SECONDS_PER_MINUTE;
        self.second = new_second;

        debug!("Seconds remaining in minute: {seconds_remaining_in_minute}");

        if seconds_to_add < seconds_remaining_in_minute {
            debug!("Did not roll over to new minute; new time: {self:?}");
            return;
        }

        let minutes_remaining_in_hour = MINUTES_PER_HOUR - self.minute;
        let minutes_to_add =
            1 + ((seconds_to_add - seconds_remaining_in_minute) / SECONDS_PER_MINUTE);
        let new_minute = (self.minute + minutes_to_add) % MINUTES_PER_HOUR;
        self.minute = new_minute;

        if minutes_to_add < minutes_remaining_in_hour {
            debug!("Did not roll over to new hour; new time: {self:?}");
            return;
        }

        let hours_remaining_in_day = HOURS_PER_DAY - self.hour;
        let hours_to_add = 1 + ((minutes_to_add - minutes_remaining_in_hour) / MINUTES_PER_HOUR);
        let new_hour = (self.hour + hours_to_add) % HOURS_PER_DAY;
        self.hour = new_hour;

        if hours_to_add < hours_remaining_in_day {
            debug!("Did not roll over to new day; new time: {self:?}");
            return;
        }

        let days_to_add = 1 + ((hours_to_add - hours_remaining_in_day) / HOURS_PER_DAY);
        let new_day = self.day + (days_to_add as usize);
        self.day = new_day;

        debug!("New time: {self:?}");
    }
}
