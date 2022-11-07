# copper

## A rust minecraft launcher

[![forthebadge](https://forthebadge.com/images/badges/0-percent-optimized.svg)](https://forthebadge.com)
[![forthebadge](https://forthebadge.com/images/badges/60-percent-of-the-time-works-every-time.svg)](https://forthebadge.com)
[![forthebadge](https://forthebadge.com/images/badges/contains-tasty-spaghetti-code.svg)](https://forthebadge.com)
[![forthebadge](https://forthebadge.com/images/badges/made-with-rust.svg)](https://forthebadge.com)
[![forthebadge](https://forthebadge.com/images/badges/mom-made-pizza-rolls.svg)](https://forthebadge.com)
[![wakatime](https://wakatime.com/badge/github/glowsquid-launcher/copper.svg?style=for-the-badge)](https://wakatime.com/badge/github/glowsquid-launcher/copper)

This is meant to be a mid-level structural launcher. It works in 3 steps:

- authenticate
- download
- launch!

You get to decide how to authenticate, either internally (via the `auth` module) or externally (Just get the token and user info and you're good).

Downloading and launching, ditto.

**MICROSOFT ONLY** becauce mojang is being removed in the future. Migrate now _or else_

Currently, this is being used as the launcher backend for the glowsquid launcher.

## Features
- [x] can actually download minecraft
- [x] parses version manifests for you
- [x] proper error handling

## Testing

TODO: refactor testing
