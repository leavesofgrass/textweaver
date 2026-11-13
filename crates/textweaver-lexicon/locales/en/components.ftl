### Optional components and updates.

## Optional components: Manage optional components, the
## first-run list, and the questions features ask. Status messages fit 40
## Braille cells, meaning first. $title is a component's title, $size a
## size ("79.3 MB"), $license its license, $features what needs it.
component-feature-dictation = dictation
component-feature-ocr = reading scanned pages
component-feature-reading-font = a reading font
component-feature-voice = a speech voice
component-feature-audio-export = M4B and MP4 export
component-feature-braille = braille files as print, contracted braille
component-feature-pandoc = more document formats
component-state-installed = installed
component-state-not-installed = not installed
component-state-partial = partly installed
component-state-damaged = damaged
component-state-downloading = downloading
component-state-installer-ready = installer downloaded
components-title = Optional components
components-intro =
    { $n ->
        [one] 1 optional component. Enter for actions.
       *[other] { $n } optional components. Enter for actions.
    }
components-item = { $title }: { $state }, { $size }, license { $license }, for { $features }
components-actions-intro = { $title }: { $state }.
components-action-download = Download, { $size }
components-action-verify = Verify the files
components-action-remove = Remove
components-action-install-zip = Install from a zip file…
components-action-install-folder = Install from a folder…
components-install-purpose = Install the component from here
component-question = Download { $title }, { $size }, license { $license }? y or n
component-remove-question = Remove { $title }? y or n
component-downloading = Downloading. Escape stops it.
component-installing = Installing from the file.
component-verifying = Checking the files.
component-progress = { $percent } percent downloaded.
component-ready = Ready: { $title }.
component-verified = Files check out: { $title }.
component-verify-failed =
    { $n ->
        [one] 1 file does not check out: { $files }.
       *[other] { $n } files do not check out: { $files }.
    }
component-removed = Removed: { $title }.
component-refused =
    { $n ->
        [one] 1 file left out: { $files }.
       *[other] { $n } files left out: { $files }.
    }
component-already = Already installed: { $title }.
component-not-there = Not installed: { $title }.
component-declined = Not downloaded.
component-not-in-build = Downloads are not in this version.
component-no-folder = No data folder to keep it in.
component-error-fetch = Not downloaded: the source failed.
component-error-size = Not installed: wrong file size.
component-error-hash = Not installed: a file did not match. Get it again, or check the source.
component-error-missing = Not installed: a file is missing.
component-error-cancelled = Download stopped; it resumes later.
component-error-busy = Already downloading one.
component-error-no-source = Not downloaded: no address for it.
component-error-name = Refused: a name is not plain.
component-error-manifest = A components list is unreadable.
component-error-io = Not installed: could not write.
component-error-unpack = Not installed: the archive did not unpack.
component-launch-question = Launch installer: { $title }, version { $version }, license { $license }? The system asks next. y or n
component-version-unknown = not given
component-launched = Installer started: { $title }. Follow the system's prompts.
component-launch-failed = Installer did not start: { $reason }
component-launch-declined = Not launched; the installer is kept.
components-token-invalid = Not a token. Check it and paste it again.
components-token-kept = Token kept in the system credential store.
components-token-not-kept = Token used, not kept: { $reason }.
components-token-forgotten = Token forgotten.
components-token-none = No token was kept.
components-token-not-forgotten = Not forgotten: { $reason }.
components-token-gh-still = The GitHub CLI sign-in is still used; gh auth logout ends it.
components-chooser-title = Optional components
components-chooser-intro = Optional extras, none chosen. Space chooses one; Download the chosen ones gets them; Escape skips.
components-chooser-item = { $mark }: { $title }, for { $features }, { $size }, license { $license }
components-chosen = Chosen
components-not-chosen = Not chosen
components-chooser-download = Download the chosen ones
components-chooser-skip = Skip for now
components-chooser-skipped = Skipped; see Manage optional components.
components-chooser-none = Nothing chosen, nothing downloaded.
dictation-model-question = Dictation needs the Whisper model, { $size }, license { $license }. Download it now? y or n
dictation-model-declined = No model, so no dictation for now.
helper-question = { $need } needs { $title }, { $size }, license { $license }. Download it now? y or n
helper-need-audio = M4B and MP4 export
helper-need-brf = Reading braille as print
helper-run-command = To get { $title }, run: { $command }
dictation-model-not-in-build = No model; this version cannot download.
dictation-model-file-missing = Dictation model lacks { $file }.
dictation-model-damaged = Dictation model damaged: { $file }.
dictation-model-no-folder = No model folder: { $dir }.

## Settings for optional components.

## Updates (B1-u1): the check, the question before downloading, and each
## step said in words. Status messages fit 40 Braille cells, meaning first.
## $version is a release ("0.2.0-beta.1"), $size a size ("92.1 MB"),
## $percent a whole number, $reason why something failed.
components-chooser-updates = { $mark }: Check for updates automatically, once a day, from GitHub
update-checking = Checking for updates.
update-none = No update: { $version } is the newest.
update-found = Update available: textweaver { $version }, { $size }. Download it? y or n
update-available-elsewhere = Update available: textweaver { $version }. Update this copy the way it was installed.
update-declined = Not downloaded. A newer release will be offered.
update-downloading = Downloading the update, { $size }.
update-progress = Update { $percent } percent downloaded.
update-installed = Update verified and installed. Restart textweaver to use it.
update-on-close = Update verified. It installs when textweaver closes.
update-busy = Already checking for updates.
update-not-in-build = Updates are not in this version.
update-automatic-question = Check for updates automatically, once a day? y or n
update-automatic-on = Updates: checked once a day.
update-automatic-off = Updates: not checked. Help, Check for updates checks any time.
update-error-check = Update check failed: { $reason }
update-error-no-package = No update package for this computer in { $version }.
update-error-no-checksum = Update refused: no checksum for it. Nothing changed.
update-error-mismatch = Update refused: checksum does not match. Nothing changed.
update-error-cancelled = Update download stopped; it resumes next time.
update-error-download = Update not downloaded: { $reason }
update-error-install = Update not installed: { $reason }. Nothing changed.
update-error-not-package = Not updated: this copy is not from a release package.
section-updates = Updates and components
