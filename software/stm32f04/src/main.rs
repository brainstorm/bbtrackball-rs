//! CDC-ACM serial port example using polling in a busy loop.
#![no_std]
#![no_main]

extern crate panic_semihosting;

use cortex_m_rt::entry;
use stm32_usbd::UsbBus;
use stm32f0xx_hal::{prelude::*, stm32};
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

#[entry]
fn main() -> ! {
    let mut dp = stm32::Peripherals::take().unwrap();

    /* Uncomment the following lines if you have a chip in TSSOP20 (STM32F042F)
       or UFQFPN28 (STM32F042G) package
       This code enables clock for SYSCFG and remaps USB pins to PA9 and PA10.
    */
    //dp.RCC.apb2enr.modify(|_, w| w.syscfgen().set_bit());
    //dp.SYSCFG.cfgr1.modify(|_, w| w.pa11_pa12_rmp().remapped());

    let mut rcc = dp
        .RCC
        .configure()
        .hsi48()
        .enable_crs(dp.CRS)
        .sysclk(48.mhz())
        .pclk(24.mhz())
        .freeze(&mut dp.FLASH);

    // Configure the on-board LED (LD3, green)
    let gpiob = dp.GPIOB.split(&mut rcc);
    let mut led = cortex_m::interrupt::free(|cs| {
        gpiob.pb3.into_push_pull_output(cs)
    });
    led.set_low().unwrap(); // Turn off

    let gpioa = dp.GPIOA.split(&mut rcc);

    let usb_dm = gpioa.pa11;
    let usb_dp = gpioa.pa12;

    let usb_bus = UsbBus::new(dp.USB, (usb_dm, usb_dp));

    let mut serial = SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")
        .device_class(USB_CLASS_CDC)
        .build();

    loop {
        if !usb_dev.poll(&mut [&mut serial]) {
            continue;
        }

        let mut buf = [0u8; 64];

        match serial.read(&mut buf) {
            Ok(count) if count > 0 => {
                led.set_high().unwrap(); // Turn on

                // Echo back in upper case
                for c in buf[0..count].iter_mut() {
                    if 0x61 <= *c && *c <= 0x7a {
                        *c &= !0x20;
                    }
                }

                let mut write_offset = 0;
                while write_offset < count {
                    match serial.write(&buf[write_offset..count]) {
                        Ok(len) if len > 0 => {
                            write_offset += len;
                        },
                        _ => {},
                    }
                }
            }
            _ => {}
        }

        led.set_low().unwrap(); // Turn off
    }
}
