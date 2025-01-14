#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::{bind_interrupts, peripherals, usb};
use embassy_time::Timer;
use usb_dfu::setup_dfu_over_usb;
use {defmt_rtt as _, panic_probe as _};

mod usb_dfu;

bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<peripherals::USBD>;
    CLOCK_POWER => usb::vbus_detect::InterruptHandler;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    setup_dfu_over_usb(&spawner, p.USBD);

    let mut led_red = Output::new(p.P0_26, Level::Low, OutputDrive::Standard);

    let blink_fut = async {
        loop {
            led_red.set_high();
            Timer::after_millis(1000).await;
            led_red.set_low();
            Timer::after_millis(1000).await;
        }
    };

    blink_fut.await;
}
