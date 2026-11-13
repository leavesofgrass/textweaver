## Live documents and help

- Join from a QR image reads the image on a helper thread, so a large or unusual image no longer holds up the interface. After a second it says "Reading the QR image." once; opening another dialog, or asking again, cancels the reading without a word.
- A live source at a Tailscale-range address now says "Connected to NAME at a Tailscale-range address. If this is Tailscale, the link is encrypted." That range is also used by carriers, so textweaver no longer claims encryption it cannot confirm.
- The keyboard reference no longer says the textweaver app lists the keyboard shortcuts with F3: F3 is Find next in both frontends, and shows or hides the text in a secret field.
- The reading guide says that following a link to a file on another computer asks first, because Windows may send your sign-in to that computer.
- `cargo xtask docs --check` fails when the What is new page still calls a tagged release "Not yet released".
