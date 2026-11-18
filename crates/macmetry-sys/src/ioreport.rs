//! The private reporting framework (`libIOReport`) that Apple's own `powermetrics` uses for energy and residency.
//!
//! Its functions are looked up with `dlopen` and `dlsym` instead of linked, so a macOS version that renames or drops
//! one only switches off what depends on it. The framework is private and undocumented: what each channel means is
//! taken from reading it on real Macs, and anything unexpected becomes [`SysError::Unsupported`] or `None`.

use crate::cf::{CFTypeRef, Owned, string_from};
use crate::error::{Result, SysError};
use macmetry_core::PowerDraw;
use std::ffi::{CStr, c_void};
use std::sync::OnceLock;
use std::time::Instant;

type CopyChannelsInGroup = unsafe extern "C" fn(CFTypeRef, CFTypeRef, u64, u64, u64) -> CFTypeRef;
type CopyAllChannels = unsafe extern "C" fn(u64, u64) -> CFTypeRef;
type CreateSubscription = unsafe extern "C" fn(*const c_void, CFTypeRef, *mut CFTypeRef, u64, CFTypeRef) -> CFTypeRef;
type CreateSamples = unsafe extern "C" fn(CFTypeRef, CFTypeRef, CFTypeRef) -> CFTypeRef;
type CreateSamplesDelta = unsafe extern "C" fn(CFTypeRef, CFTypeRef, CFTypeRef) -> CFTypeRef;
type ChannelString = unsafe extern "C" fn(CFTypeRef) -> CFTypeRef;
type ChannelFormat = unsafe extern "C" fn(CFTypeRef) -> u8;
type SimpleValue = unsafe extern "C" fn(CFTypeRef, i32) -> i64;
type StateCount = unsafe extern "C" fn(CFTypeRef) -> i32;
type StateName = unsafe extern "C" fn(CFTypeRef, i32) -> CFTypeRef;
type StateResidency = unsafe extern "C" fn(CFTypeRef, i32) -> i64;

/// The functions this module uses.
struct Api {
    copy_channels_in_group: CopyChannelsInGroup,
    copy_all_channels: CopyAllChannels,
    create_subscription: CreateSubscription,
    create_samples: CreateSamples,
    create_samples_delta: CreateSamplesDelta,
    group: ChannelString,
    sub_group: ChannelString,
    name: ChannelString,
    unit_label: ChannelString,
    format: ChannelFormat,
    simple_value: SimpleValue,
    state_count: StateCount,
    state_name: StateName,
    state_residency: StateResidency,
}

/// Looks up one function.
///
/// # Safety
/// `T` must be the `extern "C"` function pointer type that the symbol really has.
unsafe fn symbol<T: Copy>(handle: *mut c_void, name: &CStr) -> Option<T> {
    debug_assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<*mut c_void>());
    // SAFETY: `handle` came from `dlopen` and `name` is a valid C string.
    let ptr = unsafe { libc::dlsym(handle, name.as_ptr()) };
    // SAFETY: the caller guarantees `T` is the symbol's pointer type, and the sizes match.
    (!ptr.is_null()).then(|| unsafe { std::mem::transmute_copy::<*mut c_void, T>(&ptr) })
}

fn api() -> Option<&'static Api> {
    static API: OnceLock<Option<Api>> = OnceLock::new();
    API.get_or_init(|| {
        // SAFETY: a valid C string path; the library stays loaded for the life of the process.
        let handle = unsafe { libc::dlopen(c"/usr/lib/libIOReport.dylib".as_ptr(), libc::RTLD_NOW) };
        if handle.is_null() {
            return None;
        }
        // SAFETY: each type below is the documented-by-use signature of its symbol (see the powermetrics imports in
        // docs/research/collectors.md); a missing symbol yields `None`.
        unsafe {
            Some(Api {
                copy_channels_in_group: symbol(handle, c"IOReportCopyChannelsInGroup")?,
                copy_all_channels: symbol(handle, c"IOReportCopyAllChannels")?,
                create_subscription: symbol(handle, c"IOReportCreateSubscription")?,
                create_samples: symbol(handle, c"IOReportCreateSamples")?,
                create_samples_delta: symbol(handle, c"IOReportCreateSamplesDelta")?,
                group: symbol(handle, c"IOReportChannelGetGroup")?,
                sub_group: symbol(handle, c"IOReportChannelGetSubGroup")?,
                name: symbol(handle, c"IOReportChannelGetChannelName")?,
                unit_label: symbol(handle, c"IOReportChannelGetUnitLabel")?,
                format: symbol(handle, c"IOReportChannelGetFormat")?,
                simple_value: symbol(handle, c"IOReportSimpleGetIntegerValue")?,
                state_count: symbol(handle, c"IOReportStateGetCount")?,
                state_name: symbol(handle, c"IOReportStateGetNameForIndex")?,
                state_residency: symbol(handle, c"IOReportStateGetResidency")?,
            })
        }
    })
    .as_ref()
}

