# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2021-11-21

- Give the owner a chance to tap device during update
- Bump to version 0.1 so we can distinguish patch and breaking release in the future

## [0.0.7] - 2021-11-21

- Fix the Windows 10 bug (via `lpc55-host` bump)
- Fix the incorrect udev rules file
- Fix and improve the AUR Arch Linux package (@Foxboron)
- Completely redesign the update process (modeling Device, Firmware, etc.)
- Re-activate OATH (via released `flexiber`)
- Expose parts of Solo 2 PKI

## [0.0.6] - 2021-11-06

### Changed

- No more async - we're not a high throughput webserver
- Nicer user dialogs (dialoguer/indicatif)
- Model devices modes (bootloader/card)
- Add udev rules

## [0.0.5] - 2021-11-06

### Added

- Display firmware version in human-readable format
- Start using a Changelog
- Add CI with cargo clippy/fmt
- Add binary releases following [svenstaro/miniserve](https://github.com/svenstaro/miniserve)

