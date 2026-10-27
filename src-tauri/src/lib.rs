mod commands;
pub mod protocol;
pub mod spec_db;
pub mod usb;

use spec_db::DeviceProfile;
use usb::UsbPrinter;

/// Headless hardware probe used by `--probe` and by packaging smoke tests.
///
/// Besides the interface list it resolves every `MDL:` against the embedded
/// device database, so a cable can be validated without opening the window:
/// a printer whose model does not resolve will refuse to read or reset.
/// `--read-counters` goes one step further and actually runs the read-only
/// 1284.4 session, which is the way to verify the frames on a real
/// printer before trusting a reset.
pub fn probe_json(read_counters: bool) -> String {
    let database = serde_json::json!({
        "version": spec_db::version(),
        "specs": spec_db::spec_count(),
        "printers": spec_db::printer_count(),
    });

    let report = match usb::enumerate() {
        Ok(printers) => {
            let entries: Vec<serde_json::Value> = printers
                .iter()
                .map(|printer| {
                    let profile = printer.model.as_deref().and_then(spec_db::resolve);
                    let summary = profile.as_ref().map(profile_summary);
                    let counters = counter_probe(printer, profile.as_ref(), read_counters);
                    serde_json::json!({
                        "printer": printer,
                        "profile": summary.unwrap_or(serde_json::Value::Null),
                        "counters": counters,
                        "port": port_info(printer),
                    })
                })
                .collect();
            serde_json::json!({
                "ok": true,
                "platform": std::env::consts::OS,
                "database": database,
                "count": printers.len(),
                "printers": entries,
            })
        }
        Err(error) => serde_json::json!({
            "ok": false,
            "platform": std::env::consts::OS,
            "database": database,
            "error": error,
        }),
    };

    report.to_string()
}

/// Headless version of the safe write test: pick the first attached printer
/// that resolves to a device profile and write one of its own bytes back
/// unchanged.
pub fn self_test_json() -> String {
    let report = match usb::enumerate() {
        Ok(printers) => {
            let target = printers.iter().find_map(|printer| {
                let model = printer.model.clone()?;
                spec_db::resolve(&model)?;
                Some((printer.path.clone(), model))
            });
            match target {
                Some((path, model)) => match commands::self_test_write(path, model) {
                    Ok(report) => serde_json::json!({ "ok": true, "report": report }),
                    Err(error) => serde_json::json!({ "ok": false, "error": error }),
                },
                None => serde_json::json!({
                    "ok": false,
                    "error": "No attached printer resolves to a device profile.",
                }),
            }
        }
        Err(error) => serde_json::json!({ "ok": false, "error": error }),
    };
    report.to_string()
}

/// USBPRINT interface type and protocol, so a probe report says whether the
/// port is in classic (BIDI) or IPP-over-USB mode.
fn port_info(printer: &UsbPrinter) -> serde_json::Value {
    #[cfg(windows)]
    let value = match usb::port_protocol(&printer.path) {
        Ok(info) => serde_json::json!({
            "interface": info.interface_name(),
            "protocol": info.protocol_name(),
            "code": info.protocol,
        }),
        Err(error) => serde_json::json!({ "error": error }),
    };
    #[cfg(not(windows))]
    let value = {
        let _ = printer;
        serde_json::Value::Null
    };
    value
}

fn profile_summary(profile: &DeviceProfile) -> serde_json::Value {    serde_json::json!({
        "model": profile.model,
        "chain": profile.chain,
        "factory": profile.factory,
        "keyword": profile.keyword,
        "readLen": profile.read_len,
        "sendLen": profile.send_len,
        "counters": profile.counters,
        "resetPairs": profile.reset_pairs,
        "canRead": profile.counters > 0,
        "canReset": profile.reset_pairs > 0,
    })
}

