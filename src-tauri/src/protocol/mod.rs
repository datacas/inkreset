//! IEEE 1284.4 / Epson service protocol.
//!
//! Implements the session layer used by Epson service tools over the printer's
//! USB interface: socket discovery, channel open/close, credit handshakes and
//! the EEPROM read/write commands that expose the waste-ink counters.
//!
//! The module is free of third-party dependencies so it can be compiled and
//! tested on its own (`rustc --test src/protocol/mod.rs`).

use std::fmt;

// ---------------------------------------------------------------------------
// Device profile
// ---------------------------------------------------------------------------

/// One `<service>` block from the embedded device database.
#[derive(Debug, Clone, Default)]
pub struct Service {
    /// Width of the eeprom address field for reads (`<readlen>`): 1 or 2.
    ///
    /// This is *not* a channel packet size. The embedded database only ever
    /// stores `1` or `2`, and the independent EWR database for the same
    /// XP-530 says `"rlen": 2` with the comment that it is the address field
    /// width. A 1-byte command on a 2-byte model is answered `:41:NA;`.
    pub read_len: u16,
    /// Width of the eeprom address field for writes (`<sendlen>`): 1 or 2.
    pub send_len: u16,
    /// Two-byte factory token placed in every eeprom command.
    pub factory: u16,
    /// Per-model write key appended to every eeprom write (`<keyword>`).
    ///
    /// Proven by EWR's XP-530 entry: `"wkey": "Jsjthbsn"`, i.e. the database
    /// value verbatim, plus `"wkey1": "Irisgarm"` (the ROT-1 decoded form) as
    /// a retry key. It is *not* a 1284.4 service name.
    pub write_key: Vec<u8>,
}

/// How a single counter register is read.
#[derive(Debug, Clone, Default)]
pub struct Entry {
    /// Eeprom addresses of one entry, combined little endian. Wide
    /// addresses (> 0xFF) use the two byte form of the command.
    pub addrs: Vec<u16>,
    /// Optional bit mask applied to the (single) byte. `0xFFFF` means
    /// "whole byte" in the device table.
    pub mask: Option<u16>,
    /// Optional shift; -1 means "no shift" in the device table.
    pub shift: Option<i16>,
    /// Optional scale ceiling shipped with the filter (`<filter>` third
    /// field, parsed base 16).
    pub filter_max: Option<u32>,
}

/// One `<counter>` block: the registers plus the level the printer considers
/// full.
#[derive(Debug, Clone, Default)]
pub struct Counter {
    pub entries: Vec<Entry>,
    pub max: u32,
}

/// A complete waste-ink profile for one printer family.
#[derive(Debug, Clone, Default)]
pub struct Spec {
    pub id: String,
    pub service: Service,
    pub counters: Vec<Counter>,
    /// `(address, value)` pairs from `<waste><reset>`.
    pub reset: Vec<(u16, u16)>,
}

impl Spec {
    /// Non-zero when the profile can talk to a printer.
    pub fn is_usable(&self) -> bool {
        !self.service.write_key.is_empty() || self.service.factory != 0
    }
}

// ---------------------------------------------------------------------------
// Frame construction
// ---------------------------------------------------------------------------

/// 1284.4 frames are `[len_be32][header...]` where `len` counts the whole
/// packet, including the four length bytes.
pub const ENTER_TEXT: &[u8] = b"@EJL 1284.4\n@EJL\n@EJL\n";

pub fn enter() -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 1 + ENTER_TEXT.len());
    push_len(&mut out, 1 + ENTER_TEXT.len());
    out.push(0x01); // credit
    out.extend_from_slice(ENTER_TEXT);
    out
}

pub fn init() -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    push_len(&mut out, 4);
    out.extend_from_slice(&[0x01, 0x01, 0x00, 0x01]);
    out
}

pub fn exit() -> Vec<u8> {
    let mut out = Vec::with_capacity(7);
    push_len(&mut out, 3);
    out.extend_from_slice(&[0x01, 0x00, 0x08]);
    out
}

/// `01 00 09 <name>` - resolves a service name to a socket id.
pub fn get_socket_id(name: &[u8]) -> Vec<u8> {
    // The name is copied with its terminator into the buffer but only
    // `7 + len` bytes are sent, so the frame on the wire carries no trailing
    // NUL and its length field matches its size exactly.
    let mut out = Vec::with_capacity(7 + name.len());
    push_len(&mut out, 3 + name.len());
    out.extend_from_slice(&[0x01, 0x00, 0x09]);
    out.extend_from_slice(name);
    out
}

/// `01 01 01 <sock> <sock> <send> <recv> 00 00 00 00`
pub fn open_channel(socket: u8, send: u16, recv: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(17);
    push_len(&mut out, 13);
    out.extend_from_slice(&[0x01, 0x01, 0x01, socket, socket]);
    out.extend_from_slice(&send.to_be_bytes());
    out.extend_from_slice(&recv.to_be_bytes());
    out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    out
}

/// `01 00 02 <sock> <sock> 00`
pub fn close_channel(socket: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    push_len(&mut out, 6);
    out.extend_from_slice(&[0x01, 0x00, 0x02, socket, socket, 0x00]);
    out
}

/// `01 01 04 <sock> <sock> 00 80 FF FF`
///
/// The two bytes before `FF FF` are the number of credits asked for. d4lib
/// asks for `0x80`; asking for one (as an earlier version of this clone did)
/// starves the channel after a couple of transactions.
pub fn credit_request(socket: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(13);
    push_len(&mut out, 9);
    out.extend_from_slice(&[0x01, 0x01, 0x04, socket, socket]);
    out.extend_from_slice(&[0x00, 0x80, 0xFF, 0xFF]);
    out
}

/// `01 01 03 <sock> <sock> 00 01`
pub fn credit(socket: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(11);
    push_len(&mut out, 7);
    out.extend_from_slice(&[0x01, 0x01, 0x03, socket, socket]);
    out.extend_from_slice(&[0x00, 0x01]);
    out
}

/// Data packet: `[sock][sock][len_be16][credit][control][payload]`.
///
/// The length field counts the whole packet and `credit` is the credit field
/// (EWR keeps it at `0`: any non-zero value locks up the hardware buffer on
/// that generation). `control` bit 0 marks the last packet of the message.
///
/// Measured on an XP-530: with `control = 0` the printer accepts the packet
/// and stays silent, with `control = 1` it answers (`:41:NA;`, `:42:OK;`).
/// EWR wraps its payload with `credit = control = 0`; this printer requires
/// `control = 1` on the final packet of a message.
pub fn data(socket: u8, payload: &[u8]) -> Vec<u8> {
    let total = payload.len() + 6;
    let mut out = Vec::with_capacity(total);
    out.push(socket);
    out.push(socket);
    out.extend_from_slice(&(total as u16).to_be_bytes());
    out.extend_from_slice(&[0x00, 0x01]);
    out.extend_from_slice(payload);
    out
}

fn push_len(out: &mut Vec<u8>, payload: usize) {
    let total = (payload + 4) as u32;
    out.extend_from_slice(&total.to_be_bytes());
}

