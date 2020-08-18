#![deny(unsafe_code)]
#![deny(warnings)]
#![no_std]
#![no_main]

use panic_halt as _;
use rtt_target::{rtt_init_print};
//use panic_semihosting as _;
use rtic::app;

use embedded_hal::digital::v2::{InputPin, OutputPin};
use stm32f0xx_hal::{
    prelude::*,
    pac,
    usb,
};

use usb_device::prelude::*;
use usb_device::bus::UsbBusAllocator;
use usb_device::class::UsbClass as _;
use usbd_serial::{SerialPort, USB_CLASS_CDC};
// 
mod hid;
mod trackball;
mod button_scanner;

define_scanner!(Scanner, 2, 1);

#[app(device = stm32f0xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_bus: &'static UsbBusAllocator<usb::UsbBusType>,
        usb_serial: usbd_serial::SerialPort<'static, usb::UsbBusType>,
        usb_device: UsbDevice<'static, usb::UsbBusType>,
        usb_trackball: hid::HidClass<'static, usb::UsbBusType, trackball::Trackball>,
        //scanner: Scanner<Pxx<Input<PullDown>>, Pxx<Output<PushPull>>>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<usb::UsbBusType>> = None;

        // Panic RTT handler
        rtt_init_print!();

        // Alias peripherals
        let mut dp: pac::Peripherals = ctx.device;

        /* Uncomment the following lines if you have a chip in TSSOP20 (STM32F042F)
        or UFQFPN28 (STM32F042G) package
        This code enables clock for SYSCFG and remaps USB pins to PA9 and PA10.
        */
        dp.RCC.apb2enr.modify(|_, w| w.syscfgen().set_bit());
        dp.SYSCFG.cfgr1.modify(|_, w| w.pa11_pa12_rmp().remapped());

        let mut rcc = dp
            .RCC
            .configure()
            .usbsrc(stm32f0xx_hal::rcc::USBClockSource::HSI48)
            .hsi48()
            .enable_crs(dp.CRS)
            .sysclk(48.mhz())
            .pclk(24.mhz())
            .freeze(&mut dp.FLASH);

        // Set up GPIO registers for USR LED and Buttons
        let gpiob = dp.GPIOB.split(&mut rcc);
        let (mut _usr_led, mut _button3, mut _button4, mut _button5) = cortex_m::interrupt::free(|cs| {
            (
                gpiob.pb1.into_push_pull_output(cs),
                gpiob.pb5.into_pull_up_input(cs),
                gpiob.pb4.into_pull_up_input(cs),
                gpiob.pb3.into_pull_up_input(cs),
            )
        });

        // LEDs and USB
        let gpioa = dp.GPIOA.split(&mut rcc);
        let (mut _bbled_red, mut _bbled_grn, mut _bbled_blu, mut _bbled_wht, usb_dm, usb_dp) =
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

        let usb = usb::Peripheral {
            usb: dp.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp,
        };

        let (usb_serial, usb_trackball, usb_device) = {
            *USB_BUS = Some(usb::UsbBus::new(usb));
            let serial = SerialPort::new(USB_BUS.as_ref().unwrap());
            let trackball = hid::HidClass::new(trackball::Trackball::new(), USB_BUS.as_ref().unwrap());
            let usb_dev = UsbDeviceBuilder::new(
                        USB_BUS.as_ref().unwrap(),
                        UsbVidPid(0x16c0, 0x27dd)
                    )
                .manufacturer("JoshFTW")
                .product("BBTrackball")
                .serial_number("RustFW")
                .device_class(USB_CLASS_CDC)
                .build();

            (serial, trackball, usb_dev)
        };

        init::LateResources {
            usb_bus: USB_BUS.as_ref().unwrap(),
            usb_serial,
            usb_device,
            usb_trackball,
            //scanner
        }
    }
    #[idle(resources = [usb_serial, usb_trackball])]
    fn idle(ctx: idle::Context) -> ! {
        let mut r = ctx.resources;
//        let mut last_raw: i32 = 0;
        loop {
            let button_buffer = [0; 3];
            //r.scanner.scan_to_bytes(&mut button_buffer, 0);

            let report = trackball::TrackballReport::new(10, 10, button_buffer);
            r.usb_trackball.lock(|tball| {
                tball.device_mut().set_report(report.clone());
            });
            while let Ok(0) = r.usb_trackball.lock(|tball| {
                tball.write(&report.as_bytes())
            })
            {}
        }
    }

    #[task(binds = USB, resources = [usb_device, usb_trackball, usb_serial])]
    fn usbrx(cx: usbrx::Context) {
        usb_poll(cx.resources.usb_device, cx.resources.usb_serial, cx.resources.usb_trackball);
    }
    
    extern "C" {
        fn CEC_CAN();
    }
};

fn usb_poll<B: usb_device::bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    serial: &mut SerialPort<'static, B>,
    trackball: &mut hid::HidClass<'static, B, trackball::Trackball>,
) {
    if !usb_dev.poll(&mut [serial, trackball]) {
        return;
    }
    trackball.poll();
    let mut buf = [0;10];
    match serial.read(&mut buf) {
        Ok(_) => {},
        Err(UsbError::WouldBlock) => {},
        e => panic!("USB read error: {:?}", e)
    }
}