/// One channel of a sample, copied out of CoreFoundation so parsers work on plain data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChannelReading {
    pub group: String,
    pub sub_group: String,
    pub name: String,
    /// The unit label of a simple channel, for example `mJ`, `uJ` or `nJ` for energy.
    pub unit: String,
    /// The value of a simple channel; 0 for a state channel.
    pub value: i64,
    /// The `(state name, residency)` pairs of a state channel.
    pub states: Vec<(String, i64)>,
}

/// A set of channels that can be sampled.
pub struct Subscription {
    api: &'static Api,
    subscription: Owned,
    channels: Owned,
}

impl Subscription {
    /// Subscribes to every channel of a group (and, when given, one subgroup of it).
    /// [`SysError::Unsupported`] when this Mac has no such group.
    pub fn new(group: &str, sub_group: Option<&str>) -> Result<Self> {
        let api = api().ok_or(SysError::Unsupported("Energy and frequency reporting"))?;
        let group_name = Owned::string(group).ok_or_else(|| SysError::parse("a channel group", "bad name"))?;
        let sub_name = sub_group.and_then(Owned::string);
        // SAFETY: the strings are valid CFStrings (or null for "any"); the call returns a new dictionary or null.
        let found = unsafe {
            (api.copy_channels_in_group)(
                group_name.as_ptr(),
                sub_name.as_ref().map_or(std::ptr::null(), Owned::as_ptr),
                0,
                0,
                0,
            )
        };
        // SAFETY: `found` is a new object the caller owns, or null.
        let found =
            unsafe { Owned::from_create(found) }.ok_or(SysError::Unsupported("Energy and frequency reporting"))?;
        Self::subscribe(api, &found)
    }

    /// Subscribes to everything the system reports. Used by the dump example; far heavier than a group.
    pub fn all() -> Result<Self> {
        let api = api().ok_or(SysError::Unsupported("Energy and frequency reporting"))?;
        // SAFETY: the call has no pointer arguments; it returns a new dictionary or null.
        let found = unsafe { Owned::from_create((api.copy_all_channels)(0, 0)) }
            .ok_or(SysError::Unsupported("Energy and frequency reporting"))?;
        Self::subscribe(api, &found)
    }

    fn subscribe(api: &'static Api, found: &Owned) -> Result<Self> {
        // The subscription call takes a mutable dictionary and fills in the channels it actually subscribed to.
        let desired =
            found.mutable_copy().ok_or_else(|| SysError::parse("the channel list", "it could not be copied"))?;
        let mut subscribed: CFTypeRef = std::ptr::null();
        // SAFETY: `desired` is a valid mutable dictionary and `subscribed` a valid out pointer.
        let subscription = unsafe {
            (api.create_subscription)(std::ptr::null(), desired.as_ptr(), &mut subscribed, 0, std::ptr::null())
        };
        // SAFETY: both results are new objects the caller owns, or null.
        let (subscription, channels) = unsafe { (Owned::from_create(subscription), Owned::from_create(subscribed)) };
        match (subscription, channels) {
            (Some(subscription), Some(channels)) => Ok(Self { api, subscription, channels }),
            _ => Err(SysError::Unsupported("Energy and frequency reporting")),
        }
    }

    /// Reads the counters now.
    pub fn sample(&self) -> Result<Owned> {
        // SAFETY: the subscription and channel objects are valid for the life of `self`.
        let sample =
            unsafe { (self.api.create_samples)(self.subscription.as_ptr(), self.channels.as_ptr(), std::ptr::null()) };
        // SAFETY: a new object the caller owns, or null.
        unsafe { Owned::from_create(sample) }
            .ok_or_else(|| SysError::parse("a channel sample", "the system returned none"))
    }

    /// What changed between two samples.
    pub fn delta(&self, previous: &Owned, current: &Owned) -> Result<Owned> {
        // SAFETY: both samples are valid objects.
        let delta = unsafe { (self.api.create_samples_delta)(previous.as_ptr(), current.as_ptr(), std::ptr::null()) };
        // SAFETY: a new object the caller owns, or null.
        unsafe { Owned::from_create(delta) }
            .ok_or_else(|| SysError::parse("a channel delta", "the system returned none"))
    }

