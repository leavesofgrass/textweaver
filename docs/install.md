# Installing textweaver

In a hurry? The [quick start](quickstart.md) is the short version.

textweaver is in alpha. Each release on GitHub has a Windows package and a macOS package:

- `textweaver-VERSION-windows-x86_64.zip`
- `textweaver-VERSION-macos-universal.tar.gz`, for Apple silicon and Intel Macs

Both packages contain two programs:

- `textweaver`, the terminal reader. Run `textweaver FILE`.
- `tw`, the command-line tool. Run `tw --help`.

Download from the [releases page](https://github.com/leavesofgrass/textweaver/releases).

## Windows

1. Download the `.zip` and extract it to a folder of your own, for example `C:\Tools\textweaver`.
2. Optionally, add that folder to your `PATH`, so you can run `textweaver` and `tw` from any folder.
3. Open Windows Terminal or a command prompt, then run `tw backends` to see which speech engines textweaver found.

Keep the files in the folder together. The engine hosts (`textweaver-eci-host.exe`, `textweaver-sapi-host.exe`, and their `-x86` versions) must stay next to `textweaver.exe` and `tw.exe`. The `ibmtts-dictionaries` folder must stay there too. The hosts run speech engines in their own processes:

- **SAPI5 voices**, the Windows voices, work straight away. That includes OneCore voices, and any SAPI5 voices you have installed.
- **Eloquence** needs an Eloquence library you have a right to use. See `docs/eloquence.md` in the package, or run `tw eloquence --guide`.

The programs are not code-signed. Windows SmartScreen may warn the first time you run one. Choose "More info", then "Run anyway". A command-line program run from a terminal usually shows no warning.

## macOS

The macOS build is not notarized by Apple yet, because notarization needs a paid developer account. The binaries are signed ad hoc, so they run on Apple silicon. Gatekeeper still blocks the first run of a download that is not notarized, so do this once:

1. Download the `.tar.gz`. Extract it in Finder, or in Terminal:

   ```bash
   tar -xzf textweaver-*-macos-universal.tar.gz
   ```

2. Remove the quarantine flag that the browser added, then run the programs from Terminal. Replace the folder name with the one you extracted:

   ```bash
   xattr -dr com.apple.quarantine textweaver-0.1.0-alpha.3-macos-universal
   ```

   If you skip this, macOS says the program "cannot be opened because Apple cannot check it for malicious software". In that case open System Settings, go to Privacy & Security, and choose "Open Anyway" next to the message about `tw` or `textweaver`. Then run the program again.

3. Optionally, copy `textweaver` and `tw` to a folder on your `PATH`, such as `/usr/local/bin`.

textweaver speaks with Apple's voices, including the Eloquence voices built into macOS. Reed is the default when it is installed. Run `tw voices` to list the voices.

## Checking a download

Each release has a `SHA256SUMS.txt` file. To check a download on Windows:

```bash
certutil -hashfile textweaver-0.1.0-alpha.3-windows-x86_64.zip SHA256
```

On macOS:

```bash
shasum -a 256 textweaver-0.1.0-alpha.3-macos-universal.tar.gz
```

Compare the result with the line for that file in `SHA256SUMS.txt`.

## Building from source

See the README. Linux users build from source for now. The Docker image in `docker/` has everything you need, including espeak-ng.
