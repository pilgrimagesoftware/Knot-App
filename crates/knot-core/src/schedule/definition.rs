//! What a schedule is, and the one string it is stored as.
//!
//! Stored as text - `daily 09:00`, `weekdays 09:00`, `days mon,wed 09:00`,
//! `every 3h`, `cron 0 9 * * 1-5` - so a saved agent stays readable, and so a
//! value this build cannot read survives as [`Schedule::Unreadable`] instead
//! of failing the whole document. An unreadable schedule never runs.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A time of day on the local clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay {
    hour:   u8,
    minute: u8,
}

impl TimeOfDay {
    /// `None` outside 00:00..=23:59.
    #[must_use]
    pub fn new(hour: u8, minute: u8) -> Option<Self> {
        (hour < 24 && minute < 60).then_some(Self { hour, minute })
    }

    #[must_use]
    pub fn hour(self) -> u8 {
        self.hour
    }

    #[must_use]
    pub fn minute(self) -> u8 {
        self.minute
    }
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl FromStr for TimeOfDay {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        let (hour, minute) = text.split_once(':').ok_or(())?;
        Self::new(hour.parse().map_err(|_| ())?,
                  minute.parse().map_err(|_| ())?).ok_or(())
    }
}

/// A day of the week, Monday first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub const ALL: [Self; 7] = [Self::Monday,
                                Self::Tuesday,
                                Self::Wednesday,
                                Self::Thursday,
                                Self::Friday,
                                Self::Saturday,
                                Self::Sunday];

    /// The stored token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Monday => "mon",
            Self::Tuesday => "tue",
            Self::Wednesday => "wed",
            Self::Thursday => "thu",
            Self::Friday => "fri",
            Self::Saturday => "sat",
            Self::Sunday => "sun",
        }
    }

    /// Cron's day-of-week number: Sunday is 0.
    #[must_use]
    pub(super) const fn cron_number(self) -> u8 {
        match self {
            Self::Sunday => 0,
            Self::Monday => 1,
            Self::Tuesday => 2,
            Self::Wednesday => 3,
            Self::Thursday => 4,
            Self::Friday => 5,
            Self::Saturday => 6,
        }
    }

    const fn bit(self) -> u8 {
        1 << (self as u8)
    }
}

impl FromStr for Weekday {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        Self::ALL.into_iter()
                 .find(|day| day.token() == text)
                 .ok_or(())
    }
}

/// A set of weekdays. Never empty once built by [`WeekdaySet::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WeekdaySet(u8);

impl WeekdaySet {
    /// `None` for an empty set: a preset that runs on no day is not a
    /// schedule.
    #[must_use]
    pub fn new(days: impl IntoIterator<Item = Weekday>) -> Option<Self> {
        let bits = days.into_iter().fold(0, |bits, day| bits | day.bit());
        (bits != 0).then_some(Self(bits))
    }

    #[must_use]
    pub fn contains(self, day: Weekday) -> bool {
        self.0 & day.bit() != 0
    }

    /// The days in the set, Monday first.
    pub fn days(self) -> impl Iterator<Item = Weekday> {
        Weekday::ALL.into_iter()
                    .filter(move |day| self.contains(*day))
    }
}

/// A human-friendly schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Preset {
    Daily {
        at: TimeOfDay,
    },
    Weekdays {
        at: TimeOfDay,
    },
    Days {
        days: WeekdaySet,
        at:   TimeOfDay,
    },
    /// At the hours from midnight that are multiples of `hours`, on the hour.
    /// When `hours` does not divide 24, the last gap of the day is shorter.
    EveryHours {
        hours: u8,
    },
}

/// When an agent starts on its own.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Schedule {
    Preset(Preset),
    /// A five-field cron expression, already checked to parse.
    Cron(String),
    /// A stored value this build cannot read. Kept as written so saving the
    /// agent does not erase it; it never runs.
    Unreadable(String),
}

impl Schedule {
    /// A cron schedule, if `expression` is valid five-field cron.
    #[must_use]
    pub fn cron(expression: &str) -> Option<Self> {
        let expression = expression.split_whitespace().collect::<Vec<_>>().join(" ");
        super::evaluate::parse_cron(&expression).ok()?;
        Some(Self::Cron(expression))
    }

    /// Whether this schedule can run.
    #[must_use]
    pub fn is_runnable(&self) -> bool {
        !matches!(self, Self::Unreadable(_))
    }
}

impl fmt::Display for Schedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preset(Preset::Daily { at }) => write!(f, "daily {at}"),
            Self::Preset(Preset::Weekdays { at }) => write!(f, "weekdays {at}"),
            Self::Preset(Preset::Days { days, at }) => {
                let days = days.days()
                               .map(Weekday::token)
                               .collect::<Vec<_>>()
                               .join(",");
                write!(f, "days {days} {at}")
            }
            Self::Preset(Preset::EveryHours { hours }) => write!(f, "every {hours}h"),
            Self::Cron(expression) => write!(f, "cron {expression}"),
            Self::Unreadable(text) => f.write_str(text),
        }
    }
}

impl FromStr for Schedule {
    type Err = ();

    /// Strict: an error for anything this build cannot read. Loading uses
    /// [`From<String>`], which keeps such a value as [`Schedule::Unreadable`].
    fn from_str(text: &str) -> Result<Self, ()> {
        let text = text.trim();
        let (kind, rest) = text.split_once(' ').ok_or(())?;
        let rest = rest.trim();
        match kind {
            "daily" => Ok(Self::Preset(Preset::Daily { at: rest.parse()? })),
            "weekdays" => Ok(Self::Preset(Preset::Weekdays { at: rest.parse()? })),
            "days" => {
                let (days, at) = rest.split_once(' ').ok_or(())?;
                let days = days.split(',')
                               .map(str::parse)
                               .collect::<Result<Vec<Weekday>, ()>>()?;
                Ok(Self::Preset(Preset::Days { days: WeekdaySet::new(days).ok_or(())?,
                                               at:   at.trim().parse()?, }))
            }
            "every" => {
                let hours: u8 = rest.strip_suffix('h').ok_or(())?.parse().map_err(|_| ())?;
                (1..24).contains(&hours)
                       .then_some(Self::Preset(Preset::EveryHours { hours }))
                       .ok_or(())
            }
            "cron" => Self::cron(rest).ok_or(()),
            _ => Err(()),
        }
    }
}

impl From<String> for Schedule {
    fn from(text: String) -> Self {
        text.parse().unwrap_or(Self::Unreadable(text))
    }
}

impl From<Schedule> for String {
    fn from(schedule: Schedule) -> Self {
        schedule.to_string()
    }
}

impl Serialize for Schedule {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Schedule {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}
