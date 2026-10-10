# Updates

textweaver can keep itself current. When you allow it, it looks once a day for a newer release, tells you the version and the size, and downloads it only if you say yes. Before anything is installed, the download is checked against the checksum the release publishes. Your settings, notes, library, reading positions, and optional components are never touched by an update.

This guide is written to be read with a screen reader. Each section starts with what you do, then explains it.

## Turning the check on or off

The first time textweaver starts, the list of optional components ends with one more row: "Check for updates automatically, once a day, from GitHub". It starts as "Not chosen". Press Space or Enter on it to choose it; textweaver says "Updates: checked once a day." Press it again to take it back. The answer takes effect at once, whether you then download components, skip them, or press Escape.

If your copy of textweaver was installed before this row existed, it asks the same question once, as a yes-or-no question, the first time nothing else is open: "Check for updates automatically, once a day? y or n".

To change your answer later, open Settings and find "Check for updates" in the Updates section; F1 on the setting reads its help. In `settings.toml` it is:

```toml
[updates]
check = true
```

Tools, Ask again about first-run choices asks the question again at the next start.

Until you answer yes, textweaver reads nothing from the network about updates.

## Checking now

Choose Help, Check for updates (or type "check for updates" in the command palette). textweaver says "Checking for updates." and then either offers the newer release or says "No update", with the version you have. You can check this way whether or not the daily check is on.

The daily check is quieter: it runs at most once a day, when textweaver starts, and says nothing unless it finds a release you have not already declined. If it cannot reach GitHub, it says nothing and tries again the next day; the log records why.

## What textweaver asks

When a newer release exists, textweaver asks, for example:

"Update available: textweaver 0.2.0-beta.2, 92.1 MB. Download it? y or n"

The question never interrupts reading. If textweaver is reading aloud, the question waits until reading stops or pauses, and until no dialog or other question is open.

Every newer release is offered, pre-release or stable, because textweaver is in beta and each beta release is a pre-release.

- **y** downloads it. textweaver says "Downloading the update", with its size, and then how far it has got, at most every ten seconds.
- **n** or Escape keeps what you have. textweaver remembers that you declined this release and does not offer it again; the next newer release is offered as usual. Help, Check for updates still offers it if you change your mind.
- Any other key says the question again.

## How a download is checked

Every textweaver release on GitHub carries a file named `SHA256SUMS.txt`, which lists the SHA-256 checksum of each package. textweaver reads that file first, then downloads the package for your computer and compares the package with both its listed size and its checksum. Only a package that matches goes any further. If the checksum does not match, textweaver says "Update refused: checksum does not match. Nothing changed." and keeps nothing of the download. A download that stops part way (the network drops, or you close textweaver) goes on from where it stopped the next time.

GitHub also attaches a build attestation to every package, a signed record of the workflow that built it. Checking that signature needs a Sigstore verifier, which textweaver does not include, so the in-app update relies on the checksum file, read over an encrypted connection from GitHub. You can check the attestation of any package yourself with the GitHub CLI; see [Checking a download](install.md#checking-a-download).

## How it is installed

textweaver updates the same package you installed, and only that package. It finds the folder the running program came from and confirms that it is a release package, by the `NOTICE` file every package carries (on macOS, by the `textweaver.app` bundle). A copy built from source, or installed by a system package manager, is not touched; textweaver says "Not updated: this copy is not from a release package." and you update it the way you installed it.

- **Windows.** A running program cannot replace its own files, so textweaver unpacks the new package and says "Update verified. It installs when textweaver closes." When you close textweaver, a small step in the new package waits for it to close, puts the new files in place, and starts the window again. If textweaver is installed under Program Files, Windows asks once for administrator permission.
- **Linux.** A tarball install is replaced in place, and an AppImage is replaced by the new image as one file. textweaver says "Update verified and installed. Restart textweaver to use it." The copy already running keeps working until you close it.
- **macOS.** The `textweaver.app` bundle and `tw` are replaced together, wherever they sit side by side, and textweaver asks you to restart.

The new files are first copied beside the old ones and only then renamed into place, so if any step fails, the old files are put back and the message ends "Nothing changed." Files that are not part of the package, such as `install-manifest.txt` written by the install scripts, are left as they are.

AppImages carry update information for tools such as AppImageUpdate, which download only the parts that changed. textweaver downloads the whole image instead, so that it can check the complete file against the published checksum.

## From the command line

`tw update --check` says whether a newer release exists, with its version and size, and changes nothing:

```text
$ tw update --check
Update available: textweaver 0.2.0-beta.2, 92.1 MB.
```

`tw update` checks, downloads, verifies, and installs in one step, without asking: running it is the yes, which suits scripts and install scripts. Progress goes to standard error at most every ten seconds. On Windows it finishes when `tw` closes, a moment later. Neither command reads the `[updates] check` setting or the release you declined in the app.

## Privacy

The check reads one public page, the list of textweaver releases at `https://api.github.com/repos/leavesofgrass/textweaver/releases`, and the download comes from the same release. Each request carries only textweaver's neutral User-Agent. Nothing about you, your computer, or your documents is sent, and textweaver keeps no record of the check beyond the time of the last one and the release you declined, both in `settings.toml`.

## Settings

| Setting | Default | What it does |
| --- | --- | --- |
| `[updates] check` | off until you answer | Check once a day at start, and offer a newer release. |

textweaver also keeps three markers in the same table, which you do not need to change: `asked` (the first-run question was asked), `last_check` (when the daily check last ran), and `declined` (the release you said no to). The [settings reference](settings-reference.md) lists them all.

## See also

- [Installing textweaver](install.md)
- [Optional components](components.md), which update separately
- [The command line](command-line.md)
