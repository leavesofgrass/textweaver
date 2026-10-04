# Window screenshots for review

These pictures show the textweaver window (`textweaver-gui`) for a sighted design review. They are drawn without a window on screen, by Vello's CPU renderer, from the same widgets the window uses:

```text
textweaver-gui --review-screenshots FOLDER fixtures/sample.md
```

Every picture uses the sample document, `fixtures/sample.md`, and a fresh settings folder. Unless a description says otherwise, the window is 1100 by 780 pixels. The harness puts the reading position on the word "Jones" in the first paragraph: the spoken word has its band, and its sentence ("Dr. Jones arrived at 3:30 p.m.") a paler band and an underline. For the pictures, `[highlight] granularity` is "both"; the default, "word", draws the word only. File names give the theme, what is shown, the size when it is not the usual one, and the scale (100 or 200 percent).

The themes are Galaxy (the default, dark), Galaxy Light, High Contrast (black, white and yellow), Lamplight (the soft dark theme, warm brown and amber), and "system contrast", which is Windows High Contrast's own "Night sky" colors as the window follows them.

## What changed since alpha.8

All pictures were redrawn on Sunday, October 4, 2026, after Wave 9's last visual change.

- The header no longer shows the document's title; it holds only the buttons Open, Font, Start editing, Settings and Commands, each with its key.
- The document sits in a centered column of about 66 characters, with real list markers: bullets, and numbers for numbered lists.
- Edit mode draws the document's frame dashed and shows an "Editing" badge in its top right corner.
- Below 800 pixels wide, the header and the toolbar fold into one bar above the document, and the buttons hide their keys. The overflow that alpha.8's small pictures showed is gone.
- The status bar's two texts go on two lines when they do not fit on one, instead of overlapping, and the position adds the time left ("under a minute left").
- The window with no document says so in the document area: "No document is open. Press Ctrl+O to open one."
- Redrawn again on Sunday, October 4, 2026, after the visual fixes: the reading position's highlight shows again; bullets are drawn as shapes (a disc, a ring for the nested one, a square deeper); edit mode shows the source's own dashes with no drawn bullet; at 420 by 320 the bar keeps to two rows and the Contents panel gives way to the document; at 960 by 540 Slower and Faster wrap together.
- New pictures: the Reading settings dialog, Settings with a filter typed, Lamplight, and the window at 780 by 540 with the bars folded.

## Problems still visible

- Pass: no button runs past the window's edge at any review size.
- Pass: the spoken word's band and its sentence's underline show in every theme.
- Pass: bullets are shapes, so there is no missing-glyph box, and edit mode draws no bullets.
- Pass: at 420 by 320 the bar takes two rows and the document about four lines.
- Minor: at 420 by 320 some buttons are hidden (Start editing, Settings, Next sentence, Slower and Faster). Their keys work, and Commands lists them all.
- Minor: at 683 by 384 with the Contents panel, the document's title wraps and only its second line, "Document", shows at the top, because the view scrolls to the reading position.
- Minor: the Notes picture's status message is the last sample note added ("Note added on: A block quote spans a sentence.").

## The window while reading