// ---------------------------------------------------------------------------
// EEPROM commands
// ---------------------------------------------------------------------------

const EEPROM_READ_TAG: [u8; 3] = [0x41, 0xBE, 0xA0];
const EEPROM_WRITE_TAG: [u8; 3] = [0x42, 0xBD, 0x21];

/// Read one eeprom byte:
/// `7C 7C <len8> 00 <factory16 LE> 41 BE A0 <addr_lo> [addr_hi]`
///
/// `address_len` is the model's `<readlen>`: `2` sends a little-endian
/// two-byte address (inner length 7), `1` a single byte (inner length 6).
/// Verified against EWR's `GenerateReadPacket`, which builds the same frame
/// from `rkey`, `address` and `addressLength`.
pub fn eeprom_read(factory: u16, addr: u16, address_len: u8) -> Vec<u8> {
    let two_byte = address_len != 1;
    let mut out = Vec::with_capacity(11);
    out.extend_from_slice(&[0x7C, 0x7C]);
    out.push(if two_byte { 7 } else { 6 });
    out.push(0x00);
    out.extend_from_slice(&factory.to_le_bytes());
    out.extend_from_slice(&EEPROM_READ_TAG);
    out.push((addr & 0xFF) as u8);
    if two_byte {
        out.push((addr >> 8) as u8);
    }
    out
}

/// Write one eeprom byte:
/// `7C 7C <len8> 00 <factory16 LE> 42 BD 21 <addr_lo> [addr_hi] <value> <key>`
///
/// `address_len` is the model's `<sendlen>` and `key` is its `<keyword>`
/// write key. EWR's `GenerateWritePacket` appends the key verbatim after the
/// value, so the inner length is `7 + key.len()` for a one byte address and
/// `8 + key.len()` for a two byte one.
pub fn eeprom_write(factory: u16, addr: u16, value: u8, key: &[u8], address_len: u8) -> Vec<u8> {
    let two_byte = address_len != 1;
    let len = if two_byte { 8 } else { 7 } + key.len();
    let mut out = Vec::with_capacity(len + 4);
    out.extend_from_slice(&[0x7C, 0x7C]);
    out.push(len as u8);
    out.push(0x00);
    out.extend_from_slice(&factory.to_le_bytes());
    out.extend_from_slice(&EEPROM_WRITE_TAG);
    out.push((addr & 0xFF) as u8);
    if two_byte {
        out.push((addr >> 8) as u8);
    }
    out.push(value);
    out.extend_from_slice(key);
    out
}

/// Locate a tag inside a reply payload and return the text that follows it up
/// to `;` (or the end of the buffer).
pub fn tag_text<'a>(payload: &'a [u8], tag: &[u8]) -> Option<&'a [u8]> {
    let start = payload
        .windows(tag.len())
        .position(|window| window == tag)?
        + tag.len();
    let rest = &payload[start..];
    let end = rest.iter().position(|&b| b == b';').unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Parse an eeprom read reply. The printer answers `EE:<addr><value>` where
/// the address is 2 or 4 hex digits and the value is 2 hex digits; the
/// parser also accepts a value-only reply (no address echo) two characters
/// shorter.
pub fn parse_eeprom_read(payload: &[u8], expected_addr: u16, two_byte: bool) -> Result<u8, String> {
    if payload.windows(7).any(|window| window == b":41:NA;") {
        return Err(
            "printer refused the eeprom read (:41:NA;): wrong address width for this model, \
             or the printer is locked by another error state"
                .to_string(),
        );
    }
    if payload.windows(7).any(|window| window == b":41:NG;") {
        return Err("printer rejected the eeprom read (:41:NG;)".to_string());
    }
    let text = tag_text(payload, b"EE:").ok_or_else(|| "no EE: tag in eeprom reply".to_string())?;
    let full = if two_byte { 6 } else { 4 };
    if text.len() != full && text.len() != full - 2 {
        return Err(format!(
            "unexpected eeprom reply length {}: {:?}",
            text.len(),
            String::from_utf8_lossy(text)
        ));
    }
    if text.len() == full {
        let addr_len = full - 2;
        let addr = hex_u32(&text[..addr_len]).ok_or_else(|| "bad address in eeprom reply".to_string())?;
        if addr as u16 != expected_addr {
            return Err(format!(
                "eeprom reply address mismatch: got {addr:#x}, want {expected_addr:#x}"
            ));
        }
    }
    let value = hex_u32(&text[text.len() - 2..]).ok_or_else(|| "bad value in eeprom reply".to_string())?;
    Ok(value as u8)
}

/// The write acknowledgement is `:42:OK;`; older firmware answers a bare
/// `OK`. EWR documents the two failure tokens: `:42:NG;` means the write key
/// does not match the model, `:42:NA;` means the key was accepted but the
/// firmware refuses writes while another error state is active (ink out,
/// paper jam, cover open).
pub fn parse_eeprom_write(payload: &[u8]) -> Result<(), String> {
    if payload.windows(7).any(|window| window == b":42:NA;") {
        return Err(
            "printer refused the eeprom write (:42:NA;): it is locked by another error state \
             (ink out, paper jam, cover open)"
                .to_string(),
        );
    }
    if payload.windows(7).any(|window| window == b":42:NG;") {
        return Err(
            "printer rejected the eeprom write (:42:NG;): the write key does not match this model"
                .to_string(),
        );
    }
    if payload.windows(7).any(|window| window == b":42:OK;")
        || payload.windows(2).any(|window| window == b"OK")
    {
        Ok(())
    } else {
        Err(format!(
            "eeprom write not acknowledged: {:?}",
            String::from_utf8_lossy(payload)
        ))
    }
}

/// Hex digits are uppercase in Epson replies; accept both cases.
fn hex_u32(bytes: &[u8]) -> Option<u32> {
    let mut value = 0u32;
    for &b in bytes {
        let digit = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => return None,
        };
        value = value * 16 + digit as u32;
    }
    Some(value)
}

// ---------------------------------------------------------------------------
// Reply decoding
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ProtocolError {
    /// Printer returned the `0x7F` error frame.
    Printer { code: u8, text: &'static str },
    /// Reply carried a non-zero status byte at offset 7.
    Status(u8),
    Transport(String),
    Malformed(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::Printer { code, text } => {
                write!(f, "printer error {code:#04x}: {text}")
            }
            ProtocolError::Status(code) => write!(f, "1284.4 status {code:#04x}"),
            ProtocolError::Transport(msg) => write!(f, "transport: {msg}"),
            ProtocolError::Malformed(msg) => write!(f, "malformed reply: {msg}"),
        }
    }
}

impl From<String> for ProtocolError {
    fn from(value: String) -> Self {
        ProtocolError::Transport(value)
    }
}

