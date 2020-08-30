//#![deny(unsafe_code)]
//#![deny(warnings)]
#![no_std]
#![no_main]

use panic_halt as _;
use rtt_target::{rtt_init_print, rprintln};
use rtic::app;

use stm32f0xx_hal::{
    prelude::*,
    pac,
    usb,
    gpio::{ Input, Output, PushPull, PullUp },
    gpio::gpiob:: { PB1, PB4, PB3 },
    gpio::gpioa::{ PA0, PA1, PA2, PA3, PA4, PA5, PA6, PA7, PA15 },
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
//use cortex_m::interrupt as core_m_interrupts;

#[app(device = stm32f0xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_bus: &'static UsbBusAllocator<usb::UsbBusType>,
        usb_serial: usbd_serial::SerialPort<'static, usb::UsbBusType>,
        usb_device: UsbDevice<'static, usb::UsbBusType>,
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
        let (usr_led, button4, button5) = disable_interrupts (|cs| {
            (
                gpiob.pb1.into_push_pull_output(cs),
                gpiob.pb4.into_pull_up_input(cs),
                gpiob.pb3.into_pull_up_input(cs),
            )
        });

        // LEDs and USB
        let gpioa = dp.GPIOA.split(&mut rcc);
        let (bbled_red, bbled_grn, bbled_blu, bbled_wht, tb_left, tb_up, tb_right, tb_down, button3, usb_dm, usb_dp) =
            disable_interrupts (|cs| {
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
        dp.SYSCFG.exticr1.write(|w| { w.exti3().pb3() });
        // dp.SYSCFG.exticr2.write(|w| { w.exti4().pb4() }); // Disable spare button in favor of tb_left
        dp.SYSCFG.exticr4.write(|w| { w.exti15().pa15() });
        //... and for pulses on trackball
        dp.SYSCFG.exticr2.write(|w| { w.exti4().pa4() });
        dp.SYSCFG.exticr2.write(|w| { w.exti5().pa5() });
        dp.SYSCFG.exticr2.write(|w| { w.exti6().pa6() });
        dp.SYSCFG.exticr2.write(|w| { w.exti7().pa7() });

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

        let exti = dp.EXTI;

        //let mut core = cortex_m::Peripherals::steal();
        //core.NVIC. .enable(Interrupt::UART0);
        //core_m_interrupts.enable();

        init::LateResources {
            usb_bus: USB_BUS.as_ref().unwrap(),
            usb_serial,
            usb_device,
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

    // From RTIC on matrix.org:
    // "And another thing; in your ”idle” you have an empty loop that is likely to be optimized away by the compiler, 
    // causing the function to return and then it’s game over. So you should put a continue or 
    // compiler_fence(Ordering::SeqCst) or wfi()/wfe() or whatever inside the loop, depending on desired idle behaviour"

    #[idle(resources = [usb_serial])]
    fn idle(_: idle::Context) -> ! {
        loop { cortex_m::asm::wfi(); };
    }

    #[task(binds = EXTI2_3, resources = [exti])]
    fn twotothree(ctx: twotothree::Context) {
        rprintln!("Interrupts happening on EXTI2_3");

        match ctx.resources.exti.pr.read().bits() {
            0x8 => {
                rprintln!("PB3 triggered");
                ctx.resources.exti.pr.write(|w| w.pif3().set_bit()); // Clear interrupt
            },

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
            },
            0x10 => {
                rprintln!("PB4 or (tb_left???) triggered!");
                ctx.resources.exti.pr.write(|w| w.pif4().set_bit());
            },
            0x20 => {
                rprintln!("tb_up triggered!");
                ctx.resources.exti.pr.write(|w| w.pif5().set_bit());
            },
            0x40 => {
                rprintln!("tb_right triggered!");
                ctx.resources.exti.pr.write(|w| w.pif6().set_bit());
            },
            0x80 => {
                rprintln!("tb_down triggered!");
                ctx.resources.exti.pr.write(|w| w.pif7().set_bit());
            },

            _ => rprintln!("Some other bits were pushed around on EXTI4_15 ;)"),
        }
    }
    #[task(binds = USB, resources = [usb_device, usb_serial])]
    fn usbrx(ctx: usbrx::Context) {
        usb_poll(ctx.resources.usb_device, ctx.resources.usb_serial);
    }

    // XXX: Not entirely sure this works for STM32F042?
    //defmt::info!("Hello, world!");
};

fn usb_poll<B: usb_device::bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    serial: &mut SerialPort<'static, B>
) {
    if !usb_dev.poll(&mut [serial]) {
        return;
    }
}
