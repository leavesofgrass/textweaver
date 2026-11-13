### Edit mode: typing, formatting, saving, recovery, and templates.

## Edit mode: entering, leaving, saving, and typing.

# $key makes a new document.
edit-no-document = No document to edit. Press { $key } for a new one.
# $line is the line at the caret, as echoed.
edit-mode-on-brief = Edit mode on. { $line }
# $save and $finish are the keys that save and leave edit mode; $line is
# the line at the caret.
edit-mode-on = Edit mode on. Save: { $save }. Finish: { $finish }. { $line }
# Keep the letters s, d and c: they are the keys that answer.
edit-unsaved-question = { $title } has unsaved changes. Save, discard, or cancel? Press s, d, or c, or Up and Down and Enter. Escape cancels.
edit-save-changes-title = Save changes to { $title }?
edit-choice-save = Save, then continue
edit-choice-discard = Discard the changes
edit-choice-cancel = Cancel, keep editing
edit-save-failed = Could not save: { $error } Still editing. Try Save as.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = Save as, Enter for { $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } already exists. Replace it? y or n
edit-mode-off = Edit mode off.
edit-mode-off-discarded = Changes discarded. Edit mode off.
# The title of a new, unsaved document.
edit-untitled = Untitled
edit-new-document = New document ready for editing.
# $key turns on edit mode.
edit-nothing-to-save = Nothing to save. Turn on edit mode with { $key } to make changes.
# $key turns on edit mode; $what is what the user tried to do.
edit-not-editing =
    { $what ->
        [type] Turn on edit mode with { $key } to type.
        [change-text] Turn on edit mode with { $key } to change the text.
        [delete-text] Turn on edit mode with { $key } to delete text.
        [undo] Turn on edit mode with { $key } to undo.
        [redo] Turn on edit mode with { $key } to redo.
        [replace-text] Turn on edit mode with { $key } to replace text.
        [insert] Turn on edit mode with { $key } to insert into the text.
        [cut-text] Turn on edit mode with { $key } to cut text.
        [move-cells] Turn on edit mode with { $key } to move between table cells.
        [delete-words] Turn on edit mode with { $key } to delete words.
        [paste] Turn on edit mode with { $key } to paste.
        [citation] Turn on edit mode with { $key } to insert a citation.
        [bibliography] Turn on edit mode with { $key } to insert a bibliography.
       *[format] Turn on edit mode with { $key } to format text.
    }
# A paste: $n characters.
edit-pasted =
    { $n ->
        [one] Pasted 1 character.
       *[other] Pasted { $n } characters.
    }
# $start is how the pasted text starts.
edit-pasted-start =
    { $n ->
        [one] Pasted 1 character: { $start }
       *[other] Pasted { $n } characters: { $start }
    }
edit-insert-failed = Could not insert: { $error } The text is unchanged.
# $start is the first words of the pasted text.
edit-pasted-lines =
    { $n ->
        [one] Pasted 1 line: { $start }
       *[other] Pasted { $n } lines: { $start }
    }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = Cannot change characters { $start } to { $end }: the text has { $len }.
edit-change-failed = Could not change the text: { $error } The text is unchanged.
edit-delete-failed = Could not delete: { $error } The text is unchanged.
edit-list-ended = List ended.
# Said when Enter continues a bulleted list.
edit-bullet = bullet
edit-table-divider = table header divider
edit-end-of-line-stop = End of line.
edit-start-of-line-stop = Start of line.
edit-end-of-line-content = end of line

## Edit mode: formatting, undo, tables, images, and replace.

# $what names the formatting command; $level is a heading level, and
# $cols and $rows a table's size.
edit-format-done =
    { $what ->
        [bold] Bold.
        [italic] Italic.
        [underline] Underline.
        [strikethrough] Strikethrough.
        [code] Code.
        [code-block] Code block.
        [link] Link.
        [bulleted-list] Bulleted list.
        [numbered-list] Numbered list.
        [block-quote] Block quote.
        [horizontal-rule] Horizontal rule inserted.
        [table-row] Table row added.
        [heading-level] Heading level { $level }.
        [table] Inserted a table, { $cols } columns by { $rows } rows.
       *[heading] Heading.
    }
