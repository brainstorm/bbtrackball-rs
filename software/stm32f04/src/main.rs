//#![deny(unsafe_code)]
//#![deny(warnings)]
#![no_std]
#![no_main]

use panic_halt as _;
use rtt_target::{rtt_init_print, rprintln};
//use core::sync::atomic::{compiler_fence, Ordering};
use rtic::app;

use stm32f0xx_hal::{
    prelude::*,
    pac,
    usb,
    gpio::{ Input, Output, PushPull, PullUp },
    gpio::gpiob:: { PB1, PB5, PB4, PB3 },
    gpio::gpioa::{ PA1, PA2, PA3, PA4 },
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
        button3: PB5<Input<PullUp>>,
        button4: PB4<Input<PullUp>>,
        button5: PB3<Input<PullUp>>,
        bbled_red: PA1<Output<PushPull>>,
        bbled_grn: PA2<Output<PushPull>>,
        bbled_blu: PA3<Output<PushPull>>,
        bbled_wht: PA4<Output<PushPull>>,
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
        let (usr_led, button3, button4, button5) = disable_interrupts (|cs| {
            (
                gpiob.pb1.into_push_pull_output(cs),
                gpiob.pb5.into_pull_up_input(cs),
                gpiob.pb4.into_pull_up_input(cs),
                gpiob.pb3.into_pull_up_input(cs),
            )
        });

        // LEDs and USB
        let gpioa = dp.GPIOA.split(&mut rcc);
        let (bbled_red, bbled_grn, bbled_blu, bbled_wht, usb_dm, usb_dp) =
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


        // "I don't think that particular HAL has any helper functions to deal with setting up gpio exti interrupts yet, 
        // so you'll have to do modify the registers directly through the PAC..."

        // Enable external interrupt for PA15 (button)
        dp.SYSCFG.exticr4.modify(|_, w| { w.exti15().pa15() });
        // Set interrupt mask for line 15
        dp.EXTI.imr.modify(|_, w| w.mr15().set_bit());
        // Set interrupt rising trigger for line 15
        dp.EXTI.rtsr.modify(|_, w| w.tr15().set_bit());

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

    #[task(binds = USB, resources = [usb_device, usb_serial])]
    fn usbrx(ctx: usbrx::Context) {
        usb_poll(ctx.resources.usb_device, ctx.resources.usb_serial);
    }

    #[task(binds = EXTI0_1, resources = [exti, usr_led, button3, button4, button5])]
    fn zerotoone(ctx: zerotoone::Context) {
        rprintln!("Interrupts happening on EXTI0_1");
        // Clear the interrupt pending bit for line 1
        ctx.resources.exti.pr.write(|w| w.pif1().set_bit());
    }

    // #[task(binds = EXTI2_3)]
    // fn twotothree(_: twotothree::Context) {
    //     rprintln!("Interrupts happening on EXTI2_3");
    // }
    
    #[task(binds = EXTI4_15, resources = [exti, usr_led, button3, button4, button5])]
    fn gpioa(ctx: gpioa::Context) {
        let _usr_led = ctx.resources.usr_led;
        rprintln!("Interrupts happening on EXTI for PA15...");
        
        // Clear interrupt
        ctx.resources.exti.pr.write(|w| w.pif15().set_bit());
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
