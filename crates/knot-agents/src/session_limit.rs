//! A session stopped by its provider's usage limit (#318).
//!
//! The agent CLIs report it as the error that answers a prompt - Claude's
//! "You've hit your 5-hour limit · resets 5:20pm (America/Los_Angeles)",
//! Codex's "You've hit your usage limit" - so it is recognised from that
//! text. Nothing else distinguishes it from any other failed turn, and it is
//! the one failure the user can do nothing about but wait.
//!
//! Claude names the limit it hit, and the names change: "session limit"
//! became "5-hour limit", beside "weekly limit", "Opus limit" and others
//! (#397). So a notice is recognised by the lead the Claude Agent SDK
//! exports as stable (`USAGE_LIMIT_ERROR_PREFIXES`: "You've hit your",
//! "You've reached your") and a short name ending in "limit", never by a
//! list of names that the next release outgrows.

use std::time::{Duration, SystemTime};

/// Phrases, lower-case, that mark an error as a usage limit wherever they
/// appear: Codex's wording, Claude's before #397, and the wrap-up notice.
const LIMIT_PHRASES: [&str; 2] = ["session limit", "usage limit"];

/// How a limit notice opens, lower-case with a plain apostrophe - the
/// stable prefixes of the SDK's `USAGE_LIMIT_ERROR_PREFIXES` that go on to
/// name a limit.
const LIMIT_LEADS: [&str; 2] = ["you've hit your ", "you've reached your "];

/// Where the limit's name ends after a lead: the "·" before the reset time,
/// or the sentence's end. Not "-", which is inside "5-hour limit".
const NAME_ENDS: &[char] = &['\u{b7}', '.', ',', '!', ';', '(', '\u{2014}', '\n'];

/// The longest limit name accepted after a lead. "usage credit limit" is
/// the longest Claude prints; this leaves room for one it adds, and keeps
/// a sentence that merely ends in "limit" from passing as a name.
const MAX_LIMIT_NAME: usize = 32;

/// Limits Claude announces with the same lead that do not stop the session:
/// fast mode's cooldown drops the agent to normal speed and it carries on.
const NOT_A_STOP: [&str; 1] = ["fast limit"];

/// Where the reset time starts, lower-case.
const RESETS: &str = "resets ";

/// Hours on a 12-hour clock.
const HALF_DAY: i8 = 12;

/// How far past our clock a reset time may be and still be read as the
/// reset that is due now rather than tomorrow's (#368 review). A provider
/// whose clock runs a little behind refuses a prompt sent just after the
/// reset with the same "resets 5:20pm"; read literally that is 24 hours
/// away.
const LATE_RESET_GRACE: Duration = Duration::from_secs(15 * 60);

/// When to try again after such a late reset. Not at once: a held prompt
/// sent the moment the hold lifts, and refused again with the same reset,
/// would otherwise go round as fast as the queue runs.
const LATE_RESET_RETRY: Duration = Duration::from_secs(60);

/// An agent's session has hit its usage limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLimit {
    /// When it lifts, as the provider said it - "5:20pm
    /// (America/Los_Angeles)" - or `None` when it did not say.
    pub resets: Option<String>,
}

impl SessionLimit {
    /// The limit `error` reports, if it reports one.
    #[must_use]
    pub fn from_error(error: &str) -> Option<Self> {
        // ASCII-only lowering keeps every byte offset where it was, so an
        // index found in `lower` slices `error` at the same place.
        let lower = error.to_ascii_lowercase();
        if !LIMIT_PHRASES.iter().any(|phrase| lower.contains(phrase)) && !names_a_limit(&lower) {
            return None;
        }
        let resets = lower.find(RESETS)
                          .map(|at| error[at + RESETS.len()..].trim().trim_end_matches('.'))
                          .filter(|resets| !resets.is_empty())
                          .map(str::to_owned);
        Some(Self { resets })
    }

    /// Whether `lower` - a line, lower-cased - opens with a notice naming
    /// the limit that stopped the session: "you've hit your 5-hour limit ·
    /// resets 5:20pm", "you've reached your fable limit." For the terminal
    /// scan, which reads only a line's start so that an agent writing about
    /// limits mid-sentence does not stop itself.
    #[must_use]
    pub fn opens_with_named_limit(lower: &str) -> bool {
        let plain = lower.replace('\u{2019}', "'");
        LIMIT_LEADS.iter()
                   .any(|lead| plain.strip_prefix(lead).is_some_and(is_limit_name))
    }

