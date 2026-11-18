//! Threshold alerts: a rule watches one metric, and an engine turns snapshots into "fired" and "resolved" events.
//!
//! The engine is a pure state machine driven by the snapshot's own timestamp, so it is tested without a clock and
//! behaves the same whether it runs in the app or in a test. It never notifies anyone itself; the app decides what to
//! do with an [`Event`].

use crate::metric::{Metric, Unit};
use crate::model::{MemoryPressure, Snapshot, ThermalPressure};
use crate::units::format_bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How the metric is compared with the threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparator {
    Above,
    Below,
}

impl Comparator {
    fn holds(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::Above => value > threshold,
            Self::Below => value < threshold,
        }
    }
}

/// "Tell me when `metric` has been `comparator` `threshold` for `for_secs` seconds."
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    /// A short stable name such as `disk_low`, used to match a rule to its state across edits.
    pub id: String,
    pub name: String,
    pub metric: Metric,
    pub comparator: Comparator,
    pub threshold: f64,
    /// The condition must hold this long before the alert fires, so a spike does not.
    pub for_secs: u32,
    /// After firing, the same rule stays quiet for this long even if it resolves and fires again.
    pub cooldown_secs: u32,
    pub enabled: bool,
}

const MAX_FOR_SECS: u32 = 86_400;
const MAX_COOLDOWN_SECS: u32 = 7 * 86_400;

impl Rule {
    /// Checks a rule that came from outside (a settings file, a form). The message says what to change.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.len() > 40
            || !self.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err("A rule's id must be 1 to 40 letters, digits, dashes or underscores.".into());
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
            return Err("A rule's name must be 1 to 80 characters.".into());
        }
        if !self.threshold.is_finite() {
            return Err("The threshold must be a number.".into());
        }
        if self.for_secs > MAX_FOR_SECS {
            return Err("\"For how long\" can be at most one day.".into());
        }
        if self.cooldown_secs > MAX_COOLDOWN_SECS {
            return Err("The cooldown can be at most seven days.".into());
        }
        Ok(())
    }

    /// The text of a notification for this rule at `value`, in English. The app may translate it.
    pub fn message(&self, value: f64) -> String {
        // A level (memory or thermal pressure) has a threshold between two states, such as 1.5; saying "above
        // warning" for it would be wrong. The state itself is the message.
        if self.metric.unit() == Unit::Level {
            return format!("{} is {}", self.metric.label(), format_value(self.metric, value));
        }
        let how = match self.comparator {
            Comparator::Above => "above",
            Comparator::Below => "below",
        };
        format!(
            "{} is {} ({how} {})",
            self.metric.label(),
            format_value(self.metric, value),
            format_value(self.metric, self.threshold)
        )
    }
}

/// A value with its unit, for people.
pub fn format_value(metric: Metric, value: f64) -> String {
    match metric.unit() {
        Unit::Percent => format!("{value:.0}%"),
        Unit::Celsius => format!("{value:.0} °C"),
        Unit::Rpm => format!("{value:.0} rpm"),
        Unit::Watts => format!("{value:.1} W"),
        Unit::Bytes => format_bytes(value.max(0.0) as u64),
        Unit::BytesPerSecond => format!("{}/s", format_bytes(value.max(0.0) as u64)),
        Unit::PerSecond => format!("{value:.0}/s"),
        Unit::Level => match metric {
            Metric::MemoryPressure => match value.round() as i64 {
                0 => "normal".to_string(),
                1 => "warning".to_string(),
                _ => "critical".to_string(),
            },
            Metric::ThermalPressure => match value.round() as i64 {
                0 => "nominal".to_string(),
                1 => "moderate".to_string(),
                2 => "heavy".to_string(),
                3 => "trapping".to_string(),
                _ => "sleeping".to_string(),
            },
            _ => format!("{value:.2}"),
        },
    }
}

