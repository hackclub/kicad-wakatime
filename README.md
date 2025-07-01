# kicad-wakatime

**kicad-wakatime** is a WakaTime plugin for [KiCAD](https://www.kicad.org/).

## Installation

On all platforms:
1. Download the latest release of kicad-wakatime from the releases section. [Click here for downloads.](https://github.com/hackclub/kicad-wakatime/releases)
2. Open kicad-wakatime and fill out the settings.
3. Start designing!

If you know what you're doing, you can build kicad-wakatime from the main branch instead. Just run `cargo build` in the `kicad-wakatime` directory and everything should work.

The code in the main branch should be considered unstable, as some features may still be in progress between releases.

## Usage

Open `kicad-wakatime` and `kicad`. Click on "settings" in kicad wakatime and enter your API key. (Should be auto-filled if you have already installed hackatime)

Click on the first "select folder" button, and select your ".kicad_pro" file. Click OK!

Now go back to kicad, please make the following changes to your KiCAD settings (Control+,) to use this version of kicad-wakatime:

    Auto save should be set to 1 minute.
    "Automatically backup projects" should be checked.
    "Create backups when auto save occurs" should be checked.
    Minimum time between backups should be set to 0 minutes.

Your settings tab should look like this:

![](https://hc-cdn.hel1.your-objectstorage.com/s/v3/6fb6fc315989d9798771bf14417c9e70ed031125_image.png)

And you are done! Happy pcb-ing!

If you plan on doing symbol editing, select your .kicad_sym using the second button, and if you are going to do footprint editing, select your .pretty folder containing all the .kicad_mod fils using the third button.

If you are on Linux Wayland (Hyprland doesn't count - it is supported), open kicad using the following command:

```shell
GDK_BACKEND=x11 kicad
```

or if you installed the flatpak:

```shell
flatpak override --env=GDK_BACKEND=x11 org.kicad.KiCad
```

This solution only works if xwayland is running and supported by the compositor. This works by default on most compositors, but if it isn't check out https://github.com/Supreeeme/xwayland-satellite

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

Known issue: When using hierarchy sheets, kicad-wakatime will create a project for each sheet. If you are going to use this for SoM, just select all the projects applicable in their UI.

If kicad-wakatime is not doing what you expect, please [open an issue](https://github.com/hackclub/kicad-wakatime/issues).

The bug report template will ask you for a magic word to confirm that you've read this README.\
The magic word is **"dreadnought"**.
