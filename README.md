# Claude Weekly Usage Calculator

A desktop companion that helps you pace your Claude AI weekly usage at a glance.

## About

This app is intended to help users of Claude AI pace their own weekly usage by providing a quick and easy place to check the suggested weekly usage based on the current moment and the weekly reset timer found on Claude's usage page.

It runs as a tray app and a main window, showing you — right now, this minute — roughly how much of your weekly quota you "should" have used if you want to spread it evenly until the next reset.

## What it does / does not do

**It does:**

- Calculate a suggested weekly usage percentage based on where you are in your personal weekly cycle.
- Show that percentage in the tray icon and in a main window gauge.
- Let you adjust for sleep, simulate a future moment, and restyle the whole UI.

**It does not:**

- **Not** scrape, read, log in to, or sync anything from Claude's usage page.
- **Not** connect to Anthropic's API or any network service. Your data stays on your machine.
- **Not** know your reset time automatically — you must set it once in Settings to match the weekly reset shown on Claude's usage page.

## Screenshots

> _Placeholder — drop images into `docs/screenshots/` and they will render here._

![Main gauge](docs/screenshots/main.png)
![Tray icon modes](docs/screenshots/tray.png)
![Settings panel](docs/screenshots/settings.png)
![Usage simulator](docs/screenshots/simulator.png)

## Features

- **Configurable tray icon** — ring, number, or emoji modes
- **In-app data visualization** — gauge arc, linear week bar, percentage readout
- **User-set weekly reset time** — day of week, hour, minute, and timezone
- **Sleep offset** — optional sleep-adjusted pacing so your "suggested" curve doesn't climb while you're asleep
- **Simulated usage** — look-ahead view to preview what the gauge will read at a future moment
- **14 premade themes / skins** — Default, CRT Green, LCD Seven-Seg, Early Mac, Solarized Dark/Light, Nord, Dracula, Catppuccin Mocha/Latte, GitHub Light, Sepia, Rose Pine Dawn, and more
- **Configurable accent color** — custom hex
- **Light / dark mode** and **inverse mode** (show remaining instead of used)
- **Accessibility threshold announcements** at 50 / 75 / 90 / 100 %
- **Autostart on boot**
- Bundled `claude-usage` CLI binary for scripting / terminal use

## Install (prebuilt)

Prebuilt installers for **Windows, macOS, and Linux** are published on the [Releases page](https://github.com/ShadowXCC/Claude-Weekly-Usage-Calculator/releases).

**Linux**

```bash
# Debian / Ubuntu
sudo dpkg -i claude-weekly-usage-calculator_*.deb

# Fedora / RHEL
sudo rpm -i claude-weekly-usage-calculator-*.rpm

# AppImage (any distro)
chmod +x Claude-Weekly-Usage-Calculator-*.AppImage
./Claude-Weekly-Usage-Calculator-*.AppImage
```

**Windows** — run the `.msi` installer.

**macOS** — open the `.dmg` and drag the app into Applications.

## Build from source

Prerequisites: [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) (Rust toolchain + platform build tools) and Node.js.

```bash
npm install
npm run tauri:dev      # run in development
npm run tauri:build    # produce release bundles in src-tauri/target/release/bundle/
```

## Usage

On first launch, open **Settings** and set your weekly reset time to match what Claude's usage page shows (day of week, hour, minute, timezone). Pick a tray icon mode and a theme if you like. From then on, the tray icon and main window will continuously show your suggested usage for the current moment.

## License

Licensed under the **GNU General Public License v3.0**. See [LICENSE](LICENSE) for the full text.

Copyright © 2026 ShadowXCC
