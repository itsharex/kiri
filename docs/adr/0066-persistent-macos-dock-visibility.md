# ADR 0066: Persistent macOS Dock visibility

- Status: Accepted
- Date: 2026-10-02

## Decision

On macOS, Settings offers a Show in Dock switch. It is on by default to
preserve the existing behavior. Changing it immediately applies the native
application activation policy and saves the preference for future launches.
The setting belongs to the application preferences, independently of the
capture library and its location.

Changing Dock visibility preserves the open Settings WebView and its state.
Hidden library windows are not reopened, and capture focus takes priority.

Hiding the Dock icon keeps the tray, library, capture shortcut and utility
windows available. Opening the library or starting a capture must not reset
the user's choice. The switch is absent on Windows and Linux because their
taskbar and tray behavior does not use macOS activation policy.

## Verification

Check the switch in the signed fixed-path macOS app, including hiding and
showing while Settings remains open, reopening from the tray and relaunching.
Persistence tests cover an unset preference and both explicit values. A build
or mocked settings renderer does not establish native Dock behavior.
