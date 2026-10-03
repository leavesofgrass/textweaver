# Window screenshots for review

These pictures show the textweaver window (`textweaver-gui`) for a sighted design review. They are drawn without a window on screen, by Vello's CPU renderer, from the same widgets the window uses:

```text
textweaver-gui --review-screenshots FOLDER fixtures/sample.md
```

Every picture uses the sample document, `fixtures/sample.md`, and a fresh settings folder. Unless a description says otherwise, the window is 1100 by 780 pixels, and the reading position is on the word "Jones" in the first paragraph: the spoken word is bold on a solid band, and its sentence is underlined on a lighter band. File names give the theme, what is shown, the size when it is not the usual one, and the scale (100 or 200 percent).

The themes are Galaxy (the default, dark), Galaxy Light, High Contrast (black, white and yellow), and "system contrast", which is Windows High Contrast's own "Night sky" colors as the window follows them.

## The window while reading

![Galaxy theme at 100 percent: reading the sample document, the spoken word Jones bold, its sentence underlined](galaxy-100.png)

Description: dark purple-gray window. The header has the document's title and the buttons Open, Font, Start editing, Settings and Commands, each with its key. The document fills the middle, framed in purple because it has the focus. The reading toolbar below has Play (filled purple, the main action), Stop, Previous sentence, Next sentence, Slower and Faster. The status bar says "Opened Sample Markdown Document." on the left and "line 1 of 34, 0%, Ready, 265 wpm, silent" on the right.

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

## The window with no document

![No document open, Galaxy at 100 percent: empty document area and the title textweaver](galaxy-empty-100.png)

Description: the window before any document is open. The header's title is "textweaver", the document area is an empty purple frame, and the status bar says only "Ready, 265 wpm, silent" on the right.

![No document open, Galaxy at 200 percent: empty window twice the size](galaxy-empty-200.png)

Description: the empty window at 200 percent.

![No document open, High Contrast at 100 percent: empty window in white on black](high-contrast-empty-100.png)

Description: the empty window in High Contrast: black, with white borders around each region.

## Edit mode

![Edit mode in Galaxy at 100 percent: the Markdown source with the caret before the title](galaxy-edit-100.png)

Description: after Start editing, the document shows its Markdown source: the front matter between two lines of dashes, then "# Sample Markdown Document" with the caret after the number sign. Markup stays visible and styled (bold, italic, code and the link). The header's button now says "Finish editing (Ctrl+E)", and the status bar says "Edit mode on. Save: Ctrl+S. Finish: Ctrl+E." and "line 6 of 45, 7%, Edit".

![Edit mode in Galaxy at 200 percent: the Markdown source, twice the size](galaxy-edit-200.png)

Description: edit mode at 200 percent.

![Edit mode in Galaxy Light at 100 percent: the Markdown source in dark text on white](galaxy-light-edit-100.png)

Description: edit mode in Galaxy Light.

## The spoken sentence's underline

Every reading picture above shows it: the sentence being read is underlined in the sentence's text color and sits on a light band, and the spoken word is bold on a solid band. The underline is drawn in every theme, High Contrast and Night sky too, so the sentence never depends on its band color alone.

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

Description: a panel titled Contents sits left of the document. It lists "Sample Markdown Document, level 1" (marked current with a bar and a frame), "Lists, level 2", "A Table, level 2", "Quotes and Code, level 2" and "Deeper Heading, level 3". At its foot: "Enter goes there. Shift+Enter goes and returns. Escape returns." The document keeps the focus.

![Contents panel in Galaxy at 200 percent: the panel and the document, twice the size](galaxy-contents-200.png)

Description: the Contents panel at 200 percent.

![Contents panel in Galaxy Light at 100 percent: the headings list on a light panel](galaxy-light-contents-100.png)

Description: the Contents panel in Galaxy Light.

![Contents panel in High Contrast at 100 percent: white list on black with white borders](high-contrast-contents-100.png)

Description: the Contents panel in High Contrast, with the current heading framed in white.

![Contents panel in Night sky colors at 100 percent: white list on black, yellow document frame](system-contrast-contents-100.png)

Description: the Contents panel in Windows Night sky colors.

