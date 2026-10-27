use serde::Serialize;

mod ieee1284;
#[cfg(windows)]
mod windows;
#[cfg(not(windows))]
mod unsupported;

pub use ieee1284::parse_id;
#[allow(unused_imports)]
pub(crate) use ieee1284::decode_id;

/// A hex dump of the bytes exchanged with a printer, kept for hardware
/// diagnosis. The probe embeds it in its JSON so a failed handshake can be
/// inspected without a USB analyser.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trace {
    /// Frames sent to the printer, oldest first.
    pub tx: Vec<String>,
    /// Byte batches received from the printer, oldest first.
    pub rx: Vec<String>,
}

impl Trace {
    const MAX_ENTRIES: usize = 512;
    const MAX_BYTES: usize = 512;

    pub(crate) fn push_tx(&mut self, data: &[u8]) {
        push_entry(&mut self.tx, data);
    }

    pub(crate) fn push_rx(&mut self, data: &[u8]) {
        push_entry(&mut self.rx, data);
    }
}

fn push_entry(entries: &mut Vec<String>, data: &[u8]) {
    if entries.len() >= Trace::MAX_ENTRIES {
        return;
    }
    let shown = &data[..data.len().min(Trace::MAX_BYTES)];
    let mut text = String::with_capacity(shown.len() * 3);
    for (index, byte) in shown.iter().enumerate() {
        if index > 0 {
            text.push(' ');
        }
        text.push_str(&format!("{byte:02X}"));
    }
    if data.len() > shown.len() {
        text.push_str(" …");
    }
    entries.push(text);
}

/// A USB printer-class interface discovered through SetupAPI.
///
/// `path` is the device interface path (`\\?\usb#vid_...&pid_...#{guid}`) that
/// must be opened with CreateFileW before any printer-class control request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbPrinter {
    pub path: String,
    pub instance_id: String,
    pub friendly_name: String,
    pub hardware_id: String,
    pub vendor_id: String,
    pub product_id: String,
    pub ieee1284_id: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
}

impl UsbPrinter {
    pub(crate) fn apply_ieee1284(&mut self, raw: String) {
        let (manufacturer, model, serial) = parse_id(&raw);
        self.manufacturer = manufacturer;
        self.model = model;
        self.serial = serial;
        self.ieee1284_id = Some(raw);
    }
}

pub fn bridge_available() -> bool {
    cfg!(windows)
}

#[cfg(windows)]
pub use windows::{enumerate, port_protocol, read_ieee1284_id, UsbTransport};

#[cfg(not(windows))]
pub use unsupported::{enumerate, read_ieee1284_id};

/// Placeholder so the command signatures stay platform neutral on hosts where
/// the printer-class interface does not exist (every non-Windows target).
#[cfg(not(windows))]
pub struct UsbTransport;

#[cfg(not(windows))]
impl crate::protocol::Transport for UsbTransport {
    fn write_all(&mut self, _data: &[u8]) -> Result<(), String> {
        Err(NOT_WINDOWS.to_string())
    }

    fn read_some(&mut self, _buf: &mut [u8]) -> Result<usize, String> {
        Err(NOT_WINDOWS.to_string())
    }
}

#[cfg(not(windows))]
const NOT_WINDOWS: &str = "The IEEE 1284.4 transport is only available on Windows.";

/// Run a 1284.4 session over the opened printer handle.
///
/// The session is always torn down: `close()` on success (CloseChannel +
/// Exit) and `abort()` on failure, so the printer never stays in 1284.4 mode
/// because of an error path in the clone.
pub fn with_session<F, R>(
    device_path: &str,
    profile: &crate::spec_db::DeviceProfile,
    body: F,
) -> Result<R, String>
where
    F: FnOnce(&mut crate::protocol::Session<'_, UsbTransport>) -> Result<R, crate::protocol::ProtocolError>,
{
    with_session_traced(device_path, profile, body).0
}

/// Same as [`with_session`], but also returns the byte trace (when the
/// platform records one) whether the session succeeded or failed.
pub fn with_session_traced<F, R>(
    device_path: &str,
    profile: &crate::spec_db::DeviceProfile,
    body: F,
) -> (Result<R, String>, Option<Trace>)
where
    F: FnOnce(&mut crate::protocol::Session<'_, UsbTransport>) -> Result<R, crate::protocol::ProtocolError>,
{
    #[cfg(windows)]
    {
        let mut transport = match UsbTransport::open(device_path) {
            Ok(transport) => transport,
            Err(error) => return (Err(error), None),
        };
        transport.enable_trace();
        let outcome = (|| -> Result<R, String> {
            let mut session = crate::protocol::Session::open(&mut transport, &profile.spec)
                .map_err(|error| error.to_string())?;
            match body(&mut session) {
                Ok(value) => {
                    session.close().map_err(|error| error.to_string())?;
                    Ok(value)
                }
                Err(error) => {
                    session.abort();
                    Err(error.to_string())
                }
            }
        })();
        let trace = transport.take_trace();
        (outcome, trace)
    }

    #[cfg(not(windows))]
    {
        let _ = (device_path, profile, body);
        (
            Err("The IEEE 1284.4 transport is only available on Windows.".to_string()),
            None,
        )
    }
}
