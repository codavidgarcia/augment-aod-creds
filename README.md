# Augment Credit Monitor

A simple macOS menu bar app to keep track of my Augment credits. Built it because I kept running out of credits without noticing.

![Menu Bar](https://img.shields.io/badge/macOS-Menu%20Bar%20App-blue) ![Tauri](https://img.shields.io/badge/Built%20with-Tauri%202.0-orange) ![License](https://img.shields.io/badge/license-MIT-green)

## What it does

- Shows your credit balance in the menu bar (always visible)
- Tracks usage by model (Claude Sonnet, GPT-4, etc.) and activity type (Agent, Chat, CLI)
- Alerts you when credits are running low
- Charts to see where your credits are going

## Quick Start

### Download

Grab the latest `.dmg` from [Releases](https://github.com/codavidgarcia/augment-credit-monitor/releases).

### First time setup

1. Open the app
2. Click "Login with Augment"
3. Sign in with your Augment account (Google/Microsoft/GitHub)
4. Click "Connect to App" when prompted
5. Done! Your balance appears in the menu bar

## Dev Setup

If you want to build from source:

```bash
# Clone
git clone https://github.com/codavidgarcia/augment-credit-monitor.git
cd augment-credit-monitor

# Install deps
npm install

# Run dev
npm run tauri dev

# Build for production
npm run tauri build
```

**Requirements**: Node 18+, Rust 1.70+, macOS 10.15+

## Stack

- **Frontend**: SvelteKit + TailwindCSS
- **Backend**: Rust + Tauri 2.0
- **Charts**: Chart.js
- **Storage**: SQLite (local)

## Screenshots

The app lives in your menu bar showing your current balance. Click it to see the full dashboard with usage breakdown by model and activity.

## Troubleshooting

**"App can't be opened"** on macOS:
```bash
xattr -d com.apple.quarantine "/Applications/Augment Credit Monitor.app"
```

**Balance not updating**: Make sure you're logged in. Try logging out and back in.

**High CPU**: Increase the refresh interval in settings.

## License

MIT - do whatever you want with it.

---

Made by [@codavidgarcia](https://github.com/codavidgarcia) because I needed it.
