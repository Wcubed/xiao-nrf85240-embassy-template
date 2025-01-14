use crate::Irqs;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_nrf::pac;
use embassy_nrf::peripherals::USBD;
use embassy_nrf::usb::vbus_detect::{HardwareVbusDetect, VbusDetect};
use embassy_nrf::usb::{Driver, Instance};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config};

use {defmt_rtt as _, panic_probe as _};

const MAGIC_REBOOT_MESSAGE: &str = "bootloader";

/// Creates a usb serial device.
/// Sending it [MAGIC_REBOOT_MESSAGE] will reboot the device
/// into serial-only-dfu mode.
pub fn setup_dfu_over_usb(spawner: &Spawner, usbd: USBD) {
    spawner.spawn(dfu_over_usb(usbd)).unwrap();
}

#[embassy_executor::task]
async fn dfu_over_usb(usbd: USBD) {
    pac::CLOCK.tasks_hfclkstart().write_value(1);
    while pac::CLOCK.events_hfclkstarted().read() != 1 {}

    // Create the driver, from the HAL.
    let driver = Driver::new(usbd, Irqs, HardwareVbusDetect::new(Irqs));

    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut msos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor,
        &mut control_buf,
    );

    // Create classes on the builder.
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    // Do stuff with the class!
    let reboot_fut = async {
        loop {
            class.wait_connection().await;
            let _ = reboot_on_magic_message(&mut class).await;
        }
    };

    join(usb_fut, reboot_fut).await;
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn reboot_on_magic_message<'d, T: Instance + 'd, P: VbusDetect + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T, P>>,
) -> Result<(), Disconnected> {
    let mut buf = [0; 64];

    loop {
        let n = class.read_packet(&mut buf).await?;
        let data = &buf[..n];

        if data == MAGIC_REBOOT_MESSAGE.as_bytes() {
            // Reboot the controller in DFU mode.
            // The magic number has been taken from the arduino bootloader:
            // https://github.com/mike1808/PIO_SEEED_Adafruit_nRF52_Arduino/blob/master/cores/nRF5/wiring.c#L26
            let dfu_magic_serial_only_reset = 0x4E;
            pac::POWER
                .gpregret()
                .write(|w| w.0 = dfu_magic_serial_only_reset);
            cortex_m::peripheral::SCB::sys_reset();
        }
    }
}