    /// Copies every channel of a sample or delta into plain structures.
    pub fn read(&self, sample: &Owned) -> Vec<ChannelReading> {
        let api = self.api;
        let Some(list) = sample.dict_ptr("IOReportChannels") else { return Vec::new() };
        // SAFETY: `list` is borrowed from `sample`, which outlives this call. Retaining it lets `Owned` manage it.
        let Some(list) = (unsafe { Owned::retain(list) }) else { return Vec::new() };
        list.array_items()
            .into_iter()
            .map(|channel| {
                // SAFETY: `channel` is borrowed from `list`; every accessor takes a channel dictionary and returns a
                // borrowed string (or null), copied immediately.
                unsafe {
                    let text = |f: ChannelString| string_from(f(channel)).unwrap_or_default();
                    let mut reading = ChannelReading {
                        group: text(api.group),
                        sub_group: text(api.sub_group),
                        name: text(api.name),
                        unit: text(api.unit_label),
                        ..Default::default()
                    };
                    match (api.format)(channel) {
                        // kIOReportFormatState
                        2 => {
                            let count = (api.state_count)(channel).clamp(0, 256);
                            reading.states = (0..count)
                                .map(|i| {
                                    (
                                        string_from((api.state_name)(channel, i)).unwrap_or_default(),
                                        (api.state_residency)(channel, i),
                                    )
                                })
                                .collect();
                        }
                        // kIOReportFormatSimple
                        1 => reading.value = (api.simple_value)(channel, 0),
                        _ => {}
                    }
                    reading
                }
            })
            .collect()
    }
}

/// Watts from an energy counter: `value` in the channel's `unit`, accumulated over `seconds`.
pub fn watts(value: i64, unit: &str, seconds: f64) -> Option<f64> {
    if seconds <= 0.0 || value < 0 {
        return None;
    }
    let joules_per_unit = match unit {
        "J" => 1.0,
        "mJ" => 1e-3,
        "uJ" | "µJ" => 1e-6,
        "nJ" => 1e-9,
        _ => return None,
    };
    Some(value as f64 * joules_per_unit / seconds)
}

/// Power by component from the `Energy Model` channels of one delta. Pure, so it is tested with made-up channels.
///
/// The CPU figure is the channel called `CPU Energy` when there is one, otherwise the sum of the per-cluster
/// `… CPU Energy` channels, so a chip that reports both is not counted twice.
pub fn power_from(channels: &[ChannelReading], seconds: f64) -> PowerDraw {
    let in_watts = |c: &ChannelReading| watts(c.value, &c.unit, seconds);
    let named = |exact: &str| channels.iter().find(|c| c.name == exact).and_then(in_watts);
    let sum = |keep: &dyn Fn(&str) -> bool| {
        let parts: Vec<f64> = channels.iter().filter(|c| keep(&c.name)).filter_map(in_watts).collect();
        (!parts.is_empty()).then(|| parts.iter().sum())
    };
    PowerDraw {
        cpu_w: named("CPU Energy").or_else(|| sum(&|n| n.ends_with("CPU Energy"))),
        gpu_w: named("GPU Energy").or_else(|| sum(&|n| n.starts_with("GPU") && n.ends_with("Energy"))),
        ane_w: sum(&|n| n.starts_with("ANE")),
        dram_w: sum(&|n| n.starts_with("DRAM")),
        system_w: None,
    }
}

/// The size of one count of an Intel energy counter, in joules, from `MSR_RAPL_POWER_UNIT`: bits 12 to 8 hold `n`
/// and a count is 1/2^n of a joule.
pub fn rapl_joules_per_count(power_unit_register: i64) -> Option<f64> {
    let exponent = (power_unit_register >> 8) & 0x1f;
    (exponent > 0).then(|| 1.0 / (1u64 << exponent) as f64)
}

/// How far a 32-bit energy counter moved, allowing for it wrapping around.
pub fn rapl_counts_between(previous: i64, current: i64) -> u64 {
    (current.wrapping_sub(previous) as u64) & 0xffff_ffff
}

