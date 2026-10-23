use super::common::*;
use libbladerf_rs::bladerf2::{Capabilities, FpgaSource};
use libbladerf_rs::{MaybeFuture, Result, SemanticVersion};

#[test]
fn versions_parse_and_are_compatible() -> Result<()> {
    logging_init("bladerf2_fpga");
    let mut dev = open();
    let firmware = dev.firmware_version().wait()?;
    let mut rf = dev.rf_link_session().wait()?;
    assert!(rf.is_fpga_configured().wait()?, "FPGA must be loaded");
    let fpga: SemanticVersion = rf.fpga_version().wait()?.parse()?;
    println!("firmware v{firmware}, FPGA v{fpga}");
    assert!(firmware >= SemanticVersion::new(2, 0, 0));
    rf.check_compatibility().wait()?;
    let caps = rf.capabilities().wait()?;
    assert_eq!(caps, Capabilities::from_versions(firmware, fpga));
    assert!(caps.contains(Capabilities::FW_SUPPORTS_BLADERF2));
    Ok(())
}

#[test]
fn fpga_source_is_flash_or_host() -> Result<()> {
    logging_init("bladerf2_fpga");
    let mut dev = open();
    let mut rf = dev.rf_link_session().wait()?;
    let source = rf.get_fpga_source().wait()?;
    println!("FPGA source: {source:?}");
    assert!(matches!(source, FpgaSource::Flash | FpgaSource::Host));
    Ok(())
}
