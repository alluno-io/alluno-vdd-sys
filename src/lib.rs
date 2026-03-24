//! Rust bindings for Alluno Virtual Display Driver (AllunoVDD)
//!
//! # Example
//! ```no_run
//! use alluno_vdd_sys::*;
//!
//! let device = AllunoVdd::open().expect("driver not installed");
//! let version = device.get_version().unwrap();
//! println!("Driver v{}.{}.{}", version.major, version.minor, version.patch);
//!
//! device.set_watchdog(0).unwrap(); // disable watchdog
//! let result = device.add_display(1920, 1080, 60, "Test", 8, 0).unwrap();
//! println!("Added display, target_id={}", result.target_id);
//!
//! let list = device.list_displays().unwrap();
//! for d in &list {
//!     println!("{}x{} @{}Hz", d.width, d.height, d.refresh_rate);
//! }
//!
//! device.remove_all().unwrap();
//! ```

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use windows::core::{Error, Result, GUID, HRESULT};
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::DeviceIoControl;

// ============================================================================
// Constants
// ============================================================================

/// Device interface GUID: {A1142000-7DD0-4A11-4200-A114200007DD}
pub const ALLUNO_VDD_INTERFACE_GUID: GUID = GUID::from_u128(0xA1142000_7DD0_4A11_4200_A114200007DD);

pub const ALLUNO_VDD_MAX_DISPLAYS: usize = 16;
pub const ALLUNO_VDD_EDID_MAX_SIZE: usize = 256;
pub const ALLUNO_VDD_DEVICE_NAME_LEN: usize = 14;
pub const ALLUNO_VDD_SERIAL_LEN: usize = 14;

pub const ALLUNO_VDD_BPC_8: u32 = 8;
pub const ALLUNO_VDD_BPC_10: u32 = 10;
pub const ALLUNO_VDD_BPC_12: u32 = 12;

pub const ALLUNO_VDD_HDR_OFF: u32 = 0;
pub const ALLUNO_VDD_HDR_HDR10: u32 = 1;
pub const ALLUNO_VDD_HDR_HDR10_PLUS: u32 = 2;

pub const ALLUNO_VDD_PROTOCOL_MAJOR: u32 = 2;
pub const ALLUNO_VDD_PROTOCOL_MINOR: u32 = 0;
pub const ALLUNO_VDD_PROTOCOL_PATCH: u32 = 0;

// IOCTL codes: CTL_CODE(FILE_DEVICE_UNKNOWN=0x22, function, METHOD_BUFFERED=0, FILE_ANY_ACCESS=0)
#[allow(clippy::identity_op)]
const fn ctl_code(function: u32) -> u32 {
    (0x22 << 16) | (0 << 14) | (function << 2) | 0 // FILE_DEVICE_UNKNOWN, FILE_ANY_ACCESS, METHOD_BUFFERED
}

const IOCTL_ALLUNO_VDD_ADD_DISPLAY: u32 = ctl_code(0x800);
const IOCTL_ALLUNO_VDD_REMOVE_DISPLAY: u32 = ctl_code(0x801);
const IOCTL_ALLUNO_VDD_SET_RENDER_ADAPTER: u32 = ctl_code(0x802);
const IOCTL_ALLUNO_VDD_GET_WATCHDOG: u32 = ctl_code(0x803);
const IOCTL_ALLUNO_VDD_UPDATE_MODE: u32 = ctl_code(0x804);
const IOCTL_ALLUNO_VDD_LIST_DISPLAYS: u32 = ctl_code(0x805);
const IOCTL_ALLUNO_VDD_REMOVE_ALL: u32 = ctl_code(0x806);
const IOCTL_ALLUNO_VDD_SET_WATCHDOG: u32 = ctl_code(0x807);
const IOCTL_ALLUNO_VDD_SET_HDR: u32 = ctl_code(0x808);
const IOCTL_ALLUNO_VDD_SET_CUSTOM_EDID: u32 = ctl_code(0x809);
const IOCTL_ALLUNO_VDD_PING: u32 = ctl_code(0x888);
const IOCTL_ALLUNO_VDD_GET_VERSION: u32 = ctl_code(0x8FF);