/// 1284.4 error id -> text.
pub fn error_text(code: u8) -> &'static str {
    match code {
        0x01 => "Unable to begin conversation, try later.",
        0x02 => "Protocol revision not supported.",
        0x03 => "Channel is allready open.",
        0x04 => "Credit overflow or channel busy.",
        0x05 => "Channel is not open.",
        0x06 => "Service name to socket ID failed.",
        0x07 => "Init transaction failed.",
        0x08 => "Malformed packet, ignored.",
        0x09 => "No credit for received packet, ignored.",
        // Not present in the standard table (which stops at 0x09). Measured
        // on an XP-530: `GetSocketID` answers 0x0A for every name the printer
        // does not publish (`EPSON-CTRL` and `EPSON-DATA` answer 0x00).
        0x0A => "Service name is not published by this printer.",
        _ => "Unknown 1284.4 reply.",
    }
}

/// A decoded 1284.4 reply frame.
#[derive(Debug, Clone)]
pub struct Reply {
    pub raw: Vec<u8>,
    pub payload: Vec<u8>,
}

/// Validate a complete reply frame and return its payload.
///
/// Both layouts carry the total packet length in bytes `2..4` and the payload
/// from byte 6 on:
///
/// * control: `[len_be32][credit][control][socket][status]...`
/// * data:    `[socket][socket][len_be16][0x00][0x01][payload...]`
///
/// This function performs no error interpretation: a data reply starts with
/// ASCII (`EE:` / `OK`), so the `0x7F` / status checks only make sense for
/// control replies (see [`decode_control`]).
pub fn decode_reply(raw: &[u8]) -> Result<Reply, ProtocolError> {
    if raw.len() < 7 {
        return Err(ProtocolError::Malformed(format!("only {} bytes", raw.len())));
    }
    let declared = u16::from_be_bytes([raw[2], raw[3]]) as usize;
    if declared > 4096 {
        return Err(ProtocolError::Malformed(format!(
            "declared length {declared} exceeds buffer"
        )));
    }
    let end = declared.clamp(6, raw.len());
    Ok(Reply {
        raw: raw.to_vec(),
        payload: raw[6..end].to_vec(),
    })
}

/// Decode a control reply, translating the printer error frame.
///
/// `[6] == 0x7F` is the "no socket" error marker with the error id at `[9]`;
/// a non-zero byte at `[7]` is a channel status error. This matches the
/// GetSocketID and OpenChannel reply layouts.
pub fn decode_control(raw: &[u8]) -> Result<Reply, ProtocolError> {
    let reply = decode_reply(raw)?;
    if raw[6] == 0x7F {
        let code = raw.get(9).copied().unwrap_or(0);
        return Err(ProtocolError::Printer {
            code,
            text: error_text(code),
        });
    }
    if raw.len() > 7 && raw[7] != 0 {
        return Err(ProtocolError::Status(raw[7]));
    }
    Ok(reply)
}

// ---------------------------------------------------------------------------
// Handshake diagnosis
// ---------------------------------------------------------------------------

/// One request/response pair captured by [`diagnose_handshake`].
///
/// `--raw-session` prints these so a reply whose layout is still ambiguous
/// (for example the socket byte of GetSocketID) can be settled against the
/// real printer instead of a guess.
pub struct HandshakeStep {
    pub label: String,
    pub tx: Vec<u8>,
    pub rx: Vec<u8>,
}

fn read_once<T: Transport>(io: &mut T, capacity: usize) -> Vec<u8> {
    let mut buffer = vec![0u8; capacity];
    match io.read_some(&mut buffer) {
        Ok(read) => {
            buffer.truncate(read);
            buffer
        }
        Err(_) => Vec::new(),
    }
}

/// Drive a handshake step by step without decoding the replies.
///
/// Unlike [`Session::open`] this never aborts on a non-zero status, so the
/// caller can try several candidates in the same session and compare the
/// printer's answers byte for byte.
pub fn run_steps<T: Transport>(
    io: &mut T,
    steps: Vec<(String, Vec<u8>)>,
    capacity: usize,
) -> Result<Vec<HandshakeStep>, ProtocolError> {
    let mut captured = Vec::with_capacity(steps.len());
    io.drain();
    for (label, frame) in steps {
        io.write_all(&frame)?;
        let rx = read_once(io, capacity);
        captured.push(HandshakeStep {
            label,
            tx: frame,
            rx,
        });
    }
    Ok(captured)
}

/// Default diagnostic sequence: handshake, the same service twice, a bogus
/// service, then the socket candidates.
pub fn diagnose_handshake<T: Transport>(
    io: &mut T,
    service: &[u8],
    sockets: &[u8],
    send: u16,
    recv: u16,
) -> Result<Vec<HandshakeStep>, ProtocolError> {
    diagnose_names(io, &[service], sockets, send, recv)
}

/// Same as [`diagnose_handshake`] but tries several service names, which is
/// how the keyword encoding is settled against a real printer.
pub fn diagnose_names<T: Transport>(
    io: &mut T,
    names: &[&[u8]],
    sockets: &[u8],
    send: u16,
    recv: u16,
) -> Result<Vec<HandshakeStep>, ProtocolError> {
    let mut steps: Vec<(String, Vec<u8>)> =
        vec![("enter".to_string(), enter()), ("init".to_string(), init())];
    for name in names {
        let label = format!("getSocketId({})", String::from_utf8_lossy(name));
        steps.push((label, get_socket_id(name)));
    }
    for &socket in sockets {
        steps.push((
            format!("openChannel(0x{socket:02X})"),
            open_channel(socket, send, recv),
        ));
    }
    steps.push(("exit".to_string(), exit()));
    run_steps(io, steps, 64)
}

// ---------------------------------------------------------------------------
// Transport + session
// ---------------------------------------------------------------------------

/// Minimal byte pipe: the USBPRINT handle on Windows, a scripted buffer in
/// tests.
pub trait Transport {
    fn write_all(&mut self, data: &[u8]) -> Result<(), String>;
    /// Read at least one byte, up to `buf.len()`.
    fn read_some(&mut self, buf: &mut [u8]) -> Result<usize, String>;
    /// Discard bytes left in the pipe by a previous owner of the port.
    ///
    /// Read until the port reports nothing pending, right before the Enter
    /// frame: a stale status byte would otherwise be read as the answer to
    /// the next request. The default implementation does
    /// nothing, which is right for transports without buffered input.
    fn drain(&mut self) -> usize {
        0
    }
}

/// Max bytes accepted for a single 1284.4 reply.
pub const MAX_REPLY: usize = 4096;

/// How long a single reply may take before the transport is declared silent.
///
/// EEPROM answers arrive a few milliseconds after the request, but the USB
/// read itself can return empty while the printer is still working.
const REPLY_TIMEOUT_MS: u64 = 3_000;

/// IEEE 1284.4 service that carries the Epson factory (EEPROM) commands.
///
/// Proven on an XP-530: `GetSocketID("EPSON-CTRL")` answers status `0x00` and
/// socket `2`, `GetSocketID("EPSON-DATA")` answers `0x40`, while the
/// per-model `<keyword>` from the device database (`Jsjthbsn`, decoded
/// `Irisgarm`, or even a bogus name) all answer the same status `0x0A`. The
/// keyword is therefore *not* the channel name on this generation.
pub const CONTROL_SERVICE: &[u8] = b"EPSON-CTRL";

