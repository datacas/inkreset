use serde::Serialize;

use crate::protocol::{self, Counter, Session};
use crate::spec_db::{self, DeviceProfile, ProfileSummary};
use crate::usb::{self, UsbPrinter};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    pub platform: String,
    pub available: bool,
    /// SetupAPI discovery and the IEEE 1284 ID read.
    pub discovery: bool,
    /// IEEE 1284.4 counter frames are implemented and gated behind a matched
    /// device profile.
    pub counter_protocol: bool,
    /// Number of printer models in the embedded device database.
    pub profiles: usize,
    pub detail: String,
}

#[tauri::command]
pub fn bridge_status() -> BridgeStatus {
    let platform = std::env::consts::OS.to_string();
    let available = usb::bridge_available();
    BridgeStatus {
        platform,
        available,
        discovery: available,
        counter_protocol: available,
        profiles: spec_db::printer_count(),
        detail: if available {
            "SetupAPI / USBPRINT discovery and the IEEE 1284.4 frame set are ready. Counter writes stay gated behind a profile matched from the printer's IEEE 1284 model string.".to_string()
        } else {
            "The USB bridge is not available on this platform; no device is contacted.".to_string()
        },
    }
}

#[tauri::command]
pub fn list_usb_printers() -> Result<Vec<UsbPrinter>, String> {
    usb::enumerate()
}

/// Search the embedded device database.
///
/// Every record with a usable 1284.4 service block is returned (930 of the
/// 1,022 printer records), which is small enough for one local IPC
/// call and lets the index show real coverage instead of an arbitrary page.
#[tauri::command]
pub fn list_device_profiles(query: Option<String>) -> Vec<ProfileSummary> {
    let needle = query.unwrap_or_default().trim().to_ascii_lowercase();
    let all = spec_db::list_profiles();
    if needle.is_empty() {
        all.into_iter().filter(|profile| profile.has_service).collect()
    } else {
        all.into_iter()
            .filter(|profile| {
                profile.title.to_ascii_lowercase().contains(&needle)
                    || profile.short.to_ascii_lowercase().contains(&needle)
                    || profile.model.to_ascii_lowercase().contains(&needle)
                    || profile
                        .chain
                        .iter()
                        .any(|key| key.to_ascii_lowercase().contains(&needle))
            })
            .collect()
    }
}

