This is an example project that shows how to run rust, using embassy-rs onto a seeed stdio xiao-nrf85240. It flashes over USB without needing a JTAG or other debugger.

The template includes a way of rebooting the microcontroller into bootloader mode, without having to double-press the reset button. (I am not sure if this functions with the factory-default bootloader. It could be that you need the bootloader from [adafruit](https://github.com/mike1808/PIO_SEEED_Adafruit_nRF52_Arduino)).

Before running: Check COM_PORT in `flash.sh`, and replace it if necessary.

To flash the target device: `cargo run --release`. The first time you will have to double press the reset button to enter bootloader mode. After that it should enter the bootloader automaically.

- `arm-none-eabi-objcopy` is available on manjaro under the package name `extra/arm-none-eabi-binutils`.
- `adafruit-nrfutil` is available on manjaro under the package name `python-adafruit-nrfutil`.

If you get something like `permission denied: '/dev/ttyACM0'` on manjaro, you need to add yourself to the `uucp` group: `sudo usermod -a -G uucp <username>`.
Log out and log back in for the changes to take effect.

This example has taken inspiration from [Wumpf's seeed example](https://github.com/Wumpf/Seeed-nRF52840-Sense-projects/tree/main), and the [USB example](https://github.com/embassy-rs/embassy/blob/main/examples/nrf52840/src/bin/usb_serial.rs) from [embassy-rs](https://embassy.dev/).