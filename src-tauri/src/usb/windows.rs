//! Windows USB printer-class bridge.
//!
//! Enumerates `GUID_DEVINTERFACE_USBPRINT`
//! (`{28d78fad-5a12-11d1-ae5b-0000f803a8c2}`) with SetupAPI and speaks
//! IEEE 1284.4 over the opened device interface. This module enumerates the
//! interfaces, reads their IEEE 1284 device ID, and exposes an overlapped byte
//! pipe (`UsbTransport`) to the 1284.4 session. Counter writes stay gated
//! behind a profile matched from the printer's own `MDL:` string plus a typed
//! confirmation in the command layer.

use std::ffi::c_void;
use std::ptr;

use super::{Trace, UsbPrinter};

type Handle = isize;
type Bool = i32;

const INVALID_HANDLE_VALUE: Handle = -1;

const DIGCF_PRESENT: u32 = 0x0000_0002;
const DIGCF_DEVICEINTERFACE: u32 = 0x0000_0010;

const ERROR_NO_MORE_ITEMS: u32 = 259;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;

const SPDRP_DEVICEDESC: u32 = 0x0000_0000;
const SPDRP_HARDWAREID: u32 = 0x0000_0001;
const SPDRP_FRIENDLYNAME: u32 = 0x0000_000c;

const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const OPEN_EXISTING: u32 = 3;

/// `IOCTL_USBPRINT_GET_1284_ID`
/// CTL_CODE(FILE_DEVICE_UNKNOWN, 13, METHOD_BUFFERED, FILE_ANY_ACCESS)
const IOCTL_USBPRINT_GET_1284_ID: u32 = 0x0022_0034;

/// `IOCTL_USBPRINT_GET_PROTOCOL` / `SET_PROTOCOL` / `GET_INTERFACE_TYPE`.
/// CTL_CODE(FILE_DEVICE_UNKNOWN, index, METHOD_BUFFERED, FILE_ANY_ACCESS)
const IOCTL_USBPRINT_GET_PROTOCOL: u32 = 0x0022_0044;
const IOCTL_USBPRINT_SET_PROTOCOL: u32 = 0x0022_0048;
const IOCTL_USBPRINT_GET_INTERFACE_TYPE: u32 = 0x0022_004c;

/// `USB_PRINTER_PROTOCOL_BIDI` and `USB_PRINTER_PROTOCOL_IPPOVERUSB` from
/// usbprint.h. A Windows install with the Microsoft IPP class driver starts
/// every IPP-capable interface in the IPP-over-USB protocol, where the device
/// expects HTTP inside a 1284.4 socket and ignores the EJL handshake.
pub const PROTOCOL_BIDI: u32 = 2;
pub const PROTOCOL_IPP_OVER_USB: u32 = 4;

/// `USB_PRINTER_INTERFACE_CLASSIC` / `_IPP` / `_DUAL`.
pub const INTERFACE_CLASSIC: u32 = 1;
pub const INTERFACE_IPP: u32 = 2;
pub const INTERFACE_DUAL: u32 = 3;

/// What the USBPRINT interface reported about its current mode.
#[derive(Debug, Clone, Copy, Default)]
pub struct PortProtocol {
    pub interface_type: Option<u32>,
    pub protocol: Option<u32>,
    /// True when this open switched the port from IPP-over-USB to BIDI.
    pub switched_to_bidi: bool,
}

impl PortProtocol {
    pub fn protocol_name(&self) -> &'static str {
        match self.protocol {
            Some(PROTOCOL_BIDI) => "bidi",
            Some(PROTOCOL_IPP_OVER_USB) => "ipp-over-usb",
            Some(_) => "unknown",
            None => "unavailable",
        }
    }

    pub fn interface_name(&self) -> &'static str {
        match self.interface_type {
            Some(INTERFACE_CLASSIC) => "classic",
            Some(INTERFACE_IPP) => "ipp-only",
            Some(INTERFACE_DUAL) => "dual",
            Some(_) => "unknown",
            None => "unavailable",
        }
    }
}

