# solo2 library and CLI

The device can be either in 
- regular mode (VID:PID 1209:BEEE), or
- bootloader mode (VID:PID 1209:B000).

In regular mode, currently only the CCID interface over USB is implemented.  
In bootloader mode, NXP's custom HID protocol is used (via [`lpc55-host`][lpc55-host]).

Solo 2 is supported by Ludovic Rousseau's [CCID][ccid] driver, but there has not been a release.  
The included [Info.plist](Info.plist) works.


### Examples

If the key is in regular mode, and its firmware contains the management app:
- `solo2 app management uuid` reads out the serial number.
- `solo2 app management boot-to-bootrom` switches to bootloader mode.

If the key is in bootloader mode:
- `solo2 bootloader reboot` switches to regular mode (if the firmware is valid).


### Logging
Uses [`pretty_env_logger`][pretty-env-logger].  
For instance, set `SOLO2_LOG=info` in the environment.

[ccid]: https://ccid.apdu.fr/ccid/shouldwork.html#0x12090xBEEE
[lpc55-host]: https://docs.rs/lpc55
[pretty-env-logger]: https://docs.rs/pretty_env_logger/  
