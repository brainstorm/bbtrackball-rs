#![deny(unsafe_code)]
#![deny(warnings)]
#![no_std]
#![no_main]

use panic_halt as _;
use rtic::app;
use rtic::{Exclusive, Mutex};
use rtt_target::{rprintln, rtt_init_print};

use stm32f0xx_hal::{
    gpio::gpioa::{PA0, PA1, PA15, PA2, PA3, PA4, PA5, PA6, PA7},
    gpio::gpiob::{PB3},
    gpio::{Input, Output, PullUp, PushPull},
    pac,
    prelude::*,
    usb,
};

use usb_device::{bus::UsbBusAllocator, prelude::*};

use usbd_hid::descriptor::generator_prelude::*;
use usbd_hid::descriptor::MouseReport;
use usbd_hid::hid_class::HIDClass;

use cortex_m::interrupt::free as disable_interrupts;
use cortex_m::asm::delay as delay;

#[app(device = stm32f0xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_bus: &'static UsbBusAllocator<usb::UsbBusType>,
        usb_device: UsbDevice<'static, usb::UsbBusType>,
        usb_hid: HIDClass<'static, usb::UsbBusType>,
        exti: pac::EXTI,
        button_right: PA15<Input<PullUp>>,
        button_left: PB3<Input<PullUp>>,
        tb_left: PA4<Input<PullUp>>,
        tb_up: PA5<Input<PullUp>>,
        tb_right: PA6<Input<PullUp>>,
        tb_down: PA7<Input<PullUp>>,
        bbled_red: PA0<Output<PushPull>>,
        bbled_grn: PA1<Output<PushPull>>,
        bbled_blu: PA2<Output<PushPull>>,
        bbled_wht: PA3<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<usb::UsbBusType>> = None;

        // RTT handler
        rtt_init_print!();

        // Alias peripherals
        let mut dp: pac::Peripherals = ctx.device;

        // This code enables clock for SYSCFG and remaps USB pins to PA9 and PA10.
        usb::remap_pins(&mut dp.RCC, &mut dp.SYSCFG);

        // TODO: Power on bbled dance (light up LEDs in a fun pattern when powered on)

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
        let button_left = disable_interrupts(|cs|
                gpiob.pb3.into_pull_up_input(cs),
        );

        // LEDs and USB
        let gpioa = dp.GPIOA.split(&mut rcc);
        let (
            bbled_red,
            bbled_grn,
            bbled_blu,
            bbled_wht,
            tb_left,
            tb_up,
            tb_right,
            tb_down,
            button_right,
            usb_dm,
            usb_dp,
        ) = disable_interrupts(|cs| {
            (
                gpioa.pa0.into_push_pull_output(cs),
                gpioa.pa1.into_push_pull_output(cs),
                gpioa.pa2.into_push_pull_output(cs),
                gpioa.pa3.into_push_pull_output(cs),
                gpioa.pa4.into_pull_up_input(cs),
                gpioa.pa5.into_pull_up_input(cs),
                gpioa.pa6.into_pull_up_input(cs),
                gpioa.pa7.into_pull_up_input(cs),
                gpioa.pa15.into_pull_up_input(cs),
                gpioa.pa11,
                gpioa.pa12,
            )
        });

        // Enable external interrupt for 3 aux buttons...
        dp.SYSCFG.exticr1.write(|w| w.exti3().pb3());
        dp.SYSCFG.exticr4.write(|w| w.exti15().pa15());
        //... and for pulses on trackball
        dp.SYSCFG.exticr2.write(|w| w.exti4().pa4());
        dp.SYSCFG.exticr2.write(|w| w.exti5().pa5());
        dp.SYSCFG.exticr2.write(|w| w.exti6().pa6());
        dp.SYSCFG.exticr2.write(|w| w.exti7().pa7());

        // Set interrupt mask for all the above
        dp.EXTI.imr.write(|w| {
            w.mr3().set_bit();
            w.mr4().set_bit();
            w.mr5().set_bit();
            w.mr6().set_bit();
            w.mr7().set_bit();
            w.mr15().set_bit()
        });

        // Set interrupt rising trigger
        dp.EXTI.rtsr.write(|w| {
            w.tr3().set_bit();
            w.tr4().set_bit();
            w.tr5().set_bit();
            w.tr6().set_bit();
            w.tr7().set_bit();
            w.tr15().set_bit()
        });

        let usb = usb::Peripheral {
            usb: dp.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp,
        };

        *USB_BUS = Some(usb::UsbBus::new(usb));

        rprintln!("Preparing HID mouse...");
        let usb_hid = HIDClass::new(USB_BUS.as_ref().unwrap(), MouseReport::desc(), 60);

        rprintln!("Defining USB parameters...");
        let usb_device = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x6A6A, 0x5442)) // JJ TB
            .manufacturer("joshajohnson")
            .product("BBTrackball")
            .serial_number("Rust || GTFO")
            .device_class(0x00)
            .device_sub_class(0x00)
            .device_protocol(0x00)
            .build();

        rprintln!("Instantiating dp.EXTI...");
        let exti = dp.EXTI;
        rprintln!("Defining late resources...");

        init::LateResources {
            usb_bus: USB_BUS.as_ref().unwrap(),
            usb_device,
            usb_hid,
            exti,
            button_right,
            button_left,
            tb_left,
            tb_up,
            tb_right,
            tb_down,
            bbled_red,
            bbled_grn,
            bbled_blu,
            bbled_wht,
        }
    }

    #[idle(resources = [usb_device, usb_hid])]
    fn idle(_: idle::Context) -> ! {
        loop {
            // Wake From Interrupt
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = EXTI2_3, resources = [exti, usb_device, usb_hid])]
    fn exti2_3_interrupt(ctx: exti2_3_interrupt::Context) {
        rprintln!("Interrupts happening on EXTI2_3");
        let hid = ctx.resources.usb_hid;

        match ctx.resources.exti.pr.read().bits() {
            0x8 => {
                rprintln!("Button Right");
                ctx.resources.exti.pr.write(|w| w.pif3().set_bit()); // Clear interrupt
                send_mouse_report(Exclusive(hid), 0, 0, 3);
                delay(1000000);
                send_mouse_report(Exclusive(hid), 0, 0, 3);
            }

            _ => rprintln!("Some other bits were pushed around on EXTI2_3 ;)"),
        }
    }

    #[task(binds = EXTI4_15, resources = [exti, usb_device, usb_hid, bbled_red, bbled_grn, bbled_wht, bbled_blu])]
    fn exti_4_15_interrupt(ctx: exti_4_15_interrupt::Context) {
        rprintln!("Interrupts happening on EXTI for PA15...");

        let hid = ctx.resources.usb_hid;

        match ctx.resources.exti.pr.read().bits() {
            0x8000 => {
                rprintln!("Button Left");
                ctx.resources.exti.pr.write(|w| w.pif15().set_bit()); // Clear interrupt
                send_mouse_report(Exclusive(hid), 0, 0, 1);
                delay(1000000);
                send_mouse_report(Exclusive(hid), 0, 0, 1);
            }
            0x10 => {
                rprintln!("tb_left triggered!");
                ctx.resources.exti.pr.write(|w| w.pif4().set_bit());
                send_mouse_report(Exclusive(hid), 0, -5, 0);
            }
            0x20 => {
                rprintln!("tb_up triggered!");
                ctx.resources.exti.pr.write(|w| w.pif5().set_bit());
                send_mouse_report(Exclusive(hid), 5, 0, 0);
            }
            0x40 => {
                rprintln!("tb_right triggered!");
                ctx.resources.exti.pr.write(|w| w.pif6().set_bit());
                send_mouse_report(Exclusive(hid), 0, 5, 0);
            }
            0x80 => {
                rprintln!("tb_down triggered!");
                ctx.resources.exti.pr.write(|w| w.pif7().set_bit());
                send_mouse_report(Exclusive(hid), -5, 0, 0);
            }

            _ => rprintln!("Some other bits were pushed around on EXTI4_15 ;)"),
        }
    }

    #[task(binds = USB, resources = [usb_device, usb_hid])]
    fn usb_handler(ctx: usb_handler::Context) {
        rprintln!("USB interrupt received.");

        let dev = ctx.resources.usb_device;
        let hid = ctx.resources.usb_hid;

        // USB dev poll only in the interrupt handler
        dev.poll(&mut [hid]);
    }
};

fn send_mouse_report(
    mut shared_hid: impl Mutex<T = HIDClass<'static, usb::UsbBusType>>,
    x: i8,
    y: i8,
    buttons: u8,
) {
    let mr = MouseReport { x, y, buttons }; // 2U is 90 deg rotated from dev board

    shared_hid.lock(|hid| {
        rprintln!("Sending mouse report...");
        hid.push_input(&mr).ok();
    });
}