fn query_dword(handle: Handle, code: u32) -> Option<u32> {
    let mut out = [0u8; 4];
    let mut returned: u32 = 0;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            code,
            ptr::null(),
            0,
            out.as_mut_ptr() as *mut c_void,
            out.len() as u32,
            &mut returned,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        return None;
    }
    // usbprint.sys reports Information = 1 for these DWORD queries even
    // though it fills the whole buffer, so the returned count is not a
    // usable validity check.
    Some(u32::from_le_bytes(out))
}

fn set_dword(handle: Handle, code: u32, value: u32) -> Result<(), u32> {
    let mut input = value.to_le_bytes();
    let mut returned: u32 = 0;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            code,
            input.as_mut_ptr() as *const c_void,
            input.len() as u32,
            ptr::null_mut(),
            0,
            &mut returned,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        Err(unsafe { GetLastError() })
    } else {
        Ok(())
    }
}

/// Put the interface in classic BIDI mode before a 1284.4 session.
///
/// `IOCTL_USBPRINT_SET_PROTOCOL` only accepts `BIDI`, so this is a one way
/// switch: the port returns to IPP-over-USB when Windows re-enumerates the
/// device (replug), which is also how the print queue recovers. An interface
/// that only exposes the IPP alternate setting cannot host this protocol at
/// all, and saying so is better than sending frames nobody answers.
fn ensure_bidi(handle: Handle) -> Result<PortProtocol, String> {
    let interface_type = query_dword(handle, IOCTL_USBPRINT_GET_INTERFACE_TYPE);
    let protocol = query_dword(handle, IOCTL_USBPRINT_GET_PROTOCOL);
    let mut info = PortProtocol {
        interface_type,
        protocol,
        switched_to_bidi: false,
    };
    if interface_type == Some(INTERFACE_IPP) {
        return Err(
            "This USBPRINT interface only exposes the IPP-over-USB alternate setting, so the classic IEEE 1284.4 handshake cannot run on it.".to_string(),
        );
    }
    match protocol {
        Some(PROTOCOL_BIDI) => Ok(info),
        Some(PROTOCOL_IPP_OVER_USB) => {
            set_dword(handle, IOCTL_USBPRINT_SET_PROTOCOL, PROTOCOL_BIDI).map_err(|code| {
                format!("IOCTL_USBPRINT_SET_PROTOCOL(BIDI) failed with Win32 error {code}")
            })?;
            info.switched_to_bidi = true;
            info.protocol = query_dword(handle, IOCTL_USBPRINT_GET_PROTOCOL);
            if info.protocol != Some(PROTOCOL_BIDI) {
                return Err(format!(
                    "the port reported protocol {:?} after switching to BIDI",
                    info.protocol
                ));
            }
            Ok(info)
        }
        Some(other) => Err(format!(
            "the USBPRINT interface reported unsupported protocol code {other}"
        )),
        // Older usbprint.sys builds have no protocol IOCTL: the interface is
        // classic by construction, so the session can go ahead.
        None => Ok(info),
    }
}