![Galaxy at 100 percent: the sample document's title, first paragraph and bullet list, Play highlighted](galaxy-100.png)

Description: dark gray window. The header has the buttons Open, Font, Start editing, Settings and Commands, each with its key. The document fills the middle in a centered column, framed in purple because it has the focus: the purple title "Sample Markdown Document", a paragraph with bold, italic, code and a link, the heading "Lists", three bullets with a nested one, and a numbered list. The reading toolbar below has Play (filled purple, the main action), Stop, Previous sentence, Next sentence, Slower and Faster. The word "Jones" has a solid lilac band, and its sentence a paler purple band with a line under it. The status bar says "Opened Sample Markdown Document." on the left and "Line 3 of 34, 18%, Ready, 265 wpm, silent, under a minute left" on the right.

![Galaxy theme at 200 percent: the same reading view, twice the size](galaxy-200.png)

Description: the same view as above at 200 percent scale, so less of the document fits.

![Galaxy Light theme at 100 percent: reading view in dark text on a white page](galaxy-light-100.png)

Description: the reading view in Galaxy Light: near-white panels, dark text, a purple frame on the document, and Play in solid purple with white text.

![Galaxy Light theme at 200 percent: the light reading view, twice the size](galaxy-light-200.png)

Description: the Galaxy Light reading view at 200 percent.

![High Contrast theme at 100 percent: white text on black, yellow headings and spoken word](high-contrast-100.png)

Description: black background with white text and white button borders. Headings are yellow and cyan, the spoken word Jones is black on yellow, and Play is solid yellow.

![High Contrast theme at 200 percent: the high contrast reading view, twice the size](high-contrast-200.png)

Description: the High Contrast reading view at 200 percent.

![Windows Night sky contrast colors at 100 percent: white on black, links cyan, spoken word yellow](system-contrast-100.png)

Description: the window in Windows High Contrast's Night sky colors: black background, white text and borders, cyan links, the spoken word black on yellow, and a yellow frame on the document.

![Windows Night sky contrast colors at 200 percent: the same view, twice the size](system-contrast-200.png)

Description: the Night sky view at 200 percent.

![Lamplight theme at 100 percent: the reading view in warm cream text on dark brown, amber headings](lamplight-100.png)

Description: the reading view in Lamplight, the soft dark theme: dark brown panels, cream text, amber headings and an amber Play button, light blue links, and a pale blue frame on the document.

![Lamplight theme at 200 percent: the Lamplight reading view, twice the size](lamplight-200.png)

Description: the Lamplight reading view at 200 percent.

## The window with no document

![No document open, Galaxy at 100 percent: the hint No document is open, press Ctrl+O to open one](galaxy-empty-100.png)

Description: the window before any document is open. The document area is an empty purple frame whose first line says "No document is open. Press Ctrl+O to open one." The status bar says only "Ready, 265 wpm, silent" on the right.

![No document open, Galaxy at 200 percent: empty window twice the size](galaxy-empty-200.png)

Description: the empty window at 200 percent.

![No document open, High Contrast at 100 percent: empty window in white on black](high-contrast-empty-100.png)

Description: the empty window in High Contrast: black, with white borders around each region.

## Edit mode

![Edit mode in Galaxy at 100 percent: Markdown source in a dashed frame with an Editing badge](galaxy-edit-100.png)

Description: after Start editing, the document's frame turns dashed and an "Editing" badge sits in its top right corner. The document shows its Markdown source: the front matter between two lines of dashes, then "# Sample Markdown Document" with the caret after the number sign. Markup stays visible and styled (bold, italic, code and the link), and list lines start with their own dashes, with no drawn bullet. The header's button now says "Finish editing (Ctrl+E)", and the status bar says "Edit mode on. Save: Ctrl+S. Finish: Ctrl+E. # Sample Markdown Document" and "Line 6 of 45, Edit, Ready, 7%".

![Edit mode in Galaxy at 200 percent: the Markdown source, twice the size](galaxy-edit-200.png)

Description: edit mode at 200 percent.

![Edit mode in Galaxy Light at 100 percent: the Markdown source in dark text on white](galaxy-light-edit-100.png)

Description: edit mode in Galaxy Light.

![Edit mode in Lamplight at 100 percent: the Markdown source on dark brown, with the Editing badge](lamplight-edit-100.png)

Description: edit mode in Lamplight, with the dashed frame and the "Editing" badge.

## The spoken sentence's underline

By design, the sentence being read is underlined in the sentence's text color and sits on a light band, and the spoken word is bold on a solid band, in every theme, so the sentence never depends on its band color alone. The Wave 9 pictures do not show it; see "Problems still visible".

## The reading ruler

![Reading ruler in Galaxy at 100 percent: a band over the first paragraph with a bar at its left edge](galaxy-ruler-100.png)

Description: only the reading ruler is on. A light purple band covers the first paragraph's two lines and the blank line after it; a purple bar runs down its left edge, thicker on the reading line and thinner on the band's other rows. The band stops before the "Lists" heading.

![Reading ruler in Galaxy at 200 percent: the band and its bar, twice the size](galaxy-ruler-200.png)

Description: the reading ruler at 200 percent.

![Reading ruler in Galaxy Light at 100 percent: a pale lilac band with a purple bar](galaxy-light-ruler-100.png)

Description: the reading ruler in Galaxy Light: a pale lilac band and a purple bar on a white page.

![Reading ruler in High Contrast at 100 percent: a dark blue band with a light blue bar](high-contrast-ruler-100.png)

Description: the reading ruler in High Contrast: a dark blue-gray band on black, with a light blue bar at its left edge.

## The Contents and Notes panels

![Contents panel in Galaxy at 100 percent: the five headings beside the document, the first one current](galaxy-contents-100.png)

Description: a panel titled Contents sits left of the document. It lists "Sample Markdown Document, level 1" (marked current with a bar and a frame, its text cut short with an ellipsis), "Lists, level 2", "A Table, level 2", "Quotes and Code, level 2" and "Deeper Heading, level 3". At its foot: "Enter goes there. Shift+Enter goes and returns. Escape returns." The document keeps the focus.

![Contents panel in Galaxy at 200 percent: the panel and the document, twice the size](galaxy-contents-200.png)

Description: the Contents panel at 200 percent.

![Contents panel in Galaxy Light at 100 percent: the headings list on a light panel](galaxy-light-contents-100.png)

Description: the Contents panel in Galaxy Light.

![Contents panel in High Contrast at 100 percent: white list on black with white borders](high-contrast-contents-100.png)

Description: the Contents panel in High Contrast, with the current heading framed in white.

![Contents panel in Night sky colors at 100 percent: white list on black, yellow document frame](system-contrast-contents-100.png)

Description: the Contents panel in Windows Night sky colors.

![Notes panel in Galaxy at 100 percent: two sample notes listed beside the document](galaxy-notes-100.png)

Description: a panel titled Notes lists two notes: "Check the totals in the score column" and "Ask about the source of this quote", each cut short with an ellipsis by the panel's width; the first is selected. The panel's foot has the same hint as Contents. The status bar says "Note added on: A block quote spans a sentence." and "Line 3 of 34, 18%", where the reading position is.

![Notes panel in Galaxy at 200 percent: the two notes, twice the size](galaxy-notes-200.png)

Description: the Notes panel at 200 percent.

## Small windows

These show how the window fits small screens. Below 800 pixels wide, the header and the toolbar fold into one bar above the document, and the buttons hide their keys. Problems are listed under each picture and in "Problems still visible".

![Window at 960 by 540, Galaxy at 100 percent: the reading view, Slower and Faster on the toolbar's second row](galaxy-960x540-100.png)

Description: half of a 1920 by 1080 screen. The header and the toolbar are still separate and every button keeps its key; the toolbar wraps, and Slower and Faster go to its second row together. The document shows its title, first paragraph and the "Lists" heading.

![Contents panel at 960 by 540, Galaxy at 100 percent: all five headings still fit](galaxy-contents-960x540-100.png)

Description: the Contents panel at 960 by 540. All five headings fit, and the document beside it wraps its first paragraph.

![Edit mode at 960 by 540, Galaxy at 100 percent: Markdown source in a short document area](galaxy-edit-960x540-100.png)

Description: edit mode at 960 by 540, with the dashed frame and the "Editing" badge.

![Window at 780 by 540, Galaxy at 100 percent: the folded bar, two rows of buttons above the document](galaxy-780x540-100.png)

Description: just under the folding width. One bar above the document holds two rows of buttons without their keys: Open, Font, Start editing, Settings and Commands, then Play, Stop, Previous sentence and Next sentence, with Slower and Faster at the right. The document shows the title, first paragraph and the start of the list, and the status bar fits on one line.

![Edit mode at 780 by 540, Galaxy at 100 percent: the folded bar over the Markdown source](galaxy-edit-780x540-100.png)

Description: edit mode at 780 by 540, with the folded bar, the dashed frame and the "Editing" badge.

![Window at 683 by 384 and 200 percent, Galaxy: the folded bar in two rows, every button on screen](galaxy-683x384-200.png)

Description: a 1366 by 768 laptop screen at 200 percent. The folded bar shows every button in two rows. The document shows the title, the first paragraph and the "Lists" heading. The status bar's message and position sit on two lines.

![Contents panel at 683 by 384 and 200 percent: four headings, the document title cut to its second line](galaxy-contents-683x384-200.png)

Description: the Contents panel at the laptop size shows four headings above its hint. Problem: the document's title wraps, and only its second line, "Document", shows at the top.

![Edit mode at 683 by 384 and 200 percent: the source with the Editing badge, status on two lines](galaxy-edit-683x384-200.png)

Description: edit mode at the laptop size. The document shows the front matter's author line and the title with the caret; the status bar's two texts sit on two lines.

![Window at 420 by 320, Galaxy at 100 percent: two rows of buttons, about four lines of document](galaxy-420x320-100.png)

Description: the smallest review size. Below 480 pixels high, the folded bar keeps to two rows: Open, Font and Commands, then Play, Stop and Previous sentence. The other buttons are hidden from the screen, the Tab order and the screen reader; their keys still work, and Commands lists every command. The document shows about four lines, with "Jones" highlighted, and the status bar takes three lines.

![Contents panel at 420 by 320, Galaxy at 100 percent: the panel gives way, the document as without it](galaxy-contents-420x320-100.png)

Description: the Contents panel is open, but at 420 by 320 the document would be under five lines with it, so the panel is hidden and the window looks as without it. The panel's key shows it again and moves the focus there; while it has the focus, it stays above the document.

![No document at 420 by 320, Galaxy at 100 percent: two rows of buttons and the open-a-document hint](galaxy-empty-420x320-100.png)

Description: the empty window at 420 by 320: two rows of buttons, then "No document is open. Press Ctrl+O to open one." on two lines, and "Ready, 265 wpm, silent" in the status bar.

## Dialogs

![Bookmarks list dialog in Galaxy at 100 percent: five bookmarks, the second selected](galaxy-dialog-100.png)

Description: a dialog titled Bookmarks over the dimmed window, with five bookmarks, each a name and a line; "Lists, line 9" is selected. Under the list: "Enter chooses, Escape closes."

![Bookmarks list dialog in Galaxy at 200 percent: the list dialog, twice the size](galaxy-dialog-200.png)

Description: the Bookmarks dialog at 200 percent.

![Settings dialog in Galaxy at 100 percent: the speech section, on the speech rate](galaxy-settings-100.png)

Description: the Settings dialog over the dimmed window. The sections are listed on the left with their counts ("Speech, 31 settings" first). On the right, the Speech settings, from Speech engine ("automatic") down, with Rate selected at "265 words per minute" between its two arrows; Split capitals shows its switch with "off" in words. The selected setting's description, "How fast textweaver speaks.", is under the list.

![Settings dialog in Galaxy at 200 percent: the settings, twice the size](galaxy-settings-200.png)

Description: the Settings dialog at 200 percent.

![Settings dialog in Galaxy Light at 100 percent: the settings on a light dialog](galaxy-light-settings-100.png)

Description: the Settings dialog in Galaxy Light.

![Settings dialog in High Contrast at 100 percent: the settings in white on black](high-contrast-settings-100.png)

Description: the Settings dialog in High Contrast.

![Settings dialog in Lamplight at 100 percent: the speech settings on a dark brown dialog](lamplight-settings-100.png)

Description: the Settings dialog in Lamplight, with Rate selected and framed in pale blue.

![Settings with the filter voice typed, Galaxy at 100 percent: Matching voice, 16 settings, first](galaxy-filter-100.png)

Description: the Settings dialog with "voice" typed as a filter. The first section is now "Matching voice, 16 settings", selected, and the other sections follow. On the right: Pitch (selected, "0 semitones"), Voice, Preferred voice, Favorite voices, Voices by language, OneCore voices ("on" with its switch), Piper voices folder, Piper voice, and Rate and pitch per voice. The filter's text itself is not drawn; the section's name says what was typed.

![Reading settings dialog in Galaxy at 100 percent: nine reading settings and three buttons](galaxy-reading-100.png)

Description: a dialog titled "Reading settings" with one list: Rate (selected, "265 words per minute"), Font ("sans serif"), Font size ("14 points"), Font weight ("400"), Line height ("1.5"), Paragraph spacing ("1"), Word spacing ("0"), Letter spacing ("0") and Line length ("66 characters"), each between two arrows. Under the list, the selected setting's description, "How fast textweaver speaks." Then the buttons Voices, WCAG spacing and Generous spacing, the line "Changes take effect and are saved at once.", and "Close (Escape)".

![Reading settings dialog in Galaxy at 200 percent: the reading settings, twice the size](galaxy-reading-200.png)

Description: the Reading settings dialog at 200 percent.

![Reading settings dialog in High Contrast at 100 percent: the reading settings in white on black](high-contrast-reading-100.png)

Description: the Reading settings dialog in High Contrast.

![Colors dialog in Galaxy at 100 percent: chosen colors with samples and contrast](galaxy-colors-100.png)

Description: the Colors dialog, one row per color, each with a sample square and its contrast in words. The word highlight is "blue, contrast 6.1 to 1, good", the sentence highlight "orange, contrast 8.4 to 1, good", and difficult words "dark blue, contrast 1 to 1, low", the warning for a color that is hard to see on the dark page. The other rows say "the theme's color". The dialog ends with "Reset all colors" and "Close (Escape)".

![Colors dialog in Galaxy at 200 percent: the colors, twice the size](galaxy-colors-200.png)

Description: the Colors dialog at 200 percent.

![Colors dialog in Galaxy Light at 100 percent: the colors on a light dialog](galaxy-light-colors-100.png)

Description: the Colors dialog in Galaxy Light.

![Colors dialog in High Contrast at 100 percent: the colors in white on black](high-contrast-colors-100.png)

Description: the Colors dialog in High Contrast.

![Voice manager in Galaxy at 100 percent: seven sample voices of five engines](galaxy-voices-100.png)

Description: a dialog titled "Choose a voice", with Language and Engine filters, a list of seven sample voices (Eloquence, SAPI 5, Piper, DECtalk and eSpeak NG, one not on this computer), the second selected, and the buttons Use voice, Preview, Favorite, Remove and Close, each with its key.

![Voice manager in Galaxy at 200 percent: the voices, twice the size](galaxy-voices-200.png)

Description: the voice manager at 200 percent.

![Voice manager in High Contrast at 100 percent: the voices in white on black](high-contrast-voices-100.png)

Description: the voice manager in High Contrast.

## Reading aids

![All reading aids in Galaxy at 100 percent: bionic reading, difficult words, the ruler, wider spacing and RSVP](galaxy-aids-100.png)

Description: every reading aid on: the start of each word in bold (bionic reading), syllables split by raised dots, difficult words marked, the reading ruler's band and bar, and wider line, letter and word spacing. An RSVP strip under the document shows one word, "Sample", large, with its focus letter underlined, and the next word, "Markdown", small and dim. The status bar says "RSVP on. Word 1 of 109" and the RSVP rate.

![All reading aids in Galaxy at 200 percent: the aids, twice the size](galaxy-aids-200.png)

Description: every reading aid at 200 percent.

![All reading aids in Galaxy Light at 100 percent: the aids on a white page](galaxy-light-aids-100.png)

Description: every reading aid in Galaxy Light.

![All reading aids in High Contrast at 100 percent: the aids in white on black](high-contrast-aids-100.png)

Description: every reading aid in High Contrast.

![All reading aids in Night sky colors at 100 percent: the aids in Windows contrast colors](system-contrast-aids-100.png)

Description: every reading aid in Windows Night sky colors.

## See also

- [The textweaver window](../../gui.md)
- [Reading aids](../../reading-aids.md)
- [Themes](../../themes.md)
