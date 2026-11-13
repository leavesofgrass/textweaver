### Reading: navigation, reading aloud, status, statistics, the queue, and reading aids.

## Reading statistics.

stats-title = Reading statistics
stats-intro = Reading statistics. Enter on a document opens it.
stats-off = Reading statistics are off. The last item turns them on.
stats-empty = No reading recorded yet. Time is counted while textweaver reads aloud.
# $time is a duration-* message.
stats-total =
    { $time } read in all, in { $sessions ->
        [one] 1 session
       *[other] { $sessions } sessions
    }, over { $docs ->
        [one] 1 document
       *[other] { $docs } documents
    }.
stats-current =
    This document: { $time } read, furthest point { $pct } percent, { $sessions ->
        [one] 1 session
       *[other] { $sessions } sessions
    }.
stats-current-none = This document has not been read aloud yet.
stats-most-read = Most read { $rank }: { $title }, { $time }, furthest point { $pct } percent.
stats-toggle-on = Statistics are on. Enter turns them off.
stats-toggle-off = Statistics are off. Enter turns them on.
stats-turned-on = Reading statistics are on.
stats-turned-off = Reading statistics are off. What was recorded is kept.

## Lists.

study-nothing-to-delete = Nothing to delete in this list.
study-nothing-to-rename = Nothing to rename in this list.

## tw stats.

stats-clear-question =
    { $n ->
        [one] Remove the reading statistics of 1 document? y or n
       *[other] Remove the reading statistics of { $n } documents? y or n
    }
stats-cleared = Reading statistics removed.
stats-clear-item = Remove the reading statistics
stats-clear-failed = Could not remove the reading statistics: { $error }. Check that the data folder can be written.
stats-off-cli = Reading statistics are off: stats.enabled is false in the settings.

## Continue reading and every computer's statistics (the sync wave, S6).

continue-title = Continue reading
# $n is the number of documents listed.
continue-intro =
    { $n ->
        [one] Continue reading: 1 document, newest first.
       *[other] Continue reading: { $n } documents, newest first.
    }
continue-empty = No places yet. Reading saves your place.
# One row, meaning first: the title, how far in, the computer, and how
# long ago (continue-ago-*).
continue-item = { $title }, { $pct } percent, { $device }, { $when }
continue-this-computer = this computer
continue-ago-now = just now
continue-ago-minutes =
    { $n ->
        [one] 1 minute ago
       *[other] { $n } minutes ago
    }
continue-ago-hours =
    { $n ->
        [one] 1 hour ago
       *[other] { $n } hours ago
    }
continue-ago-days =
    { $n ->
        [one] 1 day ago
       *[other] { $n } days ago
    }

## The reading queue and the review list (beta 3).

queue-title = Reading queue
# $n is the number of documents in the queue.
queue-intro =
    { $n ->
        [one] Reading queue, 1 document. Enter opens it, Delete takes it out.
       *[other] Reading queue, { $n } documents. Enter opens one, Delete takes it out.
    }
# $key adds a document from the library list (Ctrl+Enter).
queue-empty = The reading queue is empty. In the library, { $key } adds a document.
# A queued document whose file is not there (a drive not plugged in).
queue-item-missing = { $title }, not found
# A document another computer queued that is not on this computer.
queue-item-elsewhere = { $title }, not on this computer
# The title of such a document when no computer knows one.
queue-untitled = A document from another computer
# Enter on such a document in the reading queue.
queue-elsewhere-chosen = { $title } is not on this computer. It stays in the reading queue, and is found once you open it here.
# $n is how many documents the queue holds now.
queue-added =
    { $n ->
        [one] Added to the reading queue: { $title }. 1 document in the queue.
       *[other] Added to the reading queue: { $title }. { $n } documents in the queue.
    }
