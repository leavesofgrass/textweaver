### Reading: navigation, reading aloud, status, statistics, the queue, and reading aids.

## إحصاءات القراءة.

stats-title = إحصاءات القراءة
stats-intro = إحصاءات القراءة. Enter على مستند يفتحه.
stats-off = إحصاءات القراءة متوقفة. العنصر الأخير يفعّلها.
stats-empty = لا قراءة مسجّلة بعد. يُحسب الوقت أثناء قراءة textweaver بصوت عالٍ.
# $time is a duration-* message.
stats-total =
    { $time } قُرئت في المجموع، في { $sessions ->
        [one] جلسة واحدة
        [two] جلستين
        [few] { $sessions } جلسات
        [many] { $sessions } جلسة
       *[other] { $sessions } جلسة
    }، عبر { $docs ->
        [one] مستند واحد
        [two] مستندين
        [few] { $docs } مستندات
        [many] { $docs } مستندًا
       *[other] { $docs } مستند
    }.
stats-current =
    هذا المستند: { $time } قُرئت، أبعد نقطة { $pct } بالمئة، { $sessions ->
        [one] جلسة واحدة
        [two] جلستين
        [few] { $sessions } جلسات
        [many] { $sessions } جلسة
       *[other] { $sessions } جلسة
    }.
stats-current-none = لم يُقرأ هذا المستند بصوت عالٍ بعد.
stats-most-read = الأكثر قراءة { $rank }: { $title }، { $time }، أبعد نقطة { $pct } بالمئة.
stats-toggle-on = الإحصاءات مفعّلة. Enter لإيقافها.
stats-toggle-off = الإحصاءات متوقفة. Enter لتفعيلها.
stats-turned-on = إحصاءات القراءة مفعّلة.
stats-turned-off = إحصاءات القراءة متوقفة. ما سُجِّل باقٍ.

## القوائم.

study-nothing-to-delete = لا شيء لحذفه في هذه القائمة.
study-nothing-to-rename = لا شيء لإعادة تسميته في هذه القائمة.

## tw stats.

stats-clear-question =
    { $n ->
        [one] إزالة إحصاءات القراءة لمستند واحد؟ y أو n
        [two] إزالة إحصاءات القراءة لمستندين؟ y أو n
        [few] إزالة إحصاءات القراءة لـ{ $n } مستندات؟ y أو n
        [many] إزالة إحصاءات القراءة لـ{ $n } مستندًا؟ y أو n
       *[other] إزالة إحصاءات القراءة لـ{ $n } مستند؟ y أو n
    }
stats-cleared = أُزيلت إحصاءات القراءة.
stats-clear-item = إزالة إحصاءات القراءة
stats-clear-failed = تعذّرت إزالة إحصاءات القراءة: { $error }. تحقّق من إمكانية الكتابة في مجلد البيانات.
stats-off-cli = إحصاءات القراءة متوقفة: stats.enabled هو false في الإعدادات.

## Continue reading and every computer's statistics (the sync wave, S6).

continue-title = متابعة القراءة
# $n is the number of documents listed.
continue-intro =
    { $n ->
        [one] متابعة القراءة: مستند واحد، الأحدث أولًا.
        [two] متابعة القراءة: مستندان، الأحدث أولًا.
        [few] متابعة القراءة: { $n } مستندات، الأحدث أولًا.
        [many] متابعة القراءة: { $n } مستندًا، الأحدث أولًا.
       *[other] متابعة القراءة: { $n } مستند، الأحدث أولًا.
    }
continue-empty = لا مواضع بعد. القراءة تحفظ موضعك.
# One row, meaning first: the title, how far in, the computer, and how
# long ago (continue-ago-*).
continue-item = { $title }، { $pct } بالمئة، { $device }، { $when }
continue-this-computer = هذا الحاسوب
continue-ago-now = الآن
continue-ago-minutes =
    { $n ->
        [one] قبل دقيقة
        [two] قبل دقيقتين
        [few] قبل { $n } دقائق
        [many] قبل { $n } دقيقة
       *[other] قبل { $n } دقيقة
    }
continue-ago-hours =
    { $n ->
        [one] قبل ساعة
        [two] قبل ساعتين
        [few] قبل { $n } ساعات
        [many] قبل { $n } ساعة
       *[other] قبل { $n } ساعة
    }