/// Read the current interface type and protocol without changing them.
pub fn port_protocol(path: &str) -> Result<PortProtocol, String> {
    let device = open_device(path, true)
        .or_else(|_| open_device(path, false))
        .map_err(|code| format!("CreateFileW failed with Win32 error {code}"))?;
    let interface_type = query_dword(device.0, IOCTL_USBPRINT_GET_INTERFACE_TYPE);
    let protocol = query_dword(device.0, IOCTL_USBPRINT_GET_PROTOCOL);
    if interface_type.is_none() && protocol.is_none() {
        return Err(format!(
            "IOCTL_USBPRINT_GET_PROTOCOL/GET_INTERFACE_TYPE failed with Win32 error {}",
            unsafe { GetLastError() }
        ));
    }
    Ok(PortProtocol {
        interface_type,
        protocol,
        switched_to_bidi: false,
    })
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

const GUID_DEVINTERFACE_USBPRINT: Guid = Guid {
    data1: 0x28d7_8fad,
    data2: 0x5a12,
    data3: 0x11d1,
    data4: [0xae, 0x5b, 0x00, 0x00, 0xf8, 0x03, 0xa8, 0xc2],
};

#[repr(C)]
struct SpDeviceInterfaceData {
    cb_size: u32,
    class_guid: Guid,
    flags: u32,
    reserved: usize,
}

#[repr(C)]
struct SpDevInfoData {
    cb_size: u32,
    class_guid: Guid,
    dev_inst: u32,
    reserved: usize,
}

#[link(name = "setupapi")]
extern "system" {
    fn SetupDiGetClassDevsW(
        class_guid: *const Guid,
        enumerator: *const u16,
        parent: isize,
        flags: u32,
    ) -> Handle;
    fn SetupDiEnumDeviceInterfaces(
        set: Handle,
        dev_info: *const SpDevInfoData,
        class_guid: *const Guid,
        index: u32,
        data: *mut SpDeviceInterfaceData,
    ) -> Bool;
    fn SetupDiGetDeviceInterfaceDetailW(
        set: Handle,
        data: *mut SpDeviceInterfaceData,
        detail: *mut u8,
        detail_size: u32,
        required: *mut u32,
        dev_info: *mut SpDevInfoData,
    ) -> Bool;
    fn SetupDiGetDeviceRegistryPropertyW(
        set: Handle,
        dev_info: *mut SpDevInfoData,
        property: u32,
        reg_type: *mut u32,
        buffer: *mut u8,
        size: u32,
        required: *mut u32,
    ) -> Bool;
    fn SetupDiGetDeviceInstanceIdW(
        set: Handle,
        dev_info: *mut SpDevInfoData,
        buffer: *mut u16,
        size: u32,
        required: *mut u32,
    ) -> Bool;
    fn SetupDiDestroyDeviceInfoList(set: Handle) -> Bool;
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *const c_void,
        disposition: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    fn DeviceIoControl(
        device: Handle,
        code: u32,
        in_buffer: *const c_void,
        in_size: u32,
        out_buffer: *mut c_void,
        out_size: u32,
        returned: *mut u32,
        overlapped: *mut c_void,
    ) -> Bool;
    fn CloseHandle(handle: Handle) -> Bool;
    fn GetLastError() -> u32;
}

struct DeviceHandle(Handle);

impl Drop for DeviceHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

fn open_device(path: &str, want_write: bool) -> Result<DeviceHandle, u32> {
    let name = wide(path);
    let access = if want_write {
        GENERIC_READ | GENERIC_WRITE
    } else {
        GENERIC_READ
    };
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            ptr::null(),
            OPEN_EXISTING,
            0,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        Err(unsafe { GetLastError() })
    } else {
        Ok(DeviceHandle(handle))
    }
}

fn read_1284_raw(path: &str) -> Result<String, String> {
    let device = open_device(path, true)
        .or_else(|_| open_device(path, false))
        .map_err(|code| format!("CreateFileW failed with Win32 error {code}"))?;

    let mut buffer = vec![0u8; 4096];
    let mut returned: u32 = 0;
    let ok = unsafe {
        DeviceIoControl(
            device.0,
            IOCTL_USBPRINT_GET_1284_ID,
            ptr::null(),
            0,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut returned,
            ptr::null_mut(),
        )
    };

    if ok == 0 {
        return Err(format!(
            "IOCTL_USBPRINT_GET_1284_ID failed with Win32 error {}",
            unsafe { GetLastError() }
        ));
    }

    let len = (returned as usize).min(buffer.len());
    Ok(super::ieee1284::decode_id(&buffer[..len]))
}

