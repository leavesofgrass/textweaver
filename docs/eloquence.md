# Getting ETI-Eloquence for textweaver

ETI-Eloquence is the synthesizer that many screen-reader users know from JAWS, NVDA, and Kurzweil 1000. Reed, its default male voice, is crisp and remains intelligible at very high rates. textweaver speaks with Eloquence and highlights each word as it is spoken, but it does not include Eloquence: the engine is proprietary, and you must obtain it from one of the licensed sources below.

To see what textweaver finds on your computer, run:

```bash
tw eloquence
```

The command lists every Eloquence engine it detected and the one it will use, and it suggests what to do if nothing was found. `tw eloquence --guide` prints this guide.

## The short version

- **Mac** (macOS 13 Ventura or later): Eloquence is already built in, free. The voices are Reed, Shelley, Rocko, Sandy, Flo, Eddy, Grandma, and Grandpa.
- **Linux**: Voxin, from Oralux. It is paid, per user.
- **Windows**: Code Factory's Eloquence for Windows. It is paid, per user.

textweaver selects Eloquence automatically when it finds a licensed engine, and uses Reed unless you choose another voice.

## Mac

Apple includes the Eloquence voices in macOS 13 Ventura and later, in each language they support. Nothing needs to be purchased or installed.

1. Start textweaver or `tw` as usual. The Apple speech engines list the Eloquence voices among the system voices.
2. Choose Reed with `--voice Reed`, or make the choice permanent in your settings file:

   ```toml
   [speech]
   voice = "Reed"
   ```

textweaver can drive Apple's voices in two ways. `nsspeech` responds fastest, which many screen-reader users prefer; `avspeech` provides the most precise word highlighting. Select one with `--backend nsspeech` or `--backend avspeech`.

## Linux: Voxin

Voxin, from Oralux (voxin.oralux.net), is the licensed way to run Eloquence on Linux. It installs a 64-bit library, `libibmeci.so`, together with a small 32-bit engine process that runs alongside it. A single installation serves Emacspeak, Orca through Speech Dispatcher, and textweaver.

1. Purchase and download Voxin for your language from voxin.oralux.net.
2. Run its installer, following its instructions.
3. Run `tw eloquence`. textweaver looks for the library at `/opt/oralux/voxin/lib/libibmeci.so`, `/usr/lib/libibmeci.so`, and `/opt/IBM/ibmtts/lib/libibmeci.so`. If your copy is elsewhere, name its location:

   ```bash
   export TEXTWEAVER_ECI_LIBRARY=/path/to/libibmeci.so
   ```

On 64-bit systems, Voxin's engine process requires the 32-bit C library (`libc6:i386` and `libstdc++6:i386` on Debian and Ubuntu).

## Windows: Code Factory's Eloquence for Windows

Code Factory (codefactoryglobal.com) sells Eloquence for Windows, which installs Eloquence as SAPI5 voices and includes the engine library that textweaver uses for exact highlighting.

1. Purchase and install Eloquence for Windows from Code Factory.
2. Tell textweaver, once, that you own it:

   ```powershell
   setx TEXTWEAVER_ECI_CODE_FACTORY 1
   ```

   Then open a new terminal. textweaver does not use a Code Factory installation until you do this, because an installed copy is not necessarily a licensed one.
3. Run `tw eloquence` to confirm that it was found.

Code Factory's library is 32-bit, so textweaver runs it in the 32-bit engine host, `textweaver-host-x86.exe`, which ships with textweaver; this lets it work with 64-bit textweaver.

Eloquence reached through SAPI5 cannot report when each word is spoken. For that reason textweaver always uses the engine library directly for Eloquence, and lists Code Factory's SAPI5 voices only after step 2.

## Eloquence that came with other software

JAWS, Kurzweil 1000, and some NVDA add-ons include Eloquence. Such copies are usually licensed for use with that product alone, and textweaver does not search for them. If your license permits other programs to use your copy, point textweaver at its engine library with `TEXTWEAVER_ECI_LIBRARY`.

## OpenEVV