/// One waste counter as read from the printer's eeprom.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterValue {
    pub index: usize,
    /// Raw value of every `<entry>` of the counter, in table order.
    pub registers: Vec<u16>,
    /// First entry, the counter proper.
    pub value: u32,
    /// Level at which the printer considers the pad full (`<max>`).
    pub max: u32,
    pub percent: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterReport {
    pub model: String,
    pub chain: Vec<String>,
    pub socket: u8,
    pub send_size: u16,
    pub recv_size: u16,
    pub factory: Vec<u8>,
    pub counters: Vec<CounterValue>,
    pub note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetReport {
    pub model: String,
    pub chain: Vec<String>,
    /// Number of eeprom bytes written by the `<reset>` table.
    pub bytes_written: usize,
    pub counters: Vec<CounterValue>,
    pub note: String,
}

const PAD_WARNING: &str =
    "Resetting the counter does not service the physical waste-ink pad. Replace or clean the pad before the printer reports it is full again.";

fn counter_value(index: usize, counter: &Counter, registers: Vec<u16>) -> CounterValue {
    let value = registers.first().copied().unwrap_or(0) as u32;
    let max = counter.max;
    let percent = if max > 0 {
        (value as f32 / max as f32) * 100.0
    } else {
        0.0
    };
    CounterValue {
        index,
        registers,
        value,
        max,
        percent,
    }
}

fn snapshot(
    session: &mut Session<'_, usb::UsbTransport>,
    profile: &DeviceProfile,
) -> Result<Vec<CounterValue>, protocol::ProtocolError> {
    profile
        .spec
        .counters
        .iter()
        .enumerate()
        .map(|(index, counter)| {
            let registers = session.read_counter(counter)?;
            Ok(counter_value(index, counter, registers))
        })
        .collect()
}

/// Resolve the model string from the printer's IEEE 1284 ID to a profile.
fn profile_for(model: &str) -> Result<DeviceProfile, String> {
    spec_db::resolve(model).ok_or_else(|| {
        format!(
            "No device profile matches {model:?}. The counter protocol is disabled for unknown printers."
        )
    })
}

/// Resolve a profile from the printer itself, never from the webview.
///
/// The caller passes the `MDL:` it saw during enumeration; this re-reads the
/// 1284 ID over the device path and refuses to continue if the hardware now
/// reports a different model. That stops a stale or forged model argument from
/// selecting the wrong eeprom table on a real printer.
fn verified_profile(device_path: &str, requested: &str) -> Result<DeviceProfile, String> {
    let raw = usb::read_ieee1284_id(device_path)?;
    let (_, device_model, _) = usb::parse_id(&raw);
    let device_model = device_model.ok_or_else(|| {
        "The printer did not report an IEEE 1284 MDL string; refusing to guess a profile."
            .to_string()
    })?;
    if !device_model.eq_ignore_ascii_case(requested.trim()) {
        return Err(format!(
            "Refusing: the device reports {device_model:?}, not {requested:?}. Re-run discovery and try again."
        ));
    }
    profile_for(&device_model)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfTestReport {
    pub model: String,
    /// Eeprom address exercised by the test.
    pub address: u16,
    pub before: u8,
    pub after: u8,
    /// True when the byte read back equals the byte that was written, which
    /// proves the write key and the `:42:OK;` path without changing anything.
    pub unchanged: bool,
    pub note: String,
}

/// Exercise the complete eeprom write path without modifying the printer.
///
/// Reads one byte of the matched profile's reset table and writes the *same*
/// value back with the model write key. The byte is then read again: if it is
/// unchanged, the gate, the 1284.4 write frame and the key were accepted by
/// the firmware, and a real reset would behave the same way. Nothing about the
/// counters changes, so this is the safe way to test the reset plumbing.
#[tauri::command]
pub fn self_test_write(device_path: String, model: String) -> Result<SelfTestReport, String> {
    let profile = verified_profile(&device_path, &model)?;
    let address = profile
        .spec
        .reset
        .first()
        .map(|(address, _)| *address)
        .ok_or_else(|| "The matched profile has no <reset> table.".to_string())?;
    let (before, after) = usb::with_session(&device_path, &profile, |session| {
        let before = session.read_byte(address)?;
        session.write_byte(address, before)?;
        let after = session.read_byte(address)?;
        Ok((before, after))
    })?;
    Ok(SelfTestReport {
        model: profile.model.clone(),
        address,
        before,
        after,
        unchanged: before == after,
        note: if before == after {
            "Write path verified: the eeprom accepted the write key and the byte read back unchanged. No counter was modified.".to_string()
        } else {
            "The byte changed after the write. Do not run a reset until this is understood.".to_string()
        },
    })
}

#[tauri::command]
pub fn read_waste_counters(device_path: String, model: String) -> Result<CounterReport, String> {
    let profile = verified_profile(&device_path, &model)?;
    let report = usb::with_session(&device_path, &profile, |session| {
        let counters = snapshot(session, &profile)?;
        Ok(CounterReport {
            model: profile.model.clone(),
            chain: profile.chain.clone(),
            socket: session.socket(),
            send_size: session.channel_sizes().0,
            recv_size: session.channel_sizes().1,
            factory: profile.spec.service.factory.to_le_bytes().to_vec(),
            counters,
            note: PAD_WARNING.to_string(),
        })
    })?;

    Ok(report)
}

/// Read every counter of a resolved profile in one session.
///
/// Shared by the headless `--read-counters` probe, which also gets the byte
/// trace so a failed handshake can be diagnosed without a USB analyser.
pub(crate) fn read_profile_counters(
    device_path: &str,
    profile: &DeviceProfile,
) -> (
    Result<Vec<CounterValue>, String>,
    Option<usb::Trace>,
) {
    usb::with_session_traced(device_path, profile, |session| {
        profile
            .spec
            .counters
            .iter()
            .enumerate()
            .map(|(index, counter)| {
                let registers = session.read_counter(counter)?;
                Ok(counter_value(index, counter, registers))
            })
            .collect()
    })
}

#[tauri::command]
pub fn reset_waste_counters(
    device_path: String,
    model: String,
    confirm: String,
) -> Result<ResetReport, String> {
    if confirm.trim() != model.trim() {
        return Err(format!(
            "Reset refused: type {model:?} exactly to confirm the write."
        ));
    }
    if model.trim().is_empty() {
        return Err("Reset refused: the printer model is unknown.".to_string());
    }

    // Re-read the identity from the printer before touching an eeprom.
    let profile = verified_profile(&device_path, &model)?;
    let reset_len = profile.spec.reset.len();
    if reset_len == 0 {
        return Err("The matched profile has no <reset> table.".to_string());
    }
    // A write without a readable counter cannot be verified afterwards. Two
    // catalogue records (PP-50PRN / PP-100PRN) have a reset table but no
    // counter addresses, so they stay read-only in this build.
    if profile.spec.counters.is_empty() {
        return Err(
            "The matched profile has a reset table but no readable counters; this build refuses an unverified write."
                .to_string(),
        );
    }

    usb::with_session(&device_path, &profile, |session| {
        let bytes_written = session.reset_waste()?;
        let counters = snapshot(session, &profile)?;
        Ok(ResetReport {
            model: profile.model.clone(),
            chain: profile.chain.clone(),
            bytes_written,
            counters,
            note: PAD_WARNING.to_string(),
        })
    })
}
