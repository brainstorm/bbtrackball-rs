#[macro_export]
macro_rules! define_scanner {
    ($name:ident, $inputs:expr, $outputs:expr) => {
        pub struct $name<I: InputPin, O: OutputPin> {
            inputs: [I; $inputs],
            outputs: [O; $outputs],
            // Time since last change on pin. Used for debouncing
            last_activity: [u32; $inputs],
        }

        impl<I, O> $name<I, O>
            where I: InputPin,
                  O: OutputPin,
                  I::Error: core::fmt::Debug,
                  O::Error: core::fmt::Debug
        {
            pub fn new(inputs: [I; $inputs], outputs: [O; $outputs]) -> Self {
                Self {
                    inputs, outputs,
                    last_activity: [0; $inputs]
                }
            }

            /**
              Sets the bits in the output u8s to the state of each input/output
              combination. One bit per button, starting with each of the inputs
              for the first output.

              Panics if result_buffer.len()*8 < inputs*outputs
            */
            pub fn scan_to_bytes(&mut self, result_buffer: &mut [u8], time: u32) {
                for byte in result_buffer.iter_mut() {
                    *byte = 0;
                }
                let mut buff_index = 0;
                let mut buff_offset = 0;
                for output in &mut self.outputs {
                    output.set_high().unwrap();
                    asm::delay(100);
                    for input in &self.inputs {
                        if input.is_high().unwrap() {
                            result_buffer[buff_index] |= 1 << buff_offset;
                        }
                        buff_offset += 1;
                        if buff_offset > 7 {
                            buff_offset = 0;
                            buff_index += 1;
                        }
                    }
                    output.set_low().unwrap();
                }
            }
        }
    }
}
