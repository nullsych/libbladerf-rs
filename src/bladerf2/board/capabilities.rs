//! Version-derived device capabilities.
//!
//! Port of `capabilities.c`: each FX3 firmware and FPGA release enables a set
//! of features. Bit positions match `BLADERF_CAP_*` in libbladeRF's
//! `board.h`.

use crate::version::SemanticVersion;
use bitflags::bitflags;

bitflags! {
    /// Features available with the connected firmware and FPGA versions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Capabilities: u64 {
        /// Sample timestamps (FPGA >= 0.1.0).
        const TIMESTAMPS = 1 << 2;
        /// Scheduled retunes (FPGA >= 0.10.0).
        const SCHEDULED_RETUNE = 1 << 3;
        /// Packet handler format (FPGA >= 0.3.0).
        const PKT_HANDLER_FMT = 1 << 4;
        /// VCTCXO trim DAC readback (FPGA >= 0.3.2).
        const VCTCXO_TRIMDAC_READ = 1 << 5;
        /// Masked expansion GPIO writes (FPGA >= 0.4.1).
        const MASKED_XBIO_WRITE = 1 << 7;
        /// Synchronized RX/TX triggers (FPGA >= 0.6.0).
        const TRX_SYNC_TRIG = 1 << 9;
        /// FPGA-based RFIC tuning (FPGA >= 0.10.1).
        const FPGA_TUNING = 1 << 11;
        /// Packet metadata format (FPGA >= 0.12.0).
        const FPGA_PACKET_META = 1 << 12;
        /// Firmware loopback (firmware >= 1.7.1).
        const FW_LOOPBACK = 1 << 32;
        /// Device-ready query (firmware >= 1.8.0).
        const QUERY_DEVICE_READY = 1 << 33;
        /// Firmware log readout (firmware >= 1.9.0).
        const READ_FW_LOG_ENTRY = 1 << 34;
        /// BladeRF2 support in the firmware (firmware >= 2.1.0).
        const FW_SUPPORTS_BLADERF2 = 1 << 35;
        /// SPI flash JEDEC ID query (firmware >= 2.3.0).
        const FW_FLASH_ID = 1 << 36;
        /// FPGA source query (firmware >= 2.3.1).
        const FW_FPGA_SOURCE = 1 << 37;
        /// Short packet support (firmware >= 2.4.0).
        const FW_SHORT_PACKET = 1 << 38;
        /// 8-bit sample format (FPGA >= 0.15.0).
        const FPGA_8BIT_SAMPLES = 1 << 39;
    }
}

const FIRMWARE: [(SemanticVersion, Capabilities); 7] = [
    (SemanticVersion::new(1, 7, 1), Capabilities::FW_LOOPBACK),
    (
        SemanticVersion::new(1, 8, 0),
        Capabilities::QUERY_DEVICE_READY,
    ),
    (
        SemanticVersion::new(1, 9, 0),
        Capabilities::READ_FW_LOG_ENTRY,
    ),
    (
        SemanticVersion::new(2, 1, 0),
        Capabilities::FW_SUPPORTS_BLADERF2,
    ),
    (SemanticVersion::new(2, 3, 0), Capabilities::FW_FLASH_ID),
    (SemanticVersion::new(2, 3, 1), Capabilities::FW_FPGA_SOURCE),
    (SemanticVersion::new(2, 4, 0), Capabilities::FW_SHORT_PACKET),
];

const FPGA: [(SemanticVersion, Capabilities); 9] = [
    (SemanticVersion::new(0, 1, 0), Capabilities::TIMESTAMPS),
    (SemanticVersion::new(0, 3, 0), Capabilities::PKT_HANDLER_FMT),
    (
        SemanticVersion::new(0, 3, 2),
        Capabilities::VCTCXO_TRIMDAC_READ,
    ),
    (
        SemanticVersion::new(0, 4, 1),
        Capabilities::MASKED_XBIO_WRITE,
    ),
    (SemanticVersion::new(0, 6, 0), Capabilities::TRX_SYNC_TRIG),
    (
        SemanticVersion::new(0, 10, 0),
        Capabilities::SCHEDULED_RETUNE,
    ),
    (SemanticVersion::new(0, 10, 1), Capabilities::FPGA_TUNING),
    (
        SemanticVersion::new(0, 12, 0),
        Capabilities::FPGA_PACKET_META,
    ),
    (
        SemanticVersion::new(0, 15, 0),
        Capabilities::FPGA_8BIT_SAMPLES,
    ),
];

fn collect(table: &[(SemanticVersion, Capabilities)], version: SemanticVersion) -> Capabilities {
    table
        .iter()
        .filter(|(min, _)| version >= *min)
        .fold(Capabilities::empty(), |caps, (_, cap)| caps | *cap)
}

impl Capabilities {
    /// Capabilities provided by the given FX3 firmware version.
    pub fn from_firmware(version: SemanticVersion) -> Self {
        collect(&FIRMWARE, version)
    }

    /// Capabilities provided by the given FPGA version.
    pub fn from_fpga(version: SemanticVersion) -> Self {
        collect(&FPGA, version)
    }

    /// Capabilities provided by the given firmware and FPGA versions together.
    pub fn from_versions(firmware: SemanticVersion, fpga: SemanticVersion) -> Self {
        Self::from_firmware(firmware) | Self::from_fpga(fpga)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u16, minor: u16, patch: u16) -> SemanticVersion {
        SemanticVersion::new(major, minor, patch)
    }

    #[test]
    fn firmware_thresholds_are_inclusive() {
        assert_eq!(
            Capabilities::from_firmware(v(1, 7, 0)),
            Capabilities::empty()
        );
        assert_eq!(
            Capabilities::from_firmware(v(1, 7, 1)),
            Capabilities::FW_LOOPBACK
        );
        let caps = Capabilities::from_firmware(v(2, 3, 0));
        assert!(caps.contains(Capabilities::FW_FLASH_ID));
        assert!(!caps.contains(Capabilities::FW_FPGA_SOURCE));
        assert!(Capabilities::from_firmware(v(2, 3, 1)).contains(Capabilities::FW_FPGA_SOURCE));
        assert_eq!(
            Capabilities::from_firmware(v(2, 4, 0)),
            FIRMWARE
                .iter()
                .fold(Capabilities::empty(), |c, (_, f)| c | *f)
        );
    }

    #[test]
    fn fpga_thresholds_are_inclusive() {
        assert_eq!(Capabilities::from_fpga(v(0, 0, 9)), Capabilities::empty());
        let caps = Capabilities::from_fpga(v(0, 10, 0));
        assert!(caps.contains(Capabilities::SCHEDULED_RETUNE));
        assert!(!caps.contains(Capabilities::FPGA_TUNING));
        assert!(Capabilities::from_fpga(v(0, 10, 1)).contains(Capabilities::FPGA_TUNING));
        assert!(Capabilities::from_fpga(v(0, 16, 0)).contains(Capabilities::FPGA_8BIT_SAMPLES));
    }

    #[test]
    fn firmware_and_fpga_bits_do_not_overlap() {
        let fw = Capabilities::from_firmware(v(9, 9, 9));
        let fpga = Capabilities::from_fpga(v(9, 9, 9));
        assert!(fw.intersection(fpga).is_empty());
        assert_eq!(
            Capabilities::from_versions(v(9, 9, 9), v(9, 9, 9)),
            fw | fpga
        );
        assert_eq!(Capabilities::FW_LOOPBACK.bits(), 1 << 32);
        assert_eq!(Capabilities::FPGA_8BIT_SAMPLES.bits(), 1 << 39);
    }
}