/// The rules offered the first time. Only the two that protect the Mac are on; the rest are one click away.
pub fn presets() -> Vec<Rule> {
    let rule = |id: &str, name: &str, metric, comparator, threshold, for_secs, cooldown_secs, enabled| Rule {
        id: id.into(),
        name: name.into(),
        metric,
        comparator,
        threshold,
        for_secs,
        cooldown_secs,
        enabled,
    };
    vec![
        rule("disk_low", "Startup disk almost full", Metric::DiskFreePct, Comparator::Below, 10.0, 60, 6 * 3600, true),
        rule("temp_critical", "Very hot sensor", Metric::TempMax, Comparator::Above, 95.0, 30, 900, true),
        rule("cpu_high", "CPU busy for a while", Metric::CpuTotal, Comparator::Above, 90.0, 120, 900, false),
        rule(
            "memory_critical",
            "Memory pressure is critical",
            Metric::MemoryPressure,
            Comparator::Above,
            f64::from(MemoryPressure::Warning.level()) + 0.5,
            30,
            900,
            false,
        ),
        rule(
            "thermal_heavy",
            "The Mac is slowing down to cool off",
            Metric::ThermalPressure,
            Comparator::Above,
            f64::from(ThermalPressure::Moderate.level()) + 0.5,
            30,
            900,
            false,
        ),
        rule("battery_low", "Battery is low", Metric::BatteryPercent, Comparator::Below, 15.0, 0, 1800, false),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Fired,
    Resolved,
}

/// Something the app should tell the person about.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub rule_id: String,
    pub kind: EventKind,
    pub value: f64,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Ok,
    /// The condition holds, and has since this time.
    Pending(u64),
    /// The alert fired. `notified` is false when the cooldown kept it quiet.
    Firing {
        notified: bool,
    },
}

#[derive(Debug, Clone, Copy)]
struct RuleState {
    state: State,
    last_fired_ms: Option<u64>,
}

/// Evaluates rules against snapshots.
#[derive(Debug, Default)]
pub struct Engine {
    rules: Vec<Rule>,
    states: HashMap<String, RuleState>,
}

