# Getting ETI-Eloquence for textweaver

ETI-Eloquence is the voice many screen-reader users know from JAWS, NVDA, and Kurzweil 1000. Reed, its default male voice, is fast, crisp, and easy to follow at high speeds. textweaver speaks with Eloquence and highlights each word exactly as it is spoken, but it does not include Eloquence itself: Eloquence is proprietary, and you get it from one of the licensed sources below.

To see what textweaver finds on your computer, run:

```bash
tw eloquence
```

It lists every Eloquence engine it detected, which one it will use, and what to do if nothing was found. `tw eloquence --guide` prints this guide.

## The short version

- **Mac** (macOS 13 Ventura or later): Eloquence is already built in, free. The voices are Reed, Shelley, Rocko, Sandy, Flo, Eddy, Grandma, and Grandpa.
- **Linux**: Voxin, from Oralux. It is paid, per user.
- **Windows**: Code Factory's Eloquence for Windows. It is paid, per user.

textweaver picks Eloquence automatically when it finds a licensed engine, and uses Reed unless you choose another voice.

## Mac

Apple includes the Eloquence voices in macOS 13 Ventura and later, in each language they support. Nothing needs to be bought or installed.

1. Start textweaver or `tw` as usual. The Apple speech engines list the Eloquence voices with the rest of the system voices.
2. Choose Reed with `--voice Reed`, or set it once in your settings file:

   ```toml
   [speech]
   voice = "Reed"
   ```

textweaver offers two ways to drive Apple's voices. `nsspeech` answers fastest, which many screen-reader users prefer; `avspeech` gives the most exact word highlighting. Choose one with `--backend nsspeech` or `--backend avspeech`.

## Linux: Voxin

Voxin, from Oralux (voxin.oralux.net), is the licensed way to run Eloquence on Linux. It installs a 64-bit library, `libibmeci.so`, and a small 32-bit engine process that runs alongside it. The same installation serves Emacspeak, Orca through Speech Dispatcher, and textweaver.

1. Buy and download Voxin for your language from voxin.oralux.net.
2. Run its installer, as its instructions describe.
3. Run `tw eloquence`. textweaver looks for the library at `/opt/oralux/voxin/lib/libibmeci.so`, `/usr/lib/libibmeci.so`, and `/opt/IBM/ibmtts/lib/libibmeci.so`. If yours is elsewhere, name it:

   ```bash
   export TEXTWEAVER_ECI_LIBRARY=/path/to/libibmeci.so
   ```

On 64-bit systems Voxin's engine process needs the 32-bit C library (`libc6:i386` and `libstdc++6:i386` on Debian and Ubuntu).

## Windows: Code Factory's Eloquence for Windows

Code Factory (codefactoryglobal.com) sells Eloquence for Windows, which installs Eloquence as SAPI5 voices and includes the engine library textweaver uses for exact highlighting.

1. Buy and install Eloquence for Windows from Code Factory.
2. Tell textweaver you own it, once:

   ```powershell
   setx TEXTWEAVER_ECI_CODE_FACTORY 1
   ```

   Then open a new terminal. textweaver does not use a Code Factory installation until you do this, because a copy on a computer is not necessarily a licensed one.
3. Run `tw eloquence` to confirm it was found.

Code Factory's library is 32-bit; textweaver runs it in a small 32-bit helper program that ships with textweaver, so this works with 64-bit textweaver.

Eloquence reached through SAPI5 cannot tell textweaver when each word is spoken, so textweaver always uses the engine library directly for Eloquence, and lists Code Factory's SAPI5 voices only after step 2.

## Eloquence that came with other software

JAWS, Kurzweil 1000, and some NVDA add-ons include Eloquence. Those copies are usually licensed for use with that product only. textweaver does not look for them. If your license allows other programs to use the copy you have, point textweaver at its engine library with `TEXTWEAVER_ECI_LIBRARY`.

## OpenEVV

OpenEVV is an independent reimplementation of IBM's Embedded ViaVoice, the engine behind Eloquence, with builds for Windows and for NVDA. Its authors state that the language data it contains is IBM's and that they cannot license it. For that reason textweaver never downloads, installs, or bundles OpenEVV. If you install it yourself, textweaver detects it (its Windows installer's location and its NVDA add-on's location) and can use its 64-bit engine; whether to install it is your decision.

## Pronunciation dictionaries

textweaver includes the community IBMTTS pronunciation dictionaries (by amirsol81, x0, thunderdrop, and many contributors, released into the public domain under CC0) and loads them into Eloquence automatically. They fix thousands of words Eloquence mispronounces. To turn them off, or to use your own copy, set `TEXTWEAVER_ECI_DICTIONARIES`. In a Windows command prompt, for the current window only:

```bat
set TEXTWEAVER_ECI_DICTIONARIES=off
```

```bat
set TEXTWEAVER_ECI_DICTIONARIES=C:\path\to\my\dictionaries
```

Or set it for good in `settings.toml`. Write `false` to turn them off, or a folder path:

```toml
[speech.eci]
dictionaries = false
```

With OpenEVV, textweaver loads the main and abbreviation dictionaries but leaves out the root dictionary: OpenEVV 0.3.0 takes about a minute to load it, and Eloquence would start silent. If an engine ever takes too long to start with the dictionaries, textweaver starts it again without them, so Eloquence still speaks.

## Checking that it works

First, list the speech engines. `eci` should be listed as available:

```bash
tw backends
```

Then list Eloquence's voices. You should hear of Reed and the other seven voices:

```bash
tw voices --backend eci
```

Finally, speak a sentence with Reed:

```bash
tw speak --voice Reed "Eloquence is working."
```

## Settings reference

These environment variables change how textweaver finds and runs Eloquence:

- `TEXTWEAVER_ECI_LIBRARY`: use this engine library instead of searching.
- `TEXTWEAVER_ECI_CODE_FACTORY`: set to `1` after buying Code Factory's Eloquence for Windows, so textweaver uses it.
- `TEXTWEAVER_ECI_DICTIONARIES`: `off`, or a folder of dictionaries to use instead of the included ones.
- `TEXTWEAVER_ECI_HOST`: use this helper program instead of the one next to textweaver.

The same choices can be kept in `settings.toml`, in the `[speech.eci]` section: `library`, `code_factory`, and `dictionaries`. An environment variable wins over the setting for that run. [Settings](settings.md#speecheci) describes them.

## Why textweaver does not include Eloquence

Eloquence belongs to Cerence, which acquired it with Nuance's text-to-speech business, and it cannot be given away without Cerence's permission. IBM's old Embedded ViaVoice developer kit, still downloadable from IBM, is licensed for building prototypes only, not for use in finished applications. If Cerence ever allows free redistribution for accessibility software, textweaver will offer Eloquence out of the box.

## See also

- [Speech engines and voices](speech.md): every engine textweaver can use, and how it chooses.
- [Troubleshooting](troubleshooting.md): when no speech is heard.
- [ADR-0007: Eloquence through an ECI host](adr/0007-eloquence-via-eci-host.md): why Eloquence runs in a helper process, and how word timing works.
- [ADR-0012: The engine host](adr/0012-engine-host.md): the protocol between textweaver and the helper.
- [ADR-0008: Apple speech](adr/0008-apple-speech.md): Eloquence on macOS.
- [Documentation index](README.md)
