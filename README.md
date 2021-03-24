# microbit-two

Experiments with BBC micro:bit V2

## Notes

 * No external LFCLK, used for speaker PWM.

## Documentation

 * [Hardware specification](https://tech.microbit.org/hardware/)
 * [Inter-MCU I2C](https://tech.microbit.org/software/spec-i2c-protocol/)
 * [Schematics](https://github.com/microbit-foundation/microbit-v2-hardware)


### Flashing

Install `probe-run`.
```
$ cargo install probe-run
```

Then run using cargo.
```
$ cargo run --bin matrix
```

#### No probe found

Add the udev rule `99-mbed.rules` in `/etc/udev/rules.d` with the content,
```
SUBSYSTEM=="usb", ATTR{idVendor}=="0d28", ATTR{idProduct}=="0204", MODE:="666"
```