queue-removed = Removed from the reading queue: { $title }.
queue-no-file = Only a document saved as a file can join the reading queue.
queue-read-failed = Could not read the library for the reading queue: { $error }. Check library.json in the data folder; tw settings path names it.
# At the end of a document, [reading] after_end = "ask". $key is the read key.
queue-next-ask = Next: { $title }. { $key } to read.
# At the end of a document, [reading] after_end = "next".
queue-next-reading = Next: { $title }.
review-title = Review list
# $n is the number of documents listed.
review-intro =
    { $n ->
        [one] Review list, 1 document, read longest ago first. Enter opens it.
       *[other] Review list, { $n } documents, read longest ago first. Enter opens one.
    }
review-empty = Nothing to review yet. Documents you read aloud are listed here.
review-stats-off = The review list needs reading statistics, which are off. Turn on Reading statistics in Settings.
# One row, meaning first: the title, how long ago it was read
# (review-days), how far, and its notes (review-notes).
review-item = { $title }, { $days }, { $pct } percent, { $notes }
review-days =
    { $n ->
        [0] today
        [one] 1 day
       *[other] { $n } days
    }
review-notes =
    { $n ->
        [0] no notes
        [one] 1 note
       *[other] { $n } notes
    }
prompt-document-details = Document details

## Edit a document's details by hand.

# $name is the document's title; said when the form opens.
details-intro = Details of { $name }. Tab moves, Enter saves, Escape cancels.
details-label-title = Title
details-label-author = Author
details-label-doi = DOI
details-label-isbn = ISBN
# A field's drawn label: its label, then $n of $total fields.
details-prompt-label = { $label }, { $n } of { $total }
# Said on moving to a field: its label and value (or nav-blank).
details-field = { $label }: { $value }
# $fields lists the fields saved, by their labels.
details-saved = Details saved: { $fields }.
details-unchanged = Details not changed.
details-cancelled = Canceled. Details not changed.
# $text is what was typed in the DOI or ISBN field.
details-not-a-doi = Not a DOI: { $text }. Fix it or clear it.
details-not-an-isbn = Not an ISBN: { $text }. Fix it or clear it.
details-no-file = This document has no file, so no details to edit.
details-save-failed = Could not save the details: { $error } Try again.
# The statistics list could not wait for the sync folder.
stats-others-slow = Other computers skipped: folder slow.
stats-untitled = Untitled document
# One computer's share of a document: $device, $time, $sessions.
stats-computer =
    { $sessions ->
        [one] { $device }: { $time }, 1 session
       *[other] { $device }: { $time }, { $sessions } sessions
    }
stats-by-computer-off = Each computer: hidden. Enter shows it.
stats-by-computer-on = Each computer: shown. Enter hides it.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = blank
# Said for a picture that has no description (no alternative text).
nav-no-description = no description
# The document overview: the title first, then the counts. $time is overview-time.
overview-line = { $title }. Headings: { $headings }, tables: { $tables }, pictures: { $pictures }, footnotes: { $footnotes }. { $time }
# Reading time left from the cursor at the current rate.
overview-time =
    { $minutes ->
        [0] Less than a minute left.
        [one] About 1 minute left.
       *[other] About { $minutes } minutes left.
    }
# Said when the reading pass changes and as reading starts in a skim. $pass is a reading-pass-* name.
reading-pass-changed = Pass: { $pass }.
reading-pass-full = full text
reading-pass-first-sentences = first sentences
reading-pass-headings = headings
# High verbosity: $label is a structure label ("Heading level 2").
nav-message-at-labelled = { $label }, line { $line }, { $pct } percent: { $content }
nav-message-at = Line { $line }, { $pct } percent: { $content }
nav-message-labelled = { $label }: { $content }
# $message is the navigation message after wrapping around.
nav-wrapped = Wrapped. { $message }
# $what is a kind-* or unit-* noun, $unit its key.
nav-no-next = No next { $what }.
nav-no-previous = No previous { $what }.
nav-nothing-to-read = No { $what } to read.
nav-label-heading-level = Heading level { $level }
nav-label-list =
    { $n ->
        [0] List
        [one] List, 1 item
       *[other] List, { $n } items
    }
nav-label-table =
    { $n ->
        [0] Table
        [one] Table, 1 row
       *[other] Table, { $n } rows
    }
