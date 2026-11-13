### Markdown lint, grammar, and spelling.

## Markdown lint (edit mode). A problem is said after "Lint: ".

lint-heading-level = heading level { $level } after level { $prev }; use level { $use }.
# $reference is the link reference's name.
lint-link-reference = link reference { $reference } has no definition.
lint-bare-url = bare web address; put it in angle brackets or make it a link with a name.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = list marker { $marker }; this list uses { $used }.
lint-marker-dash = dash
lint-marker-star = star
lint-marker-plus = plus
lint-marker-other = other
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = tabs or spaces on an empty line.
lint-trailing-tabs-line-end = tabs or spaces at the end of the line.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] 1 space on an empty line.
       *[other] { $n } spaces on an empty line.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] 1 space at the end of the line.
       *[other] { $n } spaces at the end of the line.
    }
# $key turns on edit mode.
lint-not-editing = Lint checks the Markdown you write. Turn on edit mode with { $key } first.
lint-not-markdown = Lint checks Markdown, and this document is not Markdown.
lint-none = No lint problems.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] No more lint problem. 1 lint problem in all.
       *[other] No more lint problem. { $count } lint problems in all.
    }
lint-no-earlier =
    { $n ->
        [one] No earlier lint problem. 1 lint problem in all.
       *[other] No earlier lint problem. { $count } lint problems in all.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
common-line = Line { $line }.

## Grammar checking (Harper). $message is Harper's own message, in English.

# $words are the words the problem is about.
grammar-said = Grammar: { $message } The words: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = Fix: remove them.
grammar-fix = Fix: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = Remove the words
grammar-none = No grammar problems found.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] No more grammar problem. 1 grammar problem in all.
       *[other] No more grammar problem. { $count } grammar problems in all.
    }
grammar-no-earlier =
    { $n ->
        [one] No earlier grammar problem. 1 grammar problem in all.
       *[other] No earlier grammar problem. { $count } grammar problems in all.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } lists fixes.
# Added at high verbosity.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } No fix to offer.
grammar-fixes =
    { $n ->
        [one] { $words }: 1 fix.
       *[other] { $words }: { $n } fixes.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: 1 fix. Enter makes the change.
       *[other] { $words }: { $n } fixes. Enter makes the change.
    }
common-left-as-is = Left as it is.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Turn on edit mode with { $key } to change the text.
grammar-removed = Removed.
grammar-changed = Changed to { $fix }.
grammar-change-failed = Could not change the text: { $error } Nothing was changed.

## Spell checking.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = apostrophe
spell-not-available = Spell checking is not available: this version has no word list.
spell-none-found = No misspellings found.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] No more misspelling. 1 possible misspelling in all.
       *[other] No more misspelling. { $count } possible misspellings in all.
    }
spell-no-earlier =
    { $n ->
        [one] No earlier misspelling. 1 possible misspelling in all.
       *[other] No earlier misspelling. { $count } possible misspellings in all.
    }
# Added at high verbosity.
spell-no-misspelled-word = No misspelled word at the cursor.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: no suggestions.
        [one] { $word }: 1 suggestion.
       *[other] { $word }: { $n } suggestions.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: no suggestions. Enter replaces the word.
        [one] { $word }: 1 suggestion. Enter replaces the word.
       *[other] { $word }: { $n } suggestions. Enter replaces the word.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. Turn on edit mode with { $key } to change the text.
spell-replaced = Replaced with { $word }.
spell-replace-failed = Could not replace: { $error } The word is unchanged.
spell-added-for-session = Added { $word } to your word list for this session.
spell-added = Added { $word } to your word list.
spell-save-failed = Could not save your word list: { $error } The word is known until you quit.
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] No misspellings.
        [one] 1 possible misspelling.
       *[other] { $count } possible misspellings.
    }
