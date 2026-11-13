### Tracked changes and comments in Word files.

## B1-t1: tracked changes and comments (the changes list). A row puts the
## kind first; $text is the changed text, $who is changes-by or
## changes-no-author, $when is changes-date or changes-no-date.
prompt-comment-reply = Reply
prompt-comment-text = Comment
prompt-github-token = GitHub token for your components source, Enter skips
prompt-import-cards = Import study cards from Anki package
prompt-cw-copy = Type what you heard
changes-title = Changes and comments
# Said when the list opens; $title is changes-title, $n how many rows.
changes-intro =
    { $n ->
        [one] { $title }, { $n } item. Enter goes to it. A accepts a change, R rejects it, and Shift with either does every change by its author. On a comment, F2 replies, Space resolves, Delete deletes. N adds a comment.
       *[other] { $title }, { $n } items. Enter goes to one. A accepts a change, R rejects it, and Shift with either does every change by its author. On a comment, F2 replies, Space resolves, Delete deletes. N adds a comment.
    }
changes-none = No tracked changes or comments in this document.
changes-row = { $kind }: '{ $text }', { $who }, { $when }
changes-kind-inserted = Inserted
changes-kind-deleted = Deleted
changes-kind-moved-away = Moved away
changes-kind-moved-here = Moved here
changes-by = by { $author }
changes-no-author = author not recorded
changes-no-date = date not recorded
# A date from the document, said in full: "Tuesday, March 3, 2026".
changes-date = { $weekday }, { $month } { $day }, { $year }
changes-weekday-0 = Sunday
changes-weekday-1 = Monday
changes-weekday-2 = Tuesday
changes-weekday-3 = Wednesday
changes-weekday-4 = Thursday
changes-weekday-5 = Friday
changes-weekday-6 = Saturday
changes-month-1 = January
changes-month-2 = February
changes-month-3 = March
changes-month-4 = April
changes-month-5 = May
changes-month-6 = June
changes-month-7 = July
changes-month-8 = August
changes-month-9 = September
changes-month-10 = October
changes-month-11 = November
changes-month-12 = December
changes-comment = Comment: { $text }
changes-comment-by = Comment by { $author }: { $text }
changes-replies =
    { $n ->
        [one] { $n } reply
       *[other] { $n } replies
    }
changes-resolved = resolved
# $kind is one of the changes-kind-* words.
changes-accepted = Accepted. { $kind }: '{ $text }'.
changes-rejected = Rejected. { $kind }: '{ $text }'.
changes-accepted-all =
    { $n ->
        [one] Accepted { $n } change.
       *[other] Accepted { $n } changes.
    }
changes-rejected-all =
    { $n ->
        [one] Rejected { $n } change.
       *[other] Rejected { $n } changes.
    }
changes-accepted-author =
    { $n ->
        [one] Accepted { $n } change by { $author }.
       *[other] Accepted { $n } changes by { $author }.
    }
changes-rejected-author =
    { $n ->
        [one] Rejected { $n } change by { $author }.
       *[other] Rejected { $n } changes by { $author }.
    }
changes-none-left = No tracked changes to accept or reject.
changes-not-a-change = This row is a comment. F2 replies, Space resolves, Delete deletes.
changes-not-a-comment = This row is a change. A accepts it, R rejects it.
changes-edit-mode = Changes stay as they are in edit mode. Leave edit mode to accept or reject them.
changes-replied = Reply added.
changes-resolved-done = Comment resolved.
changes-reopened = Comment open again.
changes-comment-deleted = Comment and its replies deleted.
changes-comment-added = Comment added.
changes-delete-comment-question = Delete this comment and its replies? y or n
# tw changes --out: $path is the file written, $n how many changes.
changes-written-accepted =
    { $n ->
        [one] Wrote { $path } with { $n } change accepted.
       *[other] Wrote { $path } with { $n } changes accepted.
    }
changes-written-rejected =
    { $n ->
        [one] Wrote { $path } with { $n } change rejected.
       *[other] Wrote { $path } with { $n } changes rejected.
    }
## End of B1-t1

## B1-t2: saving the review into the Word file. $file is the Word file's
## name, $backup the copy of the original kept beside it.
# Asked before Accept all changes; $n is how many changes.
changes-accept-all-question =
    { $n ->
        [one] Accept the 1 change? y or n
       *[other] Accept all { $n } changes? y or n
    }
changes-reject-all-question =
    { $n ->
        [one] Reject the 1 change? y or n
       *[other] Reject all { $n } changes? y or n
    }
changes-save-not-word = Not a Word file. Changes are saved back only into .docx files; export writes the decided text in other formats.
changes-save-nothing = Nothing to save: no change was accepted or rejected, and no comment changed.
changes-saved-backup = Saved the changes in { $file }. The original is kept as { $backup }.
changes-saved = Saved the changes in { $file }.
changes-save-unplaced =
    { $n ->
        [one] 1 new comment is at the start: its text was not found.
       *[other] { $n } new comments are at the start: their text was not found.
    }
changes-save-failed = Could not save the changes in { $file }: { $error }. The file is as it was; close it in Word if it is open, then try again.
changes-save-not-in-build = Saving changes to the Word file is not in this version of { -brand }. tw changes --in-place still does it.
# tw changes --in-place: $path is the Word file, $backup the copy of the
# original, $n how many changes.
changes-in-place-accepted =
    { $n ->
        [one] Accepted 1 change in { $path }. The original is kept as { $backup }.
       *[other] Accepted { $n } changes in { $path }. The original is kept as { $backup }.
    }
changes-in-place-rejected =
    { $n ->
        [one] Rejected 1 change in { $path }. The original is kept as { $backup }.
       *[other] Rejected { $n } changes in { $path }. The original is kept as { $backup }.
    }

## B1-t3: edits saved into a Word file as tracked changes.
changes-tracked-saved =
    { $n ->
        [one] Saved 1 tracked change in { $file }.
       *[other] Saved { $n } tracked changes in { $file }.
    }
changes-original-kept = The original is kept as { $backup }.
changes-tracked-refused =
    { $n ->
        [one] Not saved: 1 change in { $file } crosses a paragraph or sits in a link or field, so it cannot be tracked. Save As keeps your edits as Markdown.
       *[other] Not saved: { $n } changes in { $file } cross a paragraph or sit in a link or field, so they cannot be tracked. Save As keeps your edits as Markdown.
    }
## End of B1-t2