continue-ago-days =
    { $n ->
        [one] قبل يوم
        [two] قبل يومين
        [few] قبل { $n } أيام
        [many] قبل { $n } يومًا
       *[other] قبل { $n } يوم
    }

## قائمة القراءة وقائمة المراجعة (الإصدار التجريبي 3).

queue-title = قائمة القراءة
queue-intro =
    { $n ->
        [one] قائمة القراءة، مستند واحد. Enter لفتحه، Delete لإخراجه.
        [two] قائمة القراءة، مستندان. Enter لفتح واحد، Delete لإخراجه.
        [few] قائمة القراءة، { $n } مستندات. Enter لفتح واحد، Delete لإخراجه.
        [many] قائمة القراءة، { $n } مستندًا. Enter لفتح واحد، Delete لإخراجه.
       *[other] قائمة القراءة، { $n } مستند. Enter لفتح واحد، Delete لإخراجه.
    }
queue-empty = قائمة القراءة فارغة. في المكتبة، { $key } يضيف مستندًا.
queue-item-missing = { $title }، غير موجود
queue-item-elsewhere = { $title }، ليس على هذا الحاسوب
queue-untitled = مستند من حاسوب آخر
queue-elsewhere-chosen = { $title } ليس على هذا الحاسوب. يبقى في قائمة القراءة، ويُعثر عليه حين تفتحه هنا.
queue-added =
    { $n ->
        [one] أُضيف إلى قائمة القراءة: { $title }. مستند واحد في القائمة.
        [two] أُضيف إلى قائمة القراءة: { $title }. مستندان في القائمة.
        [few] أُضيف إلى قائمة القراءة: { $title }. { $n } مستندات في القائمة.
        [many] أُضيف إلى قائمة القراءة: { $title }. { $n } مستندًا في القائمة.
       *[other] أُضيف إلى قائمة القراءة: { $title }. { $n } مستند في القائمة.
    }
queue-removed = أُخرج من قائمة القراءة: { $title }.
queue-no-file = لا يدخل قائمة القراءة إلا مستند محفوظ في ملف.
queue-read-failed = تعذّرت قراءة المكتبة لقائمة القراءة: { $error }. افحص library.json في مجلد البيانات؛ يذكره الأمر tw settings path.
queue-next-ask = التالي: { $title }. { $key } للقراءة.
queue-next-reading = التالي: { $title }.
review-title = قائمة المراجعة
review-intro =
    { $n ->
        [one] قائمة المراجعة، مستند واحد، الأقدم قراءةً أولًا. Enter لفتحه.
        [two] قائمة المراجعة، مستندان، الأقدم قراءةً أولًا. Enter لفتح واحد.
        [few] قائمة المراجعة، { $n } مستندات، الأقدم قراءةً أولًا. Enter لفتح واحد.
        [many] قائمة المراجعة، { $n } مستندًا، الأقدم قراءةً أولًا. Enter لفتح واحد.
       *[other] قائمة المراجعة، { $n } مستند، الأقدم قراءةً أولًا. Enter لفتح واحد.
    }
review-empty = لا شيء للمراجعة بعد. تظهر هنا المستندات التي قُرئت بصوت عالٍ.
review-stats-off = تحتاج قائمة المراجعة إلى إحصاءات القراءة، وهي متوقفة. شغّل إحصاءات القراءة في الإعدادات.
review-item = { $title }، { $days }، { $pct } بالمئة، { $notes }
review-days =
    { $n ->
        [0] اليوم
        [one] يوم واحد
        [two] يومان
        [few] { $n } أيام
        [many] { $n } يومًا
       *[other] { $n } يوم
    }
review-notes =
    { $n ->
        [0] لا ملاحظات
        [one] ملاحظة واحدة
        [two] ملاحظتان
        [few] { $n } ملاحظات
        [many] { $n } ملاحظة
       *[other] { $n } ملاحظة
    }
prompt-document-details = تفاصيل المستند

## Edit a document's details by hand.