unsafe fn registry_string(set: Handle, info: &mut SpDevInfoData, property: u32) -> Option<String> {
    let mut buffer = vec![0u8; 2048];
    let mut needed: u32 = 0;
    let ok = SetupDiGetDeviceRegistryPropertyW(
        set,
        info,
        property,
        ptr::null_mut(),
        buffer.as_mut_ptr(),
        buffer.len() as u32,
        &mut needed,
    );
    if ok == 0 {
        return None;
    }
    let words: Vec<u16> = buffer
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let value = from_wide(&words).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

unsafe fn instance_id(set: Handle, info: &mut SpDevInfoData) -> Option<String> {
    let mut buffer = vec![0u16; 512];
    let mut needed: u32 = 0;
    let ok = SetupDiGetDeviceInstanceIdW(
        set,
        info,
        buffer.as_mut_ptr(),
        buffer.len() as u32,
        &mut needed,
    );
    if ok == 0 {
        None
    } else {
        Some(from_wide(&buffer))
    }
}

unsafe fn interface_path(
    set: Handle,
    data: &mut SpDeviceInterfaceData,
    info: &mut SpDevInfoData,
) -> Result<String, u32> {
    let mut needed: u32 = 0;
    SetupDiGetDeviceInterfaceDetailW(
        set,
        data,
        ptr::null_mut(),
        0,
        &mut needed,
        ptr::null_mut(),
    );

    if needed == 0 || GetLastError() != ERROR_INSUFFICIENT_BUFFER {
        return Err(GetLastError());
    }

    let mut detail = vec![0u8; needed as usize];
    // cbSize of SP_DEVICE_INTERFACE_DETAIL_DATA_W: 6 on x86, 8 on x64.
    let cb_size = if cfg!(target_pointer_width = "64") { 8u32 } else { 6u32 };
    detail[..4].copy_from_slice(&cb_size.to_le_bytes());

    let ok = SetupDiGetDeviceInterfaceDetailW(
        set,
        data,
        detail.as_mut_ptr(),
        detail.len() as u32,
        ptr::null_mut(),
        info,
    );
    if ok == 0 {
        return Err(GetLastError());
    }

    let path_words: Vec<u16> = detail[4..]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&c| c != 0)
        .collect();

    if path_words.is_empty() {
        Err(0)
    } else {
        Ok(String::from_utf16_lossy(&path_words))
    }
}

fn split_vid_pid(hardware_id: &str, path: &str) -> (String, String) {
    // A USBPRINT interface path spells the ids out (`usb#vid_04b8&pid_1114#`),
    // while the node's hardware id is often just `USBPRINT\EpsonXP-530...`,
    // so try the path first and the hardware id as a fallback.
    let pick = |source: &str, prefix: &str| -> Option<String> {
        let upper = source.to_ascii_uppercase();
        let at = upper.find(prefix)?;
        let start = at + prefix.len();
        upper.get(start..start + 4).map(str::to_string)
    };
    let vid = pick(path, "VID_")
        .or_else(|| pick(hardware_id, "VID_"))
        .unwrap_or_default();
    let pid = pick(path, "PID_")
        .or_else(|| pick(hardware_id, "PID_"))
        .unwrap_or_default();
    (vid, pid)
}

pub fn enumerate() -> Result<Vec<UsbPrinter>, String> {
    unsafe {
        let set = SetupDiGetClassDevsW(
            &GUID_DEVINTERFACE_USBPRINT,
            ptr::null(),
            0,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );

        if set == INVALID_HANDLE_VALUE || set == 0 {
            return Err(format!(
                "SetupDiGetClassDevsW failed with Win32 error {}",
                GetLastError()
            ));
        }

        let mut printers = Vec::new();
        let mut index = 0u32;

        loop {
            let mut data = SpDeviceInterfaceData {
                cb_size: std::mem::size_of::<SpDeviceInterfaceData>() as u32,
                class_guid: GUID_DEVINTERFACE_USBPRINT,
                flags: 0,
                reserved: 0,
            };

            if SetupDiEnumDeviceInterfaces(set, ptr::null(), &GUID_DEVINTERFACE_USBPRINT, index, &mut data)
                == 0
            {
                let code = GetLastError();
                if code == ERROR_NO_MORE_ITEMS {
                    break;
                }
                SetupDiDestroyDeviceInfoList(set);
                return Err(format!(
                    "SetupDiEnumDeviceInterfaces failed with Win32 error {code}"
                ));
            }
            index += 1;

            let mut info = SpDevInfoData {
                cb_size: std::mem::size_of::<SpDevInfoData>() as u32,
                class_guid: GUID_DEVINTERFACE_USBPRINT,
                dev_inst: 0,
                reserved: 0,
            };

            let Ok(path) = interface_path(set, &mut data, &mut info) else {
                continue;
            };

            let hardware_id = registry_string(set, &mut info, SPDRP_HARDWAREID)
                .or_else(|| registry_string(set, &mut info, SPDRP_DEVICEDESC))
                .unwrap_or_default();
            let friendly_name = registry_string(set, &mut info, SPDRP_FRIENDLYNAME)
                .or_else(|| registry_string(set, &mut info, SPDRP_DEVICEDESC))
                .unwrap_or_else(|| "USB printer".to_string());
            let (vendor_id, product_id) = split_vid_pid(&hardware_id, &path);

            let mut printer = UsbPrinter {
                path: path.clone(),
                instance_id: instance_id(set, &mut info).unwrap_or_default(),
                friendly_name,
                hardware_id,
                vendor_id,
                product_id,
                ieee1284_id: None,
                manufacturer: None,
                model: None,
                serial: None,
            };

            if let Ok(raw) = read_1284_raw(&path) {
                if !raw.is_empty() {
                    printer.apply_ieee1284(raw);
                }
            }

            printers.push(printer);
        }

        SetupDiDestroyDeviceInfoList(set);
        printers.sort_by(|a, b| a.friendly_name.cmp(&b.friendly_name));
        Ok(printers)
    }
}