![Notes panel in Galaxy at 100 percent: two sample notes listed beside the document](galaxy-notes-100.png)

Description: a panel titled Notes lists two notes: "Check the totals in the score column" and "Ask about the source of this quote", each followed by its place, which the panel's width cuts off. The status bar shows the last note added.

![Notes panel in Galaxy at 200 percent: the two notes, twice the size](galaxy-notes-200.png)

Description: the Notes panel at 200 percent.

## Small windows

These show how the window fits small screens. Some show layout problems, listed under each.

![Window at 960 by 540, Galaxy at 100 percent: the reading view with the title cut short](galaxy-960x540-100.png)

Description: half of a 1920 by 1080 screen. Everything is reachable, but the header's title is cut to "Sample Mark", and the Faster button runs past the toolbar's right edge.

![Contents panel at 960 by 540, Galaxy at 100 percent: all five headings still fit](galaxy-contents-960x540-100.png)

Description: the Contents panel at 960 by 540. All five headings fit, and the document beside it wraps its first paragraph.

![Edit mode at 960 by 540, Galaxy at 100 percent: Markdown source in a short document area](galaxy-edit-960x540-100.png)

Description: edit mode at 960 by 540.

![Window at 683 by 384 and 200 percent, Galaxy: buttons overflow and cover the title](galaxy-683x384-200.png)

Description: a 1366 by 768 laptop screen at 200 percent. Problems: the header's buttons are wider than the window, so the Open button covers the title and Commands is off the right edge; the toolbar shows only Play, Stop, Previous sentence and Next sentence, with Slower and Faster off screen; the heading at the top of the document is cut through the middle.

![Contents panel at 683 by 384 and 200 percent: only two headings show](galaxy-contents-683x384-200.png)

Description: the Contents panel at the laptop size. The panel shows two headings above its hint, and the document beside it shows one paragraph.

![Edit mode at 683 by 384 and 200 percent: status bar texts overlap](galaxy-edit-683x384-200.png)

Description: edit mode at the laptop size. Problem: the status bar's message and position are drawn over each other.

![Window at 420 by 320, Galaxy at 100 percent: header, toolbar and status bar overflow](galaxy-420x320-100.png)

Description: the smallest review size. Problems: the header's buttons cover the title and run off the right edge, the toolbar shows only Play, Stop and part of Previous sentence, the status bar's two texts overlap, and the document area is about two lines tall.

![Contents panel at 420 by 320, Galaxy at 100 percent: the panel shows no headings](galaxy-contents-420x320-100.png)

Description: the Contents panel at 420 by 320. Problem: the panel's hint takes all its height, so no heading shows, and the document beside it is very narrow.

![No document at 420 by 320, Galaxy at 100 percent: empty window with overflowing buttons](galaxy-empty-420x320-100.png)

Description: the empty window at 420 by 320, with the same header and toolbar overflow.

## Dialogs

![Bookmarks list dialog in Galaxy at 100 percent: five bookmarks, the second selected](galaxy-dialog-100.png)

Description: a dialog titled Bookmarks over the dimmed window, with five bookmarks, each a name and a line; "Lists, line 9" is selected. Under the list: "Enter chooses, Escape closes."

![Bookmarks list dialog in Galaxy at 200 percent: the list dialog, twice the size](galaxy-dialog-200.png)

Description: the Bookmarks dialog at 200 percent.

![Settings dialog in Galaxy at 100 percent: the speech section, on the speech rate](galaxy-settings-100.png)

Description: the Settings dialog over the dimmed window. The sections are listed on the left with their counts ("Speech, 30 settings" first). On the right, the Speech settings, with Rate selected at "265 words per minute" between its two arrows; Split capitals shows its switch with "off" in words. The selected setting's description, "How fast textweaver speaks.", is under the list.

![Settings dialog in Galaxy at 200 percent: the settings, twice the size](galaxy-settings-200.png)

Description: the Settings dialog at 200 percent.

![Settings dialog in Galaxy Light at 100 percent: the settings on a light dialog](galaxy-light-settings-100.png)

Description: the Settings dialog in Galaxy Light.

![Settings dialog in High Contrast at 100 percent: the settings in white on black](high-contrast-settings-100.png)

Description: the Settings dialog in High Contrast.

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
