# alluno-vdd-sys

Rust bindings for the Alluno Virtual Display Driver (AllunoVDD).

Provides safe, ergonomic access to the AllunoVDD kernel driver via Windows IOCTL, allowing userspace applications to create, configure, and remove virtual displays.

## Features

- Add/remove virtual displays with configurable resolution, refresh rate, and bit depth
- Fractional refresh rate support (vsync numerator/denominator)
- HDR support (HDR10, HDR10+) with ST.2086 mastering display metadata
- Custom EDID injection (128 or 256 bytes)
- GPU render adapter selection
- Watchdog timer management
- Display mode hot-update
- Up to 16 simultaneous virtual displays

## Requirements

- Windows 10/11
- AllunoVDD driver installed (from [alluno-vdd](https://github.com/alluno-io/alluno-vdd))

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
alluno-vdd-sys = "1.0"
```

```rust
use alluno_vdd_sys::*;

let device = AllunoVdd::open().expect("driver not installed");
let version = device.get_version().unwrap();
println!("Driver v{}.{}.{}", version.major, version.minor, version.patch);

device.set_watchdog(0).unwrap(); // disable watchdog
let result = device.add_display(1920, 1080, 60, "Test", 8, 0).unwrap();
println!("Added display, target_id={}", result.target_id);

let list = device.list_displays().unwrap();
for d in &list {
    println!("{}x{} @{}Hz", d.width, d.height, d.refresh_rate);
}

device.remove_all().unwrap();
```

## API

| Method | Description |
|---|---|
| `AllunoVdd::open()` | Open a handle to the driver |
| `get_version()` | Query driver protocol version |
| `ping()` | Reset watchdog countdown |
| `add_display()` | Add a virtual display |
| `add_display_ex()` | Add with full parameter control (fractional refresh, serial, GUID) |
| `remove_display()` | Remove a display by GUID |
| `remove_all()` | Remove all virtual displays |
| `update_mode()` | Hot-update resolution/refresh/HDR on an existing display |
| `update_mode_ex()` | Hot-update with fractional refresh rate |
| `list_displays()` | List all active virtual displays |
| `set_render_adapter()` | Set which GPU renders to virtual displays |
| `get_watchdog()` / `set_watchdog()` | Query/configure watchdog timer |
| `set_hdr()` / `set_hdr_with_metadata()` | Enable HDR with optional ST.2086 metadata |
| `set_custom_edid()` | Inject custom EDID (128 or 256 bytes) |
| `is_compatible()` | Check driver protocol compatibility |

## Testing

Tests require the AllunoVDD driver to be installed:

```sh
cargo test -- --nocapture
```

## License

[MIT](LICENSE)
