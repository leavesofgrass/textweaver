# Using DECtalk with textweaver

DECtalk is the classic synthesizer many of us learned to listen with: Perfect Paul, Beautiful Betty, Huge Harry, and the rest. textweaver can speak with a DECtalk you have installed, and it highlights each word as DECtalk says it. textweaver does not include DECtalk and never downloads it: DECtalk is proprietary, so you supply your own licensed copy.

textweaver's DECtalk support has been tested against a stand-in library that behaves like DECtalk, not yet against a licensed DECtalk. If you try it with yours, the [troubleshooting guide](troubleshooting.md) says how to send a report.

## What you need

- A licensed DECtalk for your system: DECtalk Software for Windows (from DEC, Force Computers, Fonix, or Access Solutions), or a DECtalk SDK or runtime for Linux. On Windows its library is usually `DECtalk.dll` or `dectalk.dll`, with the dictionary `dtalk_us.dic` in the same folder. On Linux it is usually `libtts.so`.
- textweaver's DECtalk hosts, the small helper programs that run DECtalk. They come with textweaver's release package. If you build textweaver yourself, `cargo xtask hosts` builds and installs them.

## Letting textweaver find DECtalk

Without any setup, textweaver looks in the usual install folders. On Windows those are:

- `DECtalk`, `Fonix\DECtalk`, and `Fonix DECtalk` inside `Program Files` and `Program Files (x86)`;
- `dectalk.dll` in `Windows\System32` and `Windows\SysWOW64`.

On Linux they are `/usr/lib`, `/usr/local/lib`, and `/opt/dectalk/lib`, looking for `libtts.so` or `libdectalk.so`.

If your DECtalk is somewhere else, name its library with an environment variable.

On Windows, in a command prompt:

```bat
setx TEXTWEAVER_DECTALK_LIBRARY "C:\Path\To\DECtalk.dll"
```

Then open a new command prompt, since `setx` affects new windows only.

On Linux, for the current shell (add the line to your shell's start-up file to keep it):

```bash
export TEXTWEAVER_DECTALK_LIBRARY=/path/to/libtts.so
```

Or name it in your settings file, where it stays:

```toml
[speech.dectalk]
library = "C:\\Path\\To\\DECtalk.dll"
```

The environment variable wins over the setting, and the setting wins over the usual folders.

To check what textweaver found, run this. The DECtalk line says whether it is available:

```bash
tw backends
```

The DECtalk hosts must sit next to `tw` and `textweaver`, as the release package puts them. To use a host from somewhere else, name it with the `TEXTWEAVER_DECTALK_HOST` environment variable.

## Choosing DECtalk and a voice

DECtalk is not chosen automatically when Eloquence or a Windows SAPI voice is available, so choose it. To hear a sentence:

```bash
tw speak --backend dectalk "Hello from DECtalk."
```

To read a document with DECtalk:

```bash
textweaver --backend dectalk book.md
```

To make DECtalk your engine for good, set it in `settings.toml`:

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

Rate is in words per minute, DECtalk's own unit, from 75 to 600. Pitch raises or lowers the speaker's own voice, and volume works as with any other voice. Audio export with `tw export-audio --backend dectalk` gives subtitles timed to each word.

## If DECtalk does not start

- "no DECtalk found": textweaver did not find a library. Name it as shown above.
- "does not look like a DECtalk library": the file you named is not DECtalk's library. Check the path.
- "DECtalk did not start": DECtalk loaded but refused to start. Make sure it is licensed on this computer and that its dictionary, `dtalk_us.dic`, is in the same folder as the library.
- "textweaver-dectalk-host-x86.exe not found": your DECtalk is a 32-bit library, which runs in the 32-bit host. Install textweaver's hosts with the release package, or build them with `cargo xtask hosts`.

## About the free DECtalk source

A DECtalk source tree is published on GitHub. Its own licence says the code belongs to Fonix and may be used only under a written licence from them, so textweaver does not include it, build it, or test against it. If you point textweaver at a DECtalk library, that choice, and the licence it needs, are yours.

## See also

- [Speech engines and voices](speech.md): every engine textweaver can use, and how it chooses.
- [Audio export](audio-export.md): DECtalk gives exact word timing in subtitles.
- [Troubleshooting](troubleshooting.md): when no speech is heard.
- [ADR-0021: DECtalk through a host process](adr/0021-dectalk.md): the design, the licensing rules, and what is not yet verified.
- [ADR-0012: The engine host](adr/0012-engine-host.md): the protocol between textweaver and the helper.
- [Documentation index](README.md)
