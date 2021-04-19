# Rust blackberry trackball firmware

WIP! WIP! WIP!

This is an experimental re-targetting to AVR (from STM32F04)... let's see how that goes. So far I cannot develop on my Apple Silicon laptop due to:

```
% rustup target add avr-unknown-gnu-atmega328
error: toolchain 'stable-aarch64-apple-darwin' does not contain component 'rust-std' for target 'avr-unknown-gnu-atmega328'
```

Getting onboard with Rust for embedded devices is like 1,2,3 (more details on the [Rust embedded book](https://rust-embedded.github.io/book/)):

```shell
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ rustup target add avr-unknown-gnu-atmega328
$ cargo install cargo-embed
```
