# Privacy

textweaver sends nothing over the network unless you ask it to. It has no account, no analytics, and no crash reports. Your documents, notes, and settings stay on your computer.

## What stays on your computer

- Your documents, and every file textweaver writes.
- Notes, highlights, bookmarks, reading places, and reading statistics.
- Settings, profiles, and word lists.
- The log file, which is for you and for a bug report you choose to send.
- Speech, OCR, dictation, and the dictionary. They run on your computer. Looking up a word's definition never goes to the internet.

## When textweaver uses the network

Only when you ask. Each item below happens after an action of yours.

- **Optional downloads.** The dictation model, the OCR models, the Lexend font, and Piper voices are not in the packages. textweaver asks first, names the size and the license, and downloads only when you say yes. Saying no is always fine. See [Optional components](components.md).
- **Adding a reference by DOI or ISBN.** The DOI is sent to doi.org, or the ISBN to Open Library. Nothing else is sent. See [Citations](citations.md#what-is-sent-over-the-network).
- **Opening a web page** by its address, with `tw open` or `tw text`. textweaver fetches the page you named.
- **Opening a link** in a document. textweaver asks first, and then your browser opens it.
- **Install and update scripts.** They download a release from GitHub and check it. Each says what it will do first.

Requests from the components downloader carry one neutral name, `textweaver-research`, with the project's address. They carry nothing about you or your computer.

## Syncing between computers

textweaver has no sync server and no network code for sync. If you turn sync on, it writes files into a folder you chose. Something else you run, such as Syncthing or a USB stick, moves them. No name, path, or computer name is written to that folder. See [Syncing between computers](sync.md).

## A components mirror

If you set a components mirror, textweaver reads that mirror's list of components from it. Without a mirror, it reads nothing until you say yes.

## What textweaver does not do

- It does not collect telemetry.
- It does not store a password or token.
- It does not send your documents anywhere.
- It does not install anything outside its own data folder, or run a downloaded program.

## Reporting a problem

A bug report is a page on GitHub that you write and send yourself. Read the log before you attach it: it can hold file names.

## See also

- [Optional components](components.md)
- [Syncing between computers](sync.md)
- [Citations](citations.md)
- [Security policy](https://github.com/leavesofgrass/textweaver/blob/main/SECURITY.md)