impl Engine {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules, states: HashMap::new() }
    }

    /// Replaces the rules. A rule that keeps its id keeps its state, so editing a threshold does not re-fire an
    /// alert that is already active; a removed rule is forgotten.
    pub fn set_rules(&mut self, rules: Vec<Rule>) {
        self.states.retain(|id, _| rules.iter().any(|r| &r.id == id));
        self.rules = rules;
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// The ids of the rules that are firing right now.
    pub fn firing(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self
            .states
            .iter()
            .filter(|(_, s)| matches!(s.state, State::Firing { .. }))
            .map(|(id, _)| id.as_str())
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Advances every rule by one snapshot and returns what changed.
    pub fn evaluate(&mut self, snapshot: &Snapshot) -> Vec<Event> {
        let now = snapshot.timestamp_ms;
        let mut events = Vec::new();
        for rule in &self.rules {
            let entry =
                self.states.entry(rule.id.clone()).or_insert(RuleState { state: State::Ok, last_fired_ms: None });
            if !rule.enabled {
                // Turning a rule off while it is firing resolves it quietly: there is nothing left to report.
                entry.state = State::Ok;
                continue;
            }
            // A metric the Mac does not provide never breaks a rule.
            let value = rule.metric.value(snapshot);
            let violating = value.is_some_and(|v| rule.comparator.holds(v, rule.threshold));
            let value = value.unwrap_or(0.0);
            entry.state = match (entry.state, violating) {
                (State::Ok, true) => advance(rule, entry, now, now, value, &mut events),
                (State::Pending(since), true) => advance(rule, entry, since, now, value, &mut events),
                (State::Pending(_), false) => State::Ok,
                (State::Firing { notified }, false) => {
                    if notified {
                        events.push(Event { rule_id: rule.id.clone(), kind: EventKind::Resolved, value, at_ms: now });
                    }
                    State::Ok
                }
                (state @ State::Firing { .. }, true) => state,
                (State::Ok, false) => State::Ok,
            };
        }
        events
    }
}

/// The condition holds since `since`; fire when it has held long enough.
fn advance(rule: &Rule, entry: &mut RuleState, since: u64, now: u64, value: f64, events: &mut Vec<Event>) -> State {
    // `saturating_sub` so a clock that moves backwards just waits instead of wrapping into a huge duration.
    if now.saturating_sub(since) < u64::from(rule.for_secs) * 1000 {
        return State::Pending(since);
    }
    let quiet = entry.last_fired_ms.is_some_and(|last| now.saturating_sub(last) < u64::from(rule.cooldown_secs) * 1000);
    if quiet {
        return State::Firing { notified: false };
    }
    entry.last_fired_ms = Some(now);
    events.push(Event { rule_id: rule.id.clone(), kind: EventKind::Fired, value, at_ms: now });
    State::Firing { notified: true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CpuUsage, Volume};

    fn rule(for_secs: u32, cooldown_secs: u32) -> Rule {
        Rule {
            id: "cpu".into(),
            name: "CPU".into(),
            metric: Metric::CpuTotal,
            comparator: Comparator::Above,
            threshold: 90.0,
            for_secs,
            cooldown_secs,
            enabled: true,
        }
    }

    fn at(secs: u64, cpu: f64) -> Snapshot {
        Snapshot {
            timestamp_ms: 1_000_000 + secs * 1000,
            cpu: Some(CpuUsage { total_pct: cpu, ..Default::default() }),
            ..Default::default()
        }
    }

    fn kinds(events: &[Event]) -> Vec<EventKind> {
        events.iter().map(|e| e.kind).collect()
    }

    #[test]
    fn fires_only_after_the_condition_held_long_enough() {
        let mut engine = Engine::new(vec![rule(10, 0)]);
        assert!(engine.evaluate(&at(0, 95.0)).is_empty(), "a first violation only starts the clock");
        assert!(engine.evaluate(&at(9, 95.0)).is_empty(), "9 seconds is not 10");
        let fired = engine.evaluate(&at(10, 96.0));
        assert_eq!(kinds(&fired), [EventKind::Fired]);
        assert_eq!(fired[0].value, 96.0, "the event carries the value that tipped it");
        assert_eq!(engine.firing(), ["cpu"]);
        assert!(engine.evaluate(&at(11, 99.0)).is_empty(), "it does not fire again while it stays active");
    }

    #[test]
    fn a_spike_that_ends_early_never_fires() {
        let mut engine = Engine::new(vec![rule(10, 0)]);
        engine.evaluate(&at(0, 95.0));
        engine.evaluate(&at(5, 50.0));
        // The clock restarted: 12 seconds after the *first* violation is not 10 seconds of a continuous one.
        assert!(engine.evaluate(&at(8, 95.0)).is_empty());
        assert!(engine.evaluate(&at(12, 95.0)).is_empty());
        assert_eq!(kinds(&engine.evaluate(&at(18, 95.0))), [EventKind::Fired]);
    }

    #[test]
    fn resolves_when_the_condition_clears() {
        let mut engine = Engine::new(vec![rule(0, 0)]);
        assert_eq!(kinds(&engine.evaluate(&at(0, 95.0))), [EventKind::Fired], "no waiting when for_secs is 0");
        let resolved = engine.evaluate(&at(5, 10.0));
        assert_eq!(kinds(&resolved), [EventKind::Resolved]);
        assert_eq!(resolved[0].value, 10.0);
        assert!(engine.firing().is_empty());
    }

    #[test]
    fn the_threshold_itself_does_not_trigger() {
        let mut engine = Engine::new(vec![rule(0, 0)]);
        assert!(engine.evaluate(&at(0, 90.0)).is_empty(), "above means strictly above");
    }

    #[test]
    fn the_cooldown_keeps_a_flapping_alert_quiet() {
        let mut engine = Engine::new(vec![rule(0, 60)]);
        assert_eq!(kinds(&engine.evaluate(&at(0, 95.0))), [EventKind::Fired]);
        assert_eq!(kinds(&engine.evaluate(&at(1, 10.0))), [EventKind::Resolved]);
        assert!(engine.evaluate(&at(2, 95.0)).is_empty(), "inside the cooldown: active, but silent");
        assert_eq!(engine.firing(), ["cpu"]);
        assert!(engine.evaluate(&at(3, 10.0)).is_empty(), "and it clears silently too, since nobody was told");
        assert_eq!(kinds(&engine.evaluate(&at(61, 95.0))), [EventKind::Fired], "after the cooldown it may speak again");
    }

    #[test]
    fn a_metric_the_mac_lacks_never_fires() {
        let battery =
            Rule { metric: Metric::BatteryPercent, comparator: Comparator::Below, threshold: 15.0, ..rule(0, 0) };
        let mut engine = Engine::new(vec![battery]);
        // A desktop has no battery: "below 15%" must not read the missing value as 0%.
        assert!(engine.evaluate(&at(0, 50.0)).is_empty());
        assert!(engine.firing().is_empty());
    }

    #[test]
    fn below_works_too() {
        let disk = Rule { metric: Metric::DiskFreePct, comparator: Comparator::Below, threshold: 10.0, ..rule(0, 0) };
        let mut engine = Engine::new(vec![disk]);
        let full = Snapshot {
            timestamp_ms: 5,
            disks: Some(crate::model::DiskStats {
                volumes: vec![Volume { mount_point: "/".into(), total: 100, free: 5, ..Default::default() }],
                io: vec![],
            }),
            ..Default::default()
        };
        assert_eq!(kinds(&engine.evaluate(&full)), [EventKind::Fired]);
    }

    #[test]
    fn a_disabled_rule_is_silent_and_forgets_it_was_firing() {
        let mut on = rule(0, 0);
        let mut engine = Engine::new(vec![on.clone()]);
        engine.evaluate(&at(0, 95.0));
        on.enabled = false;
        engine.set_rules(vec![on]);
        assert!(engine.evaluate(&at(1, 95.0)).is_empty());
        assert!(engine.firing().is_empty());
    }

    #[test]
    fn editing_a_threshold_keeps_an_active_alert_active() {
        let mut engine = Engine::new(vec![rule(0, 0)]);
        engine.evaluate(&at(0, 95.0));
        engine.set_rules(vec![Rule { threshold: 80.0, ..rule(0, 0) }]);
        assert!(engine.evaluate(&at(1, 95.0)).is_empty(), "no second notification for the same incident");
        assert_eq!(engine.firing(), ["cpu"]);
        engine.set_rules(vec![]);
        assert!(engine.firing().is_empty(), "a removed rule is forgotten");
    }

    #[test]
    fn a_clock_that_goes_backwards_does_not_fire_early() {
        let mut engine = Engine::new(vec![rule(10, 0)]);
        engine.evaluate(&at(100, 95.0));
        assert!(engine.evaluate(&at(50, 95.0)).is_empty());
    }

    #[test]
    fn validates_rules_from_outside() {
        assert!(rule(0, 0).validate().is_ok());
        assert!(Rule { id: "bad id!".into(), ..rule(0, 0) }.validate().is_err());
        assert!(Rule { id: String::new(), ..rule(0, 0) }.validate().is_err());
        assert!(Rule { name: "  ".into(), ..rule(0, 0) }.validate().is_err());
        assert!(Rule { threshold: f64::NAN, ..rule(0, 0) }.validate().is_err());
        assert!(Rule { for_secs: 100_000, ..rule(0, 0) }.validate().is_err());
        assert!(Rule { cooldown_secs: 10_000_000, ..rule(0, 0) }.validate().is_err());
    }

    #[test]
    fn presets_are_valid_unique_and_conservative() {
        let presets = presets();
        assert!(presets.iter().all(|r| r.validate().is_ok()));
        let ids: std::collections::HashSet<_> = presets.iter().map(|r| &r.id).collect();
        assert_eq!(ids.len(), presets.len());
        let enabled: Vec<_> = presets.iter().filter(|r| r.enabled).map(|r| r.id.as_str()).collect();
        assert_eq!(enabled, ["disk_low", "temp_critical"], "only the two that protect the Mac start on");
    }

    #[test]
    fn writes_messages_with_units() {
        let r = rule(0, 0);
        assert_eq!(r.message(93.4), "CPU usage is 93% (above 90%)");
        let temp = Rule { metric: Metric::TempMax, threshold: 95.0, ..rule(0, 0) };
        assert_eq!(temp.message(97.0), "Hottest sensor is 97 °C (above 95 °C)");
        let mem = Rule { metric: Metric::MemoryPressure, threshold: 1.5, ..rule(0, 0) };
        assert_eq!(mem.message(2.0), "Memory pressure is critical", "a level reads as its state, not as 'above 1.5'");
        let thermal = Rule { metric: Metric::ThermalPressure, threshold: 1.5, ..rule(0, 0) };
        assert_eq!(thermal.message(2.0), "Thermal pressure is heavy");
    }

    #[test]
    fn rules_survive_a_round_trip_through_json() {
        let json = serde_json::to_string(&presets()).unwrap();
        let back: Vec<Rule> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, presets());
        assert!(json.contains("\"disk.free_pct\""), "metrics are stored by their stable key");
    }
}
