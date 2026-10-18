//! BladeRF2 driver module.
//!
//! Re-exports the primary API surface for the BladeRF2 (xA4/xA9) SDR,
//! including the [`BladeRf2`] device handle, session types, streaming
//! builders, hardware abstraction types, and DC calibration table management.

// compile_error!("bladerf2 support is not yet implemented");

pub mod board;

pub use board::{BLADERF2_USB_PID, BLADERF2_USB_VID, BladeRf2};