    /// When the limit lifts, for one reported at `refused_at` (#368): the
    /// next time the clock in [`Self::resets`] reads its time of day, in the
    /// zone it names - "5:20pm (America/Los_Angeles)", "9am (Europe/Paris)".
    ///
    /// `None` when there is no reset time, or one in a shape this does not
    /// read (a date, a duration) or naming a zone the system does not know:
    /// a time it is not sure of is no time to send a held prompt at.
    #[must_use]
    pub fn lifts_at(&self, refused_at: SystemTime) -> Option<SystemTime> {
        let (clock, zone) = self.resets.as_deref()?.split_once('(')?;
        let zone = zone.trim().strip_suffix(')')?.trim();
        let (hour, minute) = parse_clock(clock.trim())?;
        let zone = jiff::tz::TimeZone::get(zone).ok()?;
        let refused = jiff::Timestamp::try_from(refused_at).ok()?
                                                           .to_zoned(zone.clone());
        let same_day = refused.date()
                              .at(hour, minute, 0, 0)
                              .to_zoned(zone.clone())
                              .ok()?;
        let same_day = SystemTime::from(same_day.timestamp());
        if same_day > refused_at {
            return Some(same_day);
        }
        // Only just past by our clock: the provider's runs behind, and the
        // reset is due now. Retry shortly rather than wait a day.
        if refused_at.duration_since(same_day)
                     .is_ok_and(|late| late <= LATE_RESET_GRACE)
        {
            return Some(refused_at + LATE_RESET_RETRY);
        }
        let tomorrow = refused.date()
                              .tomorrow()
                              .ok()?
                              .at(hour, minute, 0, 0)
                              .to_zoned(zone)
                              .ok()?;
        Some(tomorrow.timestamp().into())
    }
}

/// Whether a lead naming a stopping limit appears anywhere in `lower`. An
/// error wraps the notice in its own prefix ("acp error response ...:
/// You've hit your 5-hour limit"), so it is searched for, not anchored.
fn names_a_limit(lower: &str) -> bool {
    let plain = lower.replace('\u{2019}', "'");
    LIMIT_LEADS.iter().any(|lead| {
                          plain.match_indices(lead)
                               .any(|(at, _)| is_limit_name(&plain[at + lead.len()..]))
                      })
}

/// Whether `rest` - what follows a lead - opens with the name of a limit
/// that stops the session: a short name ending in "limit", up to the "·"
/// or the sentence's end. "your team's shared budget" is not one, and
/// neither is a sentence that only happens to end in "limit".
fn is_limit_name(rest: &str) -> bool {
    let name = rest.split(NAME_ENDS).next().unwrap_or_default().trim();
    name.len() <= MAX_LIMIT_NAME && name.ends_with("limit") && !NOT_A_STOP.contains(&name)
}

/// A 12-hour clock time - "5:20pm", "9am", "12:05 AM" - as a 24-hour hour
/// and minute.
fn parse_clock(clock: &str) -> Option<(i8, i8)> {
    let lower = clock.to_ascii_lowercase();
    let (digits, afternoon) = if let Some(digits) = lower.strip_suffix("pm") {
        (digits, true)
    }
    else {
        (lower.strip_suffix("am")?, false)
    };
    let (hour, minute) = match digits.trim().split_once(':') {
        Some((hour, minute)) => (hour.parse::<i8>().ok()?, minute.parse::<i8>().ok()?),
        None => (digits.trim().parse::<i8>().ok()?, 0),
    };
    if !(1..=HALF_DAY).contains(&hour) || !(0..60).contains(&minute) {
        return None;
    }
    let hour = match (hour, afternoon) {
        (HALF_DAY, false) => 0,
        (HALF_DAY, true) => HALF_DAY,
        (hour, true) => hour + HALF_DAY,
        (hour, false) => hour,
    };
    Some((hour, minute))
}

#[cfg(test)]
mod tests;
