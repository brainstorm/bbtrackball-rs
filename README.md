# Blackberry rusty HID mouse

The purpose of this project is to build an [integrated mouse for a regular (allegedly_ergonomic_but_not_really) keyboard][bb_trackball]... **in Rust** :)

![dev_board](img/dev_board.jpg)

This came about [from twitter idea to board, then to Rust Embedded](https://twitter.com/braincode/status/1275406584714104833). The code is based on [RTIC][rtic] rust-embedded framework (formerly known as RTFM). It exclusively uses interrupts *and* [also works on OSX, not only Linux][osx_not_working] (wink wink, nudge nudge, [James][jamesmunns] ;P).

## Future improvements shall you take this code with you (PRs super welcome!)

* DPI tweaking to have a better trackball accuracy or speed/stepping.
* [Button debouncer][debouncer].
* Remove one of the buttons of the board, since it conflicts with one of the trackball GPIOs... oops, hardware blopper ;)
* Add acceleration [like @LSChyi did][add_accel]?
* Perhaps a clever (optical) system to make this experiment actually practical and useful :P
* [Write a simplified RTIC example to return the favour to that amazing rust-embedded community][rtic_hid_example].

## Special thanks to

[@joshajohnson][joshajohnson]
[@mvirkkunen][lumpio]
[@therealprof][therealprof]
[@jamesmunns][jamesmunns]

And all the folks from the RTIC matrix.org community for patiently guiding and helping me in this experiment.

[bb_trackball]: https://os.mbed.com/users/AdamGreen/notebook/blackberrytrackballmouse/
[joshajohnson]: https://github.com/joshajohnson
[lumpio]: https://github.com/mvirkkunen/
[therealprof]: https://github.com/therealprof/
[rtic]: https://rtic.rs/
[jamesmunns]: https://github.com/jamesmunns
[osx_not_working]: https://github.com/jamesmunns/OtterPill-rs/commit/8e68fbd5bb1161d8131a99d98c90c3e949f49ec1
[rtic_hid_example]: https://github.com/rtic-rs/rtic-examples/issues/10#issuecomment-677464683
[add_accel]: https://github.com/LSChyi/blackberry-mini-trackball
[debouncer]: https://crates.io/crates/unflappable