# The command toggled its markup off.
edit-format-removed =
    { $what ->
        [bold] Bold removed.
        [italic] Italic removed.
        [underline] Underline removed.
        [strikethrough] Strikethrough removed.
        [code] Code removed.
        [code-block] Code block removed.
        [link] Link removed.
        [bulleted-list] Bulleted list removed.
        [numbered-list] Numbered list removed.
        [block-quote] Block quote removed.
        [horizontal-rule] Horizontal rule removed.
        [table-row] Table row removed.
        [heading-level] Heading level { $level } removed.
        [table] Table removed.
       *[heading] Heading removed.
    }
edit-format-unchanged =
    { $what ->
        [bold] Bold: nothing changed.
        [italic] Italic: nothing changed.
        [underline] Underline: nothing changed.
        [strikethrough] Strikethrough: nothing changed.
        [code] Code: nothing changed.
        [code-block] Code block: nothing changed.
        [link] Link: nothing changed.
        [bulleted-list] Bulleted list: nothing changed.
        [numbered-list] Numbered list: nothing changed.
        [block-quote] Block quote: nothing changed.
        [horizontal-rule] Horizontal rule: nothing changed.
        [table-row] Table row: nothing changed.
        [heading-level] Heading level { $level }: nothing changed.
        [table] Table: nothing changed.
       *[heading] Heading: nothing changed.
    }
# Added after a formatting message; $text is the start of the selection.
edit-format-selected = Selected: { $text }
edit-heading-level-now = Heading level { $level }.
# $line is the line at the caret after the undo or redo.
edit-undo-redo =
    { $what ->
        [undo] Undo.
       *[redo] Redo.
    }
edit-undo-redo-line =
    { $what ->
        [undo] Undo. { $line }
       *[redo] Redo. { $line }
    }
edit-nothing-to-undo = Nothing to undo.
edit-nothing-to-redo = Nothing to redo.
edit-not-a-table-size = Not a table size: { $text }. Type columns and rows, for example 3 by 2.
# $name is the image's file name.
edit-image-inserted = Inserted image { $name }. Its description is selected; type to replace it.
edit-image-failed = Could not insert the image: { $error } The text is unchanged.
# Edit mode's Obsidian aids: a wiki link, tasks, callouts.
edit-wikilink-title = Link to a note
edit-wikilink-intro =
    { $n ->
        [one] Link to a note, 1 note. Enter chooses it.
       *[other] Link to a note, { $n } notes. Enter chooses one.
    }
edit-wikilink-no-notes = There are no notes in this vault to link to.
# $note is the chosen note's title.
edit-wikilink-anchor-title = Link to part of { $note }
edit-wikilink-anchor-intro = Link to all of { $note }, or to a heading or block in it. Enter chooses.
edit-wikilink-whole = The whole note
edit-wikilink-heading = Heading { $level }: { $text }
edit-wikilink-block = Block { $id }: { $text }
# $target is the link as written, such as Cell#Parts.
edit-wikilink-inserted = Inserted a link to { $target }.
edit-callout-title = Callout type
edit-callout-intro = Callout types, { $n }. Enter inserts one.
# $type is the callout type as written, such as tip.
edit-callout-inserted = Inserted a { $type } callout. Type its text.
# The task's status on the current line after Toggle task or Cycle task
# status; $mark is the status character when it has no name.
edit-task-status =
    { $status ->
        [done] Checked.
        [open] Not checked.
        [in-progress] In progress.
        [cancelled] Canceled.
        [forwarded] Forwarded.
        [important] Important.
        [question] Question.
       *[other] Marked { $mark }.
    }
