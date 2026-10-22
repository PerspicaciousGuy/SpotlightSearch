# Search interface update

## Reference research

Reviewed Apple's [Spotlight guide](https://support.apple.com/en-gb/guide/mac-help/mchlp1008/26/mac/26), its [category filtering documentation](https://support.apple.com/en-sg/guide/mac-help/mchl4d69efd3/mac), and the guide's current desktop screenshot. This was reference research, not a hands-on macOS audit. Apple uses a compact starting surface, category controls close to the input, ranked matches, and keyboard navigation. Our interpretation uses Windows shortcuts and locally styled icons; it does not use Apple's interface assets in the app.

## Before and after

1. **Initial view — improved.** The prior 540px-tall window spent most of its area on branding and empty-state instructions. The new surface starts at the height of the input and filters, expands with a query, and clears before hiding on Escape. [Before](screenshots/before.png), [after](screenshots/after-idle.png).
2. **Results — improved.** The top match is distinct, filename matches have text emphasis, and the selected result has a separate type/location pane with open and copy-path actions. Polling the index preserves the selected item. [Results](screenshots/after-results.png).
3. **No results — improved.** The screen suggests a different filename and offers to broaden a category-filtered search. It retains focus and the query. [Empty state](screenshots/after-empty.png).
4. **Narrow view — checked.** Details collapse to prioritize the keyboard-navigable list; long names truncate. No horizontal overflow at 320, 480, 640, 768, or 1440px. [Narrow view](screenshots/after-narrow.png).

## Accessibility and verification

Checked focusable controls, arrow navigation from both the input and results region, Enter feedback, Escape clear/hide, Alt+Space reopen in the browser, Ctrl+1–4 filters, clipboard copy, empty-state recovery, and that the normal browser preview has no system results. Automated axe checks reported no WCAG A/AA violations in the tested results, empty, and idle states after fixing label contrast and scroll-region focus. This does not establish full accessibility compliance; screen-reader testing on Windows remains necessary.

The production frontend build and Rust search-core test pass. Native transparency, resizing, shortcut registration, tray interaction, and real file opening remain to be validated on Windows; this environment is Linux. The large background and preview toolbar appear only in browser mode. All displayed example files are explicitly labeled sample data.
