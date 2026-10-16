# Start here: accommodation staff

This page is for people who prepare documents and computers for students: converting course material, configuring shared or managed laptops, and keeping a record of what was provided.

## Convert course material

- `tw convert` converts a document, or a whole folder, to accessible HTML, EPUB, Word, Braille (BRF), or tagged PDF. See [Converting documents](converting.md).
- In the app, File, Batch convert processes a folder in the background.
- Each folder conversion writes `conversion-report.txt` to the output folder. It lists what was converted and which items need review. Keep it with the accommodation file.
- Scanned pages go through optical character recognition (OCR), which makes errors. Check anything important against the original.
- Publishing templates (large print, dyslexia-friendly, high contrast, APA, and AMA) are described in the same guide. They are options; none guarantees a result for any particular student.

## Prepare a computer

- Windows: use the Windows package, `textweaver-VERSION-windows-x86_64.zip`. It contains the app and the terminal program `tw`. Extract it to a folder the student can open, and run `textweaver-gui.exe`. The program is not code-signed, so Windows warns on first launch. A managed laptop may require your IT office to allow it. See [Installing textweaver](install.md).
- The zip route only extracts files. textweaver keeps its settings and notes in the user's own folders and does not change system settings.
- Settings can be exported and imported. See [Settings](settings.md).

## A computer without internet access

Some features depend on downloaded files: the dictation model, the OCR models, the Lexend font, and the helper programs ffmpeg, liblouis, and Pandoc. textweaver never downloads them unprompted, and it checks for updates only if the student agrees. For a computer that cannot reach the internet, download the files once and install them from a zip file or a folder, or point textweaver at a mirror you maintain. See [Optional components](components.md).

## Privacy

textweaver sends nothing over the network unless a person asks it to. [Privacy](privacy.md) gives the exact list.

## Accessibility and testing

The [accessibility statement](accessibility.md) and [Known limits](known-limits.md) record what has and has not been tested. textweaver is in beta, so try it with the student before relying on it.

## Helping a student get started

Give the student [Start here: students](start-students.md) and, if they use a screen reader, [Using textweaver with a screen reader](screen-readers.md).

## See also

- [Syncing between computers](sync.md)
- [Troubleshooting](troubleshooting.md)
- [What is new](whats-new.md)
