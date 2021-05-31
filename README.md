This repository is incomplete and under active development.

# 🐝 solo2 library and CLI

The device can in one of two modes (USB VID:PID in brackets):
- regular mode ([1209:BEEE][beee-pid])
- bootloader mode ([1209:B000][b000-pid])

In regular mode, only the CCID interface to apps is currently implemented.
In bootloader mode, NXP's custom HID protocol is used (via [`lpc55-host`][lpc55-host]).

Solo 2 is supported by Ludovic Rousseau's [CCID][solokeys-ccid] driver, but there has not been a release.
The included [Info.plist](Info.plist) works.

[beee-pid]: https://pid.codes/1209/BEEE/
[b000-pid]: https://pid.codes/1209/B000/
[lpc55-host]: https://docs.rs/lpc55
[solokeys-ccid]: https://ccid.apdu.fr/ccid/shouldwork.html#0x12090xBEEE

### ⚠ DANGER ⚠

If the firmware is invalid according to the bootloader, the device always stays in bootloader mode. This is OK.

**BUT**: If the firmware is valid according to the bootloader, and the device boots into it, but the firmware has issues
(e.g., panics), the only way to get back into bootloader mode and flash a new firmware is by attaching a debugger.

This is quite fiddly, and needs a [special cable][tag-connect].
We recommend using NXP's [development board][dev-board] instead.

[tag-connect]: https://www.tag-connect.com/product/tc2030-ctx-nl-6-pin-no-legs-cable-with-10-pin-micro-connector-for-cortex-processors
[dev-board]: https://www.nxp.com/design/development-boards/lpcxpresso-boards/lpcxpresso55s69-development-board:LPC55S69-EVK

### Installation

```
cargo install solo2
```

For experimental "PKI lite" support, use `cargo install --features dev-pki solo2`.
This is not intended to and will not grow into full PKI creation + management functionality,
the goal is only to enable developing and testing all functionality of all official apps.

### Examples

If the key is in regular mode, and its firmware contains the admin app:
- `solo2 app admin uuid` reads out the serial number.
- `solo2 app admin boot-to-bootrom` switches to bootloader mode.

If the key is in regular mode, and its firmware contains the NDEF app:
- `solo2 app ndef capabilities` reads out the NDEF capabilities.

If the key is in bootloader mode:
- `solo2 bootloader reboot` switches to regular mode (if the firmware is valid).

Note that subcommands are inferred, so e.g. `solo2 b r` works like `solo2 bootloader reboot`.


### Logging

Uses [`pretty_env_logger`][pretty-env-logger]. For instance, set `SOLO2_LOG=info` in the environment.

[pretty-env-logger]: https://docs.rs/pretty_env_logger/


### License

SoloKeys is fully open source.

All software, unless otherwise noted, is dual licensed under [Apache 2.0](LICENSE-APACHE) and [MIT](LICENSE-MIT).
You may use SoloKeys software under the terms of either the Apache 2.0 license or MIT license.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

All hardware, unless otherwise noted, is licensed under [CERN-OHL-S-2.0](https://spdx.org/licenses/CERN-OHL-S-2.0.html).

All documentation, unless otherwise noted, is licensed under [CC-BY-SA-4.0](https://spdx.org/licenses/CC-BY-SA-4.0.html).

The file [Info.plist](Info.plist) is from [CCID][ccid-git], which is licensed under [LGPL-2.1][ccid-license].

[ccid-git]: https://salsa.debian.org/rousseau/CCID
[ccid-license]: https://salsa.debian.org/rousseau/CCID/-/blob/master/COPYING