nav-label-list-item-level = List item, level { $level }
nav-no-heading-level =
    { $dir ->
        [next] No next heading at level { $level }.
       *[previous] No previous heading at level { $level }.
    }
# Said before a line's text on caret moves.
nav-line-heading = heading level { $level }
nav-line-row = row { $n }
nav-line-list-item-level = list item, level { $level }
# $language is a programming language name such as Python.
nav-line-code-language = code, { $language }
nav-no-chapters = This document has no chapters.
nav-chapter = Chapter
nav-no-chapter =
    { $dir ->
        [next] No next chapter.
       *[previous] No previous chapter.
    }
nav-back = Back
nav-forward = Forward
nav-page = Page
nav-percent = { $pct } percent
nav-no-earlier-history = No earlier history.
nav-no-forward-history = No forward history.
# $label is nav-back, nav-forward, nav-page, or nav-percent.
nav-label-line = { $label }, line { $line }
nav-top-of-document = Top of document
nav-end-of-document = End of document
# $edge is nav-top-of-document or nav-end-of-document; $content the line there.
nav-edge-message = { $edge }. { $content }
nav-top-of-document-stop = Top of document.
nav-end-of-document-stop = End of document.
nav-position = Line { $line } of { $lines }, { $pct } percent.
nav-position-word = Word { $word } of { $words }.
nav-position-heading = Under heading { $heading }.
nav-position-mode = { $mode } mode.

## Reading aloud.

playback-caps-no-words = Word highlight estimated: this voice reports no words.
playback-caps-words = Word highlight exact: this voice reports each word.
playback-caps-no-pitch = Pitch cannot be changed with this voice.
playback-caps-no-volume = Volume cannot be changed with this voice.
# The reading state on the title line, one word.
state-reading = Reading
state-paused = Paused
state-stopped = Stopped
state-ready = Ready
playback-reading-at = Reading at { $rate } words per minute.
playback-paused = Paused.
playback-stopped-speech-cursor-off = Stopped. Speech Cursor off.
playback-stopped = Stopped.
playback-search-cleared = Search cleared.
playback-end-of-document-content = end of document
playback-no-unit-here = No { $what } here.
playback-no-selection = No selection.
# $next is what happens now (a restart, or nothing).
playback-speech-died = Speech stopped working ({ $reason }). { $next }
playback-done-reading = Done reading.
playback-speech-restarted = Speech restarted: { $reason }. Reading on from the last word.
playback-speech-error = Speech error: { $error } Try again, or use the Restart speech command.
playback-end-of-section = End of section. { $key } to go on.
playback-time-up =
    { $minutes ->
        [one] Time is up after 1 minute. { $key } to go on.
       *[other] Time is up after { $minutes } minutes. { $key } to go on.
    }
playback-repeat-slower = Repeating slower, at { $rate } words per minute.
playback-recall-prompt = Say what you remember from { $section }. { $key } to go on.

## The title line and Say Status.

# The title line's position.
status-position = line { $line } of { $lines }
status-mode = { $mode } mode
status-modified = modified
status-self-voicing = self-voicing
status-hybrid = hybrid
status-screen-reader = screen reader mode
status-rate-spoken = { $wpm } words per minute
status-rate = { $wpm } wpm
# With CW as the engine: the CW speeds, the effective (Farnsworth) speed
# when it is below the character speed.
status-cw-rate = CW { $wpm } wpm
status-cw-rate-spoken = CW at { $wpm } words per minute
status-cw-rate-farnsworth = CW { $wpm } wpm, effective { $effective }
status-cw-rate-farnsworth-spoken = CW at { $wpm } words per minute, effective { $effective }
# The window's status bar, last: reading time left at the current rate.
status-time-left =
    { $minutes ->
        [0] under a minute left
        [one] 1 minute left
       *[other] { $minutes } minutes left
    }
