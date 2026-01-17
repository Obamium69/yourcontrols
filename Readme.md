![https://github.com/Sequal32/yourcontrol](/assets/logo.png)
[![Donate](https://img.shields.io/static/v1?label=enjoying%20the%20mod?%20&style=for-the-badge&message=DONATE&logo=paypal&labelColor=orange&color=darkorange)](https://www.paypal.com/paypalme/ctam1207)
[![Release](https://img.shields.io/github/v/tag/Sequal32/yourcontrol?label=release&style=for-the-badge)](https://github.com/sequal32/yourcontrolsinstaller/releases/latest/download/installer.zip) [![Downloads](https://img.shields.io/github/downloads/Sequal32/yourcontrolsinstaller/total?style=for-the-badge)](https://github.com/sequal32/yourcontrolsinstaller/releases/latest/download/installer.zip) [![Discord](https://img.shields.io/discord/764805300229636107?color=blue&label=discord&logo=discord&logoColor=white&style=for-the-badge)](https://discord.gg/p7Bzcv3Yjd)

# **Shared Cockpit for Microsoft Flight Simulator 2020**

## ⚠️ AI-Generated Fork Disclaimer

**This fork contains AI-generated code.**

I don't know rust very well but I wanted to try to get it running under Linux (at least via Wine/Proton). So please forgive me for using LLM's


If you prefer human-written code, use the [original project](https://github.com/Sequal32/yourcontrols).

## Changes

This fork adds **egui** as an alternative UI backend to run YourControls on **Linux with WINE/Proton**.

### What's different?

- **egui native GUI** - Works under WINE/Proton (WebView2 doesn't)
- ⚠️ **AI-generated** - egui backend created with massive AI assistance

### Building

**Windows (egui):**
```powershell
cargo build --release --no-default-features --features egui-ui-full
```

**Original WebView:**
```powershell
cargo build --release --features webview-ui
```

### Running on Linux

```bash
protontricks -c 'wine YourControls.exe' 1250410
```

---

## Click the image below for information about the mod, and how to install!

&nbsp;

[![Documentation](/assets/Documentation.png)](https://docs.yourcontrols.org)

&nbsp;

## What's new?

&nbsp;

[![Changelog](/assets/Changelog.png)](/Changelog.md)