pub fn read_ieee1284_id(path: &str) -> Result<String, String> {
    let raw = read_1284_raw(path)?;
    if raw.is_empty() {
        Err("The device returned an empty IEEE 1284 ID".to_string())
    } else {
        Ok(raw)
    }
}

// ---------------------------------------------------------------------------
// Overlapped printer-class transport (IEEE 1284.4 byte pipe)
// ---------------------------------------------------------------------------

/// `OVERLAPPED` from `minwinbase.h`. Sequential use only: one event per
/// transport, reset before every operation.
#[repr(C)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: Handle,
}

const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;
const ERROR_IO_PENDING: u32 = 997;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_TIMEOUT: u32 = 258;

#[link(name = "kernel32")]
extern "system" {
    fn ReadFile(
        file: Handle,
        buffer: *mut c_void,
        to_read: u32,
        read: *mut u32,
        overlapped: *mut Overlapped,
    ) -> Bool;
    fn WriteFile(
        file: Handle,
        buffer: *const c_void,
        to_write: u32,
        written: *mut u32,
        overlapped: *mut Overlapped,
    ) -> Bool;
    fn GetOverlappedResult(
        file: Handle,
        overlapped: *mut Overlapped,
        transferred: *mut u32,
        wait: Bool,
    ) -> Bool;
    fn CancelIoEx(file: Handle, overlapped: *mut Overlapped) -> Bool;
    fn CreateEventW(
        attributes: *const c_void,
        manual_reset: Bool,
        initial_state: Bool,
        name: *const u16,
    ) -> Handle;
    fn ResetEvent(event: Handle) -> Bool;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
}

/// 2500 ms per reply byte batch and a generous 5 s for writes.
const READ_TIMEOUT_MS: u32 = 2_500;
const WRITE_TIMEOUT_MS: u32 = 5_000;

/// A raw byte pipe over an opened USBPRINT device interface.
pub struct UsbTransport {
    handle: Handle,
    event: Handle,
    path: String,
    trace: Option<Trace>,
    protocol: PortProtocol,
}

