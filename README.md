# kicad-wakatime

**kicad-wakatime** is a WakaTime plugin for [KiCAD](https://www.kicad.org/).

## Disclaimer
As of June 2025, **this plugin is likely not suitable for accurate time tracking.**\
After freezing the codebase in January 2025, [@lux](https://github.com/sporeball) was asked to step down as core maintainer of the plugin in order to focus on other projects, leaving multiple critical bugs in the most recent release unaddressed ([1](https://github.com/hackclub/kicad-wakatime/issues/16), [2](https://github.com/hackclub/kicad-wakatime/issues/17)).\
This repository will remain public, but until it can be improved upon by [@lux](https://github.com/sporeball) or by a new core maintainer, its use is discouraged.\
**Proceed at your own risk.**

## Installation

On all platforms:
1. Download the latest release of kicad-wakatime from the releases section. [Click here for downloads.](https://github.com/hackclub/kicad-wakatime/releases)
2. Open kicad-wakatime and fill out the settings.
3. Start designing!

If you know what you're doing, you can build kicad-wakatime from the main branch instead. This requires an up-to-date version of [CMake](https://cmake.org) and [protoc](https://grpc.io/docs/protoc-installation).\
The code in the main branch should be considered unstable, as some features may still be in progress between releases.

## Note
Prior to [version 0.2.0](https://github.com/hackclub/kicad-wakatime/releases/tag/0.2.0), KiCAD 8.99 nightly or greater was required in order to use kicad-wakatime. This is no longer required, and new users should be using KiCAD 8.0.7 stable instead.\
However, users who have already saved their project using KiCAD 8.99 or greater **cannot** downgrade to an older version of KiCAD.

<details>
<summary>Downloading KiCAD 8.99</summary>

If you are a Windows user, you can download KiCAD 8.99 [here](https://downloads.kicad.org/kicad/windows/explore/nightlies) (pick an "x86_64.exe".)

If you are a macOS user, you can download KiCAD 8.99 [here](https://downloads.kicad.org/kicad/macos/explore/nightlies) (pick a ".dmg").

If you are an Ubuntu user, you can install KiCAD 8.99 using the following shell commands:

```shell
sudo add-apt-repository --yes ppa:kicad/kicad-dev-nightly
sudo apt update
sudo apt install kicad-nightly
```

</details>

## Issues

If kicad-wakatime is not doing what you expect, please [open an issue](https://github.com/hackclub/kicad-wakatime/issues).

The bug report template will ask you for a magic word to confirm that you've read this README.\
The magic word is **"dreadnought"**.