/// Packet size requested for both directions of the factory channel.
///
/// The `<readlen>`/`<sendlen>` values in the database are only ever `1` or
/// `2` (312 records use `2/2`, 52 use `1/1`), far below the 1284.4 minimum,
/// so they describe the EEPROM value width and not the channel. `0x0200` is
/// what the reference Epson client (`d4lib`) requests.
pub const CHANNEL_PACKET: u16 = 0x0200;

/// How often `Enter` is repeated when the printer answers with all zeroes.
const ENTER_ATTEMPTS: usize = 3;

pub struct Session<'a, T: Transport> {
    io: &'a mut T,
    spec: &'a Spec,
    socket: u8,
    send_size: u16,
    recv_size: u16,
}

impl<'a, T: Transport> Session<'a, T> {
    /// Enter 1284.4, initialise the link, resolve the service socket and open
    /// a channel. Mirrors the sequence in `WasteInkProcessor`.
    ///
    /// A failed handshake still tries to leave 1284.4 mode, so a printer is
    /// never left parked in the channel state because of an error here.
    pub fn open(io: &'a mut T, spec: &'a Spec) -> Result<Self, ProtocolError> {
        let mut session = Session {
            io,
            spec,
            socket: 0,
            send_size: CHANNEL_PACKET,
            recv_size: CHANNEL_PACKET,
        };
        match session.handshake() {
            Ok(()) => Ok(session),
            Err(error) => {
                session.leave();
                Err(error)
            }
        }
    }

