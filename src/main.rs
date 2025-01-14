#![no_std]
#![no_main]

use defmt::panic;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::usb::vbus_detect::{HardwareVbusDetect, VbusDetect};
use embassy_nrf::usb::{Driver, Instance};
use embassy_nrf::{bind_interrupts, pac, peripherals, usb};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<peripherals::USBD>;
    CLOCK_POWER => usb::vbus_detect::InterruptHandler;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    pac::CLOCK.tasks_hfclkstart().write_value(1);
    while pac::CLOCK.events_hfclkstarted().read() != 1 {}

    // Create the driver, from the HAL.
    let driver = Driver::new(p.USBD, Irqs, HardwareVbusDetect::new(Irqs));

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
    let echo_fut = async {
        loop {
            class.wait_connection().await;
            let _ = reboot_on_magic_message(&mut class).await;
        }
    };

    let mut led_red = Output::new(p.P0_26, Level::Low, OutputDrive::Standard);

    let blink_fut = async {
        loop {
            led_red.set_high();
            Timer::after_millis(1000).await;
            led_red.set_low();
            Timer::after_millis(1000).await;
        }
    };

    join(usb_fut, join(echo_fut, blink_fut)).await;
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

        if data == "bootloader".as_bytes() {
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
