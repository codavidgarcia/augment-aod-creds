# Augment Credit Monitor

A lightweight macOS menu bar app to help you stay on top of your Augment credits, so you’re never caught off guard in the middle of important work.

![Menu Bar](https://img.shields.io/badge/macOS-Menu%20Bar%20App-blue) ![Tauri](https://img.shields.io/badge/Built%20with-Tauri%202.0-orange) ![License](https://img.shields.io/badge/license-MIT-green)

## What it does

- Shows your credit balance in the menu bar (always visible)
- Tracks usage by model (Claude Sonnet, GPT-4, etc.) and activity type (Agent, Chat, CLI)
- Alerts you when credits are running low, so you can plan ahead
- Offers simple charts to understand where your credits are going over time

## Quick Start

### Download

Grab the latest `.dmg` from [Releases](https://github.com/codavidgarcia/augment-credit-monitor/releases).

### First time setup

1. Open the app.
2. Click **"Login with Augment"**.
3. Sign in with your Augment account (Google/Microsoft/GitHub).
4. Click **"Connect to App"** when prompted.
5. That’s it — your balance appears in the menu bar, and you can open the dashboard for more detail.

## Dev Setup

If you want to build from source:

    # Clone
    git clone https://github.com/codavidgarcia/augment-credit-monitor.git
    cd augment-credit-monitor

    # Install deps
    npm install

    # Run dev
    npm run tauri dev

    # Build for production
    npm run tauri build

**Requirements**: Node 18+, Rust 1.70+, macOS 10.15+

## Stack

- **Frontend**: SvelteKit + TailwindCSS
- **Backend**: Rust + Tauri 2.0
- **Charts**: Chart.js
- **Storage**: SQLite (local)

## Screenshots

The app lives in your menu bar showing your current balance.  
When you click it, you’ll see a focused dashboard with:

- Usage breakdown by model
- Activity type (Agent, Chat, CLI)
- Visual trends to help you anticipate when you’ll need more credits

## Troubleshooting

If something doesn’t behave as expected, these usually help:

**"App can't be opened" on macOS:**

    xattr -d com.apple.quarantine "/Applications/Augment Credit Monitor.app"

**Balance not updating**  
- Confirm you’re logged into Augment.  
- Try logging out from the app and signing back in.

**High CPU usage**  
- Open **Settings** and increase the refresh interval to reduce how often the app checks your balance.

If you run into anything else, feel free to open an issue or share feedback in the repo.

## License

MIT – you’re welcome to use, adapt, and build on this project.

---

Made by [@codavidgarcia](https://github.com/codavidgarcia) to support anyone relying on Augment day to day and give teams a simple way to keep work flowing without surprise credit outages.