status-no-document = No document
# $parts are the title line's parts, joined with commas.
status-said = { $title }: { $parts }.
status-no-message = No message yet.
# A list shown without its own introduction.
status-list-intro =
    { $n ->
        [one] { $title }, 1 item. Up and Down move, Enter chooses, Escape closes.
       *[other] { $title }, { $n } items. Up and Down move, Enter chooses, Escape closes.
    }

## Speech Cursor mode.

speechcursor-off = Speech Cursor off.
# $line is the line number.
speechcursor-on = Speech Cursor on, line { $line }. Up and Down read lines, Enter reads on, Tab or Escape leaves.
# $text is the line read, as the status line shows it.
speechcursor-on-with-text = Speech Cursor on, line { $line }: { $text }. Up and Down read lines, Enter reads on, Tab or Escape leaves.

## Scrolling the view without moving the cursor.

view-bottom-of-document = Bottom of document.
# $line is the line now at the top of the view.
view-line-at-top = Line { $line } at top.

## Math exploration.

mathx-no-math = No math here. Move to a formula, then try again.
# $math is the whole expression as spoken; $parts is yes when it has parts to go into. The keys are exploration's own.
mathx-exploring =
    { $parts ->
        [yes] Exploring math: { $math }. Down goes in, arrows move, Escape leaves.
       *[no] Exploring math: { $math }. Escape leaves.
    }
mathx-left = Left math.
mathx-last-term = Last term.
mathx-first-term = First term.
mathx-no-parts = No parts inside.
mathx-whole-expression = Whole expression.
mathx-nothing-here = Nothing here.
# $speech is what was said for the step; $code is the math braille code's name (Nemeth or UEB); $braille is the part's braille in Unicode braille cells, for the Braille display.
mathx-step-braille = { $speech } { $code }: { $braille }

## Reading aids: RSVP, bionic reading, syllables, difficult words, the ruler, and the reading level.

aids-rsvp-off = RSVP off.
aids-rsvp-leave-edit = Leave edit mode to use RSVP.
# $status is RSVP's status line (word and sentence counts, rate, state).
aids-rsvp-on = RSVP on. { $status }
aids-rsvp-no-words = No words to show.
aids-rsvp-fastest = Fastest RSVP rate.
aids-rsvp-slowest = Slowest RSVP rate.
# $wpm is the new rate in words per minute.
aids-rsvp-rate = RSVP { $wpm } words per minute.
# Where the RSVP word is shown; $position is one of nine fixed keys.
aids-rsvp-position =
    { $position ->
        [top-left] RSVP at the top left.
        [top-center] RSVP at the top center.
        [top-right] RSVP at the top right.
        [center-left] RSVP at the middle left.
        [center] RSVP at the center.
        [center-right] RSVP at the middle right.
        [bottom-left] RSVP at the bottom left.
        [bottom-right] RSVP at the bottom right.
       *[bottom-center] RSVP at the bottom center.
    }
aids-rsvp-playing = RSVP playing.
aids-rsvp-paused = RSVP paused.
aids-rsvp-end-of-text = End of text.
aids-rsvp-start-of-text = Start of text.
aids-bionic-on = Bionic reading on.
aids-bionic-off = Bionic reading off.
aids-syllables-shown = Syllables shown.
aids-syllables-hidden = Syllables hidden.
aids-difficult-on = Difficult words underlined.
aids-difficult-no-list = Difficult words on, but this version has no word list.
aids-difficult-off = Difficult words not marked.
# Added after a word at high verbosity, following a comma.
aids-difficult-word = difficult word
aids-ruler-off = Reading ruler off.
aids-ruler-current-line = Current line marked.
aids-ruler-on = Reading ruler on.
# $summary is aids-level-summary; $scope says what was measured.
aids-reading-level =
    { $scope ->
        [selection] Selection: { $summary }
       *[document] Document: { $summary }
    }
aids-reading-level-too-short = Not enough text to measure the reading level.
# $grade is the Flesch-Kincaid grade with one decimal, $band an aids-band-* message, $ease the reading ease (0 to 100), $words aids-level-words, $sentences aids-level-sentences.
aids-level-summary = Grade { $grade }, { $band }. Reading ease { $ease } out of 100. { $words } in { $sentences }.
# $n is the count, $count the same number written with thousands separators.
aids-level-words =
    { $n ->
        [one] { $count } word
       *[other] { $count } words
    }
