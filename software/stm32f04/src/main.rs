#![no_std]
#![no_main]

extern crate panic_semihosting;
use cortex_m::peripheral::Peripherals as Cortex_Peripherals;
use cortex_m_rt::entry;
use usb_device::prelude::*;
//use usbd_serial::{SerialPort, USB_CLASS_CDC};
use usbd_serial::USB_CLASS_CDC;

use stm32f0xx_hal::{
    delay::Delay,
    pac::Peripherals, //, interrupt, Interrupt},
    prelude::*,
    usb::{Peripheral, UsbBus},
};

mod report;

// const SENSITIVITY: i16 = 128;

#[entry]
fn main() -> ! {
    let mut p = Peripherals::take().unwrap();
    let cp = Cortex_Peripherals::take().unwrap();

    let mut rcc = p
        .RCC
        .configure()
        .hsi48()
        .enable_crs(p.CRS)
        .sysclk(48.mhz())
        .pclk(24.mhz())
        .freeze(&mut p.FLASH);

    // Get delay provider
    let mut delay = Delay::new(cp.SYST, &rcc);

    // Configure the on-board I/O: LEDs, spare buttons and hall sensors
    // ..."split() takes the raw peripheral struct from the PAC
    // and converts it into a struct that provides separate access and ownership for each GPIO pin"
    // https://craigjb.com/2019/12/31/stm32l0-rust/

    // USR LED and Buttons
    let gpiob = p.GPIOB.split(&mut rcc);
    let (mut usr_led, mut _button3, mut _button4, mut _button5) = cortex_m::interrupt::free(|cs| {
        (
            gpiob.pb1.into_push_pull_output(cs),
            gpiob.pb5.into_pull_up_input(cs),
            gpiob.pb4.into_pull_up_input(cs),
            gpiob.pb3.into_pull_up_input(cs),
        )
    });

    // LEDs and USB
    let gpioa = p.GPIOA.split(&mut rcc);
    let (mut bbled_red, mut bbled_grn, mut bbled_blu, mut bbled_wht, pin_dm, pin_dp) =
        cortex_m::interrupt::free(|cs| {
            (
                gpioa.pa1.into_push_pull_output(cs),
                gpioa.pa2.into_push_pull_output(cs),
                gpioa.pa3.into_push_pull_output(cs),
                gpioa.pa4.into_push_pull_output(cs),
                gpioa.pa11,
                gpioa.pa12,
            )
        });

    usr_led.set_low().ok(); // Turn off

    let usb_bus = UsbBus::new(Peripheral {
        usb: p.USB,
        pin_dm: pin_dm,
        pin_dp: pin_dp,
    });

    // Define USB device
    let mut _usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("JoshFTW")
        .product("BBTrackball")
        .serial_number("RustFW")
        .device_class(USB_CLASS_CDC)
        .build();

    loop {
        usr_led.toggle().ok();
        bbled_blu.toggle().ok();
        bbled_grn.toggle().ok();
        bbled_red.toggle().ok();
        bbled_wht.toggle().ok();

        delay.delay_ms(1000u32);
    }
}