OpenEVV is an independent reimplementation of IBM's Embedded ViaVoice, the engine behind Eloquence, with builds for Windows and for NVDA. Its authors state that the language data it contains belongs to IBM and that they cannot license it. For that reason textweaver never downloads, installs, or bundles OpenEVV. If you install it yourself, textweaver detects it (in the location used by its Windows installer and by its NVDA add-on) and can use its 64-bit engine; whether to install it is your decision.

## Pronunciation dictionaries

textweaver includes the community IBMTTS pronunciation dictionaries (by amirsol81, x0, thunderdrop, and many contributors, released into the public domain under CC0) and loads them into Eloquence automatically. They correct thousands of words that Eloquence mispronounces. To disable them, or to use your own copy, set `TEXTWEAVER_ECI_DICTIONARIES`. In a Windows command prompt, this applies to the current session only:

```bat
set TEXTWEAVER_ECI_DICTIONARIES=off
```

```bat
set TEXTWEAVER_ECI_DICTIONARIES=C:\path\to\my\dictionaries
```

To make the choice permanent, set it in `settings.toml`, using `false` to disable the dictionaries or a folder path to use your own:

```toml
[speech.eci]
dictionaries = false
```

With OpenEVV, the root dictionary is loaded after speech has started, because OpenEVV 0.3.0 takes about a minute to load it and the engine would otherwise start silent. textweaver begins speaking with the main and abbreviation dictionaries, loads all three in a second copy of the engine in the background, and switches to that copy between sentences once it is ready. The log records how long the root dictionary took; if it fails to load, the log says why and speech continues without it. If an engine takes too long to start with the dictionaries, textweaver restarts it without them, so Eloquence still speaks.

## Checking that it works

First, list the speech engines; `eci` should appear as available:

```bash
tw backends
```

Then list Eloquence's voices. The list should include Reed and the other seven voices:

```bash
tw voices --backend eci
```

Finally, have Reed speak a sentence:

```bash
tw speak --voice Reed "Eloquence is working."
```

## A 32-bit engine library

On Windows, 64-bit textweaver runs a 32-bit `eci.dll` only through its 32-bit host, `textweaver-host-x86.exe`. A 64-bit library runs in a copy of textweaver itself, started in its hidden `--engine-host` mode. Without the 32-bit host, textweaver reports "A 32-bit ECI library cannot run in this 64-bit textweaver" and names the file; install the 64-bit (x86_64) library instead. OpenEVV's 32-bit library is always refused, because it uses a different calling convention.

## Settings reference

These environment variables control how textweaver finds and runs Eloquence:

- `TEXTWEAVER_ECI_LIBRARY`: use this engine library instead of searching. An engine library in textweaver's components folder (from your own components source) is still tried first.
- `TEXTWEAVER_ECI_CODE_FACTORY`: set to `1` after purchasing Code Factory's Eloquence for Windows, so that textweaver uses it.
- `TEXTWEAVER_ECI_DICTIONARIES`: `off`, or a folder of dictionaries to use instead of the included ones.
- `TEXTWEAVER_ECI_HOST`: run Eloquence in this host program instead of textweaver itself or `textweaver-host-x86.exe` (for example a separate `textweaver-eci-host.exe` from an earlier release).

The same choices can be stored in the `[speech.eci]` section of `settings.toml`: `library`, `code_factory`, and `dictionaries`. An environment variable takes precedence over the setting for that run. [Settings](settings.md#voice-and-speech) describes them.

## Why textweaver does not include Eloquence

Eloquence belongs to Cerence, which acquired it with Nuance's text-to-speech business, and it cannot be redistributed without Cerence's permission. IBM's older Embedded ViaVoice developer kit, still downloadable from IBM, is licensed for building prototypes only, not for use in finished applications. Should Cerence ever permit free redistribution for accessibility software, textweaver will offer Eloquence out of the box.

## See also

- [Speech engines and voices](speech.md): every engine textweaver can use, and how it chooses.
- [Troubleshooting](troubleshooting.md): when no speech is heard.
- [ADR-0007: Eloquence through an ECI host](adr/0007-eloquence-via-eci-host.md): why Eloquence runs in an engine host, and how word timing works.
- [ADR-0012: The engine host](adr/0012-engine-host.md): the protocol between textweaver and the host.
- [ADR-0008: Apple speech](adr/0008-apple-speech.md): Eloquence on macOS.
- [Documentation index](README.md)
