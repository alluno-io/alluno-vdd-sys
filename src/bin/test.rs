//! AllunoVDD test — interactive virtual display management.
//!
//! Usage: alluno-vdd-test
//!
//! Press number keys to add/remove displays, or 'q' to quit.
//! All displays are cleaned up on exit (quit, Ctrl+C, or terminal close).

use alluno_vdd_sys::*;
use std::io::{self, BufRead, Write};
use std::sync::Arc;

fn main() {
    println!("AllunoVDD Test");
    println!("==============\n");

    let device = match AllunoVdd::open() {
        Ok(d) => Arc::new(d),
        Err(e) => {
            eprintln!("ERROR: Could not open AllunoVDD driver: {e}");
            eprintln!("  Is the AllunoVDD driver installed?");
            std::process::exit(1);
        }
    };

    match device.get_version() {
        Ok(v) => println!("Driver version: {}.{}.{}", v.major, v.minor, v.patch),
        Err(e) => println!("WARNING: get_version failed: {e}"),
    }

    // Disable watchdog so displays persist while the tool is running
    if let Err(e) = device.set_watchdog(0) {
        println!("WARNING: set_watchdog(0) failed: {e}");
    }

    // Register Ctrl+C / terminal close handler for cleanup
    let cleanup_device = Arc::clone(&device);
    ctrlc_cleanup(cleanup_device);

    print_displays(&device);
    print_menu();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let cmd = line.trim();

        match cmd {
            "1" => add_display(&device, 1920, 1080, 60, "1080p60", 8, 0),
            "2" => add_display(&device, 2560, 1440, 60, "1440p60", 8, 0),
            "3" => add_display(&device, 3840, 2160, 60, "4K60", 8, 0),
            "4" => add_display(&device, 1920, 1080, 144, "1080p144", 8, 0),
            "5" => add_display(&device, 2560, 1440, 144, "1440p144", 8, 0),
            "6" => add_display(&device, 3840, 2160, 120, "4K120", 8, 0),
            "7" => add_display(&device, 1280, 720, 60, "720p60", 8, 0),
            "8" => add_display(&device, 3440, 1440, 60, "UW1440p", 8, 0),
            "9" => add_display(&device, 5120, 1440, 60, "SUW1440", 8, 0),
            "h1" => add_display(&device, 1920, 1080, 60, "HDR1080", 10, ALLUNO_VDD_HDR_HDR10),
            "h2" => add_display(&device, 2560, 1440, 60, "HDR1440", 10, ALLUNO_VDD_HDR_HDR10),
            "h3" => add_display(&device, 3840, 2160, 60, "HDR4K", 10, ALLUNO_VDD_HDR_HDR10),
            "0" => remove_all(&device),
            "l" | "L" => print_displays(&device),
            "p" | "P" => match device.ping() {
                Ok(()) => println!("  Ping OK"),
                Err(e) => println!("  Ping failed: {e}"),
            },
            "q" | "Q" => break,
            _ => println!("  Unknown command: {cmd}"),
        }

        print_menu();
    }

    cleanup(&device);
}

fn add_display(device: &AllunoVdd, w: u32, h: u32, hz: u32, name: &str, bpc: u32, hdr: u32) {
    let hdr_str = match hdr {
        ALLUNO_VDD_HDR_HDR10 => " HDR10",
        ALLUNO_VDD_HDR_HDR10_PLUS => " HDR10+",
        _ => "",
    };
    print!("  Adding {name} ({w}x{h} @{hz}Hz {bpc}bpc{hdr_str})...");
    io::stdout().flush().ok();
    match device.add_display(w, h, hz, name, bpc, hdr) {
        Ok(r) => println!(" OK (target_id={}, guid={:?})", r.target_id, r.monitor_guid),
        Err(e) => println!(" FAILED: {e}"),
    }
}

fn remove_all(device: &AllunoVdd) {
    print!("  Removing all displays...");
    io::stdout().flush().ok();
    match device.remove_all() {
        Ok(()) => println!(" OK"),
        Err(e) => println!(" FAILED: {e}"),
    }
}

fn cleanup(device: &AllunoVdd) {
    print!("  Cleaning up...");
    io::stdout().flush().ok();
    match device.remove_all() {
        Ok(()) => println!(" all displays removed."),
        Err(e) => println!(" remove_all failed: {e}"),
    }
}

fn print_displays(device: &AllunoVdd) {
    match device.list_displays() {
        Ok(displays) => {
            if displays.is_empty() {
                println!("\n  No virtual displays active.\n");
            } else {
                println!("\n  Active displays ({}):", displays.len());
                for (i, d) in displays.iter().enumerate() {
                    let hdr_str = match d.hdr_mode {
                        ALLUNO_VDD_HDR_HDR10 => "  HDR10",
                        ALLUNO_VDD_HDR_HDR10_PLUS => "  HDR10+",
                        _ => "",
                    };
                    println!(
                        "    [{}] {}x{} @{}Hz  {}bpc{}  name=\"{}\"  guid={:?}",
                        i + 1,
                        d.width,
                        d.height,
                        d.refresh_rate,
                        d.bits_per_channel,
                        hdr_str,
                        d.device_name,
                        d.monitor_guid,
                    );
                }
                println!();
            }
        }
        Err(e) => println!("  list_displays failed: {e}"),
    }
}

fn print_menu() {
    println!("Commands:");
    println!("  1 = 1080p 60Hz      4 = 1080p 144Hz     7 = 720p 60Hz");
    println!("  2 = 1440p 60Hz      5 = 1440p 144Hz     8 = UW 3440x1440");
    println!("  3 = 4K 60Hz         6 = 4K 120Hz        9 = SUW 5120x1440");
    println!("  h1 = 1080p HDR10    h2 = 1440p HDR10    h3 = 4K HDR10");
    println!("  0 = Remove all      L = List displays    P = Ping");
    println!("  Q = Quit");
    print!("> ");
    io::stdout().flush().ok();
}

#[cfg(target_os = "windows")]
fn ctrlc_cleanup(device: Arc<AllunoVdd>) {
    use windows::Win32::System::Console::*;

    unsafe extern "system" fn handler(ctrl_type: u32) -> windows::core::BOOL {
        let _ = ctrl_type;
        if let Ok(dev) = AllunoVdd::open() {
            let _ = dev.remove_all();
        }
        eprintln!("\n  Cleaned up all displays.");
        windows::core::BOOL(0)
    }

    unsafe {
        let _ = SetConsoleCtrlHandler(Some(handler), true);
    }
    let _ = device;
}

#[cfg(not(target_os = "windows"))]
fn ctrlc_cleanup(_device: Arc<AllunoVdd>) {}
