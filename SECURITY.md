# Security policy

## Supported versions

Only the latest release and the default branch receive fixes.

## Reporting a vulnerability

Please do **not** open a public issue for a security problem. Use GitHub's private reporting instead: open the [Security tab](https://github.com/PerspicaciousGuy/gleam/security) of this repository and choose **Report a vulnerability**.

Include what you found, how to reproduce it, the Gleam version and your Windows version. This is a one-person project, so replies are best effort and there is no guaranteed response time.

## What is in scope

Gleam indexes the names and paths of your files and launches items you select, so problems of interest include:

- a way to make Gleam open or run something the user did not select;
- a way for another local user, a web page or another process to read or change the index, the recent searches or the settings;
- a way for crafted file names or paths to run code, break out of the UI or corrupt the index;
- anything in the installer or release workflow that could ship a tampered build.

## What is not

- The Windows SmartScreen warning: the installer is not code-signed yet. Verify downloads against `SHA256SUMS.txt` on the release page.
- The index being readable by anyone who can already read your user profile. This is documented under [Privacy and data](README.md#privacy-and-data).
- Weaknesses in Windows, WebView2 or other third-party components that Gleam does not control. Please report those to their vendors.