fn counter_probe(
    printer: &UsbPrinter,
    profile: Option<&DeviceProfile>,
    read: bool,
) -> serde_json::Value {
    if !read {
        return serde_json::Value::Null;
    }
    let Some(profile) = profile else {
        return serde_json::json!({
            "ok": false,
            "error": "no device profile matches this model",
        });
    };
    if profile.counters == 0 {
        return serde_json::json!({
            "ok": false,
            "error": "the matched profile has no waste counters",
        });
    }

    #[cfg(windows)]
    {
        let (result, trace) = commands::read_profile_counters(&printer.path, profile);
        match result {
            Ok(counters) => serde_json::json!({
                "ok": true,
                "counters": counters,
                "trace": trace,
            }),
            Err(error) => serde_json::json!({
                "ok": false,
                "error": error,
                "trace": trace,
            }),
        }
    }

    #[cfg(not(windows))]
    {
        let _ = printer;
        serde_json::json!({
            "ok": false,
            "error": "the IEEE 1284.4 transport is only available on Windows",
        })
    }
}

fn hex(data: &[u8]) -> String {
    data.iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `--raw-session`: drive the 1284.4 handshake frame by frame and dump the
/// answers, without decoding them.
///
/// This exists to settle reply layouts that the reference left
/// ambiguous (the socket byte of GetSocketID is the first one) against the
/// real printer. It never writes an eeprom address.
pub fn raw_session_json(sockets: &[u8], names: &[String]) -> String {
    #[cfg(windows)]
    {
        let printers = match usb::enumerate() {
            Ok(printers) => printers,
            Err(error) => {
                return serde_json::json!({"ok": false, "error": error}).to_string();
            }
        };
        let Some(printer) = printers.iter().find(|printer| printer.model.is_some()) else {
            return serde_json::json!({
                "ok": false,
                "error": "no USB printer reported an IEEE 1284 MDL string",
            })
            .to_string();
        };
        let model = printer.model.clone().unwrap_or_default();
        let Some(profile) = spec_db::resolve(&model) else {
            return serde_json::json!({
                "ok": false,
                "error": format!("no device profile matches {model:?}"),
            })
            .to_string();
        };
        let mut transport = match usb::UsbTransport::open(&printer.path) {
            Ok(transport) => transport,
            Err(error) => {
                return serde_json::json!({"ok": false, "error": error}).to_string();
            }
        };
        let keyword = profile.spec.service.write_key.clone();
        let candidates: Vec<&[u8]> = if names.is_empty() {
            vec![protocol::CONTROL_SERVICE]
        } else {
            names.iter().map(|name| name.as_bytes()).collect()
        };
        let outcome = protocol::diagnose_names(
            &mut transport,
            &candidates,
            sockets,
            protocol::CHANNEL_PACKET,
            protocol::CHANNEL_PACKET,
        );
        let port = transport.protocol();
        match outcome {
            Ok(steps) => {
                let steps: Vec<serde_json::Value> = steps
                    .into_iter()
                    .map(|step| {
                        serde_json::json!({
                            "label": step.label,
                            "tx": hex(&step.tx),
                            "rx": hex(&step.rx),
                            "status": step.rx.get(7).copied(),
                            "byte8": step.rx.get(8).copied(),
                        })
                    })
                    .collect();
                serde_json::json!({
                    "ok": true,
                    "model": model,
                    "keyword": String::from_utf8_lossy(&keyword),
                    "sockets": sockets,
                    "port": {
                        "interface": port.interface_name(),
                        "protocol": port.protocol_name(),
                        "switchedToBidi": port.switched_to_bidi,
                    },
                    "steps": steps,
                })
                .to_string()
            }
            Err(error) => serde_json::json!({
                "ok": false,
                "model": model,
                "error": error.to_string(),
            })
            .to_string(),
        }
    }

    #[cfg(not(windows))]
    {
        let _ = sockets;
        serde_json::json!({
            "ok": false,
            "error": "the IEEE 1284.4 transport is only available on Windows",
        })
        .to_string()
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::bridge_status,
            commands::list_usb_printers,
            commands::list_device_profiles,
            commands::read_waste_counters,
            commands::self_test_write,
            commands::reset_waste_counters
        ])
        .run(tauri::generate_context!())
        .expect("error while running InkReset");
}