/// Package power of an Intel CPU from two readings of the RAPL energy counters. Pure, so it is tested with the
/// counters of a real Mac and with made-up ones.
///
/// `CPU` is the whole package (cores, caches and the uncore); `GPU` is the integrated graphics (`PP1`), reported only
/// when it used energy. DRAM is left out because server parts count it in a different unit.
pub fn rapl_power(previous: &[ChannelReading], current: &[ChannelReading], seconds: f64) -> PowerDraw {
    let value = |readings: &[ChannelReading], name: &str| readings.iter().find(|c| c.name == name).map(|c| c.value);
    let Some(joules) = value(current, "MSR_IA32_PKG_POWER_SKU_UNIT").and_then(rapl_joules_per_count) else {
        return PowerDraw::default();
    };
    if seconds <= 0.0 {
        return PowerDraw::default();
    }
    let watts = |name: &str| {
        let (before, now) = (value(previous, name)?, value(current, name)?);
        Some(rapl_counts_between(before, now) as f64 * joules / seconds)
    };
    PowerDraw {
        cpu_w: watts("MSR_IA32_PKG_ENERGY_STATUS"),
        gpu_w: watts("MSR_IA32_PP1_ENERGY_STATUS").filter(|w| *w > 0.0),
        ..Default::default()
    }
}

/// Measures CPU package power on an Intel Mac from the RAPL counters that `pmtelemetry_cpu` publishes.
pub struct IntelEnergyCollector {
    subscription: Subscription,
    previous: Vec<ChannelReading>,
    taken: Instant,
}

impl IntelEnergyCollector {
    /// [`SysError::Unsupported`] when the Mac does not publish the counters (Apple silicon, virtual machines).
    pub fn new() -> Result<Self> {
        let subscription = Subscription::new("pmtelemetry_cpu", Some("msr"))?;
        let previous = subscription.read(&subscription.sample()?);
        if !previous.iter().any(|c| c.name == "MSR_IA32_PKG_ENERGY_STATUS") {
            return Err(SysError::Unsupported("Intel energy counters"));
        }
        Ok(Self { subscription, previous, taken: Instant::now() })
    }

    pub fn sample(&mut self) -> Result<PowerDraw> {
        let current = self.subscription.read(&self.subscription.sample()?);
        let power = rapl_power(&self.previous, &current, self.taken.elapsed().as_secs_f64());
        self.previous = current;
        self.taken = Instant::now();
        Ok(power)
    }
}

/// Measures power on Apple silicon from the `Energy Model` group.
pub struct EnergyCollector {
    subscription: Subscription,
    previous: Owned,
    taken: Instant,
}

impl EnergyCollector {
    /// [`SysError::Unsupported`] on a Mac that does not publish an `Energy Model` (Intel).
    pub fn new() -> Result<Self> {
        let subscription = Subscription::new("Energy Model", None)?;
        let previous = subscription.sample()?;
        Ok(Self { subscription, previous, taken: Instant::now() })
    }

