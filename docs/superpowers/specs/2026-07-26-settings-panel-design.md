# Settings gear panel — design (v0.1.4)

## Goal

Move misc app controls out of the main converter header into a gear **Settings** panel.

## UI

- Gear button top-right of the header
- Accent dot + “Update available — open Settings” when an update is ready
- Modal panel with:
  - **Updates** — check / install (click-to-install only)
  - **Links** — GitHub, Releases, Issues (opens browser via opener plugin)
  - **About** — version + local-first privacy note
- Escape / backdrop / Close dismisses the panel

## Non-goals

- Conversion preferences stay on the main screen
- No accounts / cloud settings
