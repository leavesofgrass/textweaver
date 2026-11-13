### Export, preview, and audio export.

## Export and preview from the reader. F5 is the browser's reload key,
## not textweaver's.

common-no-document = No document is open.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = cannot write to { $path }: { $error } Check that the folder can be written to.
publish-cannot-write = cannot write { $path }: { $error } Check that its folder can be written to.
publish-start-failed = Could not start the export: { $error } Wait a moment, then try again.
publish-export-error = Could not export: { $error } Fix that, then export again.
# The Export as prompt; $path is the file offered (the document's name
# with the format's extension, in its folder).
publish-export-as-label = Export as, Enter for { $path }
publish-export-over-source = Not exported: that is the document itself. Choose another name.
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Exporting to { $format }.
publish-theme-title = Theme for the HTML page
publish-theme-intro = Theme for the HTML page? { $first } first, { $n } choices. Escape cancels.
publish-writing-preview = Writing the preview.
publish-preview-error = Could not write the preview: { $error } Fix that, then preview again.
publish-still-exporting =
    { $secs ->
        [one] Still exporting to { $format }, { $secs } second.
       *[other] Still exporting to { $format }, { $secs } seconds.
    }
publish-still-previewing =
    { $secs ->
        [one] Still writing the preview, { $secs } second.
       *[other] Still writing the preview, { $secs } seconds.
    }
# "Browser preview follows" was chosen. $again is yes when a preview
# is open that reloads by itself only once it is opened again.
publish-follow-off = Browser preview follows nothing: press F5 in the browser after a save.
publish-follow-save =
    { $again ->
        [yes] Browser preview follows each save: the page reloads by itself. Preview in browser again to start it.
       *[no] Browser preview follows each save: the page reloads by itself.
    }
publish-follow-typing =
    { $again ->
        [yes] Browser preview follows your typing: the page reloads after each save and when typing pauses. Preview in browser again to start it.
       *[no] Browser preview follows your typing: the page reloads after each save and when typing pauses.
    }
# The first preview of a session: what the browser will do. $follow is
# the "Browser preview follows" setting: off, save, or typing.
publish-preview-first =
    { $follow ->
        [save] Preview opens in your browser and reloads after each save.
        [typing] Preview opens in your browser and follows your typing.
       *[off] Preview opens in your browser. Press F5 there after each save.
    }
# $error is the converter's reason.
publish-export-failed = Could not export to { $format }: { $error } Try another format.
publish-preview-failed = Preview failed: { $error } Save to try again.
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 warning: { $first }
       *[other] { $n } warnings; the first: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exported { $file }. Open it? y or n. Format { $format }, in { $folder }.{ $warned }
publish-report = Report saved as { $file }.
publish-report-issues =
    { $n ->
        [one] 1 item
       *[other] { $n } items
    } not made accessible; see the report.
publish-report-failed = The report could not be saved: { $error } Check that the output folder can be written to.
publish-preview-written-served = Preview written. Opening it in the browser. It reloads by itself after each save.{ $warned }
publish-preview-written = Preview written. Opening it in the browser. Saving writes it again; then press F5 in the browser.{ $warned }
publish-preview-updated = Preview updated.
publish-preview-updated-press-f5 = Preview updated. Press F5 in the browser.
# Said once for a document whose preview was cleaned of raw HTML
# (scripts, event handlers) because its folder is not trusted.
publish-preview-raw-html-left-out = Raw HTML left out of the preview. Trusted folders are in Settings.
publish-server-failed = Could not start the preview's reload server ({ $error }); opening the file instead.
publish-render-failed = Could not render the text: { $error } Leave edit mode to read the text.
publish-nothing-after-caret = Nothing to read after the cursor.
publish-listening = Listening to the rendered text.

## The preview's reload server: shown in the browser.

preview-being-written = The preview is being written. Reload in a moment.

## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = Export audio as
audio-format-intro = Export { $name } as audio: choose a format, { $n } choices.
audio-no-ffmpeg =
    { $formats } { $n ->
        [one] needs
       *[other] need
    } ffmpeg, which was not found.
audio-format-flac = FLAC: lossless, about half the size of WAV
audio-format-wav = WAV: the largest, plays everywhere
audio-format-mp3 = MP3: small, plays everywhere
audio-format-opus = Opus: the smallest, made for speech
audio-format-ogg = Ogg Vorbis: small and open, plays in most players
audio-format-m4b = M4B audiobook, through ffmpeg
audio-format-mp4 = Video with captions: MP4, needs ffmpeg
audio-format-html = Read-along page: text and audio, one file
audio-where-title = Where the audio goes
audio-where-intro = Where should the audio go?
audio-where-beside = Beside the document, { $path }
audio-where-choose = In another folder, chosen next
audio-choose-folder = Choose the folder for the audio
audio-no-engine = No speech engine here can write audio files. Install eSpeak NG, or choose another engine in the Speech menu.
audio-confirm = Export { $name } with { $voice } at { $wpm } words per minute, into { $path }? y or n
audio-started = Exporting { $name } as { $format }. Escape stops.
audio-progress = Exporting audio, { $percent } percent.
# While the karaoke video encodes, at most every ten seconds: the frames
# fed to ffmpeg ($fed), the frames in all ($all), and the minutes left.
audio-video-progress =
    { $minutes ->
        [one] Encoding video: { $fed } of { $all } frames, about 1 minute left.
       *[other] Encoding video: { $fed } of { $all } frames, about { $minutes } minutes left.
    }
audio-video-progress-soon = Encoding video: { $fed } of { $all } frames, under a minute left.
audio-busy = Already exporting { $name }. Escape stops.
audio-stop-question = Stop the export? No file is kept. y or n
audio-stopping = Stopping the export.
audio-still-exporting = Still exporting audio.
audio-stopped = Audio export stopped; no file was written.
audio-done =
    Wrote { $name }: { $length }, { $chapters ->
        [one] 1 chapter
       *[other] { $chapters } chapters
    }.
audio-subtitles = Subtitles in { $name }.
audio-failed = Could not export audio: { $error } Try another format or voice.
audio-thread-stopped = Audio export stopped unexpectedly.