// ============================================================================
// Wire structs (must match alluno-vdd-ioctl.h, packed)
// ============================================================================

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawAddParams {
    width: u32,
    height: u32,
    refresh_rate: u32,
    monitor_guid: GUID,
    device_name: [u8; ALLUNO_VDD_DEVICE_NAME_LEN],
    serial_number: [u8; ALLUNO_VDD_SERIAL_LEN],
    bits_per_channel: u32,
    hdr_mode: u32,
    vsync_numerator: u32,
    vsync_denominator: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawAddResult {
    adapter_luid: i64,
    target_id: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawRemoveParams {
    monitor_guid: GUID,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawUpdateModeParams {
    monitor_guid: GUID,
    width: u32,
    height: u32,
    refresh_rate: u32,
    bits_per_channel: u32,
    hdr_mode: u32,
    vsync_numerator: u32,
    vsync_denominator: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawSetAdapterParams {
    adapter_luid: i64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawWatchdogParams {
    timeout_ms: u32,
    countdown_ms: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawSetWatchdogParams {
    timeout_ms: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawHdrMetadata {
    red_primary_x: u32,
    red_primary_y: u32,
    green_primary_x: u32,
    green_primary_y: u32,
    blue_primary_x: u32,
    blue_primary_y: u32,
    white_point_x: u32,
    white_point_y: u32,
    max_luminance: u32,
    min_luminance_x10000: u32,
    max_content_light_level: u32,
    max_frame_avg_light_level: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawDisplayInfo {
    monitor_guid: GUID,
    width: u32,
    height: u32,
    refresh_rate: u32,
    bits_per_channel: u32,
    hdr_mode: u32,
    adapter_luid: i64,
    target_id: u32,
    device_name: [u8; ALLUNO_VDD_DEVICE_NAME_LEN],
    active: i32,
    vsync_numerator: u32,
    vsync_denominator: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawListResult {
    count: u32,
    displays: [RawDisplayInfo; ALLUNO_VDD_MAX_DISPLAYS],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawSetHdrParams {
    monitor_guid: GUID,
    hdr_mode: u32,
    bits_per_channel: u32,
    has_metadata: u32,
    metadata: RawHdrMetadata,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawSetCustomEdidParams {
    monitor_guid: GUID,
    edid_size: u32,
    edid_data: [u8; ALLUNO_VDD_EDID_MAX_SIZE],
}

// ============================================================================
// Public types
// ============================================================================

/// Result of adding a virtual display.
#[derive(Debug, Clone)]
pub struct AddResult {
    pub adapter_luid: i64,
    pub target_id: u32,
    pub monitor_guid: GUID,
}

/// Protocol version.
#[derive(Debug, Clone)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// Watchdog state.
#[derive(Debug, Clone)]
pub struct WatchdogState {
    pub timeout_ms: u32,
    pub countdown_ms: u32,
}

/// Information about an active virtual display.
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub monitor_guid: GUID,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub bits_per_channel: u32,
    pub hdr_mode: u32,
    pub adapter_luid: i64,
    pub target_id: u32,
    pub device_name: String,
    pub active: bool,
    pub vsync_numerator: u32,
    pub vsync_denominator: u32,
}

/// ST.2086 HDR mastering display metadata.
#[derive(Debug, Clone, Default)]
pub struct HdrMetadata {
    pub red_primary_x: u32,
    pub red_primary_y: u32,
    pub green_primary_x: u32,
    pub green_primary_y: u32,
    pub blue_primary_x: u32,
    pub blue_primary_y: u32,
    pub white_point_x: u32,
    pub white_point_y: u32,
    pub max_luminance: u32,
    pub min_luminance_x10000: u32,
    pub max_content_light_level: u32,
    pub max_frame_avg_light_level: u32,
}

// ============================================================================
// Device handle
// ============================================================================

/// Handle to the Alluno VDD driver.
pub struct AllunoVdd {
    handle: HANDLE,
}

impl AllunoVdd {
    /// Open a handle to the Alluno VDD driver.
    pub fn open() -> Result<Self> {
        let mut list_size: u32 = 0;
        let cr = unsafe {
            CM_Get_Device_Interface_List_SizeW(
                &mut list_size,
                &ALLUNO_VDD_INTERFACE_GUID,
                None,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if cr != CR_SUCCESS || list_size <= 1 {
            return Err(Error::new(HRESULT(-1), "AllunoVDD device not found"));
        }

        let mut list = vec![0u16; list_size as usize];
        let cr = unsafe {
            CM_Get_Device_Interface_ListW(
                &ALLUNO_VDD_INTERFACE_GUID,
                None,
                &mut list,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if cr != CR_SUCCESS || list[0] == 0 {
            return Err(Error::new(
                HRESULT(-1),
                "Failed to enumerate device interfaces",
            ));
        }

        let path = windows::core::PCWSTR(list.as_ptr());

        let handle = unsafe {
            CreateFileW(
                path,
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )?
        };

        Ok(Self { handle })
    }

    // ---- IOCTL helpers ----

    fn ioctl_in_out<I, O>(&self, code: u32, input: &I) -> Result<O> {
        unsafe {
            let mut output: O = zeroed();
            let mut returned: u32 = 0;
            DeviceIoControl(
                self.handle,
                code,
                Some(input as *const I as *const c_void),
                size_of::<I>() as u32,
                Some(&mut output as *mut O as *mut c_void),
                size_of::<O>() as u32,
                Some(&mut returned),
                None,
            )?;
            Ok(output)
        }
    }

    fn ioctl_in<I>(&self, code: u32, input: &I) -> Result<()> {
        unsafe {
            let mut returned: u32 = 0;
            DeviceIoControl(
                self.handle,
                code,
                Some(input as *const I as *const c_void),
                size_of::<I>() as u32,
                None,
                0,
                Some(&mut returned),
                None,
            )?;
            Ok(())
        }
    }

    fn ioctl_out<O>(&self, code: u32) -> Result<O> {
        unsafe {
            let mut output: O = zeroed();
            let mut returned: u32 = 0;
            DeviceIoControl(
                self.handle,
                code,
                None,
                0,
                Some(&mut output as *mut O as *mut c_void),
                size_of::<O>() as u32,
                Some(&mut returned),
                None,
            )?;
            Ok(output)
        }
    }

    fn ioctl_void(&self, code: u32) -> Result<()> {
        unsafe {
            let mut returned: u32 = 0;
            DeviceIoControl(
                self.handle,
                code,
                None,
                0,
                None,
                0,
                Some(&mut returned),
                None,
            )?;
            Ok(())
        }
    }

    // ---- Public API ----

    /// Get driver protocol version.
    pub fn get_version(&self) -> Result<Version> {
        let raw: RawVersion = self.ioctl_out(IOCTL_ALLUNO_VDD_GET_VERSION)?;
        Ok(Version {
            major: raw.major,
            minor: raw.minor,
            patch: raw.patch,
        })
    }

    /// Ping the driver (reset watchdog countdown).
    pub fn ping(&self) -> Result<()> {
        self.ioctl_void(IOCTL_ALLUNO_VDD_PING)
    }

    /// Add a virtual display.
    pub fn add_display(
        &self,
        width: u32,
        height: u32,
        refresh_rate: u32,
        name: &str,
        bits_per_channel: u32,
        hdr_mode: u32,
    ) -> Result<AddResult> {
        let guid = guid_new();
        self.add_display_ex(
            width,
            height,
            refresh_rate,
            name,
            "",
            bits_per_channel,
            hdr_mode,
            guid,
            0,
            0,
        )
    }

    /// Add a virtual display with full control over all parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn add_display_ex(
        &self,
        width: u32,
        height: u32,
        refresh_rate: u32,
        name: &str,
        serial: &str,
        bits_per_channel: u32,
        hdr_mode: u32,
        monitor_guid: GUID,
        vsync_numerator: u32,
        vsync_denominator: u32,
    ) -> Result<AddResult> {
        let mut params: RawAddParams = unsafe { zeroed() };
        params.width = width;
        params.height = height;
        params.refresh_rate = refresh_rate;
        params.monitor_guid = monitor_guid;
        params.bits_per_channel = if bits_per_channel == 0 {
            8
        } else {
            bits_per_channel
        };
        params.hdr_mode = hdr_mode;
        params.vsync_numerator = vsync_numerator;
        params.vsync_denominator = vsync_denominator;
        copy_str_to_buf(name, &mut params.device_name);
        copy_str_to_buf(serial, &mut params.serial_number);

        let raw: RawAddResult = self.ioctl_in_out(IOCTL_ALLUNO_VDD_ADD_DISPLAY, &params)?;
        Ok(AddResult {
            adapter_luid: raw.adapter_luid,
            target_id: raw.target_id,
            monitor_guid,
        })
    }

    /// Remove a virtual display by GUID.
    pub fn remove_display(&self, monitor_guid: &GUID) -> Result<()> {
        let params = RawRemoveParams {
            monitor_guid: *monitor_guid,
        };
        self.ioctl_in(IOCTL_ALLUNO_VDD_REMOVE_DISPLAY, &params)
    }

    /// Remove all virtual displays.
    pub fn remove_all(&self) -> Result<()> {
        self.ioctl_void(IOCTL_ALLUNO_VDD_REMOVE_ALL)
    }

    /// Update display mode. Pass 0 for fields to keep unchanged. Use 0xFF for hdr_mode to keep unchanged.
    pub fn update_mode(
        &self,
        monitor_guid: &GUID,
        width: u32,
        height: u32,
        refresh_rate: u32,
        bits_per_channel: u32,
        hdr_mode: u32,
    ) -> Result<()> {
        self.update_mode_ex(
            monitor_guid,
            width,
            height,
            refresh_rate,
            bits_per_channel,
            hdr_mode,
            0,
            0,
        )
    }

    /// Update display mode with fractional refresh rate.
    #[allow(clippy::too_many_arguments)]
    pub fn update_mode_ex(
        &self,
        monitor_guid: &GUID,
        width: u32,
        height: u32,
        refresh_rate: u32,
        bits_per_channel: u32,
        hdr_mode: u32,
        vsync_numerator: u32,
        vsync_denominator: u32,
    ) -> Result<()> {
        let params = RawUpdateModeParams {
            monitor_guid: *monitor_guid,
            width,
            height,
            refresh_rate,
            bits_per_channel,
            hdr_mode,
            vsync_numerator,
            vsync_denominator,
        };
        self.ioctl_in(IOCTL_ALLUNO_VDD_UPDATE_MODE, &params)
    }

    /// List all active virtual displays.
    pub fn list_displays(&self) -> Result<Vec<DisplayInfo>> {
        let raw: RawListResult = self.ioctl_out(IOCTL_ALLUNO_VDD_LIST_DISPLAYS)?;
        let count = raw.count as usize;
        let mut displays = Vec::with_capacity(count);
        for i in 0..count.min(ALLUNO_VDD_MAX_DISPLAYS) {
            let d = &raw.displays[i];
            displays.push(DisplayInfo {
                monitor_guid: d.monitor_guid,
                width: d.width,
                height: d.height,
                refresh_rate: d.refresh_rate,
                bits_per_channel: d.bits_per_channel,
                hdr_mode: d.hdr_mode,
                adapter_luid: d.adapter_luid,
                target_id: d.target_id,
                device_name: buf_to_string(&d.device_name),
                active: d.active != 0,
                vsync_numerator: d.vsync_numerator,
                vsync_denominator: d.vsync_denominator,
            });
        }
        Ok(displays)
    }

    /// Set which GPU renders to virtual displays.
    pub fn set_render_adapter(&self, adapter_luid: i64) -> Result<()> {
        let params = RawSetAdapterParams { adapter_luid };
        self.ioctl_in(IOCTL_ALLUNO_VDD_SET_RENDER_ADAPTER, &params)
    }

    /// Get watchdog state.
    pub fn get_watchdog(&self) -> Result<WatchdogState> {
        let raw: RawWatchdogParams = self.ioctl_out(IOCTL_ALLUNO_VDD_GET_WATCHDOG)?;
        Ok(WatchdogState {
            timeout_ms: raw.timeout_ms,
            countdown_ms: raw.countdown_ms,
        })
    }

    /// Set watchdog timeout in milliseconds. 0 = disable.
    pub fn set_watchdog(&self, timeout_ms: u32) -> Result<()> {
        let params = RawSetWatchdogParams { timeout_ms };
        self.ioctl_in(IOCTL_ALLUNO_VDD_SET_WATCHDOG, &params)
    }

    /// Set HDR mode on a display.
    pub fn set_hdr(&self, monitor_guid: &GUID, hdr_mode: u32, bits_per_channel: u32) -> Result<()> {
        let mut params: RawSetHdrParams = unsafe { zeroed() };
        params.monitor_guid = *monitor_guid;
        params.hdr_mode = hdr_mode;
        params.bits_per_channel = bits_per_channel;
        self.ioctl_in(IOCTL_ALLUNO_VDD_SET_HDR, &params)
    }

    /// Set HDR mode with ST.2086 mastering display metadata.
    pub fn set_hdr_with_metadata(
        &self,
        monitor_guid: &GUID,
        hdr_mode: u32,
        bits_per_channel: u32,
        metadata: &HdrMetadata,
    ) -> Result<()> {
        let mut params: RawSetHdrParams = unsafe { zeroed() };
        params.monitor_guid = *monitor_guid;
        params.hdr_mode = hdr_mode;
        params.bits_per_channel = bits_per_channel;
        params.has_metadata = 1;
        params.metadata = RawHdrMetadata {
            red_primary_x: metadata.red_primary_x,
            red_primary_y: metadata.red_primary_y,
            green_primary_x: metadata.green_primary_x,
            green_primary_y: metadata.green_primary_y,
            blue_primary_x: metadata.blue_primary_x,
            blue_primary_y: metadata.blue_primary_y,
            white_point_x: metadata.white_point_x,
            white_point_y: metadata.white_point_y,
            max_luminance: metadata.max_luminance,
            min_luminance_x10000: metadata.min_luminance_x10000,
            max_content_light_level: metadata.max_content_light_level,
            max_frame_avg_light_level: metadata.max_frame_avg_light_level,
        };
        self.ioctl_in(IOCTL_ALLUNO_VDD_SET_HDR, &params)
    }

    /// Set custom EDID for a display. edid must be 128 or 256 bytes.
    pub fn set_custom_edid(&self, monitor_guid: &GUID, edid: &[u8]) -> Result<()> {
        if edid.len() != 128 && edid.len() != 256 {
            return Err(Error::new(HRESULT(-1), "EDID must be 128 or 256 bytes"));
        }
        let mut params: RawSetCustomEdidParams = unsafe { zeroed() };
        params.monitor_guid = *monitor_guid;
        params.edid_size = edid.len() as u32;
        params.edid_data[..edid.len()].copy_from_slice(edid);
        self.ioctl_in(IOCTL_ALLUNO_VDD_SET_CUSTOM_EDID, &params)
    }

    /// Check if driver is compatible with this crate's protocol version.
    pub fn is_compatible(&self) -> Result<bool> {
        let v = self.get_version()?;
        #[allow(clippy::absurd_extreme_comparisons)]
        Ok(v.major == ALLUNO_VDD_PROTOCOL_MAJOR && v.minor >= ALLUNO_VDD_PROTOCOL_MINOR)
    }
}

impl Drop for AllunoVdd {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            let _ = unsafe { CloseHandle(self.handle) };
        }
    }
}

unsafe impl Send for AllunoVdd {}
unsafe impl Sync for AllunoVdd {}

// ============================================================================
// Helpers
// ============================================================================

fn copy_str_to_buf(s: &str, buf: &mut [u8]) {
    let bytes = s.as_bytes();
    let len = bytes.len().min(buf.len());
    buf[..len].copy_from_slice(&bytes[..len]);
}

fn buf_to_string(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).to_string()
}

fn guid_new() -> GUID {
    unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap_or(GUID::zeroed()) }
}
