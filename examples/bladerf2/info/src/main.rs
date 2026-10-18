use anyhow::Result;
use libbladerf_rs::MaybeFuture;

use libbladerf_rs::bladerf2::BladeRf2;

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .filter_module("nusb", log::LevelFilter::Info)
        .init();

    let bladerf2 = BladeRf2::from_first().wait()?;
    log::debug!("Serial: {}", bladerf2.serial().wait()?);
    log::debug!("FX3 Firmware: {}", bladerf2.fx3_firmware_version().wait()?);

    // TODO: rf_link_session, initialize, FPGA version, RF info once implemented

    Ok(())
}