# $name is the document's title; said when the form opens.
details-intro = تفاصيل { $name }. Tab للتنقل، Enter للحفظ، Escape للإلغاء.
details-label-title = العنوان
details-label-author = المؤلف
details-label-doi = DOI
details-label-isbn = ISBN
# A field's drawn label: its label, then $n of $total fields.
details-prompt-label = { $label }، { $n } من { $total }
# Said on moving to a field: its label and value (or nav-blank).
details-field = { $label }: { $value }
# $fields lists the fields saved, by their labels.
details-saved = حُفظت التفاصيل: { $fields }.
details-unchanged = لم تتغير التفاصيل.
details-cancelled = أُلغي. لم تتغير التفاصيل.
# $text is what was typed in the DOI or ISBN field.
details-not-a-doi = ليس DOI: { $text }. صحّحه أو امسحه.
details-not-an-isbn = ليس ISBN: { $text }. صحّحه أو امسحه.
details-no-file = ليس لهذا المستند ملف، فلا تفاصيل لتعديلها.
details-save-failed = تعذّر حفظ التفاصيل: { $error } حاول مرة أخرى.
# The statistics list could not wait for the sync folder.
stats-others-slow = تُركت الحواسيب الأخرى: المجلد بطيء.
stats-untitled = مستند بلا عنوان
# One computer's share of a document: $device, $time, $sessions.
stats-computer =
    { $sessions ->
        [one] { $device }: { $time }، جلسة واحدة
        [two] { $device }: { $time }، جلستان
        [few] { $device }: { $time }، { $sessions } جلسات
        [many] { $device }: { $time }، { $sessions } جلسة
       *[other] { $device }: { $time }، { $sessions } جلسة
    }
stats-by-computer-off = كل حاسوب: مخفي. Enter يظهره.
stats-by-computer-on = كل حاسوب: ظاهر. Enter يخفيه.

## التنقل. $dir هو next أو previous؛ $what اسم kind-* أو unit-*
## و$unit مفتاحه (heading، list-item، sentence)، للغات
## التي تحتاج كلماتها إلى مطابقة الاسم.

nav-blank = فارغ
# Said for a picture that has no description (no alternative text).
nav-no-description = بلا وصف
# The document overview: the title first, then the counts. $time is overview-time.
overview-line = { $title }. العناوين: { $headings }، الجداول: { $tables }، الصور: { $pictures }، الحواشي: { $footnotes }. { $time }
# Reading time left from the cursor at the current rate.
overview-time =
    { $minutes ->
        [0] تبقى أقل من دقيقة.
        [one] تبقى دقيقة واحدة تقريبًا.
        [two] تبقى دقيقتان تقريبًا.
        [few] تبقى { $minutes } دقائق تقريبًا.
       *[other] تبقى { $minutes } دقيقة تقريبًا.
    }
# Said when the reading pass changes and as reading starts in a skim. $pass is a reading-pass-* name.
reading-pass-changed = النمط: { $pass }.
reading-pass-full = النص كاملًا
reading-pass-first-sentences = الجمل الأولى
reading-pass-headings = العناوين
# High verbosity: $label is a structure label ("Heading level 2").
nav-message-at-labelled = { $label }، السطر { $line }، { $pct } بالمئة: { $content }
nav-message-at = السطر { $line }، { $pct } بالمئة: { $content }
nav-message-labelled = { $label }: { $content }
# $message is the navigation message after wrapping around.
nav-wrapped = تم الالتفاف. { $message }
nav-no-next =
    { $unit ->
        [paragraph] لا توجد { $what } تالية.
        [list] لا توجد { $what } تالية.
        [cell] لا توجد { $what } تالية.
        [graphic] لا توجد { $what } تالية.
        [code] لا توجد { $what } تالية.
        [page] لا توجد { $what } تالية.
        [footnote] لا توجد { $what } تالية.
        [word] لا توجد { $what } تالية.
        [sentence] لا توجد { $what } تالية.
       *[other] لا يوجد { $what } تالٍ.
    }
nav-no-previous =
    { $unit ->
        [paragraph] لا توجد { $what } سابقة.
        [list] لا توجد { $what } سابقة.
        [cell] لا توجد { $what } سابقة.
        [graphic] لا توجد { $what } سابقة.
        [code] لا توجد { $what } سابقة.
        [page] لا توجد { $what } سابقة.
        [footnote] لا توجد { $what } سابقة.
        [word] لا توجد { $what } سابقة.
        [sentence] لا توجد { $what } سابقة.
       *[other] لا يوجد { $what } سابق.
    }