impl UsbTransport {
    pub fn open(path: &str) -> Result<Self, String> {
        let name = wide(path);
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                0,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(format!(
                "CreateFileW({path}) failed with Win32 error {}",
                unsafe { GetLastError() }
            ));
        }
        // The EJL handshake only exists in the classic (BIDI) alternate
        // setting; an interface left in IPP-over-USB accepts the write and
        // never answers it.
        let protocol = match ensure_bidi(handle) {
            Ok(protocol) => protocol,
            Err(error) => {
                unsafe { CloseHandle(handle) };
                return Err(error);
            }
        };
        let event = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
        if event == 0 {
            let code = unsafe { GetLastError() };
            unsafe { CloseHandle(handle) };
            return Err(format!("CreateEventW failed with Win32 error {code}"));
        }
        Ok(Self {
            handle,
            event,
            path: path.to_string(),
            trace: None,
            protocol,
        })
    }

    /// Interface type and protocol the port was in when the session opened.
    pub fn protocol(&self) -> PortProtocol {
        self.protocol
    }

    /// Start recording the bytes exchanged on this pipe.
    pub fn enable_trace(&mut self) {
        self.trace.get_or_insert_with(Trace::default);
    }

    /// Stop recording and hand the trace to the caller.
    pub fn take_trace(&mut self) -> Option<Trace> {
        self.trace.take()
    }

    fn perform(
        &mut self,
        write: bool,
        buffer: *mut u8,
        length: usize,
        timeout_ms: u32,
    ) -> Result<usize, String> {
        if length == 0 {
            return Ok(0);
        }
        let mut overlapped: Overlapped = unsafe { std::mem::zeroed() };
        overlapped.event = self.event;
        unsafe { ResetEvent(self.event) };

        let started = unsafe {
            if write {
                WriteFile(
                    self.handle,
                    buffer as *const c_void,
                    length as u32,
                    ptr::null_mut(),
                    &mut overlapped,
                )
            } else {
                ReadFile(
                    self.handle,
                    buffer as *mut c_void,
                    length as u32,
                    ptr::null_mut(),
                    &mut overlapped,
                )
            }
        };

        if started == 0 {
            let code = unsafe { GetLastError() };
            if code != ERROR_IO_PENDING {
                return Err(format!(
                    "{} on {} failed with Win32 error {code}",
                    if write { "WriteFile" } else { "ReadFile" },
                    self.path
                ));
            }
        }

        match unsafe { WaitForSingleObject(self.event, timeout_ms) } {
            WAIT_OBJECT_0 => {
                let mut transferred: u32 = 0;
                let ok = unsafe {
                    GetOverlappedResult(self.handle, &mut overlapped, &mut transferred, 0)
                };
                if ok == 0 {
                    let code = unsafe { GetLastError() };
                    return Err(format!(
                        "{} on {} completed with Win32 error {code}",
                        if write { "WriteFile" } else { "ReadFile" },
                        self.path
                    ));
                }
                Ok(transferred as usize)
            }
            WAIT_TIMEOUT => {
                // Cancel, then reap the operation before this stack allocated
                // OVERLAPPED goes out of scope: GetOverlappedResult with
                // `wait = 1` blocks until the I/O manager has finished with it.
                let mut transferred: u32 = 0;
                unsafe {
                    CancelIoEx(self.handle, &mut overlapped);
                    GetOverlappedResult(self.handle, &mut overlapped, &mut transferred, 1);
                }
                Err(format!(
                    "{} on {} timed out after {timeout_ms} ms",
                    if write { "Write" } else { "Read" },
                    self.path
                ))
            }
            code => Err(format!("WaitForSingleObject returned {code}")),
        }
    }
}

impl Drop for UsbTransport {
    fn drop(&mut self) {
        unsafe {
            CancelIoEx(self.handle, ptr::null_mut());
            CloseHandle(self.handle);
            CloseHandle(self.event);
        }
    }
}

impl crate::protocol::Transport for UsbTransport {
    fn write_all(&mut self, data: &[u8]) -> Result<(), String> {
        if let Some(trace) = &mut self.trace {
            trace.push_tx(data);
        }
        let mut offset = 0;
        while offset < data.len() {
            let slice = &data[offset..];
            let written = self.perform(
                true,
                slice.as_ptr() as *mut u8,
                slice.len(),
                WRITE_TIMEOUT_MS,
            )?;
            if written == 0 {
                return Err("WriteFile accepted no bytes".to_string());
            }
            offset += written;
        }
        Ok(())
    }

    fn read_some(&mut self, buf: &mut [u8]) -> Result<usize, String> {
        let read = self.perform(false, buf.as_mut_ptr(), buf.len(), READ_TIMEOUT_MS)?;
        if read > 0 {
            if let Some(trace) = &mut self.trace {
                trace.push_rx(&buf[..read]);
            }
        }
        Ok(read)
    }

    /// Keep reading 256 byte chunks until the port reports nothing pending. A timeout is the normal end of the flush, so it is not an
    /// error here; the read timeout doubles as the "pipe is empty" signal.
    fn drain(&mut self) -> usize {
        const CHUNK: usize = 256;
        const MAX_CHUNKS: usize = 32;
        let mut scratch = [0u8; CHUNK];
        let mut discarded = 0usize;
        for _ in 0..MAX_CHUNKS {
            match self.read_some(&mut scratch) {
                Ok(0) | Err(_) => break,
                Ok(read) => discarded += read,
            }
        }
        discarded
    }
}
