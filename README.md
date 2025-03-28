# idevice

A Rust library for interacting with iOS services.
Inspired by [libimobiledevice](https://github.com/libimobiledevice/libimobiledevice)
and [pymobiledevice3](https://github.com/doronz88/pymobiledevice3),
this library interfaces with lockdownd and usbmuxd to perform actions
on an iOS device that a Mac normally would.

## State

**IMPORTANT**: Breaking changes will happen at each point release until 0.2.0.
The library is still in the development and brainstorming stage.
Pin your `Cargo.toml` to a specific version to avoid breakage.

This library is in development and research stage.
Releases are being published to crates.io for use in other projects,
but the API and feature-set are far from final or even planned.

- [x] lockdownd connection
- [x] SSL support
- [x] Heartbeat
- [x] Pairing file
- [ ] Instproxy (partial support)
- [x] afc
- [ ] amfi
- [ ] companion proxy
- [x] diagnostics
- [x] file relay
- [x] house arrest
- [ ] misagent (certificates)
- [x] RemoteXPC
- [x] mobile backup
- [x] notification proxy
- [x] DVT protocol
- [ ] screenshot
- [ ] simulate location
- [x] process control
- [ ] web inspector
- [ ] usbmuxd connection
- [ ] Documentation

## Features

To keep dependency bloat and compile time down, everything is contained in features.

- core_device_proxy
- heartbeat
- installation_proxy
- mounter
- xpc
- afc
- house_arrest
- file_relay
- diagnostics
- notification_proxy
- full

As this project is done in my free time within my busy schedule, there
is no ETA for any of these. Feel free to contribute or donate!

## Version Policy

As Apple prohibits downgrading to older versions, this library will
not keep compatibility for older versions than the current stable release.

## Developer Disk Images

doronz88 is kind enough to maintain a [repo](https://github.com/doronz88/DeveloperDiskImage)
for disk images and personalized images.
On MacOS, you can find them at ``~/Library/Developer/DeveloperDiskImages`` on a Mac.

## License

MIT
