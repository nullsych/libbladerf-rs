//! Firmware/FPGA compatibility tables.
//!
//! Port of `compatibility.c` and `version_check`/`version_check_fw` from
//! `helpers/version.c`. Tables are ordered newest first.

use crate::error::{Error, Result};
use crate::version::SemanticVersion;

struct Compat {
    version: SemanticVersion,
    requires: SemanticVersion,
}

const fn compat(version: (u16, u16, u16), requires: (u16, u16, u16)) -> Compat {
    Compat {
        version: SemanticVersion::new(version.0, version.1, version.2),
        requires: SemanticVersion::new(requires.0, requires.1, requires.2),
    }
}

const FIRMWARE_COMPAT: [Compat; 11] = [
    compat((2, 6, 1), (0, 16, 0)),
    compat((2, 6, 0), (0, 16, 0)),
    compat((2, 5, 0), (0, 16, 0)),
    compat((2, 4, 0), (0, 6, 0)),
    compat((2, 3, 2), (0, 6, 0)),
    compat((2, 3, 1), (0, 6, 0)),
    compat((2, 3, 0), (0, 6, 0)),
    compat((2, 2, 0), (0, 6, 0)),
    compat((2, 1, 1), (0, 6, 0)),
    compat((2, 1, 0), (0, 6, 0)),
    compat((2, 0, 0), (0, 6, 0)),
];

const FPGA_COMPAT: [Compat; 20] = [
    compat((0, 16, 0), (2, 6, 0)),
    compat((0, 16, 0), (2, 5, 0)),
    compat((0, 15, 3), (2, 4, 0)),
    compat((0, 15, 2), (2, 4, 0)),
    compat((0, 15, 1), (2, 4, 0)),
    compat((0, 15, 0), (2, 4, 0)),
    compat((0, 14, 0), (2, 4, 0)),
    compat((0, 12, 0), (2, 2, 0)),
    compat((0, 11, 1), (2, 1, 0)),
    compat((0, 11, 0), (2, 1, 0)),
    compat((0, 10, 2), (2, 1, 0)),
    compat((0, 10, 1), (2, 1, 0)),
    compat((0, 10, 0), (2, 1, 0)),
    compat((0, 9, 0), (2, 1, 0)),
    compat((0, 8, 0), (2, 1, 0)),
    compat((0, 7, 3), (2, 1, 0)),
    compat((0, 7, 2), (2, 1, 0)),
    compat((0, 7, 1), (2, 0, 0)),
    compat((0, 7, 0), (2, 0, 0)),
    compat((0, 6, 0), (2, 0, 0)),
];

fn find<'a>(table: &'a [Compat], version: SemanticVersion, what: &str) -> Option<&'a Compat> {
    let newest = table.first()?;
    if version > newest.version {
        log::info!(
            "{what} version (v{version}) is newer than entries in the compatibility \
             table. Please update libbladerf-rs if problems arise."
        );
        return Some(newest);
    }
    table.iter().find(|entry| entry.version == version)
}

/// Minimum FX3 firmware version supported for BladeRF2.
pub(crate) fn min_firmware() -> SemanticVersion {
    FIRMWARE_COMPAT[FIRMWARE_COMPAT.len() - 1].version
}

/// Rejects firmware older than the oldest compatibility table entry.
pub(crate) fn check_firmware(firmware: SemanticVersion) -> Result<()> {
    let required = min_firmware();
    if firmware >= required {
        Ok(())
    } else {
        Err(Error::UpdateFirmware {
            found: firmware,
            required: Some(required),
        })
    }
}

/// Checks that the firmware and FPGA versions are compatible with each other.
pub(crate) fn check(firmware: SemanticVersion, fpga: SemanticVersion) -> Result<()> {
    let fw_compat = find(&FIRMWARE_COMPAT, firmware, "Firmware").ok_or(Error::UpdateFirmware {
        found: firmware,
        required: None,
    })?;
    let fpga_compat = find(&FPGA_COMPAT, fpga, "FPGA").ok_or(Error::UpdateFpga {
        found: fpga,
        required: None,
    })?;
    if fpga < fw_compat.requires {
        return Err(Error::UpdateFpga {
            found: fpga,
            required: Some(fw_compat.requires),
        });
    }
    if firmware < fpga_compat.requires {
        return Err(Error::UpdateFirmware {
            found: firmware,
            required: Some(fpga_compat.requires),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u16, minor: u16, patch: u16) -> SemanticVersion {
        SemanticVersion::new(major, minor, patch)
    }

    #[test]
    fn tables_are_ordered_newest_first() {
        for table in [&FIRMWARE_COMPAT[..], &FPGA_COMPAT[..]] {
            assert!(table.windows(2).all(|w| w[0].version >= w[1].version));
        }
    }

    #[test]
    fn firmware_minimum_is_2_0_0() {
        assert_eq!(min_firmware(), v(2, 0, 0));
        assert!(check_firmware(v(2, 0, 0)).is_ok());
        assert!(check_firmware(v(2, 4, 1)).is_ok());
        assert!(matches!(
            check_firmware(v(1, 9, 9)),
            Err(Error::UpdateFirmware {
                required: Some(r),
                ..
            }) if r == v(2, 0, 0)
        ));
    }

    #[test]
    fn matching_pairs_pass() {
        assert!(check(v(2, 4, 0), v(0, 15, 0)).is_ok());
        assert!(check(v(2, 6, 0), v(0, 16, 0)).is_ok());
        assert!(check(v(2, 0, 0), v(0, 6, 0)).is_ok());
    }

    #[test]
    fn newer_than_table_uses_newest_entry() {
        assert!(check(v(3, 0, 0), v(0, 17, 0)).is_ok());
        assert!(matches!(
            check(v(3, 0, 0), v(0, 15, 3)),
            Err(Error::UpdateFpga { required: Some(r), .. }) if r == v(0, 16, 0)
        ));
    }

    #[test]
    fn mismatched_pairs_name_the_side_to_update() {
        assert!(matches!(
            check(v(2, 5, 0), v(0, 15, 0)),
            Err(Error::UpdateFpga { required: Some(r), .. }) if r == v(0, 16, 0)
        ));
        assert!(matches!(
            check(v(2, 3, 0), v(0, 15, 0)),
            Err(Error::UpdateFirmware { required: Some(r), .. }) if r == v(2, 4, 0)
        ));
    }

    #[test]
    fn unlisted_versions_are_rejected_like_libbladerf() {
        assert!(matches!(
            check(v(2, 4, 1), v(0, 15, 0)),
            Err(Error::UpdateFirmware { required: None, .. })
        ));
        assert!(matches!(
            check(v(2, 4, 0), v(0, 13, 0)),
            Err(Error::UpdateFpga { required: None, .. })
        ));
    }
}
