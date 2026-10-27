#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn parse_sockets(args: &[String]) -> Vec<u8> {
    let mut sockets = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let value = if arg == "--sockets" {
            iter.next().map(String::as_str)
        } else {
            arg.strip_prefix("--sockets=")
        };
        let Some(value) = value else { continue };
        for item in value.split(',') {
            if let Ok(byte) = u8::from_str_radix(item.trim().trim_start_matches("0x"), 16) {
                sockets.push(byte);
            }
        }
    }
    if sockets.is_empty() {
        sockets.push(0x0A);
        sockets.push(0xFF);
    }
    sockets
}

fn parse_names(args: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let value = if arg == "--names" {
            iter.next().map(String::as_str)
        } else {
            arg.strip_prefix("--names=")
        };
        let Some(value) = value else { continue };
        for item in value.split(',') {
            let item = item.trim();
            if !item.is_empty() {
                names.push(item.to_string());
            }
        }
    }
    names
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let probe = args.iter().any(|arg| arg == "--probe");
    let read_counters = args.iter().any(|arg| arg == "--read-counters");
    let raw_session = args.iter().any(|arg| arg == "--raw-session");
    let self_test = args.iter().any(|arg| arg == "--self-test");

    if self_test {
        let report = inkreset_studio_lib::self_test_json();
        let path = std::env::temp_dir().join("inkreset-self-test.json");
        let _ = std::fs::write(&path, &report);
        println!("{report}");
        return;
    }

    if raw_session {
        let report =
            inkreset_studio_lib::raw_session_json(&parse_sockets(&args), &parse_names(&args));
        let path = std::env::temp_dir().join("inkreset-raw.json");
        let _ = std::fs::write(&path, &report);
        println!("{report}");
        return;
    }

    if probe || read_counters {
        let report = inkreset_studio_lib::probe_json(read_counters);
        let path = std::env::temp_dir().join("inkreset-probe.json");
        let _ = std::fs::write(&path, report);
        return;
    }

    inkreset_studio_lib::run()
}
