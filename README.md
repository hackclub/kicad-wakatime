# kicad-wakatime

**kicad-wakatime** is a WakaTime plugin for [KiCAD](https://www.kicad.org/) 8.99.

## Installation

On all platforms:
1. Download the latest release of kicad-wakatime from the releases section. [Click here for downloads.](https://github.com/hackclub/kicad-wakatime/releases)
2. Enable the KiCAD API in KiCAD 8.99. (Settings -> Plugins -> Enable KiCAD API)
3. Open kicad-wakatime and fill out the settings.
4. Start designing!

If you know what you're doing, you can build kicad-wakatime from the main branch instead. This requires an up-to-date version of [CMake](https://cmake.org) and [protoc](https://grpc.io/docs/protoc-installation).\
The code in the main branch should be considered unstable, as some features may still be in progress between releases.

## Downloading KiCAD 8.99

If you are a Windows user, you can download KiCAD 8.99 [here](https://downloads.kicad.org/kicad/windows/explore/nightlies) (pick an "x86_64.exe".)

If you are a macOS user, you can download KiCAD 8.99 [here](https://downloads.kicad.org/kicad/macos/explore/nightlies) (pick a ".dmg").

If you are an Ubuntu user, you can install KiCAD 8.99 using the following shell commands:

```shell
sudo add-apt-repository --yes ppa:kicad/kicad-dev-nightly
sudo apt update
sudo apt install kicad-nightly
```

## Issues

If kicad-wakatime is not doing what you expect, please [open an issue](https://github.com/hackclub/kicad-wakatime/issues).

The bug report template will ask you for a magic word to confirm that you've read this README.\
The magic word is **"dreadnought"**.

Please make sure you're running KiCAD 8.99 (**not** KiCAD 8.0!) and the KiCAD API is enabled.
