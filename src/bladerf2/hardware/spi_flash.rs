//! SPI flash access model.
//!
//! Page reads, calibration cache access, and flash geometry detection through
//! USB vendor commands. Port of the read paths of `driver/spi_flash.c` and
//! `spi_flash_decode_flash_architecture()` from `board/bladerf1/flash.c`,
//! which libbladeRF shares between both boards.

use crate::bladerf2::board::FlashSession;
use crate::error::{Error, Result};
use crate::flash::{FpgaSize, decode_flash_size};
use crate::maybe_future::Op;
use nusb::MaybeFuture;

pub(crate) mod layout;
pub(crate) use layout::FlashMeta;
use layout::{Page, PageIndex};

/// Size of a single flash page in bytes.
pub use crate::flash::BLADERF_FLASH_PAGE_SIZE;
/// Size of a flash erase block in bytes (64 KB).
pub const BLADERF_FLASH_ERASE_BLOCK_SIZE: usize = 64 * 1_024;
/// Flash address of the firmware region.
pub const BLADERF_FLASH_ADDR_FIRMWARE: u32 = 0x00000000;
/// Size of the firmware region in bytes.
pub const BLADERF_FLASH_BYTE_LEN_FIRMWARE: u32 = 0x00030000;
/// Flash address of the calibration data region.
pub const BLADERF_FLASH_ADDR_CAL: u32 = 0x00030000;
/// Size of the calibration data region in bytes.
pub const BLADERF_FLASH_BYTE_LEN_CAL: usize = 0x100;
/// Flash address of the FPGA bitstream region.
pub const BLADERF_FLASH_ADDR_FPGA: u32 = 0x00040000;

/// JEDEC manufacturer and device ID of the SPI flash chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlashId {
    /// JEDEC manufacturer ID (e.g. `0xEF` for Winbond).
    pub manufacturer: u8,
    /// JEDEC device ID.
    pub device: u8,
}

impl FlashId {
    pub(crate) fn from_query(result: u32) -> Self {
        Self {
            manufacturer: ((result >> 8) & 0xFF) as u8,
            device: (result & 0xFF) as u8,
        }
    }

    /// Returns the flash capacity in bytes for a known chip.
    ///
    /// # Errors
    /// Returns [`Error::Unsupported`] for an unknown manufacturer or device.
    pub fn size_bytes(self) -> Result<u32> {
        decode_flash_size(self.manufacturer, self.device)
    }
}

/// Flash capacity assumed when the chip cannot be identified.
///
/// Matches libbladeRF: xA9 boards carry 128 Mbit, all others 32 Mbit.
pub(crate) fn assumed_flash_size(fpga_size: Option<FpgaSize>) -> u32 {
    match fpga_size {
        Some(FpgaSize::A9) => 128 << 17,
        _ => 32 << 17,
    }
}

impl FlashSession<'_> {
    /// Reads the on-device calibration cache into the provided buffer.
    pub(crate) fn read_cal_cache(
        &mut self,
        buf: &mut Page,
    ) -> impl MaybeFuture<Output = Result<()>> {
        Op::new(async move {
            *buf = self.nios.control().await?.read_calibration_page().await?;
            Ok(())
        })
    }

    /// Reads one complete 256-byte flash page.
    ///
    /// # Errors
    /// Returns an argument error if `buf` is not exactly 256 bytes or `page`
    /// is out of range. USB and firmware failures are propagated.
    pub fn read_page(
        &mut self,
        page: u32,
        buf: &mut [u8],
    ) -> impl MaybeFuture<Output = Result<()>> {
        Op::new(async move {
            let buf = <&mut Page>::try_from(buf).map_err(|_| {
                Error::Argument("flash page buffer must be exactly 256 bytes".into())
            })?;
            let page = self.flash_meta.page(page)?;
            self.read_page_index(page, buf).await
        })
    }

    /// Returns the JEDEC ID of the flash chip, if the firmware reported one.
    ///
    /// `None` when the firmware predates the flash ID query (< 2.3.0).
    pub fn flash_id(&self) -> Option<FlashId> {
        self.flash_id
    }

    /// Returns the total flash capacity in bytes.
    pub fn size_bytes(&self) -> u32 {
        self.flash_meta.size_bytes()
    }

    /// Returns the total number of flash pages.
    pub fn total_pages(&self) -> u32 {
        self.flash_meta.total_pages()
    }

    /// Returns the total number of erasable flash sectors.
    pub fn total_sectors(&self) -> u32 {
        self.flash_meta.total_sectors()
    }

    /// Returns the number of erasable sectors available for FPGA bitstream storage.
    pub fn fpga_flash_sectors(&self) -> u32 {
        self.flash_meta.total_sectors()
            - BLADERF_FLASH_ADDR_FPGA / BLADERF_FLASH_ERASE_BLOCK_SIZE as u32
    }

    /// Returns the total FPGA bitstream storage capacity in bytes.
    pub fn fpga_flash_bytes(&self) -> usize {
        self.fpga_flash_sectors() as usize * BLADERF_FLASH_ERASE_BLOCK_SIZE
    }

    pub(crate) fn read_page_index(
        &mut self,
        index: PageIndex,
        page: &mut Page,
    ) -> impl MaybeFuture<Output = Result<()>> {
        Op::new(async move {
            *page = self
                .nios
                .control()
                .await?
                .read_flash_page(index.get())
                .await?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_id_splits_the_query_word() {
        let id = FlashId::from_query(0x0000_EF17);
        assert_eq!(
            id,
            FlashId {
                manufacturer: 0xEF,
                device: 0x17
            }
        );
        assert_eq!(id.size_bytes().unwrap(), 16 << 20);
        assert!(FlashId::from_query(0x0000_EF99).size_bytes().is_err());
    }

    #[test]
    fn unknown_chip_falls_back_by_fpga_size() {
        assert_eq!(assumed_flash_size(Some(FpgaSize::A9)), 16 << 20);
        assert_eq!(assumed_flash_size(Some(FpgaSize::A4)), 4 << 20);
        assert_eq!(assumed_flash_size(Some(FpgaSize::A5)), 4 << 20);
        assert_eq!(assumed_flash_size(None), 4 << 20);
    }
}
