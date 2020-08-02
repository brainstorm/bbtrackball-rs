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

    // Configure the on-board LED (green) and spare buttons
    // ..."split() takes the raw peripheral struct from the PAC
    // and converts it into a struct that provides separate access and ownership for each GPIO pin"
    // https://craigjb.com/2019/12/31/stm32l0-rust/
    let gpiob = p.GPIOB.split(&mut rcc);
    let gpioa = p.GPIOA.split(&mut rcc);

    let (mut usr_led, _button3, _button4, _button5) = cortex_m::interrupt::free(|cs| {
        (
            //bbled_red, bbled_grn, bbled_blu, bbled_wht)
            gpiob.pb1.into_push_pull_output(cs),
            gpiob.pb5.into_push_pull_output(cs),
            gpiob.pb4.into_push_pull_output(cs),
            gpiob.pb3.into_push_pull_output(cs),
            // gpioa.pa1.into_push_pull_output(cs),
            // gpioa.pa2.into_push_pull_output(cs),
            // gpioa.pa3.into_push_pull_output(cs),
            // gpioa.pa4.into_push_pull_output(cs),
        )
    });

    usr_led.set_low().ok(); // Turn off

    let usb = Peripheral {
        usb: p.USB,
        pin_dm: gpioa.pa11,
        pin_dp: gpioa.pa12,
    };

    let usb_bus = UsbBus::new(usb);

    // Define USB device
    let mut _usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("JoshFTW")
        .product("BBTrackball")
        .serial_number("RustFW")
        .device_class(USB_CLASS_CDC)
        .build();

    loop {
        usr_led.toggle().ok();
        delay.delay_ms(1000u32);
    }
}
