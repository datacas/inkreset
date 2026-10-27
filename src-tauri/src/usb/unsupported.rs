use super::UsbPrinter;

pub fn enumerate() -> Result<Vec<UsbPrinter>, String> {
    Ok(Vec::new())
}

pub fn read_ieee1284_id(_path: &str) -> Result<String, String> {
    Err("The native USB bridge is only available on Windows".to_string())
}
