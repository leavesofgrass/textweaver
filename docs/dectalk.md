# Using DECtalk with textweaver

DECtalk is the classic formant synthesizer, known for voices such as Perfect Paul, Beautiful Betty, and Huge Harry. textweaver can speak with a DECtalk you have installed and highlights each word as DECtalk says it. Because DECtalk is proprietary, textweaver neither includes nor downloads it; you supply your own licensed copy.

The DECtalk support has so far been tested against a stand-in library that imitates DECtalk, not against a licensed DECtalk. If you try it with your copy, the [troubleshooting guide](troubleshooting.md) explains how to send a report.

## What you need

- A licensed DECtalk for your system: DECtalk Software for Windows (from DEC, Force Computers, Fonix, or Access Solutions), or a DECtalk SDK or runtime for Linux. On Windows its library is usually `DECtalk.dll` or `dectalk.dll`, with the dictionary `dtalk_us.dic` in the same folder. On Linux it is usually `libtts.so`.
- An engine host to run it in. A 64-bit DECtalk runs in a copy of `tw` or the app started in its hidden `--engine-host` mode. A 32-bit DECtalk, the usual kind on Windows, runs in `textweaver-host-x86.exe`, which is part of the release package; if you build textweaver yourself, `cargo xtask hosts` builds and installs it.

## Letting textweaver find DECtalk

With no configuration, textweaver searches the usual install folders. On Windows these are:

- `DECtalk`, `Fonix\DECtalk`, and `Fonix DECtalk` inside `Program Files` and `Program Files (x86)`;
- `dectalk.dll` in `Windows\System32` and `Windows\SysWOW64`.

On Linux they are `/usr/lib`, `/usr/local/lib`, and `/opt/dectalk/lib`, where textweaver looks for `libtts.so` or `libdectalk.so`.

If your DECtalk is installed elsewhere, name its library with an environment variable.

On Windows, in a command prompt:

```bat
setx TEXTWEAVER_DECTALK_LIBRARY "C:\Path\To\DECtalk.dll"
```

Then open a new command prompt, because `setx` affects only sessions started afterward.

On Linux, for the current shell (add the line to your shell's startup file to keep it):

```bash
export TEXTWEAVER_DECTALK_LIBRARY=/path/to/libtts.so
```

Alternatively, name it in your settings file, where it persists:

```toml
[speech.dectalk]
library = "C:\\Path\\To\\DECtalk.dll"
```

The environment variable takes precedence over the setting, and the setting over the usual folders.

To check what textweaver found, run the following; the DECtalk line states whether the engine is available:

```bash
tw backends
```

The 32-bit host, `textweaver-host-x86.exe`, must sit beside `tw` and `textweaver`, as the release package arranges it. To use a host stored elsewhere, name it with the `TEXTWEAVER_DECTALK_HOST` environment variable.

## Choosing DECtalk and a voice

DECtalk is not selected automatically when Eloquence or a Windows SAPI voice is available, so you must choose it. To hear a sentence:

```bash
tw speak --backend dectalk "Hello from DECtalk."
```

To read a document with DECtalk:

```bash
tw --backend dectalk book.md
```

To make DECtalk your permanent engine, set it in `settings.toml`:

```toml
[speech]
backend = "dectalk"
```

The voices are the nine DECtalk speakers:

- `dectalk:paul`, Perfect Paul (the default)
- `dectalk:harry`, Huge Harry
- `dectalk:frank`, Frail Frank
- `dectalk:dennis`, Doctor Dennis
- `dectalk:betty`, Beautiful Betty
- `dectalk:ursula`, Uppity Ursula
- `dectalk:wendy`, Whispering Wendy
- `dectalk:rita`, Rough Rita
- `dectalk:kit`, Kit the Kid

Pick one with `--voice dectalk:betty`, or by its short name, such as `--voice Betty`. `tw voices --backend dectalk` lists them.

The rate is measured in words per minute, DECtalk's native unit, from 75 to 600. Pitch raises or lowers the speaker's own voice, and volume behaves as with any other voice. Audio export with `tw export-audio --backend dectalk` produces subtitles timed to each word.

## If DECtalk does not start

- "no DECtalk found": textweaver did not find a library. Name it as described above.
- "does not look like a DECtalk library": the file you named is not a DECtalk library. Check the path.
- "DECtalk did not start": the library loaded but refused to start. Confirm that it is licensed on this computer and that its dictionary, `dtalk_us.dic`, is in the same folder as the library.
- "textweaver-host-x86.exe not found": your DECtalk is a 32-bit library, which requires the 32-bit host. Keep `textweaver-host-x86.exe` from the release package beside `textweaver.exe`, or build it with `cargo xtask hosts`.

## About the free DECtalk source

A DECtalk source tree is published on GitHub. Its own license states that the code belongs to Fonix and may be used only under a written license from them, so textweaver does not include it, build it, or test against it. If you point textweaver at a DECtalk library, that choice, and the license it requires, are yours.

## See also

- [Speech engines and voices](speech.md): every engine textweaver can use, and how it chooses.
- [Audio export](audio-export.md): DECtalk gives exact word timing in subtitles.
- [Troubleshooting](troubleshooting.md): when no speech is heard.
- [ADR-0021: DECtalk through a host process](adr/0021-dectalk.md): the design, the licensing rules, and what is not yet verified.
- [ADR-0012: The engine host](adr/0012-engine-host.md): the protocol between textweaver and the host.
- [Documentation index](README.md)
