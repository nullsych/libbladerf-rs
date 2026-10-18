//! Board-level operations for BladeRF2.
//!
//! This module contains [`BladeRf2`] — the primary device handle.

use crate::error::Error;
use crate::maybe_future::{Op, await_maybe, sleep};
use crate::nios_client::NiosCore;
use crate::usb::{
    BladeRf1DeviceCommands, BladeRf1UsbInterfaceCommands, DeviceCommands, UsbTransport,
};
#[cfg(not(target_os = "android"))]
use nusb::DeviceInfo;
use nusb::{Device, MaybeFuture, Speed};
use std::fmt;

/// Nuand BladeRF2 USB Vendor ID.
pub const BLADERF2_USB_VID: u16 = 0x2CF0;

/// Nuand BladeRF2 USB Product ID.
pub const BLADERF2_USB_PID: u16 = 0x5250;

#[cfg(not(target_os = "android"))]
fn is_bladerf2(dev: &DeviceInfo) -> bool {
    dev.vendor_id() == BLADERF2_USB_VID && dev.product_id() == BLADERF2_USB_PID
}

/// Primary device handle for the BladeRF2.
///
/// Owns the USB device and the internal `NiosCore`.
/// Construct via [`from_first`](BladeRf2::from_first),
/// [`from_serial`](BladeRf2::from_serial), or
/// [`from_device`](BladeRf2::from_device) with an already opened
/// [`nusb::Device`].
///
/// Every I/O method returns a [`MaybeFuture`]: call `.wait()` to block the
/// current thread (native targets only) or `.await` it from async code.
pub struct BladeRf2 {
    device: Device,
    nios: NiosCore,
}

impl BladeRf2 {
    /// Lists all BladeRF2 devices currently connected to the host.
    #[cfg(not(target_os = "android"))]
    pub fn list_bladerf2()
    -> impl MaybeFuture<Output = crate::Result<impl Iterator<Item = DeviceInfo>>> {
        nusb::list_devices()
            .map_ok(|devices| devices.filter(is_bladerf2))
            .map_err(Error::from)
    }

    fn build(device: Device) -> impl MaybeFuture<Output = crate::Result<Self>> {
        Op::new(async move {
            let manufacturer = device.manufacturer().await;
            let product = device.product().await;
            let serial = device.serial().await;
            log::debug!("Manufacturer: {manufacturer:?}");
            log::debug!("Product: {product:?}");
            log::debug!("Serial: {serial:?}");
            log::debug!("Speed: {:?}", device.speed());
            let interface = await_maybe(device.detach_and_claim_interface(0)).await?;
            let speed = device.speed().ok_or(Error::UnsupportedSpeed)?;
            if speed < Speed::High {
                log::error!("BladeRF requires High/Super/SuperPlus speeds");
                return Err(Error::UnsupportedSpeed);
            }
            let nios = NiosCore::new(UsbTransport::new(interface, speed));
            let mut result = Self { device, nios };
            result.wait_until_ready().await?;
            Ok(result)
        })
    }

    fn wait_until_ready(&mut self) -> impl MaybeFuture<Output = crate::Result<()>> {
        Op::new(async move {
            const MAX_RETRIES: u32 = 30;
            for i in 0..MAX_RETRIES {
                match self.nios.control().await?.usb_is_firmware_ready().await {
                    Ok(true) => return Ok(()),
                    Ok(false) => {
                        if i == 0 {
                            log::info!("Waiting for device to become ready...");
                        } else {
                            log::debug!("Retry {}/{}.", i + 1, MAX_RETRIES);
                        }
                        sleep(std::time::Duration::from_secs(1)).await;
                    }
                    Err(e) => {
                        log::warn!(
                            "Firmware does not support device ready query ({e:#}). \
                             Ensure flash-autoloading completes before opening the device."
                        );
                        return Ok(());
                    }
                }
            }
            log::debug!("Timed out while waiting for device.");
            Err(Error::Timeout)
        })
    }

    /// Opens the first BladeRF2 device found.
    #[cfg(not(target_os = "android"))]
    pub fn from_first() -> impl MaybeFuture<Output = crate::Result<Self>> {
        Op::new(async move {
            let info = await_maybe(Self::list_bladerf2())
                .await?
                .next()
                .ok_or(Error::NotFound)?;
            let device = await_maybe(info.open()).await?;
            Self::build(device).await
        })
    }

    /// Opens a BladeRF2 device matching the given serial number string.
    #[cfg(not(target_os = "android"))]
    pub fn from_serial(serial: &str) -> impl MaybeFuture<Output = crate::Result<Self>> {
        Op::new(async move {
            let info = await_maybe(Self::list_bladerf2())
                .await?
                .find(|dev| dev.serial_number() == Some(serial))
                .ok_or(Error::NotFound)?;
            let device = await_maybe(info.open()).await?;
            Self::build(device).await
        })
    }

    /// Opens a BladeRF2 from an already opened [`nusb::Device`].
    ///
    /// The device must be a BladeRF2; no VID/PID check is performed.
    pub fn from_device(device: Device) -> impl MaybeFuture<Output = crate::Result<Self>> {
        Self::build(device)
    }

    /// Returns the device serial number string.
    pub fn serial(&self) -> impl MaybeFuture<Output = crate::Result<String>> {
        self.device.serial()
    }

    /// Returns the FX3 firmware version as a string.
    pub fn fx3_firmware_version(&self) -> impl MaybeFuture<Output = crate::Result<String>> {
        self.device.fx3_firmware_version()
    }
}

impl fmt::Debug for BladeRf2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BladeRf2")
            .field("connection", &self.nios)
            .finish_non_exhaustive()
    }
}
