#!/bin/bash
# Cloud environment setup for silver-system.
#
# Runs once when a new cloud environment is created, before Claude Code starts.
# The resulting filesystem snapshot is cached (~7 days) and reused by every
# session in this environment, so keep slow toolchain installs here rather than
# in a per-session hook.
#
# Point the cloud environment's "setup script" at this file (paste its contents
# into the web UI, or reference .claude/setup.sh).
set -euo pipefail

# Playwright is used globally by scripts/shot*.js (see the NODE_PATH note in
# .claude/settings.json — env.NODE_PATH=/usr/local/lib/node_modules lets
# require('playwright') resolve the global install from this project).
npm install -g playwright@1.62

# Fetch the Chromium build Playwright drives for the headless screenshot scripts.
# --with-deps pulls the system libraries Chromium needs on a clean VM.
NODE_PATH=/usr/local/lib/node_modules npx playwright install --with-deps chromium

# There are no npm dependencies to install (package.json has no deps); the sim
# and its tests run on plain Node. Sanity-check the toolchain is wired up.
node --version
NODE_PATH=/usr/local/lib/node_modules node -e "require('playwright'); console.log('playwright OK')"
