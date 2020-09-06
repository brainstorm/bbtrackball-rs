//#![deny(unsafe_code)]
//#![deny(warnings)]
#![no_std]
#![no_main]

use panic_halt as _;
use rtic::app;
use rtt_target::{rprintln, rtt_init_print};

use stm32f0xx_hal::{
    gpio::gpioa::{PA0, PA1, PA15, PA2, PA3, PA4, PA5, PA6, PA7},
    gpio::gpiob::{PB1, PB3, PB4},
    gpio::{Input, Output, PullUp, PushPull},
    pac,
    prelude::*,
    usb,
};

use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;

use usbd_hid::descriptor::generator_prelude::*;
use usbd_hid::descriptor::MouseReport;
use usbd_hid::hid_class::HIDClass;

//use cortex_m::asm::delay as cycle_delay;
use cortex_m::interrupt::free as disable_interrupts;
//use cortex_m::interrupt as core_m_interrupts;

type UsbAlloc = &'static UsbBusAllocator<usb::UsbBusType>;
//type UsbDevice = UsbDevice<'static, usb::UsbBusType>;

#[app(device = stm32f0xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_bus: UsbAlloc,
        usb_device: UsbDevice<'static, usb::UsbBusType>,
        usb_hid: HIDClass<'static, usb::UsbBusType>,
        exti: pac::EXTI,
        usr_led: PB1<Output<PushPull>>,
        button3: PA15<Input<PullUp>>,
        button4: PB4<Input<PullUp>>,
        button5: PB3<Input<PullUp>>,
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

        /* Uncomment the following lines if you have a chip in TSSOP20 (STM32F042F)
        or UFQFPN28 (STM32F042G) package
        This code enables clock for SYSCFG and remaps USB pins to PA9 and PA10.
        */

        //XXX: Use usb_bus.remap_pins() instead of the two low level lines below
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
        let (usr_led, button4, button5) = disable_interrupts(|cs| {
            (
                gpiob.pb1.into_push_pull_output(cs),
                gpiob.pb4.into_pull_up_input(cs),
                gpiob.pb3.into_pull_up_input(cs),
            )
        });

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
            button3,
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

        // "I don't think that particular HAL has any helper functions to deal with setting up gpio exti interrupts yet,
        // so you'll have to do modify the registers directly through the PAC..."

        // Enable external interrupt for 3 aux buttons...
        dp.SYSCFG.exticr1.write(|w| w.exti3().pb3());
        // dp.SYSCFG.exticr2.write(|w| { w.exti4().pb4() }); // Disable spare button in favor of tb_left
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

        rprintln!("Defining USB parameters");

        let exti = dp.EXTI;
        let (usb_device, usb_hid) = {
            *USB_BUS = Some(usb::UsbBus::new(usb));

            let usb_dev =
                UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
                    .manufacturer("JoshFTW")
                    .product("BBTrackball")
                    .serial_number("RustFW")
                    .device_class(0x3) // HID
                    .build();

            let usb_hid = HIDClass::new(USB_BUS.as_ref().unwrap(), MouseReport::desc(), 60);

            (usb_dev, usb_hid)
        };

        //let mut core = cortex_m::Peripherals::steal();
        //core.NVIC. .enable(Interrupt::UART0);
        //core_m_interrupts.enable();

        init::LateResources {
            usb_bus: USB_BUS.as_ref().unwrap(),
            usb_device,
            usb_hid,
            exti,
            usr_led,
            button3,
            button4,
            button5,
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

    #[idle()]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = EXTI2_3, resources = [exti])]
    fn twotothree(ctx: twotothree::Context) {
        rprintln!("Interrupts happening on EXTI2_3");

        match ctx.resources.exti.pr.read().bits() {
            0x8 => {
                rprintln!("PB3 triggered");
                ctx.resources.exti.pr.write(|w| w.pif3().set_bit()); // Clear interrupt
            }

            _ => rprintln!("Some other bits were pushed around on EXTI2_3 ;)"),
        }
    }

    #[task(binds = EXTI4_15, resources = [exti])]
    fn gpioa(ctx: gpioa::Context) {
        rprintln!("Interrupts happening on EXTI for PA15...");

        match ctx.resources.exti.pr.read().bits() {
            0x8000 => {
                rprintln!("PA15 triggered");
                ctx.resources.exti.pr.write(|w| w.pif15().set_bit()); // Clear interrupt
            }
            0x10 => {
                rprintln!("tb_left triggered!");
                ctx.resources.exti.pr.write(|w| w.pif4().set_bit());
            }
            0x20 => {
                rprintln!("tb_up triggered!");
                ctx.resources.exti.pr.write(|w| w.pif5().set_bit());
            }
            0x40 => {
                rprintln!("tb_right triggered!");
                ctx.resources.exti.pr.write(|w| w.pif6().set_bit());
            }
            0x80 => {
                rprintln!("tb_down triggered!");
                ctx.resources.exti.pr.write(|w| w.pif7().set_bit());
            }

            _ => rprintln!("Some other bits were pushed around on EXTI4_15 ;)"),
        }
    }
    #[task(binds = USB, resources = [usb_device, usb_hid])]
    fn usb_handler(ctx: usb_handler::Context) {
        rprintln!("USB interrupt received.");

        let usb_dev = ctx.resources.usb_device;
        let usb_hid = ctx.resources.usb_hid;

        let mr = MouseReport {
            x: 0,
            y: 0,
            buttons: 0,
        };

        usb_hid.push_input(&mr).ok();
        usb_dev.poll(&mut [usb_hid]);

        //let mut hid_result = usb_hid.push_input(&MouseReport{x: 0, y: -4, buttons:0}).ok().unwrap_or(0);
        //if usb_dev.poll(&mut [ctx.resources.usb_hid]) {
            // HIDClass::new(USB_BUS.as_ref().unwrap(), MouseReport::desc(), 60);
            //ctx.resources.usb_hid.poll();
        //}
        //usb_dev.poll(&mut hid_result);

        // usb_hid.map(|hid| {
        //     usb_dev.poll(&mut [hid]);
        // });
        //usb_poll(ctx.resources.usb_device, ctx.resources.usb_hid);
    }

    // XXX: Not entirely sure this works for STM32F042?
    //defmt::info!("Hello, world!");
};

// fn usb_poll<B: usb_device::bus::UsbBus, C: usb_device::bus::UsbBus>(
//     usb_dev: &mut UsbDevice<'static, B>,
//     usb_hid: &mut HIDClass<'static, C>,
// ) {
//     //usb_dev.bus().poll();
//     rprintln!("USB polling...");
//     //usb_hid.map(|hid| usb_dev.poll(&mut [hid]));
//     //usb_bus.map(|usb_dev| {

//     // usb_dev.bus().write(endpoint???, MouseReport.x.into());
//     // usb_dev.bus().write(ep_addr, buf)

//     // let usb_hid = HIDClass::new(usb_bus,
//     //                         MouseReport::desc(), 60);

//     //usb_hid.push_input(r: &IR);
// }

// fn push_mouse_movement(usb_bus: UsbBusAllocator<usb::UsbBusType>, report: MouseReport) -> Result<usize, usb_device::UsbError> {
//     disable_interrupts(|_| {
//         unsafe {
//             usb_bus.map(|hid| {
//                 hid.push_input(&report)
//             })
//         }
//     }).unwrap()
// }