    /// Power since the previous call, using the real time between the two readings.
    pub fn sample(&mut self) -> Result<PowerDraw> {
        let current = self.subscription.sample()?;
        let seconds = self.taken.elapsed().as_secs_f64();
        let delta = self.subscription.delta(&self.previous, &current)?;
        let power = power_from(&self.subscription.read(&delta), seconds);
        self.previous = current;
        self.taken = Instant::now();
        Ok(power)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(name: &str, unit: &str, value: i64) -> ChannelReading {
        ChannelReading {
            group: "Energy Model".into(),
            name: name.into(),
            unit: unit.into(),
            value,
            ..Default::default()
        }
    }

    #[test]
    fn converts_energy_to_watts() {
        assert_eq!(watts(2_000, "mJ", 1.0), Some(2.0));
        assert_eq!(watts(5_000_000, "uJ", 2.0), Some(2.5));
        assert_eq!(watts(3_000_000_000, "nJ", 1.0), Some(3.0));
        assert_eq!(watts(7, "J", 1.0), Some(7.0));
        assert_eq!(watts(1, "furlongs", 1.0), None, "an unknown unit is not guessed");
        assert_eq!(watts(1, "mJ", 0.0), None, "no time, no rate");
        assert_eq!(watts(-5, "mJ", 1.0), None, "a counter that went backwards");
    }

    #[test]
    fn splits_power_by_component() {
        let channels = [
            energy("CPU Energy", "mJ", 4_000),
            energy("GPU Energy", "mJ", 1_000),
            energy("ANE", "mJ", 250),
            energy("DRAM", "mJ", 500),
            energy("ECPU Energy", "mJ", 999_999),
        ];
        let p = power_from(&channels, 1.0);
        assert_eq!(p.cpu_w, Some(4.0), "the total channel wins over the per-cluster one");
        assert_eq!(p.gpu_w, Some(1.0));
        assert_eq!(p.ane_w, Some(0.25));
        assert_eq!(p.dram_w, Some(0.5));
        assert_eq!(p.system_w, None);
    }

    #[test]
    fn sums_the_clusters_when_there_is_no_total() {
        let channels = [energy("ECPU Energy", "mJ", 1_000), energy("PCPU Energy", "mJ", 3_000)];
        assert_eq!(power_from(&channels, 2.0).cpu_w, Some(2.0));
    }

    #[test]
    fn no_channels_means_no_power() {
        assert_eq!(power_from(&[], 1.0), PowerDraw::default());
    }

    const RAPL: &str = include_str!("../tests/fixtures/intel-macpro7-1/ioreport-rapl.jsonl");

    fn real_rapl() -> Vec<ChannelReading> {
        RAPL.lines()
            .map(|line| {
                let v: serde_json::Value = serde_json::from_str(line).unwrap();
                ChannelReading {
                    group: v["group"].as_str().unwrap().into(),
                    sub_group: v["subgroup"].as_str().unwrap().into(),
                    name: v["name"].as_str().unwrap().into(),
                    unit: v["unit"].as_str().unwrap().into(),
                    value: v["value"].as_i64().unwrap(),
                    ..Default::default()
                }
            })
            .collect()
    }

    fn advanced(readings: &[ChannelReading], name: &str, counts: i64) -> Vec<ChannelReading> {
        readings
            .iter()
            .cloned()
            .map(|mut c| {
                if c.name == name {
                    c.value = (c.value + counts) & 0xffff_ffff;
                }
                c
            })
            .collect()
    }

    #[test]
    fn reads_the_energy_unit_of_a_real_xeon() {
        let unit = real_rapl().into_iter().find(|c| c.name == "MSR_IA32_PKG_POWER_SKU_UNIT").unwrap().value;
        // 658947 is 0xA0E03: the energy unit field (bits 12 to 8) is 14, so a count is 2^-14 J (61 microjoules), the
        // usual value for a Xeon. Client chips use 2^-16.
        assert_eq!(rapl_joules_per_count(unit), Some(1.0 / 16_384.0));
        assert_eq!(rapl_joules_per_count(16 << 8), Some(1.0 / 65_536.0), "a client chip");
        assert_eq!(rapl_joules_per_count(0), None);
    }

    #[test]
    fn computes_package_power_from_counters() {
        let before = real_rapl();
        // 819,200 counts of 2^-14 J is 50 J; in one second that is 50 W.
        let after = advanced(&before, "MSR_IA32_PKG_ENERGY_STATUS", 819_200);
        let power = rapl_power(&before, &after, 1.0);
        assert!((power.cpu_w.unwrap() - 50.0).abs() < 1e-9);
        assert!((rapl_power(&before, &after, 2.0).cpu_w.unwrap() - 25.0).abs() < 1e-9);
        assert_eq!(power.gpu_w, None, "the integrated graphics used no energy");
    }

    #[test]
    fn the_32_bit_counter_wraps() {
        assert_eq!(rapl_counts_between(0xffff_fff0, 0x10), 0x20);
        let mut before = real_rapl();
        for c in &mut before {
            if c.name == "MSR_IA32_PKG_ENERGY_STATUS" {
                c.value = 0xffff_ff00;
            }
        }
        let after = advanced(&before, "MSR_IA32_PKG_ENERGY_STATUS", 16_384);
        assert!((rapl_power(&before, &after, 1.0).cpu_w.unwrap() - 1.0).abs() < 1e-9, "1 J in 1 s across the wrap");
    }

    #[test]
    fn missing_counters_or_time_give_no_power() {
        assert_eq!(rapl_power(&[], &[], 1.0), PowerDraw::default());
        let real = real_rapl();
        assert_eq!(rapl_power(&real, &real, 0.0), PowerDraw::default());
        assert_eq!(rapl_power(&real, &real, 1.0).cpu_w, Some(0.0), "no energy used is 0 W, not unknown");
    }

    #[test]
    fn this_intel_mac_reports_package_power() {
        match IntelEnergyCollector::new() {
            Ok(mut collector) => {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let cpu = collector.sample().unwrap().cpu_w.expect("a package power");
                assert!((0.0..=1000.0).contains(&cpu), "{cpu} W is not plausible");
            }
            Err(e) => assert!(e.is_unsupported(), "unexpected error: {e}"),
        }
    }

    #[test]
    fn this_mac_either_reports_energy_or_says_so() {
        match EnergyCollector::new() {
            Ok(mut collector) => {
                std::thread::sleep(std::time::Duration::from_millis(200));
                let power = collector.sample().unwrap();
                assert!(power.cpu_w.is_none_or(|w| (0.0..=1000.0).contains(&w)));
            }
            Err(e) => assert!(e.is_unsupported(), "unexpected error: {e}"),
        }
    }
}