aids-level-sentences =
    { $n ->
        [one] { $count } sentence
       *[other] { $count } sentences
    }
aids-band-elementary = elementary
aids-band-middle-school = middle school
aids-band-high-school = high school
aids-band-college = college
aids-band-graduate = graduate

## The Braille pass: pages in paged documents such as a PDF.
## $page and $n are page numbers, $label a printed page label such as iv,
## $pages the number of pages. Keep the page first: a 40-cell Braille
## display shows the start of the line.

# On the title line, in place of status-position.
status-page = page { $page } of { $pages }
status-page-labelled = page { $label }, { $n } of { $pages }
# The title line's percentage, after the place (in edit and Speech Cursor
# modes after the mode, "modified" and the reading state); % is shown, not said.
status-percent = { $pct }%
# Said first by the position report.
pages-position = Page { $page } of { $pages }.
pages-position-labelled = Page { $label }, { $n } of { $pages }.
pages-none = This document has no pages.
pages-no-such-page = No page { $page }. Pages go from 1 to { $pages }.
pages-label = Page { $label }
# An outline item: $text is the page's first words.
pages-outline-item = Page { $label }: { $text }
lists-pages-title =
    { $n ->
        [one] Pages, { $n } page
       *[other] Pages, { $n } pages
    }
lists-pages-title-filtered = Pages, { $shown } of { $n } match { $filter }
lists-pages-intro =
    { $n ->
        [one] Pages, { $n } page. Type to filter, Enter goes to a page, Escape closes.
       *[other] Pages, { $n } pages. Type to filter, Enter goes to a page, Escape closes.
    }
# $heading is the outline item of the page the cursor is on.
lists-pages-here = You are on { $heading }.
lists-filter-cleared-pages =
    { $n ->
        [one] Filter cleared, { $n } page.
       *[other] Filter cleared, { $n } pages.
    }
lists-filter-none-pages = No pages match { $query }. Backspace removes letters.
lists-filter-matched-pages =
    { $n ->
        [one] { $n } page match.
       *[other] { $n } pages match.
    }
# The go-to prompt in a paged document: a bare number is a page there.
prompt-go-to-pages = Go to page, or line 12, percent, start, or end
goto-not-a-target-pages = Not a go-to target: { $text }. Type a page number, line and a number, a percentage such as 50%, start, or end.
# Typed before a page label at the go-to prompt; page and p always work.
goto-word-page = page

## Summaries and difficult-word definitions.
# The summary list's title: $n sentences of the whole document.
summary-title =
    { $n ->
        [one] Summary, { $n } sentence
       *[other] Summary, { $n } sentences
    }
# The summary of the chapter at the cursor.
summary-title-chapter =
    { $n ->
        [one] Chapter summary, { $n } sentence
       *[other] Chapter summary, { $n } sentences
    }
# The summary of the selection.
summary-title-selection =
    { $n ->
        [one] Selection summary, { $n } sentence
       *[other] Selection summary, { $n } sentences
    }
# Said when the summary list opens; $title is one of the titles above.
summary-intro = { $title }. Enter goes to the sentence and says it.
# The same, when a long text was read in samples.
summary-intro-sampled = { $title }, from samples of this long text. Enter goes to the sentence and says it.
summary-none = Nothing to summarize: no sentence of four words or more.
# tw summarize, on standard error, when a long text was read in samples: $read of $total characters.
summary-sampled-cli = A long text: the summary comes from { $read } of its { $total } characters, read in samples.
# After a difficult word at high verbosity, with definitions on: its first definition.
aids-difficult-word-defined = difficult word: { $definition }
settings-unit-sentences =
    { $n ->
        [one] sentence
       *[other] sentences
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }, { -brand }.
# Said once after an update. $version is the new version, such as 0.1.0-beta.1.
gui-updated-to = Updated to { $version }. Help, Documentation describes what is new.
