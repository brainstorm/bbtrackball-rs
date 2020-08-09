#![no_std]
#![no_main]

use panic_semihosting as _;
use rtic::app;

use stm32f0xx_hal::{
    prelude::*,
    pac,
    usb,
};

use usb_device::prelude::*;
use usb_device::bus::UsbBusAllocator;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

mod hid;
mod trackball;
mod button_scanner;


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

        // Alias peripherals
        let mut dp: pac::Peripherals = ctx.device;

        // Set up core registers
        // let mut rcc = dp.RCC
        //     .configure()
        //     .hse(8.mhz(), stm32f0xx_hal::rcc::HSEBypassMode::NotBypassed)
        //     .sysclk(72.mhz())
        //     .pclk(24.mhz())
        //     .freeze(&mut dp.FLASH);

        // assert!(rcc.clocks. usbclk_valid());
        let mut rcc = dp
            .RCC
            .configure()
            //.usbsrc(USBClockSource::)
            .hsi48()
            .enable_crs(dp.CRS)
            .sysclk(48.mhz())
            .pclk(24.mhz())
            .freeze(&mut dp.FLASH);
        
        //assert!(rcc.usbclk_valid());


        // Set up GPIO registers
        // USR LED and Buttons
        let gpiob = dp.GPIOB.split(&mut rcc);
        let (mut usr_led, mut _button3, mut _button4, mut _button5) = cortex_m::interrupt::free(|cs| {
            (
                gpiob.pb1.into_push_pull_output(cs),
                gpiob.pb5.into_pull_up_input(cs),
                gpiob.pb4.into_pull_up_input(cs),
                gpiob.pb3.into_pull_up_input(cs),
            )
        });

        // LEDs and USB
        let gpioa = dp.GPIOA.split(&mut rcc);
        let (mut bbled_red, mut bbled_grn, mut bbled_blu, mut bbled_wht, usb_dm, usb_dp) =
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

        // XXX: Use suitable (new) functions for delay
        //asm::delay(clocks.sysclk().0 / 100);

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

        // Set up button scanner
        // let scanner = Scanner::new(
        //         [
        //             gpiob.pb12.into_pull_down_input(&mut gpiob.crh).downgrade(),
        //             gpiob.pb13.into_pull_down_input(&mut gpiob.crh).downgrade(),
        //             gpiob.pb14.into_pull_down_input(&mut gpiob.crh).downgrade(),
        //             gpiob.pb15.into_pull_down_input(&mut gpiob.crh).downgrade(),
        //         ],
        //         [
        //             gpiob.pb11.into_push_pull_output(&mut gpiob.crh).downgrade(),
        //             gpiob.pb10.into_push_pull_output(&mut gpiob.crh).downgrade(),
        //             gpiob.pb1.into_push_pull_output(&mut gpiob.crl).downgrade(),
        //             gpiob.pb0.into_push_pull_output(&mut gpiob.crl).downgrade(),
        //             gpioa.pa7.into_push_pull_output(&mut gpioa.crl).downgrade(),
        //             gpioa.pa6.into_push_pull_output(&mut gpioa.crl).downgrade(),
        //         ]
        //     );


        init::LateResources {
            usb_bus: USB_BUS.as_ref().unwrap(),
            usb_serial,
            usb_device,
            usb_trackball,
            //scanner
        }
    }
};