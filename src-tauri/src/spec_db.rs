//! Embedded device database.
//!
//! `specs.json` holds the service profile for each supported printer model.
//! It keeps only what the bench needs:
//!
//! * one record per printer model, pointing at one or more spec keys
//!   (`EPSON`, `XP530`, `EP13`, `ESXX`, ...);
//! * one record per spec key with the `<service>` block, the waste counters
//!   and the `<reset>` address/value pairs.
//!
//! A spec chain is merged in order: `service` comes from the first key that
//! has one, counters and reset likewise.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::protocol::{Counter, Entry, Service, Spec};

const SPECS_JSON: &str = include_str!("specs.json");

#[derive(Debug, Deserialize)]
struct Database {
    version: String,
    printers: Vec<PrinterRecord>,
    specs: HashMap<String, RawSpec>,
}

#[derive(Debug, Deserialize)]
struct PrinterRecord {
    title: String,
    short: String,
    model: String,
    #[serde(default)]
    specs: Vec<String>,
    #[serde(default)]
    class: String,
    #[serde(default)]
    brand: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawSpec {
    #[serde(default)]
    service: Option<RawService>,
    #[serde(default)]
    counters: Vec<RawCounter>,
    #[serde(default)]
    reset: Vec<[u16; 2]>,
    #[serde(default)]
    memory: Option<[u16; 2]>,
}

#[derive(Debug, Deserialize)]
struct RawService {
    readlen: u16,
    sendlen: u16,
    #[serde(default)]
    factory: Vec<u8>,
    #[serde(default)]
    keyword: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct RawCounter {
    #[serde(default)]
    entries: Vec<RawEntry>,
    #[serde(default)]
    max: u32,
}

#[derive(Debug, Deserialize)]
struct RawEntry {
    #[serde(default)]
    addrs: Vec<u16>,
    #[serde(default)]
    mask: Option<u16>,
    #[serde(default)]
    shift: Option<i16>,
    #[serde(default, rename = "filterMax")]
    filter_max: Option<u32>,
}

/// A printer model as listed in the device database.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub key: String,
    pub title: String,
    pub short: String,
    pub model: String,
    pub chain: Vec<String>,
    pub brand: String,
    pub class: String,
    pub has_service: bool,
    pub counters: usize,
    /// Number of `(address, value)` pairs in the `<reset>` table.
    pub reset_pairs: usize,
}

/// A resolved waste-ink profile for one printer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    /// Spec keys that contributed, in chain order.
    pub chain: Vec<String>,
    pub model: String,
    /// Two byte eeprom token, low byte first as it goes on the wire.
    pub factory: Vec<u8>,
    /// 1284.4 service name used by GetSocketID.
    pub keyword: String,
    pub read_len: u16,
    pub send_len: u16,
    pub counters: usize,
    pub reset_pairs: usize,
    pub memory: Option<[u16; 2]>,
    /// Full protocol description, already merged.
    #[serde(skip)]
    pub spec: Spec,
}

fn database() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(|| {
        serde_json::from_str(SPECS_JSON).expect("embedded specs.json must parse")
    })
}

fn convert_service(raw: &RawService) -> Service {
    let factory = match raw.factory.as_slice() {
        [lo, hi, ..] => u16::from_le_bytes([*lo, *hi]),
        [lo] => *lo as u16,
        _ => 0,
    };
    Service {
        read_len: raw.readlen,
        send_len: raw.sendlen,
        factory,
        write_key: raw.keyword.clone(),
    }
}

fn convert_counter(raw: &RawCounter) -> Counter {
    Counter {
        entries: raw
            .entries
            .iter()
            .map(|entry| Entry {
                addrs: entry.addrs.clone(),
                mask: entry.mask,
                shift: entry.shift,
                filter_max: entry.filter_max,
            })
            .collect(),
        max: raw.max,
    }
}

/// Merge a spec chain: the first key with a `<service>` wins, the first with
/// counters/reset wins, and so on.
fn merge(chain: &[String]) -> Option<DeviceProfile> {
    let db = database();
    let mut spec = Spec::default();
    let mut memory = None;
    let mut used = Vec::new();
    let mut have_service = false;
    let mut have_counters = false;
    let mut have_reset = false;

    for key in chain {
        let Some(raw) = db.specs.get(key) else {
            continue;
        };
        if !have_service {
            if let Some(service) = &raw.service {
                spec.service = convert_service(service);
                spec.id = key.clone();
                have_service = true;
                used.push(key.clone());
            }
        }
        if !have_counters && !raw.counters.is_empty() {
            spec.counters = raw.counters.iter().map(convert_counter).collect();
            have_counters = true;
            used.push(key.clone());
        }
        if !have_reset && !raw.reset.is_empty() {
            spec.reset = raw.reset.iter().map(|pair| (pair[0], pair[1])).collect();
            have_reset = true;
            used.push(key.clone());
        }
        if memory.is_none() {
            if let Some(bounds) = raw.memory {
                memory = Some(bounds);
            }
        }
    }

    used.sort();
    used.dedup();
    if !spec.is_usable() {
        return None;
    }
    Some(DeviceProfile {
        chain: used,
        model: String::new(),
        factory: spec.service.factory.to_le_bytes().to_vec(),
        keyword: String::from_utf8_lossy(&spec.service.write_key)
            .trim_end_matches('\0')
            .to_string(),
        read_len: spec.service.read_len,
        send_len: spec.service.send_len,
        counters: spec.counters.len(),
        reset_pairs: spec.reset.len(),
        memory,
        spec,
    })
}

