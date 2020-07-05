#![no_main]
#![no_std]

extern crate panic_halt;

use lpc8xx_hal::cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let p = lpc8xx_hal::Peripherals::take().unwrap();

    let mut syscon = p.SYSCON.split();
    let gpio = p.GPIO.enable(&mut syscon.handle);

    let button = p.pins.pio0_4.into_input_pin(gpio.tokens.pio0_4);
    let trackball_button = p.pins.pio0_19.into_input_pin(gpio.tokens.pio0_19);
    let up = p.pins.pio0_20.into_input_pin(gpio.tokens.pio0_20);
    let down = p.pins.pio0_16.into_input_pin(gpio.tokens.pio0_16);
    let left = p.pins.pio0_18.into_input_pin(gpio.tokens.pio0_18);
    let right = p.pins.pio0_17.into_input_pin(gpio.tokens.pio0_17); 

    // On board leds
    let mut led_green = p
        .pins
        .pio1_2
        .into_output_pin(gpio.tokens.pio1_2, lpc8xx_hal::gpio::Level::High);
    let mut led_blue = p
        .pins
        .pio1_1
        .into_output_pin(gpio.tokens.pio1_1, lpc8xx_hal::gpio::Level::High);
    let mut led_red = p
        .pins
        .pio1_0
        .into_output_pin(gpio.tokens.pio1_0, lpc8xx_hal::gpio::Level::High);

    // Leds off by default
    led_blue.set_low();
    led_green.set_low();
    led_red.set_low();

    loop {
        // Trackball movements
        if up.is_high() {
            led_blue.set_high();
        } else {
            led_blue.set_low();
        }

        if down.is_high() {
            led_green.set_high();
        } else {
            led_green.set_low();
        }

        if left.is_high() {
            led_red.set_high();
        } else {
            led_red.set_low();
        }

        if right.is_high() {
            led_red.set_high();
        } else {
            led_red.set_low();
        }
        
        // Trackball button
        if trackball_button.is_high() {
            led_red.set_high();
        } else {
            led_red.set_low();
        } 


        // Board button
        if button.is_high() {
            led_blue.set_high();
        } else {
            led_blue.set_low();
        }
    }
}
