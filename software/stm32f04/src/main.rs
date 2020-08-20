#![deny(unsafe_code)]
#![deny(warnings)]
#![no_std]
#![no_main]

use panic_halt as _;
use rtt_target::{rtt_init_print, rprintln};
use rtic::app;

use stm32f0xx_hal::{
    prelude::*,
    pac,
    usb,
};

use usb_device::prelude::*;
use usb_device::bus::UsbBusAllocator;
// use usb_device::class::UsbClass as _;

// use usbd_hid::hid_class::{HIDClass};
// use usbd_hid::descriptor::MouseReport;
// use usbd_hid::descriptor::generator_prelude::*;

use usbd_serial::{SerialPort, USB_CLASS_CDC};

//use cortex_m::asm::delay as cycle_delay;
use cortex_m::interrupt::free as disable_interrupts;
// use cortex_m::interrupt;

#[app(device = stm32f0xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_bus: &'static UsbBusAllocator<usb::UsbBusType>,
        usb_serial: usbd_serial::SerialPort<'static, usb::UsbBusType>,
        usb_device: UsbDevice<'static, usb::UsbBusType>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<usb::UsbBusType>> = None;

        // RTT handler
        rtt_init_print!();

        // Alias peripherals
        let mut dp: pac::Peripherals = ctx.device;

        /* Uncomment the following lines if you have a chip in TSSOP20 (STM32F042F)
        or UFQFPN28 (STM32F042G) package
        This code enables clock for SYSCFG and remaps USB pins to PA9 and PA10.
        */
        dp.RCC.apb2enr.modify(|_, w| w.syscfgen().set_bit());
        dp.SYSCFG.cfgr1.modify(|_, w| w.pa11_pa12_rmp().remapped());

        rprintln!("Initializing peripherals");

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
        let (mut _usr_led, mut _button3, mut _button4, mut _button5) = disable_interrupts (|cs| {
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
            disable_interrupts (|cs| {
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

        rprintln!("Defining USB parameters");

        let (usb_serial, usb_device) = {
            *USB_BUS = Some(usb::UsbBus::new(usb));
            let serial = SerialPort::new(USB_BUS.as_ref().unwrap());
            let usb_dev = UsbDeviceBuilder::new(
                        USB_BUS.as_ref().unwrap(),
                        UsbVidPid(0x16c0, 0x27dd)
                    )
                .manufacturer("JoshFTW")
                .product("BBTrackball")
                .serial_number("RustFW")
                .device_class(USB_CLASS_CDC)
                .build();

            (serial, usb_dev)
        };

        init::LateResources {
            usb_bus: USB_BUS.as_ref().unwrap(),
            usb_serial,
            usb_device,
        }
    }
    #[idle(resources = [usb_serial])]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(binds = USB, resources = [usb_device, usb_serial])]
    fn usbrx(ctx: usbrx::Context) {
        usb_poll(ctx.resources.usb_device, ctx.resources.usb_serial);
    }

    #[task(binds = EXTI0_1)]
    fn zerotoone(_: zerotoone::Context) {
        rprintln!("Interrupts happening on EXTI0_1");
    }

    #[task(binds = EXTI2_3)]
    fn twotothree(_: twotothree::Context) {
        rprintln!("Interrupts happening on EXTI2_3");
    }


    #[task(binds = EXTI4_15)]
    fn gpioa(_: gpioa::Context) {
        rprintln!("Interrupts happening on EXTI4_15... pins from 4 to 15? PA OR PB? GPIOA or GPIOB?");
    }
    
    extern "C" {
        fn CEC_CAN();
    }
};


fn usb_poll<B: usb_device::bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    serial: &mut SerialPort<'static, B>
) {
    if !usb_dev.poll(&mut [serial]) {
        return;
    }
}