fn normalise(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace(' ', "")
        .replace('_', "")
        .trim()
        .to_string()
}

/// Look a printer up by the `MDL:` string from its IEEE 1284 device ID
/// (`XP-530 Series`), falling back to a fuzzy title match.
pub fn resolve(model: &str) -> Option<DeviceProfile> {
    let db = database();
    let wanted = normalise(model);
    if wanted.is_empty() {
        return None;
    }

    let record = db
        .printers
        .iter()
        .find(|record| normalise(&record.model) == wanted)
        .or_else(|| {
            db.printers
                .iter()
                .find(|record| normalise(&record.short) == wanted)
        })
        .or_else(|| {
            db.printers.iter().find(|record| {
                let title = normalise(&record.title);
                let short = normalise(&record.short);
                !short.is_empty() && (title.contains(&wanted) || wanted.contains(&short))
            })
        })?;

    let mut profile = merge(&record.specs)?;
    profile.model = record.model.clone();
    Some(profile)
}

/// Explicit lookup by spec key (used by tests and the manual picker).
pub fn resolve_key(key: &str) -> Option<DeviceProfile> {
    let mut profile = merge(&[key.to_string()])?;
    profile.model = key.to_string();
    Some(profile)
}

/// Every printer known to the embedded database.
pub fn list_profiles() -> Vec<ProfileSummary> {
    let db = database();
    db.printers
        .iter()
        .map(|record| {
            let profile = merge(&record.specs);
            ProfileSummary {
                key: record.specs.first().cloned().unwrap_or_default(),
                title: record.title.clone(),
                short: record.short.clone(),
                model: record.model.clone(),
                chain: record.specs.clone(),
                brand: record.brand.clone(),
                class: record.class.clone(),
                has_service: profile.as_ref().map(|p| p.spec.is_usable()).unwrap_or(false),
                counters: profile
                    .as_ref()
                    .map(|p| p.spec.counters.len())
                    .unwrap_or(0),
                reset_pairs: profile
                    .as_ref()
                    .map(|p| p.spec.reset.len())
                    .unwrap_or(0),
            }
        })
        .collect()
}

pub fn version() -> &'static str {
    &database().version
}

pub fn spec_count() -> usize {
    database().specs.len()
}

pub fn printer_count() -> usize {
    database().printers.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_is_complete() {
        assert_eq!(printer_count(), 1022);
        assert!(spec_count() >= 370);
    }

    #[test]
    fn xp530_resolves_with_service_and_counters() {
        let profile = resolve("XP-530 Series").expect("XP-530 must resolve");
        assert!(profile.spec.is_usable());
        assert_eq!(profile.spec.service.factory, 0x0928);
        assert_eq!(profile.spec.service.write_key, b"Jsjthbsn");
        assert_eq!(profile.spec.service.read_len, 2);
        assert_eq!(profile.spec.service.send_len, 2);
        assert_eq!(profile.spec.counters.len(), 2);
        assert_eq!(profile.spec.counters[0].max, 4806);
        assert_eq!(profile.spec.counters[0].entries[0].addrs, vec![0x10, 0x11]);
        assert_eq!(profile.spec.counters[0].entries[1].mask, Some(0x0F));
        assert_eq!(profile.spec.counters[1].entries[1].mask, Some(0xF0));
        assert!(profile.spec.reset.contains(&(0x34, 0x5E)));
    }

    #[test]
    fn et8550_resolves_with_service_only() {
        // The ET-8550 chain has a <service> block but no <waste> table, so
        // the database exposes no counter reset for it.
        let profile = resolve("ET-8550 Series").expect("ET-8550 must resolve");
        assert!(profile.spec.is_usable());
        assert_eq!(profile.spec.service.write_key, b"Nbibnfsv");
        assert_eq!(profile.spec.service.factory, 0x204A);
        assert!(profile.spec.counters.is_empty());
    }

    #[test]
    fn unknown_model_is_none() {
        assert!(resolve("Definitely Not A Printer 9000").is_none());
    }
}