edit-task-new = New task, not checked.
edit-tasks-changed =
    { $n ->
        [one] Task changed on 1 line.
       *[other] Tasks changed on { $n } lines.
    }
# $query is the text to find.
common-no-matches = No matches for { $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] 1 match for { $query }. Replace with?
       *[other] { $n } matches for { $query }. Replace with?
    }

## Edit mode: autosave and recovering unsaved work.

common-recovery-write-failed = Could not write the recovery copy: { $error }. Save soon; { -brand } will keep trying.
common-recovery-writing-again = The recovery copy is being written again.
# $title is the document; $when is how long ago its work was saved.
edit-recovery-offer = Recover unsaved edits to { $title }? Saved { $when }. Up and Down choose, Enter confirms.
edit-recovery-title = Recover unsaved work in { $title }?
edit-recovery-yes = Yes, recover { $title } and keep editing
edit-recovery-no = No, discard the unsaved changes
edit-recovery-discarded = Discarded the unsaved changes to { $title }.
edit-recovered = Recovered unsaved work in { $title }. Remember to save.
edit-recovery-postponed = Recovery postponed. The unsaved work will be offered again next time.

## Saving in the background.

writes-still-saving = Still saving. Please wait.
writes-not-written-in-time = Some changes could not be written in time: the disk is not answering.
# $error is the system's reason.
writes-save-failed = Could not save: { $error }. Still editing.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-not-saved = Bookmark { $name } is set for now, but could not be saved: { $error } Check that the data folder can be written to.
# $name is the saved file's name.
writes-saved = Saved { $name }. Still editing.

## Files changed on disk. $name is a file name. Keep the letters y and
## n: they are the keys that answer.

disk-replace-question = { $name } already exists. Replace it? y or n
# A prompt label, also said with a full stop after it.
disk-not-replaced = Not replaced. Type another name
# $key is the key for Save As.
disk-not-saved = Not saved. Still editing. Save As, { $key }, keeps both versions.
disk-kept-open-version = Kept the open version.
disk-overwrite-question = { $name } changed on disk since you opened it. Save over those changes? y or n
disk-reload-question = { $name } changed on disk. Reload it? y or n

## Marks found again after a file changed outside textweaver.

relocate-reading-position = your reading position
relocate-bookmarks =
    { $n ->
        [one] 1 bookmark
       *[other] { $n } bookmarks
    }
relocate-notes =
    { $n ->
        [one] 1 note
       *[other] { $n } notes
    }
relocate-highlights =
    { $n ->
        [one] 1 highlight
       *[other] { $n } highlights
    }
# Lists of relocate-* items: "a and b", and "a, b, and c", where $rest is
# every item but the last, joined by commas.
relocate-join-two = { $a } and { $b }
relocate-join-more = { $rest }, and { $last }
# $items is a list of the items above; $n how many marks it counts in all.
relocate-moved =
    { $n ->
        [one] { $items } was moved to match
       *[other] { $items } were moved to match
    }
relocate-lost =
    { $n ->
        [one] { $items } could not be found and is marked
       *[other] { $items } could not be found and are marked
    }
# $clauses are relocate-moved and relocate-lost, joined by a comma.
relocate-changed = The file changed; { $clauses }.

## New documents from templates.

# The built-in templates' names.
templates-essay = Essay
templates-report = Report
templates-notes = Notes
# One of the user's own templates in the list; $name is its file name.
templates-yours = { $name }, your template
# $folder is where the user's own templates go.
templates-intro =
    { $n ->
        [one] New document from a template, 1 template. Enter chooses. Your own templates go in { $folder }.
       *[other] New document from a template, { $n } templates. Enter chooses. Your own templates go in { $folder }.
    }
# The title given when none is typed.
templates-untitled = Untitled
# $template is the template's name, $title the document's, $date today's date (2026-09-26).
templates-created = New document from the { $template } template: { $title }. Dated { $date }. The cursor is where the writing starts. Remember to save.