nav-nothing-to-read = لا يوجد { $what } لقراءته.
nav-label-heading-level = مستوى العنوان { $level }
nav-label-list =
    { $n ->
        [0] قائمة
        [one] قائمة، عنصر واحد
        [two] قائمة، عنصران
        [few] قائمة، { $n } عناصر
        [many] قائمة، { $n } عنصرًا
       *[other] قائمة، { $n } عنصر
    }
nav-label-table =
    { $n ->
        [0] جدول
        [one] جدول، صف واحد
        [two] جدول، صفان
        [few] جدول، { $n } صفوف
        [many] جدول، { $n } صفًا
       *[other] جدول، { $n } صف
    }
nav-label-list-item-level = عنصر قائمة، المستوى { $level }
nav-no-heading-level =
    { $dir ->
        [next] لا يوجد عنوان تالٍ في المستوى { $level }.
       *[previous] لا يوجد عنوان سابق في المستوى { $level }.
    }
# Said before a line's text on caret moves.
nav-line-heading = عنوان من المستوى { $level }
nav-line-row = الصف { $n }
nav-line-list-item-level = عنصر قائمة، المستوى { $level }
# $language is a programming language name such as Python.
nav-line-code-language = شيفرة، { $language }
nav-no-chapters = لا فصول في هذا المستند.
nav-chapter = فصل
nav-no-chapter =
    { $dir ->
        [next] لا يوجد فصل تالٍ.
       *[previous] لا يوجد فصل سابق.
    }
nav-back = رجوع
nav-forward = تقدّم
nav-page = صفحة
nav-percent = { $pct } بالمئة
nav-no-earlier-history = لا سجل أقدم.
nav-no-forward-history = لا سجل تالٍ.
# $label is nav-back, nav-forward, nav-page, or nav-percent.
nav-label-line = { $label }، السطر { $line }
nav-top-of-document = أعلى المستند
nav-end-of-document = نهاية المستند
# $edge is nav-top-of-document or nav-end-of-document; $content the line there.
nav-edge-message = { $edge }. { $content }
nav-top-of-document-stop = أعلى المستند.
nav-end-of-document-stop = نهاية المستند.
nav-position = السطر { $line } من { $lines }، { $pct } بالمئة.
nav-position-word = الكلمة { $word } من { $words }.
nav-position-heading = تحت العنوان { $heading }.
nav-position-mode = وضع { $mode }.

## القراءة بصوت عالٍ.

playback-caps-no-words = هذا الصوت لا يبلّغ عن الكلمات، لذا يُقدَّر تمييز الكلمات تقديرًا.
playback-caps-words = هذا الصوت يبلّغ عن كل كلمة، لذا يتبعها التمييز بدقة.
playback-caps-no-pitch = لا يمكن تغيير حدة هذا الصوت.
playback-caps-no-volume = لا يمكن تغيير مستوى هذا الصوت.
# The reading state on the title line, one word.
state-reading = قراءة
state-paused = متوقّف مؤقتًا
state-stopped = متوقّف
state-ready = جاهز
playback-reading-at = القراءة بسرعة { $rate } كلمة في الدقيقة.
playback-paused = متوقّف مؤقتًا.
playback-stopped-speech-cursor-off = متوقّف. مؤشّر الكلام متوقف.
playback-stopped = متوقّف.
playback-search-cleared = مُسح البحث.
playback-end-of-document-content = نهاية المستند
playback-no-unit-here = لا { $what } هنا.
playback-no-selection = لا تحديد.
# $next is what happens now (a restart, or nothing).
playback-speech-died = توقف الكلام عن العمل ({ $reason }). { $next }
playback-done-reading = انتهت القراءة.
playback-speech-restarted = أُعيد تشغيل الكلام: { $reason }. متابعة القراءة من آخر كلمة.
playback-speech-error = خطأ في الكلام: { $error } حاول مرة أخرى، أو استخدم أمر إعادة تشغيل النطق.
playback-end-of-section = نهاية القسم. { $key } للمتابعة.
playback-time-up =
    { $minutes ->
        [one] انتهى الوقت بعد دقيقة واحدة. { $key } للمتابعة.
        [two] انتهى الوقت بعد دقيقتين. { $key } للمتابعة.
        [few] انتهى الوقت بعد { $minutes } دقائق. { $key } للمتابعة.
       *[other] انتهى الوقت بعد { $minutes } دقيقة. { $key } للمتابعة.
    }