    fn handshake(&mut self) -> Result<(), ProtocolError> {
        // Flush the pipe, then enter 1284.4. Enter is resent whenever the
        // 8 byte reply is all zeroes, which is what a printer that ignored the
        // magic string looks like. Three attempts is enough in practice.
        self.io.drain();
        let mut entered = false;
        for attempt in 0..ENTER_ATTEMPTS {
            let reply = self.exchange_control(&enter(), 8)?;
            if reply.raw.iter().any(|&byte| byte != 0) {
                entered = true;
                break;
            }
            if attempt + 1 < ENTER_ATTEMPTS {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        if !entered {
            return Err(ProtocolError::Transport(
                "the printer did not acknowledge 1284.4 mode".to_string(),
            ));
        }
        self.exchange_control(&init(), 8)?;
        self.socket = self.resolve_socket()?;
        self.open_channel()
    }

    /// Resolve the channel for the Epson factory-command service.
    ///
    /// `EPSON-CTRL` is what both the reference client and the EWR project
    /// open for EEPROM transactions; EWR also falls back to the well-known
    /// socket 2 when the printer does not answer the lookup. An XP-530 on
    /// this desk answers `0x00` with socket `2`, and answers `0x0A`
    /// (unknown service) for the per-model `<keyword>`, which is a write key
    /// and not a service name.
    fn resolve_socket(&mut self) -> Result<u8, ProtocolError> {
        /// Well-known control socket used by Epson printers.
        const CONTROL_SOCKET: u8 = 2;
        let frame = get_socket_id(CONTROL_SERVICE);
        self.io.write_all(&frame)?;
        let raw = self.read_frame(frame.len() + 2)?;
        let status = raw.get(7).copied().unwrap_or(0xFF);
        if status == 0 {
            if let Some(&socket) = raw.get(8) {
                if socket != 0 && socket != 0xFF {
                    return Ok(socket);
                }
            }
        }
        // A printer that does not publish the service can still accept the
        // factory channel on the well-known socket.
        Ok(CONTROL_SOCKET)
    }

    /// Best effort leave of 1284.4 mode: no channel to close, only Exit.
    fn leave(&mut self) {
        let _ = self.exchange_control(&exit(), 7);
    }

    fn open_channel(&mut self) -> Result<(), ProtocolError> {
        let mut attempt = 0;
        loop {
            let frame = open_channel(self.socket, self.send_size, self.recv_size);
            match self.exchange_control(&frame, 16) {
                Ok(reply) => {
                    if reply.raw.len() >= 14 {
                        self.send_size =
                            u16::from_be_bytes([reply.raw[10], reply.raw[11]]).max(1);
                        self.recv_size =
                            u16::from_be_bytes([reply.raw[12], reply.raw[13]]).max(1);
                    }
                    return Ok(());
                }
                // `0x03` ("channel is allready open") means the link is usable
                // as it stands, so it is not an error for us.
                Err(ProtocolError::Printer { code: 0x03, .. }) => return Ok(()),
                // `0x04` (busy / credit overflow) is transient: retry a few
                // times before giving up. The retry budget is deliberately
                // bounded.
                Err(ProtocolError::Printer { code: 0x04, .. }) if attempt < 4 => {
                    attempt += 1;
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// Send a control frame and read one reply.
    pub fn exchange_control(&mut self, frame: &[u8], expected: usize) -> Result<Reply, ProtocolError> {
        self.io.write_all(frame)?;
        let raw = self.read_frame(expected)?;
        decode_control(&raw)
    }

    /// Send a data packet and read one reply (no control error markers).
    pub fn exchange_data(&mut self, frame: &[u8], expected: usize) -> Result<Reply, ProtocolError> {
        self.io.write_all(frame)?;
        let raw = self.read_frame(expected)?;
        decode_reply(&raw)
    }

    fn read_frame(&mut self, expected: usize) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer = Vec::with_capacity(MAX_REPLY);
        let mut chunk = [0u8; 1024];
        let started = std::time::Instant::now();
        // The transport reports a timeout as a zero-byte read, not as an
        // error. An EEPROM reply can land a few milliseconds after the data
        // packet (measured on an XP-530), so a single empty read must not be
        // treated as "no answer": keep polling until the deadline.
        let deadline = std::time::Duration::from_millis(REPLY_TIMEOUT_MS);
        // Read the header first: both layouts carry the total packet length in
        // bytes 2..4 (control replies use a 32 bit length, data replies a 16
        // bit one, so the low half is all we need).
        while buffer.len() < 4 {
            if self.pump(&mut buffer, &mut chunk)? == 0 {
                if started.elapsed() >= deadline {
                    return Err(ProtocolError::Transport("no reply header".into()));
                }
            }
        }
        let declared = u16::from_be_bytes([buffer[2], buffer[3]]) as usize;
        let target = if (7..=MAX_REPLY).contains(&declared) {
            declared
        } else {
            expected.max(6).min(MAX_REPLY)
        };
        while buffer.len() < target {
            if self.pump(&mut buffer, &mut chunk)? == 0 {
                if started.elapsed() >= deadline {
                    return Err(ProtocolError::Transport("incomplete reply".into()));
                }
            }
        }
        buffer.truncate(target);
        Ok(buffer)
    }

    /// Move whatever the transport has into `buffer`; returns the byte count.
    fn pump(&mut self, buffer: &mut Vec<u8>, chunk: &mut [u8]) -> Result<usize, ProtocolError> {
        let read = self.io.read_some(chunk)?;
        if read == 0 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            return Ok(0);
        }
        buffer.extend_from_slice(&chunk[..read]);
        Ok(read)
    }

    /// Send a data payload on the open channel and return the reply payload.
    ///
    /// Mirrors `d4lib`'s `printer_transact`: request credit, write the packet,
    /// give the printer one credit (`readData`), then read the answer. The
    /// `Credit` reply is consumed on its own so it cannot be mistaken for the
    /// data reply.
    fn request(&mut self, payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        // EWR sends a credit pair before every `EPSON-CTRL` packet: grant one
        // credit to the printer, then request credits for the host, then the
        // data packet. d4lib instead grants the credit after writing; on an
        // XP-530 that ordering leaves a two byte address read unanswered,
        // while the EWR order gets an answer. Both reference clients agree on
        // the frames themselves, only the ordering differs.
        match self.exchange_control(&credit(self.socket), 11) {
            Ok(_) => {}
            Err(ProtocolError::Transport(error)) => return Err(ProtocolError::Transport(error)),
            // A printer that does not ack the grant still accepts the request.
            Err(_) => {}
        }
        self.exchange_control(&credit_request(self.socket), 13)?;
        self.io.write_all(&data(self.socket, payload))?;
        let raw = self.read_frame(payload.len() + 8)?;
        Ok(decode_reply(&raw)?.payload)
    }

    /// Read one eeprom byte.
    pub fn read_byte(&mut self, addr: u16) -> Result<u8, ProtocolError> {
        let two_byte = self.spec.service.read_len != 1;
        let command = eeprom_read(self.spec.service.factory, addr, self.spec.service.read_len as u8);
        let payload = self.request(&command)?;
        parse_eeprom_read(&payload, addr, two_byte).map_err(ProtocolError::Malformed)
    }

    /// Write one eeprom byte, with the model write key appended.
    pub fn write_byte(&mut self, addr: u16, value: u8) -> Result<(), ProtocolError> {
        let command = eeprom_write(
            self.spec.service.factory,
            addr,
            value,
            &self.spec.service.write_key,
            self.spec.service.send_len as u8,
        );
        let payload = self.request(&command)?;
        parse_eeprom_write(&payload).map_err(ProtocolError::Malformed)
    }

    /// Read a whole register entry (little endian, as the counter tables do).
    ///
    /// `<entry><filter>mask shift target</filter>` is applied only as
    /// `(byte & mask) >> shift` with a non-negative shift. The embedded
    /// catalogue uses `-1` and `0` for `shift`; the meaning of the negative
    /// sentinel is not proven, so it is treated as "no shift" instead of
    /// guessing a left or right shift.
    pub fn read_entry(&mut self, entry: &Entry) -> Result<u16, ProtocolError> {
        let mut value: u16 = 0;
        for (index, &addr) in entry.addrs.iter().enumerate() {
            let byte = self.read_byte(addr)?;
            let byte = match (entry.mask, entry.shift) {
                (Some(mask), Some(shift)) if shift >= 0 => {
                    ((byte as u16 & mask) >> shift) as u8
                }
                (Some(mask), _) => (byte as u16 & mask) as u8,
                _ => byte,
            };
            value |= (byte as u16) << (8 * index.min(1));
        }
        Ok(value)
    }

    /// Read every register of a waste counter.
    pub fn read_counter(&mut self, counter: &Counter) -> Result<Vec<u16>, ProtocolError> {
        counter
            .entries
            .iter()
            .map(|entry| self.read_entry(entry))
            .collect()
    }

    /// Apply the `<waste><reset>` table.
    pub fn reset_waste(&mut self) -> Result<usize, ProtocolError> {
        let mut written = 0;
        for &(addr, value) in &self.spec.reset {
            self.write_byte(addr, value as u8)?;
            written += 1;
        }
        Ok(written)
    }

    pub fn socket(&self) -> u8 {
        self.socket
    }

    pub fn channel_sizes(&self) -> (u16, u16) {
        (self.send_size, self.recv_size)
    }

    /// Close the channel and leave 1284.4 mode.
    pub fn close(mut self) -> Result<(), ProtocolError> {
        let _ = self.exchange_control(&close_channel(self.socket), 10);
        let _ = self.exchange_control(&exit(), 7);
        Ok(())
    }

    /// Leave 1284.4 mode even when the caller already has an error.
    pub fn abort(mut self) {
        let _ = self.exchange_control(&close_channel(self.socket), 10);
        let _ = self.exchange_control(&exit(), 7);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn xp530() -> Spec {
        Spec {
            id: "XP530".into(),
            service: Service {
                read_len: 0x02,
                send_len: 0x02,
                factory: u16::from_le_bytes([0x28, 0x09]),
                write_key: b"Jsjthbsn".to_vec(),
            },
            counters: vec![Counter {
                entries: vec![
                    Entry {
                        addrs: vec![0x10, 0x11],
                        ..Default::default()
                    },
                    Entry {
                        addrs: vec![0x06],
                        mask: Some(0x0F),
                        shift: Some(-1),
                        filter_max: Some(0x0268),
                    },
                ],
                max: 4806,
            }],
            reset: vec![(0x10, 0), (0x11, 0)],
        }
    }

    #[test]
    fn enter_frame_matches_image() {
        let frame = enter();
        assert_eq!(frame.len(), 27);
        assert_eq!(
            frame,
            [
                0x00, 0x00, 0x00, 0x1B, 0x01, b'@', b'E', b'J', b'L', b' ', b'1', b'2', b'8',
                b'4', b'.', b'4', b'\n', b'@', b'E', b'J', b'L', b'\n', b'@', b'E', b'J', b'L',
                b'\n'
            ]
        );
    }

    #[test]
    fn init_and_exit_frames() {
        assert_eq!(init(), [0, 0, 0, 0x08, 0x01, 0x01, 0x00, 0x01]);
        assert_eq!(exit(), [0, 0, 0, 0x07, 0x01, 0x00, 0x08]);
    }

    #[test]
    fn socket_and_channel_frames() {
        assert_eq!(
            get_socket_id(b"Jsjthbsn"),
            [
                0, 0, 0, 15, 0x01, 0x00, 0x09, b'J', b's', b'j', b't', b'h', b'b', b's', b'n'
            ]
        );
        assert_eq!(
            open_channel(0x10, 0x02, 0x02),
            [
                0, 0, 0, 17, 1, 1, 1, 0x10, 0x10, 0x00, 0x02, 0x00, 0x02, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            close_channel(0x10),
            [0, 0, 0, 10, 1, 0, 2, 0x10, 0x10, 0]
        );
        assert_eq!(
            credit_request(0x10),
            [0, 0, 0, 13, 1, 1, 4, 0x10, 0x10, 0, 0x80, 0xFF, 0xFF]
        );
        assert_eq!(credit(0x10), [0, 0, 0, 11, 1, 1, 3, 0x10, 0x10, 0, 1]);
    }

    #[test]
    fn data_packet_counts_header() {
        assert_eq!(data(0x10, &[1, 2, 3]), [0x10, 0x10, 0x00, 0x09, 0, 1, 1, 2, 3]);
    }

    #[test]
    fn eeprom_read_one_byte_address() {
        let frame = eeprom_read(0x0928, 0x10, 1);
        assert_eq!(
            frame,
            vec![0x7C, 0x7C, 0x06, 0x00, 0x28, 0x09, 0x41, 0xBE, 0xA0, 0x10]
        );
    }

    #[test]
    fn eeprom_read_two_byte_address() {
        // XP-530: `<readlen>` is 2, so address 0x10 still goes out as two
        // bytes. Sending one byte is what made the printer answer `:41:NA;`.
        let frame = eeprom_read(0x0928, 0x10, 2);
        assert_eq!(
            frame,
            vec![0x7C, 0x7C, 0x07, 0x00, 0x28, 0x09, 0x41, 0xBE, 0xA0, 0x10, 0x00]
        );
    }

    #[test]
    fn eeprom_write_byte() {
        let frame = eeprom_write(0x0928, 0x10, 0x00, &[], 1);
        assert_eq!(
            frame,
            vec![0x7C, 0x7C, 0x07, 0x00, 0x28, 0x09, 0x42, 0xBD, 0x21, 0x10, 0x00]
        );
    }

    #[test]
    fn eeprom_write_appends_the_model_key() {
        // EWR builds the same frame for XP-530: rkey 0x0928, two byte
        // address, value, then `wkey` = "Jsjthbsn" (8 bytes) appended.
        let frame = eeprom_write(0x0928, 0x10, 0x00, b"Jsjthbsn", 2);
        assert_eq!(
            frame,
            vec![
                0x7C, 0x7C, 0x10, 0x00, 0x28, 0x09, 0x42, 0xBD, 0x21, 0x10, 0x00, 0x00, b'J',
                b's', b'j', b't', b'h', b'b', b's', b'n'
            ]
        );
    }

    #[test]
    fn eeprom_write_two_byte_address_with_extra() {
        let frame = eeprom_write(0x0928, 0x01ED, 0x5E, &[0xAA, 0xBB], 2);
        assert_eq!(
            frame,
            vec![
                0x7C, 0x7C, 0x0A, 0x00, 0x28, 0x09, 0x42, 0xBD, 0x21, 0xED, 0x01, 0x5E, 0xAA,
                0xBB
            ]
        );
    }

    #[test]
    fn eeprom_reply_parsing() {
        assert_eq!(parse_eeprom_read(b"EE:1012;", 0x10, false).unwrap(), 0x12);
        assert_eq!(parse_eeprom_read(b"EE:01ED12;", 0x01ED, true).unwrap(), 0x12);
        assert_eq!(parse_eeprom_read(b"EE:12;", 0x10, false).unwrap(), 0x12);
        assert!(parse_eeprom_read(b"EE:1112;", 0x10, false).is_err());
        assert!(parse_eeprom_read(b"nope", 0x10, false).is_err());
        assert!(parse_eeprom_write(b"\x00OK\x00").is_ok());
        assert!(parse_eeprom_write(b"\x00NO\x00").is_err());
    }

    #[test]
    fn error_frame_decoding() {
        let raw = [0, 0, 0, 10, 0, 0, 0x7F, 0, 0, 0x03];
        match decode_control(&raw) {
            Err(ProtocolError::Printer { code: 3, .. }) => {}
            other => panic!("expected printer error, got {other:?}"),
        }
    }

    /// Scripted transport: replies are queued per write, and every written
    /// frame is recorded for assertions.
    struct Mock {
        writes: Vec<Vec<u8>>,
        replies: VecDeque<Vec<u8>>,
    }

    impl Transport for Mock {
        fn write_all(&mut self, data: &[u8]) -> Result<(), String> {
            self.writes.push(data.to_vec());
            Ok(())
        }

        fn read_some(&mut self, buf: &mut [u8]) -> Result<usize, String> {
            match self.replies.pop_front() {
                Some(mut reply) => {
                    let n = reply.len().min(buf.len());
                    buf[..n].copy_from_slice(&reply[..n]);
                    reply.drain(..n);
                    if !reply.is_empty() {
                        self.replies.push_front(reply);
                    }
                    Ok(n)
                }
                None => Ok(0),
            }
        }
    }

    /// Data reply: `[sock][sock][len][0x00][0x01][payload]`.
    fn frame(payload: &[u8]) -> Vec<u8> {
        let total = payload.len() + 6;
        let mut out = vec![0, 0];
        out.extend_from_slice(&(total as u16).to_be_bytes());
        out.extend_from_slice(&[0x00, 0x01]);
        out.extend_from_slice(payload);
        out
    }

    /// Control reply: `[len][credit][control][payload]`, so the command code
    /// sits at raw[6], the status at raw[7] and data from raw[8] on.
    fn control_frame(payload: &[u8]) -> Vec<u8> {
        let total = payload.len() + 6;
        let mut out = vec![0, 0];
        out.extend_from_slice(&(total as u16).to_be_bytes());
        out.extend_from_slice(&[0x01, 0x00]);
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn session_reads_and_resets() {
        let spec = xp530();
        let enter_reply = control_frame(&[0x08, 0x00]);
        let init_reply = control_frame(&[0x01, 0x00, 0x00]);
        let socket_reply = control_frame(&[0x09, 0x00, 0x10]);
        let open_reply =
            control_frame(&[0x01, 0x00, 0x10, 0x10, 0x00, 0x02, 0x00, 0x02]);
        let credit_req_reply = control_frame(&[0x04, 0x00]);
        let credit_reply = control_frame(&[0x03, 0x00]);
        let data_reply = frame(b"EE:001012;");
        let write_credit = control_frame(&[0x04, 0x00]);
        let write_credit2 = control_frame(&[0x03, 0x00]);
        let write_reply = frame(b"OK;");

        let mut mock = Mock {
            writes: Vec::new(),
            replies: VecDeque::from(vec![
                enter_reply,
                init_reply,
                socket_reply,
                open_reply,
                credit_req_reply,
                credit_reply,
                data_reply,
                write_credit,
                write_credit2,
                write_reply,
            ]),
        };

        let mut session = Session::open(&mut mock, &spec).expect("session opens");
        assert_eq!(session.socket(), 0x10);
        assert_eq!(session.read_byte(0x10).unwrap(), 0x12);
        session.write_byte(0x10, 0).unwrap();
        let first = &mock.writes[0];
        assert_eq!(first, &enter());
        let last_data = mock
            .writes
            .iter()
            .rev()
            .find(|frame| frame.len() > 6 && frame[0] == 0x10 && frame[1] == 0x10)
            .expect("a data packet was written");
        let expected_write = eeprom_write(0x0928, 0x10, 0x00, b"Jsjthbsn", 2);
        assert_eq!(&last_data[6..], expected_write.as_slice());
    }

    /// The session flushes the pipe before sending Enter, so a stale byte
    /// from a previous owner is never taken as a reply.
    #[test]
    fn session_drains_stale_bytes_before_enter() {
        struct DrainMock {
            events: Vec<&'static str>,
            stale: usize,
            replies: VecDeque<Vec<u8>>,
        }

        impl Transport for DrainMock {
            fn write_all(&mut self, _data: &[u8]) -> Result<(), String> {
                self.events.push("write");
                Ok(())
            }

            fn read_some(&mut self, buf: &mut [u8]) -> Result<usize, String> {
                self.events.push("read");
                match self.replies.pop_front() {
                    Some(reply) => {
                        let n = reply.len().min(buf.len());
                        buf[..n].copy_from_slice(&reply[..n]);
                        Ok(n)
                    }
                    None => Ok(0),
                }
            }

            fn drain(&mut self) -> usize {
                self.events.push("drain");
                std::mem::take(&mut self.stale)
            }
        }

        let spec = xp530();
        let mut mock = DrainMock {
            events: Vec::new(),
            stale: 3,
            replies: VecDeque::from(vec![
                control_frame(&[0x08, 0x00]),
                control_frame(&[0x01, 0x00, 0x00]),
                control_frame(&[0x09, 0x00, 0x10]),
                control_frame(&[0x01, 0x00, 0x10, 0x10, 0x00, 0x02, 0x00, 0x02]),
            ]),
        };

        let session = Session::open(&mut mock, &spec).expect("session opens");
        let socket = session.socket();
        drop(session);
        assert_eq!(mock.events[..2], ["drain", "write"]);
        assert_eq!(socket, 0x10);
    }

    /// An all-zero answer means "not in 1284.4 mode yet", so the Enter frame
    /// is resent.
    #[test]
    fn enter_is_retried_when_the_reply_is_all_zeroes() {
        let spec = xp530();
        let mut mock = Mock {
            writes: Vec::new(),
            replies: VecDeque::from(vec![
                vec![0u8; 8],
                control_frame(&[0x08, 0x00]),
                control_frame(&[0x01, 0x00, 0x00]),
                control_frame(&[0x09, 0x00, 0x10]),
                control_frame(&[0x01, 0x00, 0x10, 0x10, 0x00, 0x02, 0x00, 0x02]),
            ]),
        };

        let session = Session::open(&mut mock, &spec).expect("session opens");
        assert_eq!(session.socket(), 0x10);
        drop(session);
        assert_eq!(mock.writes[0], enter());
        assert_eq!(mock.writes[1], enter());
    }

    #[test]
    fn enter_gives_up_after_three_silent_replies() {
        let spec = xp530();
        let mut mock = Mock {
            writes: Vec::new(),
            replies: VecDeque::from(vec![vec![0u8; 8], vec![0u8; 8], vec![0u8; 8]]),
        };
        let error = match Session::open(&mut mock, &spec) {
            Err(error) => error,
            Ok(_) => panic!("a silent printer must fail the session"),
        };
        assert!(error.to_string().contains("did not acknowledge"));
        assert_eq!(mock.writes.len(), ENTER_ATTEMPTS + 1);
        assert_eq!(mock.writes.last().unwrap(), &exit());
    }

    /// An error control reply: `[7F][00][00][code]` inside a normal frame.
    fn error_frame(code: u8) -> Vec<u8> {
        control_frame(&[0x7F, 0x00, 0x00, code])
    }

    /// Enter / Init / GetSocketID answers, so a test can focus on what the
    /// OpenChannel exchange does.
    fn handshake_replies() -> Vec<Vec<u8>> {
        vec![
            control_frame(&[0x08, 0x00]),
            control_frame(&[0x01, 0x00, 0x00]),
            control_frame(&[0x09, 0x00, 0x10]),
        ]
    }

    /// `0x03` is "channel is allready open": the link is usable, so the
    /// session must not fail on it.
    #[test]
    fn open_channel_accepts_already_open() {
        let spec = xp530();
        let mut replies = handshake_replies();
        replies.push(error_frame(0x03));
        let mut mock = Mock {
            writes: Vec::new(),
            replies: VecDeque::from(replies),
        };

        let session = match Session::open(&mut mock, &spec) {
            Ok(session) => session,
            Err(error) => panic!("already-open must succeed, got {error}"),
        };
        assert_eq!(session.socket(), 0x10);
        assert_eq!(
            session.channel_sizes(),
            (CHANNEL_PACKET, CHANNEL_PACKET),
            "no sizes are read on an already-open channel"
        );
    }

    /// `0x04` is transient (busy / credit overflow): the clone retries and
    /// keeps the channel sizes from the first successful answer.
    #[test]
    fn open_channel_retries_on_busy() {
        let spec = xp530();
        let mut replies = handshake_replies();
        replies.push(error_frame(0x04));
        replies.push(error_frame(0x04));
        replies.push(control_frame(&[
            0x01, 0x00, 0x10, 0x10, 0x00, 0x08, 0x00, 0x08,
        ]));
        let mut mock = Mock {
            writes: Vec::new(),
            replies: VecDeque::from(replies),
        };

        let session = Session::open(&mut mock, &spec).expect("busy is retried");
        assert_eq!(session.channel_sizes(), (8, 8));
        let opens = mock
            .writes
            .iter()
            .filter(|frame| frame.len() > 6 && frame[4] == 0x01 && frame[6] == 0x01)
            .count();
        assert_eq!(opens, 3, "two failures plus the successful open");
    }

    /// A persistent busy answer is bounded instead of looping forever, and
    /// the failed handshake still leaves 1284.4 mode.
    #[test]
    fn open_channel_gives_up_when_busy_persists() {
        let spec = xp530();
        let mut replies = handshake_replies();
        for _ in 0..8 {
            replies.push(error_frame(0x04));
        }
        let mut mock = Mock {
            writes: Vec::new(),
            replies: VecDeque::from(replies),
        };

        let error = match Session::open(&mut mock, &spec) {
            Err(error) => error,
            Ok(_) => panic!("persistent busy must fail"),
        };
        assert!(matches!(error, ProtocolError::Printer { code: 0x04, .. }));
        assert_eq!(mock.writes.last().unwrap(), &exit());
    }

    /// Stateful printer model: it answers the frames the session builds and
    /// keeps a tiny EEPROM, so the whole read/reset flow runs end to end
    /// without hardware. The frame bytes themselves are pinned by the tests
    /// above; this one exercises the sequence and the channel state machine.
    struct FakePrinter {
        replies: VecDeque<Vec<u8>>,
        /// Answer produced by a data packet but not released yet: a real
        /// printer can only send it once the host grants a credit, so the
        /// Credit transaction's ack goes out first.
        pending: Option<Vec<u8>>,
        eeprom: std::collections::HashMap<u16, u8>,
        socket: u8,
        events: Vec<&'static str>,
    }

    impl FakePrinter {
        fn new(socket: u8) -> Self {
            Self {
                replies: VecDeque::new(),
                pending: None,
                eeprom: std::collections::HashMap::new(),
                socket,
                events: Vec::new(),
            }
        }

        fn control(payload: &[u8]) -> Vec<u8> {
            control_frame(payload)
        }

        fn data(&mut self, payload: &[u8]) {
            let mut reply = vec![self.socket, self.socket];
            reply.extend_from_slice(&((payload.len() + 6) as u16).to_be_bytes());
            reply.extend_from_slice(&[0x00, 0x01]);
            reply.extend_from_slice(payload);
            // The host reads the answer right after writing the data packet
            // (EWR grants its credit before the packet, not after).
            self.replies.push_back(reply);
        }

        fn eeprom_command(&mut self, payload: &[u8]) {
            assert_eq!(&payload[..2], &[0x7C, 0x7C], "eeprom prefix");
            assert_eq!(
                &payload[4..6],
                &[0x28, 0x09],
                "factory token must be sent little endian"
            );
            let length = payload[2] as usize;
            assert_eq!(length + 4, payload.len(), "length byte counts the tail");
            match &payload[6..9] {
                [0x41, 0xBE, 0xA0] => {
                    // A read is 6 bytes with a one byte address, 7 with two.
                    let two_byte = length >= 7;
                    let mut addr = payload[9] as u16;
                    if two_byte {
                        addr |= (payload[10] as u16) << 8;
                    }
                    let value = self.eeprom.get(&addr).copied().unwrap_or(0);
                    let text = if two_byte {
                        format!("EE:{addr:04X}{value:02X}")
                    } else {
                        format!("EE:{addr:02X}{value:02X}")
                    };
                    self.data(text.as_bytes());
                }
                [0x42, 0xBD, 0x21] => {
                    // A write is 7 bytes with a one byte address, 8 with two.
                    let two_byte = length >= 8;
                    let mut addr = payload[9] as u16;
                    if two_byte {
                        addr |= (payload[10] as u16) << 8;
                    }
                    let value = payload[10 + two_byte as usize];
                    self.eeprom.insert(addr, value);
                    self.data(b"OK");
                }
                other => panic!("unexpected eeprom tag {other:?}"),
            }
        }
    }

    impl Transport for FakePrinter {
        fn write_all(&mut self, data: &[u8]) -> Result<(), String> {
            let total = u16::from_be_bytes([data[2], data[3]]) as usize;
            assert_eq!(total, data.len(), "declared length must match the frame");
            let frame = data;
            if frame.len() >= 16 && &frame[5..16] == b"@EJL 1284.4" {
                self.events.push("enter");
                self.replies.push_back(Self::control(&[0x08, 0x00]));
                return Ok(());
            }
            match (frame[4], frame[5], frame[6]) {
                (0x01, 0x01, 0x00) => {
                    self.events.push("init");
                    self.replies.push_back(Self::control(&[0x01, 0x00, 0x00]));
                }
                (0x01, 0x00, 0x09) => {
                    self.events.push("getsocket");
                    let name = &frame[7..];
                    assert_eq!(name, CONTROL_SERVICE, "service name frame");
                    self.replies
                        .push_back(Self::control(&[0x09, 0x00, self.socket]));
                }
                (0x01, 0x01, 0x01) => {
                    self.events.push("open");
                    assert_eq!(&frame[7..9], &[self.socket, self.socket]);
                    let send = u16::from_be_bytes([frame[9], frame[10]]).max(1) + 2;
                    let recv = u16::from_be_bytes([frame[11], frame[12]]).max(1) + 2;
                    self.replies.push_back(Self::control(&[
                        0x01,
                        0x00,
                        self.socket,
                        self.socket,
                        (send >> 8) as u8,
                        send as u8,
                        (recv >> 8) as u8,
                        recv as u8,
                    ]));
                }
                (0x01, 0x01, 0x04) => {
                    self.events.push("credit-request");
                    self.replies.push_back(Self::control(&[0x04, 0x00]));
                }
                (0x01, 0x01, 0x03) => {
                    self.events.push("credit");
                    self.replies.push_back(Self::control(&[0x03, 0x00]));
                    // The credit is what lets the printer release the answer
                    // it prepared when the data packet arrived.
                    if let Some(reply) = self.pending.take() {
                        self.replies.push_back(reply);
                    }
                }
                (0x01, 0x00, 0x02) => {
                    self.events.push("close");
                    self.replies.push_back(Self::control(&[0x02, 0x00]));
                }
                (0x01, 0x00, 0x08) => {
                    self.events.push("exit");
                    self.replies.push_back(Self::control(&[0x08, 0x00]));
                }
                // Data packets carry `[sock][sock][len][00][eoj]`; control
                // packets always start with `01` in the sixth byte.
                (0x00, _, _) => {
                    self.events.push("data");
                    let payload = frame[6..].to_vec();
                    self.eeprom_command(&payload);
                }
                other => panic!("unexpected frame header {other:?}"),
            }
            Ok(())
        }

        fn read_some(&mut self, buf: &mut [u8]) -> Result<usize, String> {
            match self.replies.pop_front() {
                Some(mut reply) => {
                    let n = reply.len().min(buf.len());
                    buf[..n].copy_from_slice(&reply[..n]);
                    reply.drain(..n);
                    if !reply.is_empty() {
                        self.replies.push_front(reply);
                    }
                    Ok(n)
                }
                None => Ok(0),
            }
        }
    }

    #[test]
    fn full_session_reads_and_resets_a_waste_counter() {
        let spec = xp530();
        let mut printer = FakePrinter::new(0x17);
        printer.eeprom.insert(0x10, 0x12);
        printer.eeprom.insert(0x11, 0x34);
        printer.eeprom.insert(0x06, 0x5F);

        let mut session = Session::open(&mut printer, &spec).expect("handshake");
        assert_eq!(session.socket(), 0x17, "socket resolved from GetSocketID");
        assert_eq!(
            session.channel_sizes(),
            (CHANNEL_PACKET + 2, CHANNEL_PACKET + 2),
            "sizes from OpenChannel"
        );

        let counter = &spec.counters[0];
        let before = session.read_counter(counter).unwrap();
        assert_eq!(before, vec![0x3412, 0x0F], "little endian registers + mask");

        assert_eq!(session.reset_waste().unwrap(), 2);
        let after = session.read_counter(counter).unwrap();
        assert_eq!(after, vec![0x0000, 0x0F]);

        session.close().unwrap();
        assert_eq!(&printer.events[..4], ["enter", "init", "getsocket", "open"]);
        assert_eq!(printer.events.last().copied(), Some("exit"));
        assert_eq!(printer.eeprom.get(&0x10), Some(&0));
        assert_eq!(printer.eeprom.get(&0x11), Some(&0));
        assert!(printer.events.iter().any(|event| *event == "credit-request"));
        assert!(printer.events.iter().any(|event| *event == "credit"));
    }

    #[test]
    fn full_session_reads_a_two_byte_address() {
        let mut spec = xp530();
        spec.counters[0].entries = vec![Entry {
            addrs: vec![0x01ED],
            ..Default::default()
        }];
        spec.counters.truncate(1);

        let mut printer = FakePrinter::new(0x10);
        printer.eeprom.insert(0x01ED, 0x7A);

        let mut session = Session::open(&mut printer, &spec).expect("handshake");
        assert_eq!(session.read_counter(&spec.counters[0]).unwrap(), vec![0x7A]);
        session.close().unwrap();
    }
}
