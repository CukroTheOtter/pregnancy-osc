# Security

## Supported versions

Only the latest release gets fixes. The app updates itself (Settings > Updates), so please update before reporting.

## Reporting a problem

Please don't open a public issue for security problems.

Report it privately instead: go to the [Security tab](https://github.com/CukroTheOtter/pregnancy-osc/security) and click **Report a vulnerability**.

Include what you found, how to reproduce it, and which version you're on. I'll reply as soon as I can and let you know when it's fixed.

## What the app touches

So you know what's in scope:

- **OSC and OSCQuery** with VRChat, on your own computer only (127.0.0.1).
- **VRChat's log and OSC folders**, read-only.
- **SteamVR's config**, only if "Start with SteamVR" is on, to add or remove the app from SteamVR's startup list.
- **SteamVR's tracking** (headset and waist tracker), read-only, only for "Resting holds off labor" with "Use SteamVR tracking" on. **SteamVR notifications**, only if that's on (as the fallback under Settings > Notifications > In VR).
- **XSOverlay and OVR Toolkit** notification APIs, on your own computer only (127.0.0.1), only if those are on under Settings > Notifications > In VR.
- **vrchat.com**, only if "Show VRChat profile pictures" is on, to load public profile and avatar pictures.
- **GitHub releases**, to check for updates. Updates are signed and the app refuses unsigned ones.