playback-repeat-slower = إعادة أبطأ، بسرعة { $rate } كلمة في الدقيقة.
playback-recall-prompt = قل ما تتذكره من { $section }. { $key } للمتابعة.

## سطر العنوان وقول الحالة.

# The title line's position.
status-position = السطر { $line } من { $lines }
status-mode = وضع { $mode }
status-modified = مُعدَّل
status-self-voicing = نطق ذاتي
status-hybrid = مختلط
status-screen-reader = وضع قارئ الشاشة
status-rate-spoken = { $wpm } كلمة في الدقيقة
status-rate = { $wpm } ك/د
status-cw-rate = CW { $wpm } ك/د
status-cw-rate-spoken = CW بسرعة { $wpm } كلمة في الدقيقة
status-cw-rate-farnsworth = CW { $wpm } ك/د، الفعلية { $effective }
status-cw-rate-farnsworth-spoken = CW بسرعة { $wpm } كلمة في الدقيقة، والسرعة الفعلية { $effective }
# The window's status bar, last: reading time left at the current rate.
status-time-left =
    { $minutes ->
        [0] أقل من دقيقة متبقية
        [one] دقيقة واحدة متبقية
       *[other] { $minutes } دقيقة متبقية
    }
status-no-document = لا يوجد مستند
# $parts are the title line's parts, joined with commas.
status-said = { $title }: { $parts }.
status-no-message = لا توجد رسالة بعد.
# A list shown without its own introduction.
status-list-intro =
    { $n ->
        [one] { $title }، عنصر واحد. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإغلاق.
        [two] { $title }، عنصران. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإغلاق.
        [few] { $title }، { $n } عناصر. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإغلاق.
        [many] { $title }، { $n } عنصرًا. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإغلاق.
       *[other] { $title }، { $n } عنصر. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإغلاق.
    }

## وضع مؤشّر الكلام.

speechcursor-off = مؤشّر الكلام متوقف.
# $line is the line number.
speechcursor-on = مؤشّر الكلام مفعّل، السطر { $line }. لأعلى ولأسفل لقراءة الأسطر، Enter لمتابعة القراءة، Tab أو Escape للمغادرة.
# $text is the line read, as the status line shows it.
speechcursor-on-with-text = مؤشّر الكلام مفعّل، السطر { $line }: { $text }. لأعلى ولأسفل لقراءة الأسطر، Enter لمتابعة القراءة، Tab أو Escape للمغادرة.

## تمرير العرض دون تحريك المؤشر.

view-bottom-of-document = أسفل المستند.
# $line is the line now at the top of the view.
view-line-at-top = السطر { $line } في الأعلى.

## استكشاف الرياضيات.

mathx-no-math = لا رياضيات هنا. انتقل إلى معادلة، ثم أعد المحاولة.
# $math is the whole expression as spoken; $parts is yes when it has parts to go into. The keys are exploration's own.
mathx-exploring =
    { $parts ->
        [yes] استكشاف الرياضيات: { $math }. لأسفل للدخول، الأسهم للتنقل، Escape للمغادرة.
       *[no] استكشاف الرياضيات: { $math }. Escape للمغادرة.
    }
mathx-left = غادر الرياضيات.
mathx-last-term = الحد الأخير.
mathx-first-term = الحد الأول.
mathx-no-parts = لا أجزاء بداخله.
mathx-whole-expression = التعبير كاملًا.
mathx-nothing-here = لا شيء هنا.
# $speech is what was said for the step; $code is the math braille code's name (Nemeth or UEB); $braille is the part's braille in Unicode braille cells, for the Braille display.
mathx-step-braille = { $speech } { $code }: { $braille }

## أدوات القراءة: العرض السريع، والقراءة البيونية، والمقاطع، والكلمات الصعبة، والمسطرة، ومستوى القراءة.

