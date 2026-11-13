### Voices, the voice manager, and restarting speech.

## Restarting speech.

restart-silent-now = { -brand } is silent now; restart it to hear speech again.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = { -brand } is silent now. Restart speech with { $keys }.
restart-restarting = Restarting speech.
restart-not-here = Speech cannot be restarted here.
restart-already = Speech is already restarting.
# $error is the system's reason, in its own words.
restart-failed = Could not restart speech: { $error } Wait a moment, then try again.
restart-start-failed = Could not restart speech: starting it failed.
restart-no-engine = No speech engine available; staying silent. Troubleshooting, No speech at all, has the checks.
restart-done-silent = Speech restarted, but silent: no engine is available.
restart-done = Speech restarted.

## The voice manager's list.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] { $n } voice: { $language }, { $engine }.
       *[other] { $n } voices: { $language }, { $engine }.
    }
voices-all-languages = all languages
voices-all-engines = all engines
# The filter rows at the top of the list.
voices-language-row = Language: { $language }
voices-engine-row = Engine: { $engine }
voices-fetch-row = Fetch the Piper voice list from the internet
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = download { $size }
voices-licence-public-domain = public domain
voices-licence-attribution = free with credit
voices-licence-share-alike = free with credit, share alike
voices-licence-non-commercial = non-commercial
voices-licence-unknown = license shown before download
voices-favourite = favorite
voices-current = current

## Language names in the voice manager's language filter.

voices-language-ar = Arabic
voices-language-ca = Catalan
voices-language-cs = Czech
voices-language-cy = Welsh
voices-language-da = Danish
voices-language-de = German
voices-language-el = Greek
voices-language-en = English
voices-language-es = Spanish
voices-language-fa = Persian
voices-language-fi = Finnish
voices-language-fr = French
voices-language-hi = Hindi
voices-language-hu = Hungarian
voices-language-is = Icelandic
voices-language-it = Italian
voices-language-ja = Japanese
voices-language-ka = Georgian
voices-language-kk = Kazakh
voices-language-ko = Korean
voices-language-lb = Luxembourgish
voices-language-lv = Latvian
voices-language-nl = Dutch
voices-language-no = Norwegian
voices-language-pl = Polish
voices-language-pt = Portuguese
voices-language-ro = Romanian
voices-language-ru = Russian
voices-language-sk = Slovak
voices-language-sl = Slovenian
voices-language-sr = Serbian
voices-language-sv = Swedish
voices-language-sw = Swahili
voices-language-tr = Turkish
voices-language-uk = Ukrainian
voices-language-vi = Vietnamese
voices-language-zh = Chinese

## Voices: the voice manager, rate, pitch, and volume.

# Spoken by a newly chosen voice as its sample.
voice-sample = The quick brown fox jumps over the lazy dog.
voice-list-title = Choose a voice
voice-still-loading = The voices are still loading. The list opens when they are ready.
voice-list-failed = Could not list the voices: { $error } Choose another engine in the Speech menu.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Voice manager. { $shown } Enter uses a voice and speaks a sample, or downloads one; { $preview } previews a voice; Space marks a favorite; Delete removes a downloaded voice; Escape closes.
voice-more-ready =
    { $n ->
        [one] One more voice from another engine is listed.
       *[other] { $n } more voices from other engines are listed.
    }
voice-preview = Preview: { $voice }.
voice-preview-sample = { $voice }. The quick brown fox jumps over the lazy dog.
voice-preview-starting = Preview: { $voice }, starting { $engine }.
voice-preview-not-installed = { $voice } is not downloaded yet. Enter downloads it, after a question.
voice-preview-unavailable = { $engine } cannot start here for a preview. Enter switches to it.
voice-preview-engine-failed = Could not start { $engine } to preview { $voice }.
voice-preview-failed = Could not preview { $voice }: { $error } Try another voice.
# $keys names the Choose Voice key.
voice-ready = The voices are ready. { $keys } lists them.
voice-fetch-catalog-question = Download the list of Piper voices, about 250 kilobytes, from Hugging Face? y or n
voice-fetch-catalog-question-short = Download the list of Piper voices? y or n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = Voice { $voice }, on { $engine }. Switching engine.
voice-download-in-progress = A voice download is already in progress.
voice-no-data-folder = There is no data folder to keep Piper voices in.
voice-not-in-list = That voice is not in the Piper voice list any more.
voice-download-start-failed = Could not start the download.
voice-reading-licence = Reading the license of { $voice }.
voice-remove-question = Remove the voice { $voice }? y or n
voice-only-piper-removable = Only downloaded Piper voices can be removed.
# $plan describes the download: the voice, its size and license.
voice-download-question = { $plan } y or n
voice-in-use = { $voice } is the voice in use. Choose another voice first.
voice-removed = { $voice } removed.
voice-remove-failed = Could not remove { $voice }: { $error } Check that its folder can be written to.
voice-downloading-catalog = Downloading the Piper voice list.
voice-downloading = Downloading { $voice }.
voice-downloading-percent = Downloading { $voice }, { $pct } percent.
voice-details-failed = Could not read the voice's details: { $error } Check the connection, then try again.
voice-download-stopped = The download stopped.
voice-catalog-fetched = The Piper voice list has { $voices } voices in { $languages } languages. The Voices command lists them.
voice-catalog-failed = Could not download the voice list: { $error } Check the connection, then try again.
# $licence describes the voice's license, in a sentence of its own.
voice-installed = { $voice } is installed. { $licence } The Voices command lists it.
voice-download-failed = Could not download { $voice }: { $error } Choose the voice again to retry.
voice-only-voice-favourite = Only a voice can be a favorite.
voice-favourite-added = { $voice } added to favorites.
voice-favourite-removed = { $voice } removed from favorites.
voice-chosen = Voice { $voice }.
voice-chosen-rate = Voice { $voice }, { $wpm } words per minute.
voice-fastest-rate = Fastest rate.
voice-slowest-rate = Slowest rate.
voice-rate = { $wpm } words per minute.
voice-highest-pitch = Highest pitch.
voice-lowest-pitch = Lowest pitch.
voice-pitch-normal = Normal pitch.
# $n is a number of semitones.
voice-pitch-plus = Pitch plus { $n }.
voice-pitch-minus = Pitch minus { $n }.
voice-full-volume = Full volume.
voice-volume-off = Volume off.
voice-volume = Volume { $pct } percent.
voice-no-speed-presets = No speed presets.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = { $name }, { $wpm } words per minute.
voice-line-numbers-on = Line numbers on.
voice-line-numbers-off = Line numbers off.