aids-rsvp-off = العرض السريع متوقف.
aids-rsvp-leave-edit = غادر وضع التحرير لاستخدام العرض السريع.
# $status is RSVP's status line (word and sentence counts, rate, state).
aids-rsvp-on = العرض السريع مفعّل. { $status }
aids-rsvp-no-words = لا كلمات لعرضها.
aids-rsvp-fastest = أسرع سرعة للعرض السريع.
aids-rsvp-slowest = أبطأ سرعة للعرض السريع.
# $wpm is the new rate in words per minute.
aids-rsvp-rate = العرض السريع { $wpm } كلمة في الدقيقة.
# Where the RSVP word is shown; $position is one of nine fixed keys.
aids-rsvp-position =
    { $position ->
        [top-left] العرض السريع أعلى اليسار.
        [top-center] العرض السريع أعلى الوسط.
        [top-right] العرض السريع أعلى اليمين.
        [center-left] العرض السريع وسط اليسار.
        [center] العرض السريع في الوسط.
        [center-right] العرض السريع وسط اليمين.
        [bottom-left] العرض السريع أسفل اليسار.
        [bottom-right] العرض السريع أسفل اليمين.
       *[bottom-center] العرض السريع أسفل الوسط.
    }
aids-rsvp-playing = العرض السريع قيد التشغيل.
aids-rsvp-paused = العرض السريع متوقف مؤقتًا.
aids-rsvp-end-of-text = نهاية النص.
aids-rsvp-start-of-text = بداية النص.
aids-bionic-on = القراءة البيونية مفعّلة.
aids-bionic-off = القراءة البيونية متوقفة.
aids-syllables-shown = المقاطع ظاهرة.
aids-syllables-hidden = المقاطع مخفية.
aids-difficult-on = الكلمات الصعبة مسطّرة.
aids-difficult-no-list = الكلمات الصعبة مفعّلة، لكن قائمة الكلمات مفقودة من هذه النسخة.
aids-difficult-off = الكلمات الصعبة غير موسومة.
# Added after a word at high verbosity, following a comma.
aids-difficult-word = كلمة صعبة
aids-ruler-off = مسطرة القراءة متوقفة.
aids-ruler-current-line = السطر الحالي موسوم.
aids-ruler-on = مسطرة القراءة مفعّلة.
# $summary is aids-level-summary; $scope says what was measured.
aids-reading-level =
    { $scope ->
        [selection] التحديد: { $summary }
       *[document] المستند: { $summary }
    }
aids-reading-level-too-short = لا يوجد نص كافٍ لقياس مستوى القراءة.
# $grade is the Flesch-Kincaid grade with one decimal, $band an aids-band-* message, $ease the reading ease (0 to 100), $words aids-level-words, $sentences aids-level-sentences.
aids-level-summary = الصف { $grade }، { $band }. سهولة القراءة { $ease } من 100. { $words } في { $sentences }.
# $n is the count, $count the same number written with thousands separators.
aids-level-words =
    { $n ->
        [one] كلمة واحدة
        [two] كلمتان
        [few] { $count } كلمات
        [many] { $count } كلمة
       *[other] { $count } كلمة
    }
aids-level-sentences =
    { $n ->
        [one] جملة واحدة
        [two] جملتان
        [few] { $count } جمل
        [many] { $count } جملة
       *[other] { $count } جملة
    }
aids-band-elementary = ابتدائي
aids-band-middle-school = متوسط
aids-band-high-school = ثانوي
aids-band-college = جامعي
aids-band-graduate = دراسات عليا

## The Braille pass: pages in paged documents such as a PDF.
## $page and $n are page numbers, $label a printed page label such as iv,
## $pages the number of pages. Keep the page first: a 40-cell Braille
## display shows the start of the line.

status-page = الصفحة { $page } من { $pages }
status-page-labelled = الصفحة { $label }، { $n } من { $pages }
status-percent = { $pct }%
pages-position = الصفحة { $page } من { $pages }.
pages-position-labelled = الصفحة { $label }، { $n } من { $pages }.
pages-none = لا صفحات في هذا المستند.
pages-no-such-page = لا توجد صفحة { $page }. الصفحات من 1 إلى { $pages }.
pages-label = الصفحة { $label }
pages-outline-item = الصفحة { $label }: { $text }
lists-pages-title =
    { $n ->
        [one] الصفحات، صفحة واحدة
        [two] الصفحات، صفحتان
        [few] الصفحات، { $n } صفحات
        [many] الصفحات، { $n } صفحةً
       *[other] الصفحات، { $n } صفحة
    }
lists-pages-title-filtered = الصفحات، { $shown } من { $n } يطابق { $filter }
lists-pages-intro =
    { $n ->
        [one] الصفحات، صفحة واحدة. اكتب للتصفية، Enter للانتقال إلى صفحة، Escape للإغلاق.
        [two] الصفحات، صفحتان. اكتب للتصفية، Enter للانتقال إلى صفحة، Escape للإغلاق.
        [few] الصفحات، { $n } صفحات. اكتب للتصفية، Enter للانتقال إلى صفحة، Escape للإغلاق.
        [many] الصفحات، { $n } صفحةً. اكتب للتصفية، Enter للانتقال إلى صفحة، Escape للإغلاق.
       *[other] الصفحات، { $n } صفحة. اكتب للتصفية، Enter للانتقال إلى صفحة، Escape للإغلاق.
    }
lists-pages-here = أنت في { $heading }.
lists-filter-cleared-pages =
    { $n ->
        [one] مُسحت التصفية، صفحة واحدة.
        [two] مُسحت التصفية، صفحتان.
        [few] مُسحت التصفية، { $n } صفحات.
        [many] مُسحت التصفية، { $n } صفحةً.
       *[other] مُسحت التصفية، { $n } صفحة.
    }
lists-filter-none-pages = لا صفحات تطابق { $query }. Backspace لحذف الحروف.
lists-filter-matched-pages =
    { $n ->
        [one] صفحة واحدة مطابقة.
        [two] صفحتان مطابقتان.
        [few] { $n } صفحات مطابقة.
        [many] { $n } صفحةً مطابقة.
       *[other] { $n } صفحة مطابقة.
    }
prompt-go-to-pages = الانتقال إلى صفحة، أو السطر 12، أو نسبة مئوية أو start أو end
goto-not-a-target-pages = ليس هدف انتقال: { $text }. اكتب رقم صفحة، أو كلمة line ورقمًا، أو نسبة مئوية مثل 50%، أو start، أو end.
goto-word-page = صفحة

## Summaries and difficult-word definitions.
# The summary list's title: $n sentences of the whole document.
summary-title =
    { $n ->
        [one] ملخص، جملة واحدة
        [two] ملخص، جملتان
        [few] ملخص، { $n } جمل
        [many] ملخص، { $n } جملة
       *[other] ملخص، { $n } جملة
    }
# The summary of the chapter at the cursor.
summary-title-chapter =
    { $n ->
        [one] ملخص الفصل، جملة واحدة
        [two] ملخص الفصل، جملتان
        [few] ملخص الفصل، { $n } جمل
        [many] ملخص الفصل، { $n } جملة
       *[other] ملخص الفصل، { $n } جملة
    }
# The summary of the selection.
summary-title-selection =
    { $n ->
        [one] ملخص التحديد، جملة واحدة
        [two] ملخص التحديد، جملتان
        [few] ملخص التحديد، { $n } جمل
        [many] ملخص التحديد، { $n } جملة
       *[other] ملخص التحديد، { $n } جملة
    }
# Said when the summary list opens; $title is one of the titles above.
summary-intro = { $title }. Enter ينتقل إلى الجملة وينطقها.
# The same, when a long text was read in samples.
summary-intro-sampled = { $title }، من عينات من هذا النص الطويل. Enter ينتقل إلى الجملة وينطقها.
summary-none = لا شيء للتلخيص: لا توجد جملة من أربع كلمات أو أكثر.
# tw summarize, on standard error, when a long text was read in samples: $read of $total characters.
summary-sampled-cli = نص طويل: الملخص مأخوذ من { $read } من أصل { $total } حرفًا، قُرئت في عينات.
# After a difficult word at high verbosity, with definitions on: its first definition.
aids-difficult-word-defined = كلمة صعبة: { $definition }
settings-unit-sentences =
    { $n ->
        [one] جملة
        [two] جمل
        [few] جمل
        [zero] جمل
       *[other] جملة
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }، { -brand }.
gui-updated-to = تم التحديث إلى { $version }. مساعدة، التوثيق يصف الجديد.
