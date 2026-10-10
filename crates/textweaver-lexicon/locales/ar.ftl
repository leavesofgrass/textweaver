### رسائل واجهة textweaver باللغة العربية.
###
### بصيغة Fluent (https://projectfluent.org/)، في الجزء الذي يقرؤه
### textweaver-lexicon. المعرّفات والمتغيرات هي نفسها في en.ftl، وأي رسالة
### غير موجودة هنا تُقال بالإنجليزية.

-brand = textweaver

## أقسام الكلام.

pos-noun = اسم
pos-verb = فعل
pos-adjective = صفة
pos-adverb = ظرف

## تعريف كلمة: المصادر.

source-glossary = مسردك
source-wordnet = قاموس WordNet الإنجليزي المفتوح
source-cmudict = قاموس CMU للنطق

## تعريف كلمة: قائمة المعاني.

# $word is the word looked up, $n the number of senses, $source a source-* message.
define-title =
    { $n ->
        [0] نطق { $word }، من { $source }
        [one] تعريف { $word }، معنى واحد، من { $source }
        [two] تعريفا { $word }، معنيان، من { $source }
        [few] تعريفات { $word }، { $n } معانٍ، من { $source }
        [many] تعريفات { $word }، { $n } معنى، من { $source }
       *[other] تعريفات { $word }، { $n } معنى، من { $source }
    }
# $lemma is the headword, $pos a pos-* message, $i the sense number, $n how many.
define-sense-head = { $lemma }، { $pos }، { $i } من { $n }
define-sense-head-nopos = { $lemma }، { $i } من { $n }
define-sense = { $head }: { $definition }.
define-example = مثال: { $text }.
define-synonyms = مرادفات: { $words }.
define-antonyms = العكس: { $words }.
define-kind-of = نوع من: { $words }.
# $say is a respelling such as RUN-ing, with the stressed syllable in capitals.
define-pronounced = يُنطق { $say }.
define-pronounced-or = يُنطق { $say }، أو { $other }.

## طلبات الإدخال.

prompt-define-word = تعريف أي كلمة؟
prompt-profile-name = اسم لملف الإعدادات الجديد
prompt-profile-rename = اسم جديد لملف الإعدادات، Enter للإبقاء عليه
prompt-profiles-import = استيراد ملفات الإعدادات من ملف
prompt-profiles-export = تصدير ملفات الإعدادات إلى ملف، مثل textweaver-profiles.json

## كلمات شائعة.

common-cancelled = أُلغي.
# Durations: $h hours, $m minutes, $s seconds.
duration-hours =
    { $h ->
        [one] ساعة واحدة
        [two] ساعتان
        [few] { $h } ساعات
        [many] { $h } ساعة
       *[other] { $h } ساعة
    } و{ $m ->
        [one] دقيقة واحدة
        [two] دقيقتان
        [few] { $m } دقائق
        [many] { $m } دقيقة
       *[other] { $m } دقيقة
    }
duration-minutes =
    { $m ->
        [one] دقيقة واحدة
        [two] دقيقتان
        [few] { $m } دقائق
        [many] { $m } دقيقة
       *[other] { $m } دقيقة
    } و{ $s ->
        [one] ثانية واحدة
        [two] ثانيتان
        [few] { $s } ثوانٍ
        [many] { $s } ثانية
       *[other] { $s } ثانية
    }
duration-seconds =
    { $s ->
        [one] ثانية واحدة
        [two] ثانيتان
        [few] { $s } ثوانٍ
        [many] { $s } ثانية
       *[other] { $s } ثانية
    }

## تعريف كلمة في القارئ.

# $title is define-title.
define-intro = { $title }. السهمان لأعلى ولأسفل يتنقلان بين المعاني، Enter لنسخ أحدها، Escape للإغلاق.
define-nothing-here = لا توجد كلمة عند المؤشر.
define-not-found = لم يُعثر على تعريف لـ{ $word }.
define-no-dictionary = ملف القاموس غير مثبَّت، لذا بُحث في مسردك فقط. دليل القراءة يوضح كيفية تثبيته.
define-dictionary-damaged = تعذّرت قراءة ملف القاموس: { $error } يُبحث في مسردك فقط.
define-glossary-problem = تعذّرت قراءة مسردك: { $error } يُبحث في القاموس فقط.
define-glossary-skipped =
    { $n ->
        [one] سطر واحد في مسردك بلا تعريف، فتم تخطّيه.
        [two] سطران في مسردك بلا تعريف، فتم تخطّيهما.
        [few] { $n } أسطر في مسردك بلا تعريف، فتم تخطّيها.
        [many] { $n } سطرًا في مسردك بلا تعريف، فتم تخطّيها.
       *[other] { $n } سطر في مسردك بلا تعريف، فتم تخطّيها.
    }
define-copied = نُسخ.

## ملفات الإعدادات.

profiles-title =
    { $n ->
        [0] ملفات الإعدادات، لا شيء محفوظ بعد
        [one] ملفات الإعدادات، ملف واحد
        [two] ملفات الإعدادات، ملفان
        [few] ملفات الإعدادات، { $n } ملفات
        [many] ملفات الإعدادات، { $n } ملفًا
       *[other] ملفات الإعدادات، { $n } ملف
    }
# $title is profiles-title.
profiles-intro = { $title }. Enter للتبديل إلى ملف، F2 لإعادة تسميته، Delete لحذفه.
# $summary is profile-summary-* parts joined by commas.
profiles-item = { $name }: { $summary }
profiles-item-active = { $name }، قيد الاستخدام: { $summary }
profiles-save-new = حفظ الإعدادات الحالية كملف إعدادات جديد
profiles-update = حفظ الإعدادات الحالية في { $name }
profiles-import = استيراد ملفات الإعدادات من ملف
profiles-export = تصدير كل ملفات الإعدادات إلى ملف
profile-summary-voice = الصوت { $voice }
profile-summary-rate = السرعة { $rate }
profile-summary-theme = السمة { $theme }
# $mode is self voicing, hybrid, or screen reader.
profile-summary-access = وضع { $mode }
profile-summary-empty = لا شيء محفوظ
profile-switched = تم التبديل إلى { $name }.
profile-switched-backend = تم التبديل إلى { $name }. يُستخدم محرك الكلام الخاص به من بدء التشغيل التالي.
# $keys lists settings such as speech.pitch.
profile-dropped =
    { $n ->
        [one] إعداد واحد فيه غير مستخدم في هذا الإصدار: { $keys }.
        [two] إعدادان فيه غير مستخدمَين في هذا الإصدار: { $keys }.
        [few] { $n } إعدادات فيه غير مستخدمة في هذا الإصدار: { $keys }.
        [many] { $n } إعدادًا فيه غير مستخدم في هذا الإصدار: { $keys }.
       *[other] { $n } إعداد فيه غير مستخدم في هذا الإصدار: { $keys }.
    }
profile-saved = حُفظت الإعدادات الحالية باسم { $name }.
profile-replaced = حُفظت الإعدادات الحالية في { $name }.
profile-renamed = أُعيدت تسمية { $old } إلى { $new }.
profile-delete-question = حذف ملف الإعدادات { $name }؟ y أو n
profile-deleted = حُذف { $name }.
profile-kept = أُبقي عليه.
profile-not-found = لا يوجد ملف إعدادات باسم { $name }.
profile-needs-name = يحتاج ملف الإعدادات إلى اسم.
profile-exists = يوجد بالفعل ملف إعدادات باسم { $name }.
profiles-not-an-export = { $detail }
profiles-no-persistence = لا تُحفظ ملفات الإعدادات في هذه الجلسة.
profiles-read-failed = تعذّرت قراءة ملف ملفات الإعدادات، لذا يُعامل كأنه فارغ: { $error } حفظ ملف إعدادات يستبدل الملف.
profiles-save-failed = تعذّر حفظ ملفات الإعدادات: { $error } تحقّق من إمكانية الكتابة في مجلد الإعدادات.
profiles-none-to-export = لا توجد ملفات إعدادات لتصديرها بعد.
profiles-exported =
    { $n ->
        [one] صُدِّر ملف إعدادات واحد إلى { $file }.
        [two] صُدِّر ملفا إعدادات إلى { $file }.
        [few] صُدِّرت { $n } ملفات إعدادات إلى { $file }.
        [many] صُدِّر { $n } ملف إعدادات إلى { $file }.
       *[other] صُدِّر { $n } ملف إعدادات إلى { $file }.
    }
profiles-export-failed = تعذّر تصدير ملفات الإعدادات: { $error } تحقّق من إمكانية الكتابة في المجلد.
profiles-imported =
    { $n ->
        [0] لم تكن هناك ملفات إعدادات في { $file }.
        [one] استُورد ملف إعدادات واحد من { $file }: { $names }.
        [two] استُورد ملفا إعدادات من { $file }: { $names }.
        [few] استُوردت { $n } ملفات إعدادات من { $file }: { $names }.
        [many] استُورد { $n } ملف إعدادات من { $file }: { $names }.
       *[other] استُورد { $n } ملف إعدادات من { $file }: { $names }.
    }

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
name-continue-reading = متابعة القراءة
action-continue-reading = متابعة القراءة: المستندات على هذا الحاسوب التي لها موضع محفوظ، من أي حاسوب، الأحدث أولًا
name-add-library-folder = إضافة مجلد إلى المكتبة
action-add-library-folder = إضافة مجلد إلى المكتبة: اختره في مستعرض الملفات
name-edit-document-details = تعديل التفاصيل
action-edit-document-details = تعديل تفاصيل المستند: العنوان والمؤلف وDOI وISBN
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

## أسماء البنية والوحدات والأوضاع (تُقال داخل رسائل أخرى).

kind-heading = عنوان
kind-paragraph = فقرة
kind-list-item = عنصر قائمة
kind-list = قائمة
kind-table = جدول
kind-row = صف
kind-cell = خلية
kind-link = رابط
kind-graphic = صورة
kind-code = شيفرة
kind-quote = اقتباس
kind-page = صفحة
kind-section = قسم
kind-bold = عريض
kind-italic = مائل
kind-underline = تسطير
kind-footnote = حاشية
kind-strikethrough = يتوسطه خط
kind-separator = فاصل
kind-math = رياضيات
unit-character = حرف
unit-word = كلمة
unit-sentence = جملة
unit-line = سطر
unit-paragraph = فقرة
unit-document = مستند
# $what is a kind-* noun, $unit its key.
unit-with-level = { $what } من المستوى { $level }
mode-browse = تصفح
mode-speech-cursor = مؤشّر الكلام
mode-edit = تحرير
mode-find = بحث
mode-command = أمر
mode-go-to = الانتقال إلى
mode-open = فتح
mode-prompt = إدخال
common-on = تشغيل
common-off = إيقاف

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
# $key is the key that turns edit mode off.
playback-still-editing = ما زلت في وضع التحرير. { $key } لإنهائه.
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

## الأسئلة والأجوبة. أبقِ على الحرفين y وn: فهما المفتاحان للإجابة.

common-press-y-or-n = اضغط y أو n.
common-kept = أُبقي عليه.
confirm-quit = إنهاء textweaver؟ y أو n
confirm-delete-note = حذف هذه الملاحظة أو التمييز؟ y أو n
notes-remove-highlight-question = إزالة هذا التمييز؟ y أو n
notes-delete-note-question = حذف هذه الملاحظة؟ y أو n
list-nothing-to-mark = لا شيء لتحديده في هذه القائمة.
notes-editing = تحرير ملاحظة: { $text }
# $key opens a document.
app-no-document-open = لا يوجد مستند مفتوح. اضغط { $key } لفتح واحد.
app-window-only = يعمل هذا الأمر في نافذة textweaver.
app-terminal-only = يعمل هذا الأمر في قارئ الطرفية.
settings-save-failed = تعذّر حفظ الإعدادات: { $error } تبقى تغييراتك سارية حتى تخرج.
settings-outside-kept = تم الاحتفاظ بالإعدادات التي تغيّرت خارج textweaver.
edit-still-editing = ما زلت في وضع التحرير.
goto-not-a-target = ليس هدف انتقال: { $text }. اكتب رقم سطر، أو نسبة مئوية مثل 50%، أو start، أو end.

## فتح مستند.

open-opened = فُتح { $title }.
open-resumed = فُتح { $title }. استُؤنفت القراءة عند { $pct } بالمئة.
open-resumed-synced = فُتح { $title }. استُؤنفت القراءة عند { $pct } بالمئة، من جهاز آخر.

## الطلبات: تُعرض التسمية وتُقال عند فتح الطلب.

prompt-find = بحث
prompt-go-to = الانتقال إلى سطر أو نسبة مئوية أو start أو end
prompt-open = فتح ملف
prompt-command = أمر
# $label is prompt-command.
prompt-command-palette-intro = { $label }. اكتب جزءًا من اسم؛ Tab لإكماله، والسهمان لأعلى ولأسفل لسرد التطابقات.
prompt-save-as = حفظ باسم
prompt-table-size = حجم الجدول، أعمدة × صفوف، مثل 3 × 2
prompt-image-path = ملف صورة
prompt-replace-find = استبدال، ابحث عن
prompt-replace-with = استبدل بـ
prompt-note = ملاحظة
prompt-edit-note = تحرير ملاحظة، Enter للإبقاء عليها
prompt-rename-bookmark = اسم جديد للإشارة المرجعية، Enter للإبقاء عليه
prompt-export-settings = تصدير الإعدادات إلى ملف، مثل textweaver-settings.json
prompt-import-settings = استيراد الإعدادات من ملف
prompt-citation-locator = رقم الصفحة أو موضع آخر، مثل 12 أو chapter 2؛ Enter لعدم تحديد شيء
prompt-reference-identifier = DOI أو ISBN لإضافته
prompt-import-references = استيراد المراجع من ملف
prompt-template-title = عنوان المستند الجديد
prompt-setting-value = القيمة الجديدة، Enter للإبقاء عليها

## المفاتيح المذكورة في الرسائل والمساعدة.

help-the-command-palette = لوحة الأوامر
help-not-bound = غير مرتبط بمفتاح
# Two keys, or a key and a list of keys: "p or Ctrl+P".
help-or = { $a } أو { $b }
# A command without keys: $name is its palette name, such as list highlights.
help-the-command = الأمر { $name }
# One line of the keyboard shortcuts list: the command's name-* (first, so
# type-ahead finds commands), its keys (inside a 40-cell Braille line), an
# action-* help, and its category-* title.
help-entry = { $name }: { $keys }. { $help }. { $category }
help-unknown-command = أمر غير معروف: { $text }.
help-shortcuts-intro = اختصارات لوحة المفاتيح، { $n } أمرًا. اكتب للتصفية. ينتقل Page Down إلى المجموعة التالية، ويشرح F1 الأمر، ويشغّله Enter، ويغلق Escape.
help-shortcuts-title = اختصارات لوحة المفاتيح
# The keyboard shortcuts list, filtered: $filter is what was typed.
help-shortcuts-title-matching = اختصارات لوحة المفاتيح المطابقة لـ { $filter }
# $n commands of $total match the filter.
help-shortcuts-filter-match = { $n } من { $total } أمرًا مطابقة.
help-shortcuts-filter-none = لا يطابق أي أمر { $query }. يزيل Backspace الأحرف.
help-shortcuts-filter-cleared = مُسح عامل التصفية، { $n } أمرًا.
# Moving into a group of the keyboard shortcuts list: its name, its
# size, then the row ($item, with its place in the list).
help-shortcuts-group-item =
    { $group }، { $n ->
        [one] أمر واحد
       *[other] { $n } أوامر
    }. { $item }
help-title = مساعدة
help-intro = مساعدة. السهمان لأعلى ولأسفل للتنقل، Escape للإغلاق.

## قائمة المساعدة. كل قيمة هي مفتاح أو مفاتيح من خريطة المفاتيح.

help-about = يقرأ textweaver المستندات بصوت عالٍ. المفاتيح أدناه هي الارتباطات الحالية.
help-open = فتح مستند: { $open }. المكتبة والملفات الأخيرة: { $library }.
help-play = تشغيل أو إيقاف مؤقت: { $key }.
help-read-from-cursor = القراءة من المؤشر: { $key }.
help-stop = إيقاف: { $key }.
help-sentences = الجملة التالية والسابقة: { $next } و{ $previous }.
help-paragraphs = الفقرة التالية والسابقة: { $next } و{ $previous }.
help-headings = العنوان التالي والسابق: { $next } و{ $previous }. عنوان في مستوى معيّن: من { $first } إلى { $last }، مع Shift للسابق.
help-read-headings = القراءة من العنوان التالي والسابق: { $next } و{ $previous }.
help-quick-keys = مفاتيح سريعة، كما في NVDA وJAWS: قائمة { $list }، عنصر قائمة { $item }، جدول { $table }، رابط { $link }، اقتباس { $quote }، فاصل { $separator }، صورة { $graphic }، قسم { $section }. مع Shift للانتقال إلى السابق.
help-speech-cursor = مؤشّر الكلام، سطرًا سطرًا: { $key }.
help-find = بحث: { $key }.
help-bookmark = إضافة إشارة مرجعية: { $key }.
help-history = الرجوع والتقدّم عبر قفزاتك: { $back } و{ $forward }.
help-rate = أسرع وأبطأ: { $faster } و{ $slower }.
help-where = أين أنا: { $key }.
help-repeat = سماع الرسالة الأخيرة مجددًا: { $repeat }. الرسالة الأخيرة والحالة: الوضع والسرعة والمحرك والموضع: { $status }.
help-notes = الملاحظات: إضافة { $add }، سرد { $list }، التالية والسابقة { $next } و{ $previous }، حذف التي عند المؤشر { $delete }. في القائمة، Delete للحذف وF2 للتحرير.
help-highlights = تمييز التحديد أو الجملة، أو إزالة تمييز: { $highlight }. سرد أشرطة التمييز: { $list }.
help-bookmarks-list = قائمة الإشارات المرجعية: Delete لحذف إشارة، F2 لإعادة تسميتها.
help-edit = تحرير المستند: { $edit }. حفظ: { $save }. حفظ باسم: { $saveas }. مستند جديد: { $new }.
help-editing = أثناء التحرير: تراجع { $undo }، إعادة { $redo }، عريض { $bold }. كل أمر تنسيق موجود في اختصارات لوحة المفاتيح.
help-outline = مخطط العناوين، اكتب للتصفية: { $outline }. اتباع رابط أو حاشية: { $follow }.
help-tables = الجداول: { $nextrow } و{ $previousrow } للتنقل صفًا صفًا، { $nextcell } و{ $previouscell } خلية خلية.
help-citations = الاستشهادات أثناء التحرير: إدراج { $insert }، إضافة مرجع بـDOI أو ISBN { $reference }. التدقيق الإملائي: الخطأ التالي والسابق { $next } و{ $previous }، الاقتراحات { $suggestions }.
help-export = التصدير إلى HTML أو PDF أو Word أو EPUB أو برايل، والمعاينة في المتصفح، والبدء من قالب: اكتب export أو preview أو template في لوحة الأوامر.
help-verbosity = مقدار ما يُقال: { $verbosity }. مقدار علامات الترقيم: { $punctuation }.
help-voice = اختيار صوت: { $voice }. إعادة تشغيل الكلام إذا توقف: { $restart }.
help-access = مع قارئ شاشة، من يتحدث: { $key } يبدّل بين النطق الذاتي والوضع المختلط ووضع قارئ الشاشة.
help-access-window = من يقرأ: { $key } يبدّل بين يقرأ textweaver بصوت عالٍ وقارئ شاشتي يقرأ.
help-character-keys = الاختصارات أحادية المفتاح تشغيلًا أو إيقافًا، للإملاء: { $keys }. الإعدادات: { $settings }.
help-all-shortcuts = كل اختصارات لوحة المفاتيح: { $key }.
help-palette = تشغيل أي أمر بالاسم: { $key }.
# Keep the letters y, n, and a: they are the keys that answer.
help-quit = إنهاء، مع حفظ موضعك: { $key }، ثم y للتأكيد؛ n أو a أو Escape للإلغاء.

## فئات المساعدة.

category-reading = القراءة
category-navigation = التنقل
category-speech-cursor = مؤشّر الكلام
category-voice = الصوت
category-search = البحث
category-bookmarks = الإشارات المرجعية والملاحظات
category-file = ملف
category-editing = التحرير
category-view = العرض والمساعدة

## أسماء المفاتيح كما ينطقها صوت textweaver. الأسماء المكتوبة (Ctrl+S)
## لا تُترجم.

keyname-control = Control
keyname-command = Command
keyname-alt = Alt
keyname-shift = Shift
keyname-period = نقطة
keyname-comma = فاصلة
keyname-semicolon = فاصلة منقوطة
keyname-colon = نقطتان
keyname-apostrophe = فاصلة عليا
keyname-quote = علامة اقتباس
keyname-grave-accent = علامة نبر خلفية
keyname-tilde = تيلدا
keyname-exclamation-mark = علامة تعجب
keyname-question-mark = علامة استفهام
keyname-at-sign = علامة at
keyname-number-sign = علامة الرقم
keyname-dollar-sign = علامة الدولار
keyname-percent = علامة النسبة المئوية
keyname-caret = سقف
keyname-ampersand = علامة العطف
keyname-asterisk = نجمة
keyname-left-parenthesis = قوس يسار
keyname-right-parenthesis = قوس يمين
keyname-left-bracket = قوس معقوف يسار
keyname-right-bracket = قوس معقوف يمين
keyname-left-brace = قوس مزهر يسار
keyname-right-brace = قوس مزهر يمين
keyname-less-than = أصغر من
keyname-greater-than = أكبر من
keyname-plus = زائد
keyname-minus = ناقص
keyname-equals = يساوي
keyname-underscore = شرطة سفلية
keyname-slash = شرطة مائلة
keyname-backslash = شرطة مائلة عكسية
keyname-vertical-bar = شرطة عمودية
keyname-page-up = Page Up
keyname-page-down = Page Down
keyname-up-arrow = سهم لأعلى
keyname-down-arrow = سهم لأسفل
keyname-left-arrow = سهم لليسار
keyname-right-arrow = سهم لليمين
keyname-escape = Escape
keyname-space = مسافة
keyname-enter = Enter
keyname-tab = Tab
keyname-backspace = Backspace
keyname-delete = حذف
keyname-insert = Insert
keyname-home = Home
keyname-end = End

## الأوامر: سطر المساعدة الخاص بكل أمر، في قائمة اختصارات لوحة المفاتيح
## ولوحة الأوامر. تبقى معرّفاتها (play_pause) كما هي.

action-play-pause = تشغيل أو إيقاف مؤقت للقراءة من الكلمة الحالية
action-stop = إيقاف القراءة
action-read-from-cursor = القراءة المتواصلة من المؤشر
action-read-document = قراءة المستند كاملًا من البداية
action-read-current-character = نطق الحرف عند المؤشر
action-read-current-word = نطق الكلمة عند المؤشر
action-read-current-sentence = نطق الجملة عند المؤشر دون تحريكه
action-read-current-line = نطق السطر عند المؤشر
action-read-paragraph = نطق الفقرة عند المؤشر دون تحريكه
action-read-selection = قراءة النص المحدد
action-say-position = نطق الموضع: السطر والنسبة المئوية ورقم الكلمة والعنوان
action-say-status = نطق الرسالة الأخيرة مجددًا، ثم الحالة: الوضع وحالة القراءة والموضع والسرعة ومحرك الكلام؛ وفي قائمة، مقدمتها
action-repeat-message = نطق الرسالة الأخيرة مجددًا
action-word-count = نطق عدد كلمات المستند، أو التحديد
action-link-address = نطق عنوان الرابط عند المؤشر
action-replay-sentence = إعادة القراءة من بداية الجملة الحالية
action-replay-paragraph = إعادة القراءة من بداية الفقرة الحالية
action-repeat-sentence-slower = نطق الجملة عند المؤشر مرة أخرى بسرعة أبطأ، ثم العودة إلى السرعة المعتادة
action-rsvp-toggle = إظهار أو إخفاء العرض السريع للكلمات، كلمة كلمة من المؤشر
action-rsvp-play-pause = بدء أو إيقاف مؤقت للعرض السريع للكلمات
action-rsvp-faster = تسريع العرض السريع للكلمات
action-rsvp-slower = إبطاء العرض السريع للكلمات
action-rsvp-position-next = نقل كلمة العرض السريع إلى الموضع التالي على الشاشة
action-reading-level = نطق مستوى قراءة المستند أو التحديد
action-document-overview = قول عنوان المستند، وعدد العناوين والجداول والصور والحواشي فيه، وكم دقيقة تبقى تقريبًا
action-reading-pass = تغيير ما تقوله القراءة: النص كاملًا، أو الجملة الأولى من كل فقرة مع العناوين، أو العناوين فقط
action-define-word = تعريف الكلمة عند المؤشر، أو الكلمات المحددة: المعاني والأمثلة والمرادفات والنطق
action-toggle-citations = تشغيل أو إيقاف الاستشهادات في القراءة المتواصلة: الإيقاف يتخطاها، والتشغيل ينطقها بالكلمات
action-explore-math = استكشاف الرياضيات عند المؤشر حدًا حدًا: الأسهم للتنقل، لأسفل للدخول في جزء، لأعلى للخروج منه، Escape للمغادرة
action-listen-rendered = الاستماع إلى المستند كما سيظهر عند العرض، دون مغادرة وضع التحرير
action-next-sentence = الانتقال إلى الجملة التالية
action-previous-sentence = الانتقال إلى الجملة السابقة، أو إلى بداية هذه الجملة عند تجاوز ثلاث كلمات فيها
action-next-paragraph = الانتقال إلى الفقرة التالية
action-previous-paragraph = الانتقال إلى الفقرة السابقة
action-next-heading = القراءة من العنوان التالي
action-previous-heading = القراءة من العنوان السابق
action-skip-next-heading = الانتقال إلى العنوان التالي دون قراءته
action-skip-previous-heading = الانتقال إلى العنوان السابق دون قراءته
action-outline = سرد العناوين: اكتب للتصفية، Enter للقفز إلى أحدها
action-next-heading-level-1 = الانتقال إلى العنوان التالي في المستوى 1
action-next-heading-level-2 = الانتقال إلى العنوان التالي في المستوى 2
action-next-heading-level-3 = الانتقال إلى العنوان التالي في المستوى 3
action-next-heading-level-4 = الانتقال إلى العنوان التالي في المستوى 4
action-next-heading-level-5 = الانتقال إلى العنوان التالي في المستوى 5
action-next-heading-level-6 = الانتقال إلى العنوان التالي في المستوى 6
action-previous-heading-level-1 = الانتقال إلى العنوان السابق في المستوى 1
action-previous-heading-level-2 = الانتقال إلى العنوان السابق في المستوى 2
action-previous-heading-level-3 = الانتقال إلى العنوان السابق في المستوى 3
action-previous-heading-level-4 = الانتقال إلى العنوان السابق في المستوى 4
action-previous-heading-level-5 = الانتقال إلى العنوان السابق في المستوى 5
action-previous-heading-level-6 = الانتقال إلى العنوان السابق في المستوى 6
action-next-table = الانتقال إلى الجدول التالي
action-previous-table = الانتقال إلى الجدول السابق
action-next-list = الانتقال إلى القائمة التالية
action-previous-list = الانتقال إلى القائمة السابقة
action-next-list-item = الانتقال إلى عنصر القائمة التالي
action-previous-list-item = الانتقال إلى عنصر القائمة السابق
action-next-link = الانتقال إلى الرابط التالي
action-previous-link = الانتقال إلى الرابط السابق
action-next-block-quote = الانتقال إلى الاقتباس التالي
action-previous-block-quote = الانتقال إلى الاقتباس السابق
action-next-separator = الانتقال إلى الفاصل التالي (خط أفقي)
action-previous-separator = الانتقال إلى الفاصل السابق (خط أفقي)
action-next-graphic = الانتقال إلى الصورة التالية
action-previous-graphic = الانتقال إلى الصورة السابقة
action-follow-link = اتباع الرابط عند المؤشر، أو الانتقال بين حاشية وملاحظتها
action-table-next-row = في جدول، النزول صفًا في العمود نفسه
action-table-previous-row = في جدول، الصعود صفًا في العمود نفسه
action-table-next-column = في جدول، الانتقال إلى الخلية التالية في الصف
action-table-previous-column = في جدول، الانتقال إلى الخلية السابقة في الصف
action-next-chapter = الانتقال إلى الفصل أو القسم التالي
action-previous-chapter = الانتقال إلى الفصل أو القسم السابق
action-history-back = الرجوع إلى حيث كنت قبل آخر قفزة
action-history-forward = التقدّم مجددًا بعد الرجوع
action-go-to = الانتقال إلى سطر أو نسبة مئوية أو موضع
action-document-start = الانتقال إلى بداية المستند
action-document-end = الانتقال إلى نهاية المستند
action-caret-next-word = نقل المؤشر إلى الكلمة التالية
action-caret-previous-word = نقل المؤشر إلى الكلمة السابقة
action-caret-next-line = نقل المؤشر إلى السطر التالي
action-caret-previous-line = نقل المؤشر إلى السطر السابق
action-select-next-word = توسيع التحديد إلى الكلمة التالية
action-select-previous-word = توسيع التحديد إلى الكلمة السابقة
action-select-next-line = توسيع التحديد إلى السطر التالي
action-select-previous-line = توسيع التحديد إلى السطر السابق
action-page-down = النزول شاشة واحدة
action-page-up = الصعود شاشة واحدة
action-scroll-down = التمرير لأسفل سطرًا واحدًا دون تحريك المؤشر
action-scroll-up = التمرير لأعلى سطرًا واحدًا دون تحريك المؤشر
action-speech-cursor-toggle = الدخول في وضع مؤشّر الكلام (السطر) أو مغادرته
action-speech-cursor-next-line = مؤشّر الكلام: قراءة السطر التالي
action-speech-cursor-previous-line = مؤشّر الكلام: قراءة السطر السابق
action-speech-cursor-reread-line = مؤشّر الكلام: إعادة قراءة السطر الحالي
action-speech-cursor-exit-and-read = مؤشّر الكلام: المغادرة والقراءة من هذا السطر
action-rate-up = تسريع الكلام
action-rate-down = إبطاء الكلام
action-pitch-up = رفع حدة الصوت
action-pitch-down = خفض حدة الصوت
action-volume-up = أعلى صوتًا
action-volume-down = أخفض صوتًا
action-cycle-speed-preset = التنقل بين إعدادات السرعة الجاهزة (سريعة، عادية، دراسة، بطيئة)
action-choose-voice = اختيار صوت
action-restart-speech = إعادة تشغيل الكلام بالإعدادات الحالية (بعد توقف محرك الكلام عن العمل)
action-cycle-verbosity = التنقل بين مستويات ما يقوله textweaver: منخفض، عادي، مرتفع
action-cycle-punctuation = التنقل بين مقدار علامات الترقيم المنطوقة: لا شيء، بعضها، كلها
action-find = البحث عن نص في المستند
action-find-next = البحث عن التطابق التالي
action-find-previous = البحث عن التطابق السابق
action-search-options = اختيار طريقة المطابقة في البحث والاستبدال: حالة الأحرف، الكلمات الكاملة، التعبير النمطي، عبر الأسطر
action-next-misspelling = الانتقال إلى الكلمة الخطأ إملائيًا التالية، وتهجئتها
action-previous-misspelling = الانتقال إلى الكلمة الخطأ إملائيًا السابقة، وتهجئتها
action-spelling-suggestions = سرد اقتراحات الكلمة الخطأ إملائيًا عند المؤشر، أو إضافتها إلى قائمة كلماتك
action-next-grammar-problem = الانتقال إلى المشكلة النحوية التالية، ونطقها مع إصلاحها
action-previous-grammar-problem = الانتقال إلى المشكلة النحوية السابقة، ونطقها مع إصلاحها
action-next-lint-problem = في وضع التحرير، الانتقال إلى مشكلة فحص ماركداون التالية، ونطقها
action-previous-lint-problem = في وضع التحرير، الانتقال إلى مشكلة فحص ماركداون السابقة، ونطقها
action-add-bookmark = إضافة إشارة مرجعية عند المؤشر
action-list-bookmarks = سرد الإشارات المرجعية
action-next-bookmark = الانتقال إلى الإشارة المرجعية التالية
action-previous-bookmark = الانتقال إلى الإشارة المرجعية السابقة
action-add-note = إضافة ملاحظة إلى التحديد أو الجملة عند المؤشر
action-list-notes = سرد الملاحظات
action-next-note = الانتقال إلى الملاحظة التالية
action-previous-note = الانتقال إلى الملاحظة السابقة
action-delete-note = حذف الملاحظة أو التمييز عند المؤشر
action-highlight-selection = تمييز التحديد، أو الجملة عند المؤشر
action-export-study-sheet = تصدير الملاحظات والتمييزات كورقة دراسة بصيغة ماركداون، مجمّعة حسب العنوان
action-self-test = اختبر نفسك في الملاحظات والتمييزات: Enter يُظهر كل إجابة
action-open = فتح مستند
action-open-path = فتح مستند بكتابة مساره
action-open-library = فتح المكتبة: مستندات مجلدات مكتبتك والملفات الأخيرة
action-new-document = بدء مستند جديد في وضع التحرير
action-save = حفظ (ماركداون والنص في مكانه؛ الصيغ الأخرى كماركداون)
action-save-as = الحفظ باسم جديد
action-export-settings = تصدير الإعدادات وتجاوزات المفاتيح إلى ملف JSON أو TOML
action-import-settings = استيراد الإعدادات من ملف JSON أو TOML، بعد الإجابة بنعم أو لا
action-reading-statistics = سرد إحصاءات القراءة: الوقت المقروء وأبعد نقطة والجلسات والمستندات الأكثر قراءة
action-new-from-template = بدء مستند جديد من قالب، بعنوان ومؤلف وتاريخ وعنوان مراجع
action-export-html = تصدير المستند كصفحة ويب (HTML) بجانبه
action-export-pdf = تصدير المستند كملف PDF موسوم بجانبه
action-export-docx = تصدير المستند كملف Word ‏(DOCX) بجانبه
action-export-epub = تصدير المستند ككتاب EPUB بجانبه
action-export-brf = تصدير المستند كبرايل (BRF) بجانبه
action-preview-in-browser = معاينة المستند في متصفح الويب، مع الرياضيات؛ كل حفظ يعيد كتابة المعاينة
action-toggle-preview-auto-reload = تشغيل أو إيقاف إعادة التحميل التلقائي لمعاينة المتصفح
action-toggle-preview-live = تشغيل أو إيقاف المعاينة الحية: مع إعادة التحميل التلقائي، تُعاد المعاينة أيضًا عند توقف الكتابة
action-quit = الإنهاء، مع حفظ موضع القراءة
action-toggle-edit-mode = التبديل بين القراءة والتحرير
action-undo = تراجع
action-redo = إعادة
action-bold = جعل التحديد عريضًا
action-italic = جعل التحديد مائلًا
action-underline = تسطير التحديد
action-strikethrough = وضع خط يتوسط التحديد
action-inline-code = وسم التحديد كشيفرة
action-code-block = جعل الأسطر المحددة كتلة شيفرة
action-insert-link = جعل التحديد رابطًا
action-heading = جعل السطر الحالي عنوانًا
action-bullet-list = جعل الأسطر المحددة قائمة نقطية
action-numbered-list = جعل الأسطر المحددة قائمة مرقّمة
action-block-quote = جعل الأسطر المحددة اقتباسًا
action-horizontal-rule = إدراج خط أفقي
action-insert-table = إدراج جدول
action-add-table-row = إضافة صف إلى الجدول عند المؤشر
action-insert-image = إدراج صورة
action-replace = بحث واستبدال
action-copy = نسخ التحديد، أو الجملة عند المؤشر، إلى الحافظة
action-cut = قص التحديد إلى الحافظة
action-next-table-cell = في جدول، الانتقال إلى الخلية التالية ونطق عمودها؛ وفي غير ذلك، كتابة علامة جدولة
action-previous-table-cell = في جدول، الانتقال إلى الخلية السابقة ونطق عمودها
action-cycle-typing-echo = التنقل بين ترديد الكتابة: الحروف والكلمات، الحروف، الكلمات، أو لا شيء
action-select-all = تحديد كل النص
action-delete-word-before = حذف الكلمة قبل المؤشر
action-delete-word-after = حذف الكلمة بعد المؤشر
action-paste = لصق آخر نص نُسخ أو قُصّ في textweaver؛ يعمل لصق الطرفية أيضًا
action-insert-citation = إدراج استشهاد: اختيار مرجع، ثم إعطاء رقم صفحة أو موضع آخر
action-add-reference = إضافة مرجع إلى مكتبتك بـDOI أو ISBN
action-insert-bibliography = إدراج قائمة مراجع الأعمال المستشهد بها، عند المؤشر
action-check-citations = فحص الاستشهادات: عددها، وأي المفاتيح غير موجود في مكتبتك
action-import-references = استيراد مراجع من ملف BibTeX أو RIS أو CSL-JSON إلى مكتبتك
action-next-theme = التبديل إلى السمة اللونية التالية
action-toggle-line-numbers = إظهار أو إخفاء أرقام الأسطر
action-toggle-character-keys = تشغيل أو إيقاف الاختصارات أحادية المفتاح، حتى لا يفعّل الإملاء والكتابة أي أوامر
action-cycle-access-mode = التنقل بين أوضاع إمكانية الوصول: النطق الذاتي، أو المختلط، أو قارئ الشاشة
action-settings-profiles = سرد ملفات الإعدادات: التبديل إلى أحدها، أو حفظ الإعدادات الحالية كملف، أو إعادة التسمية، أو الحذف، أو الاستيراد، أو التصدير
action-bionic-toggle = تشغيل أو إيقاف القراءة البيونية: بداية كل كلمة بخط عريض
action-ruler-cycle = التنقل بين مسطرة القراءة: إيقاف، السطر الحالي، مسطرة
action-syllables-toggle = إظهار أو إخفاء المقاطع: تُقسَّم الكلمات بنقطة وسطى
action-difficult-words-toggle = تشغيل أو إيقاف وسم الكلمات الصعبة: تُسطَّر، وتُذكر عند التنقل بين الكلمات في مستوى التفصيل المرتفع
action-text-larger = تكبير نص المستند
action-text-smaller = تصغير نص المستند
action-text-size-reset = إعادة نص المستند إلى حجمه العادي
action-choose-font = اختيار خط نص المستند
action-contents-panel = إظهار لوحة المحتويات بجانب المستند والانتقال إليها، أو إغلاقها من داخلها: Enter ينتقل إلى عنوان
action-notes-panel = إظهار لوحة الملاحظات بجانب المستند والانتقال إليها، أو إغلاقها من داخلها: Enter ينتقل إلى ملاحظة
action-toggle-header = إظهار الترويسة أو إخفاؤها، شريط الأوامر فوق المستند
action-toggle-toolbar = إظهار شريط الأدوات أو إخفاؤه، شريط أزرار القراءة
action-show-preview = إظهار المعاينة بجانب المحرر أو إخفاؤها في وضع التحرير: المستند كما يُقرأ، ويُحدَّث عند توقف الكتابة
action-next-region = الانتقال إلى الجزء التالي من النافذة: الترويسة، أو اللوحة، أو المستند، أو شريط الأدوات
action-previous-region = الانتقال إلى الجزء السابق من النافذة
action-command-palette = تشغيل أي أمر بالاسم
action-settings = فتح الإعدادات: كل خيار مع مساعدته؛ يسار ويمين لتغيير قيمة
action-keyboard-help = سرد اختصارات لوحة المفاتيح
action-help = فتح المساعدة

## لغة الواجهة. $language هو اسم اللغة بذاتها (Español)، $voice اسم صوت.

language-voice-changed = الصوت الآن { $voice }، للغة { $language }.
language-voice-kept = لا يوجد صوت للغة { $language } في محرك الكلام هذا، لذا يستمر { $voice } في التحدث.
language-list-title = اللغة
language-list-intro =
    { $n ->
        [one] اللغة، خيار واحد. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
        [two] اللغة، خياران. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
        [few] اللغة، { $n } خيارات. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
        [many] اللغة، { $n } خيارًا. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
       *[other] اللغة، { $n } خيار. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
    }

## إعادة تشغيل الكلام.

restart-silent-now = { -brand } صامت الآن؛ أعد تشغيله لسماع الكلام مجددًا.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = { -brand } صامت الآن. أعد تشغيل الكلام بـ{ $keys }.
restart-restarting = إعادة تشغيل الكلام.
restart-not-here = لا يمكن إعادة تشغيل الكلام هنا.
restart-already = الكلام تجري إعادة تشغيله بالفعل.
# $error is the system's reason, in its own words.
restart-failed = تعذّرت إعادة تشغيل الكلام: { $error } انتظر قليلًا، ثم حاول مرة أخرى.
restart-start-failed = تعذّرت إعادة تشغيل الكلام: فشل بدء تشغيله.
restart-no-engine = لا يتوفر محرك كلام؛ يبقى { -brand } صامتًا. راجع Troubleshooting، No speech at all، في الوثائق.
restart-done-silent = أُعيد تشغيل الكلام، لكن لا يتوفر محرك كلام؛ يبقى { -brand } صامتًا.
restart-done = أُعيد تشغيل الكلام.

## إكمال المسارات تلقائيًا في الطلبات.

# $folder is the folder's full path.
pathc-no-folder = لا يوجد مجلد { $folder }.
# $prefix is what was typed after the last separator.
pathc-no-match = لا يوجد ملف أو مجلد يبدأ بـ{ $prefix }.
# The one name that matched; $kind is folder or file.
pathc-one =
    { $kind ->
        [folder] { $name }، مجلد
       *[file] { $name }، ملف
    }
# $n names matched; $names are the first few, joined with commas; $more is
# yes when more matched than are read out.
pathc-many =
    { $more ->
        [yes] { $n } تطابقات: { $names }، والمزيد.
       *[no] { $n } تطابقات: { $names }.
    }

## تصدير الإعدادات واستيرادها.

# $n settings differ; $name is the file's name.
settingsio-import-question =
    { $n ->
        [one] استيراد إعداد واحد مُغيَّر من { $name }؟ y أو n
        [two] استيراد إعدادين مُغيَّرين من { $name }؟ y أو n
        [few] استيراد { $n } إعدادات مُغيَّرة من { $name }؟ y أو n
        [many] استيراد { $n } إعدادًا مُغيَّرًا من { $name }؟ y أو n
       *[other] استيراد { $n } إعداد مُغيَّر من { $name }؟ y أو n
    }
settingsio-no-persistence = لا تُحفظ الإعدادات في هذه الجلسة، لذا لا يمكن تصديرها أو استيرادها.
# $path is the file written.
settingsio-exported = صُدِّرت الإعدادات إلى { $path }.
settingsio-export-failed = تعذّر تصدير الإعدادات: { $error } تحقّق من إمكانية الكتابة في المجلد.
# $path is the file; $error the system's reason.
settingsio-read-failed = تعذّرت قراءة { $path }: { $error } تحقّق من الملف، ثم استورد مرة أخرى.
settingsio-nothing-to-import = لا شيء لاستيراده: إعداداتك مطابقة لذلك الملف بالفعل.
settingsio-cancelled-unchanged = أُلغي. لم يتغيّر شيء.
settingsio-import-failed = تعذّر استيراد الإعدادات: { $error } تحقّق من إمكانية الكتابة في مجلد الإعدادات.
# $summary lists what changed (from the settings store, in English).
settingsio-imported = استُوردت الإعدادات. { $summary }
settingsio-backend-next-start = يُستخدم محرك الكلام الجديد من بدء التشغيل التالي.
settingsio-backed-up = نُسخت الإعدادات القديمة احتياطيًا.

## فتح مستند: الإخفاقات والفتح في الخلفية.

# $name is the file's name.
opening-is-folder = { $name } مجلد، وليس مستندًا. أعطِ اسم ملف بداخله.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = لا يوجد ملف باسم { $name } في { $folder }. تحقّق من الاسم.
opening-no-file-here = لا يوجد ملف باسم { $name } هنا. تحقّق من الاسم.
opening-no-permission = ليست لديك صلاحية قراءته.
opening-damaged-rtf = ليس ملف RTF قابلًا للقراءة؛ قد يكون تالفًا.
opening-damaged-odt = ليس ملف نص OpenDocument قابلًا للقراءة؛ قد يكون تالفًا.
opening-damaged-latex = ليس ملف LaTeX قابلًا للقراءة؛ قد يكون تالفًا أو كبيرًا جدًا.
opening-damaged-email = ليست رسالة بريد إلكتروني قابلة للقراءة؛ قد تكون تالفة أو كبيرة جدًا.
opening-damaged-mhtml = ليس أرشيف ويب قابلًا للقراءة؛ قد يكون تالفًا أو كبيرًا جدًا.
opening-pdf-password = إنه محمي بكلمة مرور. أزل كلمة المرور في برنامج PDF، ثم افتحه مرة أخرى.
opening-old-office = إنه ملف Microsoft Office قديم. احفظه بتنسيق أحدث مثل docx، ثم افتح الملف الجديد.
opening-rar = إنه أرشيف RAR لا يُفتح. استخرجه أولًا، أو استخدم ZIP أو 7z.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = تعذّر فتح { $name }: { $reason }
opening-started = يجري فتح { $name }. Escape للإلغاء.
opening-stopped = توقّف فتح { $name }.
opening-still = ما زال { $name } قيد الفتح، { $secs } ثانية.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = ما زال { $name } قيد الفتح: { $step }
opening-stopped-unexpectedly = تعذّر فتح { $name }: توقف التحميل بشكل غير متوقع.

## بناء دون ميزة النشر. "tw convert" أمر يُكتب في الطرفية: أبقِ عليه كما هو.

lean-citations-not-in-build = الاستشهادات غير متوفرة في نسخة { -brand } هذه.
lean-publish-not-in-build = التصدير والمعاينة غير متوفرين في نسخة { -brand } هذه. ما زال tw convert يحوّل.

## قائمة مدير الأصوات.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] صوت واحد: { $language }، { $engine }.
        [two] صوتان: { $language }، { $engine }.
        [few] { $n } أصوات: { $language }، { $engine }.
        [many] { $n } صوتًا: { $language }، { $engine }.
       *[other] { $n } صوت: { $language }، { $engine }.
    }
voices-all-languages = كل اللغات
voices-all-engines = كل المحركات
# The filter rows at the top of the list.
voices-language-row = اللغة: { $language }
voices-engine-row = المحرك: { $engine }
voices-fetch-row = جلب قائمة أصوات Piper من الإنترنت
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = التنزيل { $size }
voices-licence-public-domain = ملك عام
voices-licence-attribution = مجاني مع ذكر المصدر
voices-licence-share-alike = مجاني مع ذكر المصدر، مشاركة بالمثل
voices-licence-non-commercial = غير تجاري
voices-licence-unknown = الترخيص يُعرض قبل التنزيل
voices-favourite = مفضّل
voices-current = الحالي

## أسماء اللغات في مرشِّح لغة مدير الأصوات.

voices-language-ar = العربية
voices-language-ca = الكتالانية
voices-language-cs = التشيكية
voices-language-cy = الويلزية
voices-language-da = الدنماركية
voices-language-de = الألمانية
voices-language-el = اليونانية
voices-language-en = الإنجليزية
voices-language-es = الإسبانية
voices-language-fa = الفارسية
voices-language-fi = الفنلندية
voices-language-fr = الفرنسية
voices-language-hi = الهندية
voices-language-hu = الهنغارية
voices-language-is = الآيسلندية
voices-language-it = الإيطالية
voices-language-ja = اليابانية
voices-language-ka = الجورجية
voices-language-kk = الكازاخية
voices-language-ko = الكورية
voices-language-lb = اللوكسمبورغية
voices-language-lv = اللاتفية
voices-language-nl = الهولندية
voices-language-no = النرويجية
voices-language-pl = البولندية
voices-language-pt = البرتغالية
voices-language-ro = الرومانية
voices-language-ru = الروسية
voices-language-sk = السلوفاكية
voices-language-sl = السلوفينية
voices-language-sr = الصربية
voices-language-sv = السويدية
voices-language-sw = السواحلية
voices-language-tr = التركية
voices-language-uk = الأوكرانية
voices-language-vi = الفيتنامية
voices-language-zh = الصينية

## الأصوات: مدير الأصوات، والسرعة، والحدة، والمستوى.

# Spoken by a newly chosen voice as its sample.
voice-sample = الثعلب البني السريع يقفز فوق الكلب الكسول.
voice-list-title = اختيار صوت
voice-still-loading = ما زالت الأصوات قيد التحميل. تُفتح القائمة عندما تكون جاهزة.
voice-list-failed = تعذّر سرد الأصوات: { $error } اختر محركًا آخر من قائمة الكلام.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = مدير الأصوات. { $shown } Enter لاستخدام صوت ونطق عينة منه، أو لتنزيله؛ { $preview } لمعاينة صوت؛ Space لوضع علامة مفضّل؛ Delete لإزالة صوت مُنزَّل؛ Escape للإغلاق.
voice-more-ready =
    { $n ->
        [one] أُضيف إلى القائمة صوت آخر من محرك آخر.
        [two] أُضيف إلى القائمة صوتان آخران من محركات أخرى.
        [few] أُضيفت إلى القائمة { $n } أصوات أخرى من محركات أخرى.
       *[other] أُضيف إلى القائمة { $n } صوتًا آخر من محركات أخرى.
    }
voice-preview = معاينة: { $voice }.
voice-preview-sample = { $voice }. الثعلب البني السريع يقفز فوق الكلب الكسول.
voice-preview-starting = معاينة: { $voice }، جارٍ تشغيل { $engine }.
voice-preview-not-installed = { $voice } لم يُنزَّل بعد. Enter ينزّله بعد سؤال.
voice-preview-unavailable = لا يمكن تشغيل { $engine } هنا للمعاينة. Enter ينتقل إليه.
voice-preview-engine-failed = تعذّر تشغيل { $engine } لمعاينة { $voice }.
voice-preview-failed = تعذّرت معاينة { $voice }: { $error } جرّب صوتًا آخر.
# $keys names the Choose Voice key.
voice-ready = الأصوات جاهزة. { $keys } لسردها.
voice-fetch-catalog-question = تنزيل قائمة أصوات Piper، نحو 250 كيلوبايت، من Hugging Face؟ y أو n
voice-fetch-catalog-question-short = تنزيل قائمة أصوات Piper؟ y أو n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = الصوت { $voice }، على { $engine }. تبديل المحرك.
voice-download-in-progress = تنزيل صوت قيد التقدّم بالفعل.
voice-no-data-folder = لا يوجد مجلد بيانات لحفظ أصوات Piper فيه.
voice-not-in-list = لم يعد ذلك الصوت في قائمة أصوات Piper.
voice-download-start-failed = تعذّر بدء التنزيل.
voice-reading-licence = قراءة ترخيص { $voice }.
voice-remove-question = إزالة الصوت { $voice }؟ y أو n
voice-only-piper-removable = يمكن إزالة أصوات Piper المُنزَّلة فقط.
# $plan describes the download: the voice, its size and licence.
voice-download-question = { $plan } y أو n
voice-in-use = { $voice } هو الصوت قيد الاستخدام. اختر صوتًا آخر أولًا.
voice-removed = أُزيل { $voice }.
voice-remove-failed = تعذّرت إزالة { $voice }: { $error } تحقّق من إمكانية الكتابة في مجلده.
voice-downloading-catalog = تنزيل قائمة أصوات Piper.
voice-downloading = تنزيل { $voice }.
voice-downloading-percent = تنزيل { $voice }، { $pct } بالمئة.
voice-details-failed = تعذّرت قراءة تفاصيل الصوت: { $error } تحقّق من الاتصال، ثم حاول مرة أخرى.
voice-download-stopped = توقّف التنزيل.
voice-catalog-fetched = تحتوي قائمة أصوات Piper على { $voices } صوتًا في { $languages } لغة. يسردها أمر الأصوات.
voice-catalog-failed = تعذّر تنزيل قائمة الأصوات: { $error } تحقّق من الاتصال، ثم حاول مرة أخرى.
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } مثبَّت. { $licence } يسرده أمر الأصوات.
voice-download-failed = تعذّر تنزيل { $voice }: { $error } اختر الصوت مرة أخرى لإعادة المحاولة.
voice-only-voice-favourite = يمكن أن يكون صوت فقط مفضّلًا.
voice-favourite-added = أُضيف { $voice } إلى المفضّلة.
voice-favourite-removed = أُزيل { $voice } من المفضّلة.
voice-chosen = الصوت { $voice }.
voice-chosen-rate = الصوت { $voice }، { $wpm } كلمة في الدقيقة.
voice-fastest-rate = أسرع سرعة.
voice-slowest-rate = أبطأ سرعة.
voice-rate = { $wpm } كلمة في الدقيقة.
voice-highest-pitch = أعلى حدة.
voice-lowest-pitch = أدنى حدة.
voice-pitch-normal = حدة عادية.
# $n is a number of semitones.
voice-pitch-plus = الحدة زائد { $n }.
voice-pitch-minus = الحدة ناقص { $n }.
voice-full-volume = أقصى مستوى صوت.
voice-volume-off = الصوت متوقف.
voice-volume = مستوى الصوت { $pct } بالمئة.
voice-no-speed-presets = لا توجد إعدادات سرعة جاهزة.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = { $name }، السرعة { $wpm }.
voice-line-numbers-on = أرقام الأسطر مفعّلة.
voice-line-numbers-off = أرقام الأسطر متوقفة.

## التصدير والمعاينة من القارئ. F5 هو مفتاح إعادة التحميل في المتصفح،
## وليس مفتاحًا في textweaver.

common-no-document = لا يوجد مستند مفتوح.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = تعذّرت الكتابة إلى { $path }: { $error } تحقّق من إمكانية الكتابة في المجلد.
publish-cannot-write = تعذّرت كتابة { $path }: { $error } تحقّق من إمكانية الكتابة في مجلده.
publish-start-failed = تعذّر بدء التصدير: { $error } انتظر قليلًا، ثم حاول مرة أخرى.
publish-export-error = تعذّر التصدير: { $error } أصلح ذلك، ثم صدّر مرة أخرى.
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = التصدير إلى { $format }.
publish-theme-title = سمة صفحة HTML
publish-theme-intro = سمة صفحة HTML؟ { $first } أولًا، { $n } خيارات. Escape للإلغاء.
publish-writing-preview = كتابة المعاينة.
publish-preview-error = تعذّرت كتابة المعاينة: { $error } أصلح ذلك، ثم افتح المعاينة مرة أخرى.
publish-still-exporting =
    { $secs ->
        [one] ما زال التصدير إلى { $format } جاريًا، ثانية واحدة.
        [two] ما زال التصدير إلى { $format } جاريًا، ثانيتان.
        [few] ما زال التصدير إلى { $format } جاريًا، { $secs } ثوانٍ.
        [many] ما زال التصدير إلى { $format } جاريًا، { $secs } ثانية.
       *[other] ما زال التصدير إلى { $format } جاريًا، { $secs } ثانية.
    }
publish-still-previewing =
    { $secs ->
        [one] ما زالت كتابة المعاينة جارية، ثانية واحدة.
        [two] ما زالت كتابة المعاينة جارية، ثانيتان.
        [few] ما زالت كتابة المعاينة جارية، { $secs } ثوانٍ.
        [many] ما زالت كتابة المعاينة جارية، { $secs } ثانية.
       *[other] ما زالت كتابة المعاينة جارية، { $secs } ثانية.
    }
# $again is yes when a preview is open already.
publish-auto-reload-on =
    { $again ->
        [yes] إعادة التحميل التلقائي للمعاينة مفعّلة: بعد كل حفظ يعيد المتصفح تحميل الصفحة بنفسه. شغّل معاينة في المتصفح مجددًا لاستخدامها.
       *[no] إعادة التحميل التلقائي للمعاينة مفعّلة: بعد كل حفظ يعيد المتصفح تحميل الصفحة بنفسه.
    }
publish-auto-reload-off = إعادة التحميل التلقائي للمعاينة متوقفة: اضغط F5 في المتصفح بعد الحفظ.
publish-live-on = المعاينة الحية مفعّلة: تُعاد المعاينة أيضًا عند توقف الكتابة.
# "toggle preview auto reload" is the command's name in the command palette.
publish-live-on-needs-reload = المعاينة الحية مفعّلة. تعمل مع إعادة التحميل التلقائي، وهي متوقفة؛ فعّلها بأمر إعادة تحميل المعاينة تلقائيًا.
publish-live-off = المعاينة الحية متوقفة: تُعاد المعاينة بعد الحفظ فقط.
# $error is the converter's reason.
publish-export-failed = فشل التصدير إلى { $format }: { $error } جرّب تنسيقًا آخر.
publish-preview-failed = فشلت المعاينة: { $error } احفظ للمحاولة مرة أخرى.
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] تحذير واحد: { $first }
        [two] تحذيران: { $first }
        [few] { $n } تحذيرات؛ الأول: { $first }
        [many] { $n } تحذيرًا؛ الأول: { $first }
       *[other] { $n } تحذير؛ الأول: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = صُدِّر إلى { $format }: { $file }. فتحه؟ y أو n. في { $folder }.{ $warned }
publish-report = حُفظ التقرير باسم { $file }.
publish-report-issues = عناصر لم تُجعل متاحة: { $n }. انظر التقرير.
publish-report-failed = تعذّر حفظ التقرير: { $error } تحقّق من إمكانية الكتابة في مجلد الإخراج.
publish-preview-written-served = كُتبت المعاينة. يجري فتحها في المتصفح. تُعاد تلقائيًا بعد كل حفظ.{ $warned }
publish-preview-written = كُتبت المعاينة. يجري فتحها في المتصفح. الحفظ يكتبها مجددًا؛ ثم اضغط F5 في المتصفح.{ $warned }
publish-preview-updated = تحدّثت المعاينة.
publish-preview-updated-press-f5 = تحدّثت المعاينة. اضغط F5 في المتصفح.
publish-server-failed = تعذّر بدء خادم إعادة تحميل المعاينة ({ $error })؛ يجري فتح الملف بدلًا من ذلك.
publish-render-failed = تعذّر عرض النص: { $error } اخرج من وضع التحرير لقراءة النص.
publish-nothing-after-caret = لا شيء لقراءته بعد المؤشر.
publish-listening = الاستماع إلى النص المعروض.

## خادم إعادة تحميل المعاينة: يظهر في المتصفح.

preview-being-written = المعاينة قيد الكتابة. أعد التحميل بعد لحظة.

## الملاحظات والتمييزات.

notes-nothing-to-attach = لا شيء هنا لإرفاق ملاحظة به.
# $on is the start of the passage the note is on.
notes-added = أُضيفت ملاحظة عند: { $on }
# $tags are the note's tags, joined with commas.
notes-added-with-tags = أُضيفت ملاحظة بالوسوم { $tags } عند: { $on }
# An item in the notes list. $anchor is the passage; $lost is yes when the
# passage was not found after the file changed.
notes-item =
    { $lost ->
        [yes] { $note }، السطر { $line }. عند: { $anchor } لم يُعثر عليها بعد تغيّر الملف.
       *[no] { $note }، السطر { $line }. عند: { $anchor }
    }
notes-none = لا ملاحظات. لإضافة واحدة: { $key }.
notes-list-title = الملاحظات
notes-list-intro =
    { $n ->
        [one] الملاحظات، عنصر واحد. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها.
        [two] الملاحظات، عنصران. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها.
        [few] الملاحظات، { $n } عناصر. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها.
        [many] الملاحظات، { $n } عنصرًا. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها.
       *[other] الملاحظات، { $n } عنصر. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها.
    }
# Said on jumping to a note: its text, then the passage it is on.
notes-note-content = { $note }. عند: { $anchor }
# $i is the note's number, $n how many notes there are.
notes-note-label = الملاحظة { $i } من { $n }
notes-deleted = حُذفت الملاحظة: { $text }.
notes-none-here = لا ملاحظة أو تمييز هنا.
notes-unchanged = لم تتغيّر الملاحظة.
notes-updated = تحدّثت الملاحظة.
notes-nothing-to-highlight = لا شيء هنا لتمييزه.
notes-highlight-removed = أُزيل التمييز: { $text }
notes-highlighted-at = مُيِّز عند { $pct } بالمئة: { $text }
notes-highlighted = مُيِّز: { $text }
# An item in the highlights list. $color is the highlight's color name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $text }، السطر { $line }، { $color }، لم يُعثر عليه بعد تغيّر الملف
       *[no] { $text }، السطر { $line }، { $color }
    }
notes-no-highlights = لا تمييزات. لإنشاء تمييز: { $key }.
notes-highlights-title = التمييزات
notes-highlights-intro =
    { $n ->
        [one] التمييزات، عنصر واحد. Enter للانتقال إلى أحدها، Delete لإزالته.
        [two] التمييزات، عنصران. Enter للانتقال إلى أحدها، Delete لإزالته.
        [few] التمييزات، { $n } عناصر. Enter للانتقال إلى أحدها، Delete لإزالته.
        [many] التمييزات، { $n } عنصرًا. Enter للانتقال إلى أحدها، Delete لإزالته.
       *[other] التمييزات، { $n } عنصر. Enter للانتقال إلى أحدها، Delete لإزالته.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = تمييز
# Shown while reading reaches a note's passage.
notes-signal = ملاحظة: { $text }
# Said after moving onto a note's passage.
notes-has-note = تحتوي على ملاحظة: { $text }

## Relations between notes (the knowledge graph as lists).

relations-type-conflicts-with = يتعارض مع
relations-type-supports = يدعم
relations-type-is-example-of = مثال على
relations-type-cites = يستشهد بـ
relations-type-contradicts = يناقض
relations-type-defines = يعرّف
relations-type-extends = يوسّع
relations-type-see-also = انظر أيضًا
relations-type-precedes = يسبق
relations-type-follows = يلي
relations-count = الروابط: { $out } صادرة، { $in } واردة.
relations-note-title = روابط: { $note }
relations-title-filtered = { $title }، التصفية: { $filter }
relations-note-intro = روابط { $note }: { $out } صادرة، { $in } واردة. Enter لاتباع رابط، F2 لتغييره، Delete لإزالته. اكتب للتصفية حسب النوع.
relations-out-item = { $type }: { $target }
relations-target-in = { $note }، في { $doc }
relations-target-missing = ملاحظة غير موجودة
relations-empty-note = ملاحظة فارغة
relations-incoming-row =
    { $n ->
        [0] ما يرتبط هنا: لا شيء بعد
        [one] ما يرتبط هنا: ملاحظة واحدة
        [two] ما يرتبط هنا: ملاحظتان
       *[other] ما يرتبط هنا: { $n } ملاحظات
    }
relations-add-row = إضافة رابط
relations-backlinks-title = ما يرتبط هنا: { $note }
relations-backlinks-intro =
    { $n ->
        [one] ما يرتبط بـ { $note }: ملاحظة واحدة. Enter للانتقال إليها. اكتب للتصفية حسب النوع.
       *[other] ما يرتبط بـ { $note }: { $n } ملاحظات. Enter للانتقال إلى إحداها. اكتب للتصفية حسب النوع.
    }
relations-backlink-item = { $type } هذه، من: { $note }
relations-none-in = لا شيء يرتبط بهذه الملاحظة بعد.
relations-types-title = نوع الرابط لـ: { $note }
relations-types-intro = اختر نوع الرابط، 10 أنواع. اكتب للتصفية.
relations-targets-title = { $type }: أي ملاحظة؟
relations-targets-intro =
    { $n ->
        [0] لا توجد ملاحظة أخرى هنا. اختر ملاحظة في مستند آخر.
        [one] اختر الملاحظة المراد الربط بها: ملاحظة واحدة. Enter للربط.
       *[other] اختر الملاحظة المراد الربط بها: { $n } ملاحظات. Enter للربط.
    }
relations-other-document-row = ملاحظة في مستند آخر
relations-documents-title = مستندات بها ملاحظات
relations-documents-intro = مستندات بها ملاحظات: { $n }. Enter لعرض ملاحظات مستند.
relations-document-item =
    { $n ->
        [one] { $title }، ملاحظة واحدة
       *[other] { $title }، { $n } ملاحظات
    }
relations-no-other-documents = لا يوجد مستند آخر في المكتبة به ملاحظات.
relations-linked = تم الربط: { $type } { $target }.
relations-changed = تم تغيير الرابط: { $type } { $target }.
relations-already = مرتبط من قبل: { $type } { $target }.
relations-removed = تمت إزالة الرابط: { $type } { $target }.
relations-remove-question = إزالة هذا الرابط؟ y أو n
relations-nothing-to-remove = يمكن إزالة رابط فقط هنا.
relations-note-gone = الملاحظة غير موجودة: حُذفت، أو اختفى مستندها.
relations-document-missing = المستند غير موجود: { $file }.
relations-no-note-here = لا توجد ملاحظة هنا. الروابط تخص الملاحظات؛ أضف واحدة: { $key }.
relations-filter-cleared = تم مسح التصفية، { $n } معروضة.
relations-filter-none = لا شيء يطابق { $filter }.
relations-filter-matched = التصفية { $filter }: { $n } معروضة.

## الإشارات المرجعية: إعادة التسمية والحذف.

notes-choose-bookmark-delete = اختر إشارة مرجعية واضغط Delete.
notes-choose-bookmark-rename = اختر إشارة مرجعية واضغط F2 لإعادة تسميتها.
notes-bookmark-deleted = حُذفت الإشارة المرجعية { $name }.
notes-renaming-bookmark = إعادة تسمية الإشارة المرجعية { $name }.
notes-bookmark-unchanged = لم تتغيّر الإشارة المرجعية.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = توجد بالفعل إشارة مرجعية باسم { $name }. لم تتغيّر الإشارة المرجعية { $old }.
notes-bookmark-renamed = أُعيدت تسمية { $old } إلى { $name }.

## ورقة الدراسة.

notes-nothing-to-export = لا ملاحظات أو تمييزات لتصديرها.
# Keep the letters y and n: they are the keys that answer. $file is the
# sheet's file name, $folder the folder it was saved in.
notes-study-sheet-saved-notes =
    ورقة دراسة تحتوي { $n ->
        [one] ملاحظة واحدة
        [two] ملاحظتين
        [few] { $n } ملاحظات
        [many] { $n } ملاحظة
       *[other] { $n } ملاحظة
    } حُفظت باسم { $file }. فتحها؟ y أو n. في { $folder }.
notes-study-sheet-saved-highlights =
    ورقة دراسة تحتوي { $h ->
        [one] تمييزًا واحدًا
        [two] تمييزين
        [few] { $h } تمييزات
        [many] { $h } تمييزًا
       *[other] { $h } تمييز
    } حُفظت باسم { $file }. فتحها؟ y أو n. في { $folder }.
notes-study-sheet-saved-both =
    ورقة دراسة تحتوي { $n ->
        [one] ملاحظة واحدة
        [two] ملاحظتين
        [few] { $n } ملاحظات
        [many] { $n } ملاحظة
       *[other] { $n } ملاحظة
    } و{ $h ->
        [one] تمييزًا واحدًا
        [two] تمييزين
        [few] { $h } تمييزات
        [many] { $h } تمييزًا
       *[other] { $h } تمييز
    } حُفظت باسم { $file }. فتحها؟ y أو n. في { $folder }.
notes-study-sheet-failed = تعذّرت كتابة ورقة الدراسة: { $error } تحقّق من إمكانية الكتابة في المجلد.
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = ورقة دراسة: { $title }
notes-sheet-exported = صُدِّرت من { -brand } في { $date }.
notes-sheet-before-first-heading = قبل العنوان الأول
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (الوسوم: { $tags })
# $color is the highlight's color name.
notes-sheet-highlighted = مُيِّز، { $color }.

## الاختبار الذاتي: أسئلة بإجابات مخفية (crate::reveal).

reveal-self-test-title = اختبار ذاتي: { $title }
reveal-self-test-intro =
    { $n ->
        [one] اختبار ذاتي، سؤال واحد. Enter يُظهر الإجابة. مسافة للإجابة بصوت عالٍ.
        [two] اختبار ذاتي، سؤالان. Enter يُظهر كل إجابة. مسافة للإجابة بصوت عالٍ.
        [few] اختبار ذاتي، { $n } أسئلة. Enter يُظهر كل إجابة. مسافة للإجابة بصوت عالٍ.
        [zero] اختبار ذاتي، { $n } سؤال. Enter يُظهر كل إجابة. مسافة للإجابة بصوت عالٍ.
       *[other] اختبار ذاتي، { $n } سؤالًا. Enter يُظهر كل إجابة. مسافة للإجابة بصوت عالٍ.
    }
reveal-nothing-to-test = لا ملاحظات أو تمييزات للاختبار. أضف ملاحظة أو تمييزًا أولًا.
reveal-prompt-note = { $note } (في { $section })
reveal-prompt-highlight = ماذا ميّزت في { $section }؟
reveal-row-shown = { $prompt } الإجابة: { $answer }
reveal-answer = الإجابة: { $answer }
reveal-listening = أجب بصوت عالٍ الآن. مسافة للإيقاف.
reveal-you-said = قلت: { $words }. Enter يُظهر الإجابة.
reveal-heard-nothing = لم تُسمع إجابة. مسافة للمحاولة مرة أخرى.
reveal-no-dictation = الإجابة بصوت عالٍ تحتاج الإملاء، وهو غير موجود في هذا الإصدار.

## البحث، والإشارات المرجعية، والتحديد.

marks-cannot-search = تعذّر البحث: { $error } تحقّق من النمط بين الشرطتين المائلتين.
# $pattern is the text searched for.
marks-no-matches = لا تطابقات لـ{ $pattern }.
# The label of a match reached by Find, at high verbosity; $number is its place among $n matches.
marks-match-label = التطابق { $number } من { $n }
# Find wrapped past an end of the document; $dir is next (to the top) or previous (to the bottom); $message says the match.
marks-find-wrapped =
    { $dir ->
        [next] التف إلى الأعلى. { $message }
       *[previous] التف إلى الأسفل. { $message }
    }
# $name is the bookmark's name, such as mark1.
marks-bookmark-already-here = الإشارة المرجعية { $name } موجودة هنا بالفعل.
common-bookmark-set = وُضعت الإشارة المرجعية { $name } عند { $pct } بالمئة.
marks-no-bookmarks = لا إشارات مرجعية. لإضافة واحدة: { $key }.
marks-bookmarks-intro =
    { $n ->
        [one] الإشارات المرجعية، عنصر واحد. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
        [two] الإشارات المرجعية، عنصران. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
        [few] الإشارات المرجعية، { $n } عناصر. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
        [many] الإشارات المرجعية، { $n } عنصرًا. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
       *[other] الإشارات المرجعية، { $n } عنصر. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
    }
# One line of the bookmark list; $lost is yes when the bookmark's text was not found after the file changed; $text is the start of its line.
marks-bookmark-item =
    { $lost ->
        [yes] { $name } (لم يُعثر عليها بعد تغيّر الملف)، السطر { $line }، { $pct } بالمئة: { $text }
       *[no] { $name }، السطر { $line }، { $pct } بالمئة: { $text }
    }
marks-bookmarks-title = الإشارات المرجعية
# The label of a bookmark reached, at high verbosity.
marks-bookmark-label = الإشارة المرجعية { $name }
marks-selection-cleared = مُسح التحديد.
# $text is the selected text, shortened.
marks-selected = حُدِّد { $text }

## قوائم التأليف: المخطط، وأداة اختيار الاستشهاد، والتدقيق الإملائي، والنحو، والقوالب.

# An outline item: $text is the heading's text, $level its level.
lists-outline-item = { $text }، المستوى { $level }
# $n is the number of headings.
lists-outline-title =
    { $n ->
        [one] المخطط، عنوان واحد
        [two] المخطط، عنوانان
        [few] المخطط، { $n } عناوين
        [many] المخطط، { $n } عنوانًا
       *[other] المخطط، { $n } عنوان
    }
# $shown headings of $n match the filter $filter typed so far.
lists-outline-title-filtered = المخطط، { $shown } من { $n } يطابق { $filter }
# $n is the number of references.
lists-citations-title =
    { $n ->
        [one] إدراج استشهاد، مرجع واحد
        [two] إدراج استشهاد، مرجعان
        [few] إدراج استشهاد، { $n } مراجع
        [many] إدراج استشهاد، { $n } مرجعًا
       *[other] إدراج استشهاد، { $n } مرجع
    }
# $shown references of $n match the filter $filter typed so far.
lists-citations-title-filtered = إدراج استشهاد، { $shown } من { $n } يطابق { $filter }
# $word is the misspelled word.
lists-spelling-title = تهجئة { $word }
lists-spelling-add = إضافة { $word } إلى قائمة كلماتك
lists-leave-as-is = تركها كما هي
# $words are the words the grammar fixes are for.
lists-grammar-title = إصلاحات نحوية لـ{ $words }
# $n is the number of templates.
lists-templates-title = مستند جديد من قالب، { $n } قوالب
lists-no-filter = هذه القائمة لا تُصفَّى.
# The filter was emptied: $n items are shown.
lists-filter-cleared-headings =
    { $n ->
        [one] مُسحت التصفية، عنوان واحد.
        [two] مُسحت التصفية، عنوانان.
        [few] مُسحت التصفية، { $n } عناوين.
        [many] مُسحت التصفية، { $n } عنوانًا.
       *[other] مُسحت التصفية، { $n } عنوان.
    }
lists-filter-cleared-references =
    { $n ->
        [one] مُسحت التصفية، مرجع واحد.
        [two] مُسحت التصفية، مرجعان.
        [few] مُسحت التصفية، { $n } مراجع.
        [many] مُسحت التصفية، { $n } مرجعًا.
       *[other] مُسحت التصفية، { $n } مرجع.
    }
lists-filter-cleared-items =
    { $n ->
        [one] مُسحت التصفية، عنصر واحد.
        [two] مُسحت التصفية، عنصران.
        [few] مُسحت التصفية، { $n } عناصر.
        [many] مُسحت التصفية، { $n } عنصرًا.
       *[other] مُسحت التصفية، { $n } عنصر.
    }
# Nothing matches the filter $query.
lists-filter-none-headings = لا عناوين تطابق { $query }. Backspace لحذف الحروف.
lists-filter-none-references = لا مراجع تطابق { $query }. Backspace لحذف الحروف.
lists-filter-none-items = لا عناصر تطابق { $query }. Backspace لحذف الحروف.
# $n items match the filter.
lists-filter-matched-headings =
    { $n ->
        [one] عنوان واحد مطابق.
        [two] عنوانان مطابقان.
        [few] { $n } عناوين مطابقة.
        [many] { $n } عنوانًا مطابقًا.
       *[other] { $n } عنوان مطابق.
    }
lists-filter-matched-references =
    { $n ->
        [one] مرجع واحد مطابق.
        [two] مرجعان مطابقان.
        [few] { $n } مراجع مطابقة.
        [many] { $n } مرجعًا مطابقًا.
       *[other] { $n } مرجع مطابق.
    }
lists-filter-matched-items =
    { $n ->
        [one] عنصر واحد مطابق.
        [two] عنصران مطابقان.
        [few] { $n } عناصر مطابقة.
        [many] { $n } عنصرًا مطابقًا.
       *[other] { $n } عنصر مطابق.
    }
lists-no-headings = لا عناوين في هذا المستند.
# $n is the number of headings; the keys are the outline list's own.
lists-outline-intro =
    { $n ->
        [one] المخطط، عنوان واحد. اكتب للتصفية، Enter للانتقال إلى عنوان، Escape للإغلاق.
        [two] المخطط، عنوانان. اكتب للتصفية، Enter للانتقال إلى عنوان، Escape للإغلاق.
        [few] المخطط، { $n } عناوين. اكتب للتصفية، Enter للانتقال إلى عنوان، Escape للإغلاق.
        [many] المخطط، { $n } عنوانًا. اكتب للتصفية، Enter للانتقال إلى عنوان، Escape للإغلاق.
       *[other] المخطط، { $n } عنوان. اكتب للتصفية، Enter للانتقال إلى عنوان، Escape للإغلاق.
    }
# $heading is the text of the heading the cursor is under.
lists-outline-here = أنت تحت { $heading }.

## القوائم والطلبات المشتركة بين كل واجهة.

# The focused list item: $item is its text, $k its place, $n the number of items.
listmodel-item-position = { $k } من { $n }، { $item }
# $letter is the letter or digit typed.
listmodel-no-item-starts = لا عنصر يبدأ بـ{ $letter }.
listmodel-top-of-list = أعلى القائمة.
listmodel-end-of-list = نهاية القائمة.
listmodel-no-matching-commands = لا أوامر مطابقة.
listmodel-no-earlier-entries = لا مدخلات أقدم.
# Tab in the command palette: $n commands match (always more than one); $names lists the first few, joined by commas.
listmodel-command-matches = { $n } تطابقات: { $names }.

## المكتبة.

# $n is how many documents the scan has found.
library-still-scanning = ما زال فحص المكتبة جاريًا: عُثر على { $n } حتى الآن.
library-scan-failed = تعذّر فحص المكتبة: { $error } تحقّق من مجلدات المكتبة في الإعدادات.
library-scanning = فحص المكتبة.
library-scan-progress = فحص المكتبة: عُثر على { $n } حتى الآن.
library-scan-stopped = توقّف فحص المكتبة على نحو غير متوقع. افتح المكتبة مرة أخرى لإعادة المحاولة.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = المكتبة فارغة. أضف مجلدًا من الإعدادات، تحت مجلدات المكتبة، أو افتح ملفًا بـ{ $key }.
library-add-folder-choose = اختر المجلد الذي ستضيفه إلى المكتبة
library-folder-added = أُضيف { $name } إلى المكتبة. افتح المكتبة لترى مستنداته.
library-folder-already = { $name } موجود في المكتبة بالفعل.
library-intro =
    { $n ->
        [one] المكتبة، مستند واحد. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل.
        [two] المكتبة، مستندان. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل.
        [few] المكتبة، { $n } مستندات. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل.
        [many] المكتبة، { $n } مستندًا. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل.
       *[other] المكتبة، { $n } مستند. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل.
    }
library-title = المكتبة

## اتباع الروابط والحواشي.

links-none-here = لا رابط أو حاشية عند المؤشر.
# $text is the link's text.
common-link-no-address = الرابط { $text } بلا عنوان.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] فتح رابط البريد؟ y أو n. { $target }
       *[web] فتح رابط الويب؟ y أو n. { $target }
    }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = عنوان
# $anchor is the heading name the link gives.
links-no-heading = لا عنوان باسم { $anchor } في هذا المستند.
# $file is the file the link names.
links-file-not-found = الرابط يذهب إلى { $file }، الذي لم يُعثر عليه.
# $file is the file's name; $key names the History Back command's keys.
links-followed = اتُّبع الرابط إلى { $file }. الرجوع: { $key }.
links-back-in = الرجوع في { $file }.
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = الرجوع إلى مرجع الحاشية { $label }، السطر { $line }.
# $text is the start of the note.
links-footnote = الحاشية { $label }: { $text }
links-footnote-unreferenced = لا مرجع للحاشية { $label } في النص.
links-footnote-no-note = الحاشية { $label } بلا ملاحظة.

## الاستشهادات: الإدراج، والبحث، والاستيراد، والفحص، وقائمة المراجع.

citations-on = الاستشهادات مفعّلة.
citations-off = الاستشهادات متوقفة.
# $key names the Add Reference command's keys.
citations-library-empty = مكتبة مراجعك فارغة. أضف مرجعًا بـDOI أو ISBN بـ{ $key }، أو شغّل استيراد مراجع من لوحة الأوامر.
# $n is how many references the picker lists.
citations-picker-intro =
    { $n ->
        [one] إدراج استشهاد، مرجع واحد. اكتب للتصفية، Enter للاختيار، Escape للإلغاء.
        [two] إدراج استشهاد، مرجعان. اكتب للتصفية، Enter للاختيار، Escape للإلغاء.
        [few] إدراج استشهاد، { $n } مراجع. اكتب للتصفية، Enter للاختيار، Escape للإلغاء.
        [many] إدراج استشهاد، { $n } مرجعًا. اكتب للتصفية، Enter للاختيار، Escape للإلغاء.
       *[other] إدراج استشهاد، { $n } مرجع. اكتب للتصفية، Enter للاختيار، Escape للإلغاء.
    }
# $text is what was typed at the locator prompt.
citations-locator-unreadable = تعذّرت قراءة الموضع { $text }. اكتب رقم صفحة مثل 12، أو صفحات مثل 3-5، أو chapter 2؛ Enter وحدها لعدم تحديد شيء.
citations-insert-failed = تعذّر إدراج الاستشهاد: { $error } لم يتغيّر النص.
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = البحث عن { $what }.
citations-lookup-not-started = تعذّر بدء البحث: { $error } انتظر قليلًا، ثم حاول مرة أخرى.
# $input is the DOI or ISBN as typed.
citations-lookup-failed = تعذّر البحث عن { $input }: { $error } تحقّق من رقم DOI أو ISBN ومن الاتصال.
citations-no-library-to-add-to = لا توجد مكتبة لإضافة إليها: لا يحتفظ { -brand } بملفات في هذه الجلسة.
citations-library-save-failed = تعذّر حفظ المكتبة: { $error } تحقّق من إمكانية الكتابة في مجلده.
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] عُثر على استشهاد واحد. لا يحتفظ { -brand } بمكتبة في هذه الجلسة.
        [two] عُثر على استشهادين. لا يحتفظ { -brand } بمكتبة في هذه الجلسة.
        [few] عُثر على { $n } استشهادات. لا يحتفظ { -brand } بمكتبة في هذه الجلسة.
        [many] عُثر على { $n } استشهادًا. لا يحتفظ { -brand } بمكتبة في هذه الجلسة.
       *[other] عُثر على { $n } استشهاد. لا يحتفظ { -brand } بمكتبة في هذه الجلسة.
    }
citations-check-failed = تعذّر فحص الاستشهادات: { $error } تحقّق من ملف مكتبة المراجع.
citations-no-library-to-import-into = لا توجد مكتبة لاستيراد إليها: لا يحتفظ { -brand } بملفات في هذه الجلسة.
# $file is the file's path.
citations-import-failed = تعذّر استيراد { $file }: { $error } تحقّق من أنه ملف بامتداد bib أو ris أو json.
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = لا يمكن استخدام نمط الاستشهاد { $style }: { $error } تحقّق من اسم النمط في أعلى المستند.
citations-format-failed = تعذّر تنسيق الاستشهادات: { $error } تحقّق من مفاتيح الاستشهاد ومن المكتبة.
# $key names the Insert Citation command's keys.
citations-none-yet = لا استشهادات في المستند بعد. أدرج واحدًا بـ{ $key }.
citations-nothing-to-list = لا يوجد أي عمل مستشهد به في مكتبتك، فلا شيء لسرده.
# $n is how many entries went in; $style is the style's name, such as apa.
citations-bibliography-inserted =
    { $n ->
        [one] أُدرجت قائمة المراجع، مدخل واحد، نمط { $style }.
        [two] أُدرجت قائمة المراجع، مدخلان، نمط { $style }.
        [few] أُدرجت قائمة المراجع، { $n } مدخلات، نمط { $style }.
        [many] أُدرجت قائمة المراجع، { $n } مدخلًا، نمط { $style }.
       *[other] أُدرجت قائمة المراجع، { $n } مدخل، نمط { $style }.
    }
# Follows citations-bibliography-inserted; $keys are citation keys joined with commas.
citations-not-in-library = ليست في المكتبة: { $keys }.
citations-bibliography-insert-failed = تعذّر إدراج قائمة المراجع: { $error } لم يتغيّر النص.

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

## العمل في الخلفية، وفتح الملفات والعناوين.

# $what names the export, as its own message says it.
tasks-stopped = توقّف { $what } بشكل غير متوقع.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = توقّف البحث عن { $input } بشكل غير متوقع.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = فتح برامج أخرى متوقف في جلسة لا تحتفظ بملفات
tasks-opening = يجري الفتح.
# $target is a file or a web address; $error says why.
tasks-could-not-open = تعذّر فتح { $target }: { $error }
open-refused-scheme = لم يُفتح: رابط { $scheme } محظور.
open-refused-missing = لم يُفتح: الملف غير موجود.
open-refused-invalid = لم يُفتح: العنوان غير صالح.
tasks-not-opened = لم يُفتح.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = فتحه؟ y أو n

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

## السمات.

message-error = خطأ: { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = لا توجد سمة باسم { $name }؛ يُستخدم { $used }.
# $theme is the new theme's display name.
themes-next = السمة { $theme }.
themes-soft-dark-name = { $theme }، داكن هادئ
themes-choice-aa = { $theme }، يستوفي AA
themes-choice-below-aa = { $theme }، دون AA
themes-below-aa =
    { $count ->
        [one] أقل من AA: فحص واحد لا يبلغ الحد الأدنى.
       *[other] أقل من AA: { $count } فحوص لا تبلغ الحد الأدنى.
    }

## أوضاع إمكانية الوصول وسؤال أول تشغيل.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] وضع النطق الذاتي. ينطق textweaver كل شيء.
        [screen-reader] وضع قارئ الشاشة. textweaver صامت؛ قارئ شاشتك يقرأ سطر الحالة.
       *[hybrid] الوضع المختلط. يقرأ textweaver المستندات بصوت عالٍ؛ قارئ شاشتك ينطق الرسائل والكتابة.
    }
# Yes to the first-run question; $key names the keys that change the mode.
access-hybrid-chosen = الوضع المختلط. يقرأ textweaver المستندات بصوت عالٍ؛ قارئ شاشتك ينطق الرسائل والكتابة. { $key } يغيّر الوضع.
# No to the first-run question; $key names the keys that change the mode.
access-hybrid-declined = البقاء في وضع النطق الذاتي. { $key } يغيّر الوضع.
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-hybrid-question = { $reader } قيد التشغيل. استخدام الوضع المختلط، حيث يقرأ textweaver المستندات بصوت عالٍ وينطق قارئ شاشتك الرسائل والكتابة؟ y أو n
access-a-screen-reader = قارئ شاشة
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = { $reader } قيد التشغيل: يقرأ textweaver المستندات بصوت عالٍ ويترك الرسائل لقارئ شاشتك. { $key } يغيّر ذلك.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = يقرأ textweaver بصوت عالٍ
access-window-choice-screen-reader = قارئ شاشتي يقرأ
access-window-mode-help = من يقرأ في النافذة: صوت textweaver، أو قارئ الشاشة وحده.
access-window-mode-changed =
    { $mode ->
        [screen-reader] قارئ شاشتي يقرأ: يصمت textweaver ويرسل النص إلى قارئ شاشتك. { $key } يغيّر ذلك.
        [speaks-messages] يقرأ textweaver بصوت عالٍ وينطق رسائله. { $key } يغيّر ذلك.
       *[reads-aloud] يقرأ textweaver بصوت عالٍ؛ تذهب الرسائل إلى قارئ شاشتك. { $key } يغيّر ذلك.
    }

## الحروف والتحديدات، كما تُنطق.

# The name of a white-space character read on its own; $name is a fixed key.
text-char-name =
    { $name ->
        [space] مسافة
        [new-line] سطر جديد
        [tab] علامة جدولة
        [no-break-space] مسافة غير فاصلة
       *[white-space] مسافة بيضاء
    }
# Said after a selection grows ($change is selected) or shrinks (unselected); $text is the text or a character's name.
text-selection-change =
    { $change ->
        [selected] { $text } مُحدَّد
       *[unselected] { $text } غير مُحدَّد
    }

## شاشة الإعدادات. $label هو تسمية setting-*، $value قيمتها كما هو موصوف أدناه.

settings-not-set = غير مضبوط
settings-none = لا شيء
settings-empty = فارغ
# A number and its unit (a settings-unit-* message): "300 words per minute".
settings-number-unit = { $n } { $unit }
settings-entries =
    { $n ->
        [0] لا شيء
        [one] مدخل واحد
        [two] مدخلان
        [few] { $n } مدخلات
        [many] { $n } مدخلًا
       *[other] { $n } مدخل
    }
settings-type-on-or-off = اكتب on أو off.
settings-type-a-number = اكتب رقمًا من { $min } إلى { $max }.
settings-outside = { $n } خارج نطاق { $min } إلى { $max }.
# $names are the choices, joined with commas.
settings-choose-one-of = اختر واحدًا من: { $names }.
settings-edit-table = حرّر { $label } في settings.toml؛ يحمل أسماء وقيمًا.
# $path is a key such as speech.rate, not translated.
settings-no-such-setting = لا يوجد إعداد { $path }.
settings-cannot-be = لا يمكن أن يكون { $label } كذلك: { $error } اختر قيمة أخرى.
settings-changed = { $label }، { $value }.
settings-clamped = خارج النطاق، فتُستخدم أقرب قيمة.
settings-restart-speech = أعد تشغيل الكلام لاستخدامه.
settings-next-start = يُستخدم من بدء التشغيل التالي.
settings-intro = الإعدادات، { $n } إعدادًا. اكتب للتصفية. يسار ويمين لتغيير قيمة، Enter لتغيير قيمة أو كتابتها، Delete لإعادة القيمة الافتراضية، Escape للإغلاق.
settings-item = { $label }: { $value }
settings-title = الإعدادات
settings-title-matching = الإعدادات المطابقة لـ{ $filter }
settings-closed = أُغلقت الإعدادات.
settings-filter-cleared =
    { $n ->
        [one] مُسحت التصفية، إعداد واحد.
        [two] مُسحت التصفية، إعدادان.
        [few] مُسحت التصفية، { $n } إعدادات.
        [many] مُسحت التصفية، { $n } إعدادًا.
       *[other] مُسحت التصفية، { $n } إعداد.
    }
settings-filter-none = لا إعدادات تطابق { $query }. Backspace لحذف الحروف.
settings-filter-match =
    { $n ->
        [one] إعداد واحد مطابق.
        [two] إعدادان مطابقان.
        [few] { $n } إعدادات مطابقة.
        [many] { $n } إعدادًا مطابقًا.
       *[other] { $n } إعداد مطابق.
    }
settings-largest = أكبر قيمة، { $value }.
settings-smallest = أصغر قيمة، { $value }.
settings-press-enter = { $label }: اضغط Enter لكتابة قيمة جديدة.
settings-table-item = { $label }: { $value }. حرّره في settings.toml.
# $help is the setting's help (setting-*-help), which may be empty.
settings-editing = { $label }، الآن { $value }. { $help }

## الإعدادات: التسميات والمساعدة والخيارات، كما تعرضها شاشة الإعدادات
## وتنطقها. تتبع المعرّفات المفتاح في settings.toml (speech.rate هو
## setting-speech-rate).

setting-speech-backend = محرك الكلام
setting-speech-backend-help = محرك الكلام: يختار automatic الأفضل المتاح. التغيير يعيد تشغيل الكلام.
choice-speech-backend-auto = تلقائي
choice-speech-backend-eci = Eloquence
choice-speech-backend-sapi = أصوات SAPI 5
choice-speech-backend-espeak = eSpeak NG
choice-speech-backend-speechd = Speech Dispatcher
choice-speech-backend-nsspeech = Apple NSSpeech
choice-speech-backend-avspeech = Apple AVSpeech
choice-speech-backend-dectalk = DECtalk
choice-speech-backend-omnivox = Omnivox
choice-speech-backend-null = صامت
setting-speech-rate = السرعة
setting-speech-rate-help = سرعة كلام textweaver.
setting-speech-volume = مستوى الصوت
setting-speech-volume-help = مدى علو كلام textweaver.
setting-speech-pitch = الحدة
setting-speech-pitch-help = أعلى أو أدنى من حدة الصوت نفسه.
setting-speech-voice = الصوت
setting-speech-voice-help = معرّف الصوت؛ عدم الضبط يختار واحدًا تلقائيًا. يسردها أمر الأصوات.
setting-speech-prefer-voice = الصوت المفضّل
setting-speech-prefer-voice-help = عند عدم ضبط صوت، أول صوت يحتوي اسمه على هذا، مثل eloquence.
setting-speech-favorite-voices = الأصوات المفضّلة
setting-speech-favorite-voices-help = الأصوات المسرودة أولًا في أمر الأصوات، حسب المعرّف، مفصولة بفواصل.
setting-speech-punctuation = علامات الترقيم
setting-speech-punctuation-help = مقدار علامات الترقيم المنطوقة.
choice-speech-punctuation-none = لا شيء
choice-speech-punctuation-some = بعضها
choice-speech-punctuation-all = كلها
setting-speech-split-caps = فصل الأحرف الكبيرة
setting-speech-split-caps-help = نطق الكلمات المتصلة بأحرف كبيرة، مثل TextWeaver، ككلمات منفصلة.
setting-speech-caps = الأحرف الكبيرة
setting-speech-caps-help = كيف يُوسَم الحرف الكبير عند نطق الحروف وكتابتها.
choice-speech-caps-none = غير موسوم
choice-speech-caps-tone = نغمة
choice-speech-caps-pitch = حدة أعلى
choice-speech-caps-say-cap = قول cap
setting-speech-auto-play = القراءة عند الفتح
setting-speech-auto-play-help = بدء القراءة عند فتح مستند.
setting-speech-skip-code = تخطي كتل الشيفرة
setting-speech-skip-code-help = عدم نطق كتل الشيفرة.
setting-speech-speed-presets = إعدادات السرعة الجاهزة
setting-speech-speed-presets-help = سرعات مسمّاة يتنقل بينها F8.
setting-speech-voices-by-language = الأصوات حسب اللغة
setting-speech-voices-by-language-help = الصوت لكل لغة واجهة، حسب رمز اللغة، مثل es = معرّف الصوت. لغة غير مدرجة تستخدم أول صوت للمحرك لها.
setting-speech-latency-offset-ms = تأخير التمييز
setting-speech-latency-offset-ms-help = المدة بعد إبلاغ المحرك عن كلمة حتى يتحرك التمييز، للمحركات المؤقتة بساعة الصوت.
setting-speech-pause-heading-ms = التوقف بعد العناوين
setting-speech-pause-heading-ms-help = صمت بعد العنوان، أقصر عند السرعات الأعلى. القيمة 0 توقفه.
setting-speech-pause-paragraph-ms = التوقف بعد الفقرات
setting-speech-pause-paragraph-ms-help = صمت بعد الفقرة، أقصر عند السرعات الأعلى. القيمة 0 توقفه.
setting-speech-pause-list-item-ms = التوقف بعد عناصر القائمة
setting-speech-pause-list-item-ms-help = صمت بعد عنصر القائمة، أقصر عند السرعات الأعلى. القيمة 0 توقفه.
setting-speech-markup-pauses = التوقفات المكتوبة كترميز
setting-speech-markup-pauses-help = قراءة ترميز التوقف في المستند، مثل <break time="1s"/>، كتوقف. أوقفه للمستندات التي تقتبس هذا الترميز.
setting-speech-output-device = جهاز الإخراج
setting-speech-output-device-help = جهاز الصوت الذي يُشغَّل عليه الكلام، بمعرّفه. إن لم يُضبط يُستخدم الجهاز الافتراضي للنظام، وكذلك إن لم يكن الجهاز متصلًا.
setting-speech-verbosity = مستوى التفصيل
setting-speech-verbosity-help = مقدار ما يقوله textweaver عما يفعله.
choice-speech-verbosity-low = منخفض
choice-speech-verbosity-normal = عادي
choice-speech-verbosity-high = مرتفع
setting-speech-eci-dictionaries = قواميس Eloquence
setting-speech-eci-dictionaries-help = قواميس النطق المجتمعية لـEloquence: مفعّلة، أو متوقفة، أو مجلد خاص بك.
choice-speech-eci-dictionaries-true = مفعّلة
choice-speech-eci-dictionaries-false = متوقفة
setting-speech-eci-library = مكتبة Eloquence
setting-speech-eci-library-help = مكتبة ECI المراد تحميلها؛ عدم الضبط يبحث في الأماكن المعتادة.
setting-speech-eci-code-factory = البحث عن Eloquence من Code Factory
setting-speech-eci-code-factory-help = ابحث أيضًا عن Eloquence for Windows من Code Factory. قد لا يغطي ترخيصها برامج أخرى.
setting-speech-sapi-onecore = أصوات OneCore
setting-speech-sapi-onecore-help = اسرد أيضًا أصوات Windows OneCore عبر SAPI 5.
setting-speech-apple-backend = محرك كلام Apple
setting-speech-apple-backend-help = أي محركات كلام Apple يُستخدم في macOS.
choice-speech-apple-backend-auto = تلقائي
choice-speech-apple-backend-nsspeech = NSSpeechSynthesizer
choice-speech-apple-backend-avspeech = AVSpeechSynthesizer
setting-highlight-enabled = تمييز النص المنطوق
setting-highlight-enabled-help = تمييز الكلمة أو الجملة قيد القراءة.
setting-highlight-granularity = التمييز
setting-highlight-granularity-help = ما يغطيه تمييز القراءة.
choice-highlight-granularity-word = الكلمة
choice-highlight-granularity-sentence = الجملة
choice-highlight-granularity-both = الكلمة والجملة
setting-highlight-lead-words = تقدّم التمييز
setting-highlight-lead-words-help = رسم التمييز بهذا العدد من الكلمات قبل الكلمة المسموعة (1 هي الكلمة المسموعة).
setting-highlight-speed = سرعة التمييز
setting-highlight-speed-help = سرعة التمييز المؤقت للمحركات التي لا تبلّغ عن كلمات.
setting-highlight-color = لون تمييز الكلمة
setting-highlight-color-help = اللون خلف الكلمة المقروءة. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-highlight-sentence-color = لون تمييز الجملة
setting-highlight-sentence-color-help = اللون خلف الجملة المقروءة. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-normalization-math = نطق الرياضيات
setting-normalization-math-help = نطق رموز الرياضيات بالكلمات.
setting-normalization-math-verbosity = تفصيل الرياضيات
setting-normalization-math-verbosity-help = مدى وضوح الرياضيات المنطوقة: low تقول a على b، وnormal وhigh تقولان المزيد.
choice-normalization-math-verbosity-low = منخفض
choice-normalization-math-verbosity-normal = عادي
choice-normalization-math-verbosity-high = مرتفع
setting-normalization-asciimath-delimiter = فاصل ASCIIMath
setting-normalization-asciimath-delimiter-help = الحرف المحيط بـASCIIMath، عادة علامة نبر خلفية؛ عدم الضبط لا يقرأ أي ASCIIMath.
setting-normalization-abbreviations = توسيع الاختصارات
setting-normalization-abbreviations-help = نطق الاختصارات كاملة، مثل Doctor بدلًا من Dr.
setting-normalization-abbrev-expansions = اختصاراتك
setting-normalization-abbrev-expansions-help = اختصاراتك الخاصة وما ترمز إليه.
setting-normalization-numbers = الأرقام بالكلمات
setting-normalization-numbers-help = نطق الأرقام والتواريخ والأوقات والمبالغ بالكلمات.
setting-normalization-use-pronunciations = استخدام النطقيات
setting-normalization-use-pronunciations-help = تطبيق قائمة نطقياتك.
setting-normalization-pronunciations = النطقيات
setting-normalization-pronunciations-help = الكلمات وكيفية نطقها.
setting-normalization-table-mode = الجداول
setting-normalization-table-mode-help = كيفية قراءة الجداول.
choice-normalization-table-mode-structured = بصفوف وأعمدة
choice-normalization-table-mode-flat = كنص
choice-normalization-table-mode-skip = تُتخطى
setting-normalization-footnote-mode = الحواشي
setting-normalization-footnote-mode-help = أين تُقرأ الحواشي.
choice-normalization-footnote-mode-inline = حيث تُوسم
choice-normalization-footnote-mode-deferred = في النهاية
choice-normalization-footnote-mode-skip = تُتخطى
setting-normalization-community-lexicon-enabled = المعجم المجتمعي
setting-normalization-community-lexicon-enabled-help = تطبيق قواميس النطق المجتمعية لمحركات غير Eloquence.
setting-normalization-community-lexicon-dir = مجلد المعجم المجتمعي
setting-normalization-community-lexicon-dir-help = المجلد الحامل لملفات القاموس؛ عدم الضبط يبحث بجانب textweaver.
setting-normalization-community-lexicon-language = لغة المعجم المجتمعي
setting-normalization-community-lexicon-language-help = لغة القواميس.
choice-normalization-community-lexicon-language-enu = الإنجليزية الأمريكية
choice-normalization-community-lexicon-language-deu = الألمانية
setting-normalization-medical-lexicon-enabled = المعجم الطبي
setting-normalization-medical-lexicon-enabled-help = قراءة أسماء الأدوية والمصطلحات السريرية واختصارات الجرعات من قائمة نطق طبية.
setting-normalization-medical-lexicon-overlay = ملف المعجم الطبي
setting-normalization-medical-lexicon-overlay-help = نطقك الطبي الخاص، وله الأولوية على المدمج. عدم الضبط يقرأ medical-lexicon.toml في مجلد الإعدادات.
setting-reading-auto-resume = استئناف من حيث توقفت
setting-reading-auto-resume-help = العودة إلى الموضع المحفوظ عند فتح مستند.
setting-reading-nav-history-size = سجل الرجوع
setting-reading-nav-history-size-help = عدد الأماكن التي يتذكرها Back.
setting-reading-wrap-navigation = التفاف التنقل
setting-reading-wrap-navigation-help = تجاوز نهاية المستند يتابع من البداية.
setting-reading-cursor-follows-speech = المؤشر يتبع الكلام
setting-reading-cursor-follows-speech-help = يتحرك المؤشر مع الكلمة قيد القراءة.
setting-reading-citations = الاستشهادات
setting-reading-citations-help = الاستشهادات في القراءة المتواصلة: تُتخطى، أو تُقال بالكلمات.
choice-reading-citations-off = تُتخطى
choice-reading-citations-words = بالكلمات
setting-reading-ocr = التعرف على الصفحات الممسوحة
setting-reading-ocr-help = قراءة نص ملفات PDF والصور الممسوحة ضوئيًا بالتعرف عليه (OCR).
setting-reading-ocr-lang = لغة النص الممسوح
setting-reading-ocr-lang-help = لغة النص الممسوح ضوئيًا، برموز Tesseract مثل fra أو deu+eng. الفراغ يعني لغة المستند نفسها، وإلا الإنجليزية.
choice-reading-ocr-lang- = لغة المستند
choice-reading-ocr-lang-eng = الإنجليزية
choice-reading-ocr-lang-fra = الفرنسية
choice-reading-ocr-lang-deu = الألمانية
choice-reading-ocr-lang-spa = الإسبانية
setting-reading-ocr-engine = محرك OCR
setting-reading-ocr-engine-help = أي محرك يتعرف على الصفحات الممسوحة. ocrs للإنجليزية وTesseract للغات الأخرى، أو أحدهما دائمًا.
choice-reading-ocr-engine-auto = تلقائي
choice-reading-ocr-engine-ocrs = ocrs
choice-reading-ocr-engine-tesseract = Tesseract
choice-reading-ocr-engine-paddle = PaddleOCR (تجريبي)
setting-reading-math-engine = نطق الرياضيات
setting-reading-math-engine-help = أي محرك يقرأ الرياضيات بصوت عالٍ. محرك textweaver الخاص، أو MathCAT بنمط ClearSpeak أو SimpleSpeak، بلغة المستند. يحتاج MathCAT نسخة تتضمنه؛ وإلا يُستخدم محرك textweaver الخاص.
choice-reading-math-engine-builtin = textweaver
choice-reading-math-engine-mathcat = MathCAT ClearSpeak
choice-reading-math-engine-mathcat-simplespeak = MathCAT SimpleSpeak
setting-braille-math-code = برايل الرياضيات
setting-braille-math-code-help = رمز برايل للرياضيات في ملفات BRF وعند استكشاف معادلة باستخدام MathCAT. نيميث أو رياضيات UEB. يحتاج إلى نسخة تتضمن MathCAT؛ وإلا تُكتب الرياضيات بكلماتها المنطوقة.
choice-braille-math-code-nemeth = Nemeth
choice-braille-math-code-ueb = UEB
setting-braille-table-format = جداول برايل
setting-braille-table-format-help = كيف ترتب ملفات BRF الجداول. خطي، صف واحد في كل سطر مع فواصل منقوطة بين الإدخالات؛ أو مسرد، كل صف عنوان وكل إدخال في سطر خاص بعد عنوان عموده؛ أو متدرج، كل إدخال بعد الذي قبله بخليتين إلى اليمين، للجداول ذات أربعة أعمدة على الأكثر.
choice-braille-table-format-linear = خطي
choice-braille-table-format-listed = مسرد
choice-braille-table-format-stairstep = متدرج
setting-reading-math-display = الرياضيات على الشاشة
setting-reading-math-display-help = كيف تبدو الرياضيات في عرض القراءة. كمصدرها، مثل x^2، أو بيونيكود، مثل x بأس علوي 2. الكلام ووضع التحرير يستخدمان المصدر دائمًا.
choice-reading-math-display-source = المصدر
choice-reading-math-display-unicode = يونيكود
setting-reading-revisions = التغييرات المتعقبة
setting-reading-revisions-help = كيف تُقرأ التغييرات المتعقبة في ملفات Word وOpenDocument وRTF. تُقال في مكانها عند الإسهاب العالي (تلقائي)، أو دائمًا، أو أبدًا مع قراءة النص النهائي. يُطبَّق عند فتح المستند.
choice-reading-revisions-auto = تلقائي
choice-reading-revisions-marked = قلها دائمًا
choice-reading-revisions-final = النص النهائي فقط
setting-reading-stop-at = التوقف عند نهاية القسم
setting-reading-stop-at-help = أين تتوقف القراءة المستمرة من تلقاء نفسها وتقول نهاية القسم. أبدًا، أو عند العنوان التالي من أي مستوى، أو عند الفصل التالي: فاصل قسم، وإلا عنوان من المستوى 1. تتابع القراءة من العنوان بمفتاح القراءة.
choice-reading-stop-at-off = أبدًا
choice-reading-stop-at-heading = العنوان التالي
choice-reading-stop-at-chapter = الفصل التالي
setting-reading-stop-after-minutes = مؤقت القراءة
setting-reading-stop-after-minutes-help = تتوقف القراءة المستمرة عند نهاية الجملة بعد هذا العدد من دقائق القراءة، وتقول ذلك. الإيقاف المؤقت يوقف الساعة، والإيقاف يبدأها من جديد. 0 يطفئ المؤقت.
setting-reading-recall-prompts = أسئلة التذكر
setting-reading-recall-prompts-help = عند نهاية القسم، تطلب منك القراءة أن تقول ما تتذكره. إذا كان التوقف عند نهاية القسم على أبدًا، تتوقف القراءة لذلك عند العنوان التالي. تتابع القراءة بمفتاح القراءة.
setting-display-theme = السمة
setting-display-theme-help = السمة اللونية.
setting-display-follow-os-theme = اتباع سمة النظام
setting-display-follow-os-theme-help = عند بدء التشغيل، استخدام سمة فاتحة أو داكنة أو عالية التباين كالنظام، إلا إذا اخترت واحدة.
setting-display-wrap-width = عرض الالتفاف
setting-display-wrap-width-help = التفاف الأسطر عند هذا العدد من الأعمدة؛ 0 يستخدم العرض كاملًا.
setting-display-measure = طول السطر
setting-display-measure-help = عدد الأحرف التي يتسع لها السطر في النافذة، من 25 إلى 90. 0 يملأ النافذة. تستخدم الطرفية عرض الالتفاف.
setting-display-tab-width = عرض علامة الجدولة
setting-display-tab-width-help = الأعمدة التي تأخذها علامة الجدولة.
setting-display-show-line-numbers = أرقام الأسطر
setting-display-show-line-numbers-help = إظهار أرقام الأسطر.
setting-display-scroll-margin = هامش التمرير
setting-display-scroll-margin-help = الأسطر المُبقاة في العرض أعلى المؤشر وأسفله.
setting-display-hints = سطر تلميحات المفاتيح
setting-display-hints-help = هل يعرض قارئ الطرفية تلميحات المفاتيح في سطره الأخير. التلقائي يعرضها مع النطق الذاتي ويخفيها مع قارئ الشاشة. يذكر F1 وقائمة اختصارات لوحة المفاتيح المفاتيح دائمًا.
choice-display-hints-auto = تلقائي
choice-display-hints-on = مفعّل
choice-display-hints-off = متوقف
setting-editing-autosave-recovery = لقطات الاسترداد
setting-editing-autosave-recovery-help = الاحتفاظ بنسخة من العمل غير المحفوظ وعرضها بعد تعطّل.
setting-editing-autosave-interval-secs = فاصل اللقطات
setting-editing-autosave-interval-secs-help = الثواني بين لقطات الاسترداد أثناء وجود تغييرات غير محفوظة.
setting-editing-echo-characters = ترديد الحروف
setting-editing-echo-characters-help = نطق كل حرف يُكتب.
setting-editing-echo-words = ترديد الكلمات
setting-editing-echo-words-help = نطق كل كلمة تُكتب.
setting-editing-echo-deletions = ترديد الحذف
setting-editing-echo-deletions-help = نطق ما يحذفه Backspace وDelete.
setting-editing-echo-lines-on-move = ترديد الأسطر
setting-editing-echo-lines-on-move-help = نطق السطر عند انتقال المؤشر إلى سطر آخر.
setting-editing-undo-steps = خطوات التراجع
setting-editing-undo-steps-help = أكثر خطوات تراجع محفوظة أثناء التحرير.
setting-editing-undo-memory-mb = ذاكرة التراجع
setting-editing-undo-memory-mb-help = أكثر ذاكرة قد يستخدمها سجل التراجع.
setting-library-recent-limit = الملفات الأخيرة
setting-library-recent-limit-help = عدد الملفات الأخيرة المتذكَّرة.
setting-library-folders = مجلدات المكتبة
setting-library-folders-help = المجلدات التي تسرد المكتبة مستنداتها، وتُزامَن مواضعها بين الحواسيب. افصل بين المجلدات بفواصل منقوطة.
setting-keyboard-character-keys = الاختصارات أحادية المفتاح
setting-keyboard-character-keys-help = مفاتيح تصفح مثل h ونقطة. عند الإيقاف، لا يفعّل الإملاء والكتابة أي أوامر أبدًا.
setting-keyboard-preset = المفاتيح
setting-keyboard-preset-help = المفاتيح الافتراضية: كوضع تصفح NVDA وJAWS، أو مفاتيح textweaver السابقة. تُستخدم من بدء التشغيل التالي.
choice-keyboard-preset-default = بأسلوب قارئ الشاشة
choice-keyboard-preset-classic = كلاسيكية
setting-keyboard-digit-row = صف الأرقام
setting-keyboard-digit-row-help = كيف تتعرف الطرفية على مفاتيح الأرقام لمستويات العناوين. تلقائي، أو لوحة مفاتيح AZERTY فرنسية.
choice-keyboard-digit-row-auto = تلقائي
choice-keyboard-digit-row-azerty = AZERTY
setting-accessibility-mode = وضع إمكانية الوصول
setting-accessibility-mode-help = النطق الذاتي ينطق كل شيء؛ قارئ الشاشة يترك الكلام لقارئ شاشتك؛ الوضع المختلط ينطق القراءة فقط.
choice-accessibility-mode-self-voicing = نطق ذاتي
choice-accessibility-mode-screen-reader = قارئ شاشة
choice-accessibility-mode-hybrid = مختلط
setting-accessibility-say-all = القراءة الكاملة مع قارئ شاشة
setting-accessibility-say-all-help = القراءة المتواصلة في وضع قارئ الشاشة. جملة تلو الأخرى على سطر الحالة، أو بصوت textweaver.
choice-accessibility-say-all-screen = على سطر الحالة
choice-accessibility-say-all-voice = بصوت textweaver
setting-accessibility-quiet-screen = شاشة هادئة أثناء القراءة
setting-accessibility-quiet-screen-help = إبقاء الشاشة ثابتة أثناء قراءة textweaver بصوت عالٍ. مفعّلة افتراضيًا في الوضع المختلط.
choice-accessibility-quiet-screen-auto = تلقائي
choice-accessibility-quiet-screen-true = مفعّلة
choice-accessibility-quiet-screen-false = متوقفة
setting-accessibility-cursor = المؤشر
setting-accessibility-cursor-help = أين ينتظر مؤشر الطرفية. على ما تعمل عليه، أو على سطر الحالة.
choice-accessibility-cursor-follow = يتبع التركيز
choice-accessibility-cursor-status = على سطر الحالة
setting-export-audio-format = صيغة تصدير الصوت
setting-export-audio-format-help = الصيغة التي يعرضها تصدير الصوت أولًا. يستخدمها tw export-audio أيضًا لاسم ملف بلا امتداد.
choice-export-audio-format-flac = FLAC
choice-export-audio-format-mp3 = MP3
choice-export-audio-format-opus = Opus
choice-export-audio-format-ogg = Ogg Vorbis
choice-export-audio-format-wav = WAV
choice-export-audio-format-m4b = كتاب صوتي M4B
choice-export-audio-format-mp4 = فيديو MP4 مع ترجمة نصية
setting-export-subtitle-format = صيغة الترجمة النصية
setting-export-subtitle-format-help = صيغة الترجمة النصية المكتوبة دون اسم ملف.
choice-export-subtitle-format-srt = SubRip
choice-export-subtitle-format-vtt = WebVTT
setting-export-subtitle-word-level = ترجمة نصية بالكلمة
setting-export-subtitle-word-level-help = ترجمة نصية واحدة لكل كلمة بدلًا من أسطر تعليق.
setting-export-subtitles-with-audio = الترجمة النصية مع الصوت
setting-export-subtitles-with-audio-help = كتابة الترجمة النصية دائمًا بجانب الصوت المصدَّر.
choice-export-subtitle-format-ass = كاريوكي ASS
setting-export-subtitle-karaoke = كاريوكي الترجمة النصية
setting-export-subtitle-karaoke-help = كيف تُظهر أسطر الترجمة النصية الكلمة المقروءة. متوقف، أو تسطير الكلمة عند نطقها (وسوم WebVTT)، أو ترجمة نصية لكل كلمة بخط عريض مسطَّر.
choice-export-subtitle-karaoke-off = متوقف
choice-export-subtitle-karaoke-tags = تسطير عند النطق
choice-export-subtitle-karaoke-lines = ترجمة نصية لكل كلمة
setting-export-subtitle-chapters = ملف الفصول
setting-export-subtitle-chapters-help = كتابة ملف فصول WebVTT أيضًا بجانب الترجمة النصية أو الصوت.
# Names for chapters the document leaves untitled, in audio export.
export-chapter-untitled = كتاب صوتي
export-chapter-numbered = الفصل { $number }
# The read-along page (tw export-audio essay.md --out essay.html).
readalong-skip = الانتقال إلى النص
readalong-controls = الصوت
readalong-play = تشغيل
readalong-pause = إيقاف مؤقت
readalong-back = الجملة السابقة
readalong-forward = الجملة التالية
readalong-follow = المتابعة مع القراءة
readalong-speed = السرعة
readalong-contents = المحتويات
readalong-play-section = تشغيل القسم: { $title }
setting-reading-aids-rsvp-wpm = سرعة العرض السريع
setting-reading-aids-rsvp-wpm-help = كلمات في الدقيقة للعرض المتتابع السريع.
setting-reading-aids-rsvp-pacing = إيقاع العرض السريع
setting-reading-aids-rsvp-pacing-help = ما يحرّك كلمة العرض السريع: مؤقتها الخاص، أو الكلام.
choice-reading-aids-rsvp-pacing-timer = مؤقتها الخاص
choice-reading-aids-rsvp-pacing-external = الكلام
setting-reading-aids-rsvp-clause-pause = وقفة الجملة الفرعية في العرض السريع
setting-reading-aids-rsvp-clause-pause-help = وقت إضافي بعد فاصلة أو نقطتين أو شرطة أو قوس، بنسبة مئوية من وقت الكلمة.
setting-reading-aids-rsvp-sentence-pause = وقفة الجملة في العرض السريع
setting-reading-aids-rsvp-sentence-pause-help = وقت إضافي في نهاية الجملة، بالنسبة المئوية.
setting-reading-aids-rsvp-paragraph-pause = وقفة الفقرة في العرض السريع
setting-reading-aids-rsvp-paragraph-pause-help = وقت إضافي في نهاية الفقرة، بالنسبة المئوية.
setting-reading-aids-rsvp-long-word-len = الكلمة الطويلة في العرض السريع
setting-reading-aids-rsvp-long-word-len-help = الكلمات الأطول من هذا العدد من الحروف تأخذ وقتًا إضافيًا.
setting-reading-aids-rsvp-long-word-step = خطوة الكلمة الطويلة في العرض السريع
setting-reading-aids-rsvp-long-word-step-help = وقت إضافي لكل حرف بعد طول الكلمة الطويلة، بالنسبة المئوية.
setting-reading-aids-rsvp-long-word-max = أقصى وقت للكلمة الطويلة في العرض السريع
setting-reading-aids-rsvp-long-word-max-help = أكثر وقت إضافي تأخذه كلمة طويلة، بالنسبة المئوية.
setting-reading-aids-rsvp-show-previous = الكلمة السابقة في العرض السريع
setting-reading-aids-rsvp-show-previous-help = إظهار الكلمة السابقة أيضًا.
setting-reading-aids-rsvp-show-next = الكلمة التالية في العرض السريع
setting-reading-aids-rsvp-show-next-help = إظهار الكلمة التالية أيضًا.
setting-reading-aids-rsvp-position = موضع العرض السريع
setting-reading-aids-rsvp-position-help = أين تظهر كلمة العرض السريع.
choice-reading-aids-rsvp-position-top-left = أعلى اليسار
choice-reading-aids-rsvp-position-top-center = أعلى الوسط
choice-reading-aids-rsvp-position-top-right = أعلى اليمين
choice-reading-aids-rsvp-position-center-left = وسط اليسار
choice-reading-aids-rsvp-position-center = الوسط
choice-reading-aids-rsvp-position-center-right = وسط اليمين
choice-reading-aids-rsvp-position-bottom-left = أسفل اليسار
choice-reading-aids-rsvp-position-bottom-center = أسفل الوسط
choice-reading-aids-rsvp-position-bottom-right = أسفل اليمين
setting-reading-aids-rsvp-font-size-pt = حجم العرض السريع
setting-reading-aids-rsvp-font-size-pt-help = حجم كلمة العرض السريع في الواجهة الرسومية.
setting-reading-aids-rsvp-lead-words = تقدّم العرض السريع
setting-reading-aids-rsvp-lead-words-help = مع إيقاع الكلام، إظهار هذا العدد من الكلمات قبل الكلمة المنطوقة.
setting-reading-aids-bionic = القراءة البيونية
setting-reading-aids-bionic-help = رسم بداية كل كلمة بخط عريض.
setting-reading-aids-bionic-options-ratio = نسبة القراءة البيونية
setting-reading-aids-bionic-options-ratio-help = مقدار عرض كل كلمة بخط عريض.
setting-reading-aids-bionic-options-min-word-len = أقصر كلمة للقراءة البيونية
setting-reading-aids-bionic-options-min-word-len-help = تُترك الكلمات الأقصر من هذا وحدها.
setting-reading-aids-bionic-options-skip-numbers = القراءة البيونية تتخطى الأرقام
setting-reading-aids-bionic-options-skip-numbers-help = تُترك الكلمات التي فيها أرقام وحدها.
setting-reading-aids-bionic-options-skip-urls = القراءة البيونية تتخطى العناوين
setting-reading-aids-bionic-options-skip-urls-help = تُترك عناوين الويب والبريد الإلكتروني وحدها.
setting-reading-aids-bionic-options-skip-code = القراءة البيونية تتخطى الشيفرة
setting-reading-aids-bionic-options-skip-code-help = تُترك الشيفرة وحدها.
setting-reading-aids-spacing-line-height = ارتفاع السطر
setting-reading-aids-spacing-line-height-help = ارتفاع السطر بمضاعفات حجم الخط؛ قيمة WCAG هي 1.5.
setting-reading-aids-spacing-paragraph-spacing = تباعد الفقرات
setting-reading-aids-spacing-paragraph-spacing-help = المسافة بعد كل فقرة، بمضاعفات حجم الخط.
setting-reading-aids-spacing-letter-spacing = تباعد الحروف
setting-reading-aids-spacing-letter-spacing-help = مسافة إضافية بين الحروف، بمضاعفات حجم الخط.
setting-reading-aids-spacing-word-spacing = تباعد الكلمات
setting-reading-aids-spacing-word-spacing-help = مسافة إضافية بين الكلمات، بمضاعفات حجم الخط.
setting-reading-aids-font-family = الخط
setting-reading-aids-font-family-help = خط القراءة في الواجهة الرسومية؛ يمكن كتابة أي خط مثبَّت.
choice-reading-aids-font-family-system-ui = خط النظام
choice-reading-aids-font-family-sans = بلا زوائد
choice-reading-aids-font-family-serif = بزوائد
choice-reading-aids-font-family-monospace = أحادي التباعد
choice-reading-aids-font-family-atkinson = Atkinson Hyperlegible
choice-reading-aids-font-family-opendyslexic = OpenDyslexic
choice-reading-aids-font-family-lexend = Lexend
setting-reading-aids-font-size-pt = حجم الخط
setting-reading-aids-font-size-pt-help = حجم خط الواجهة الرسومية.
setting-reading-aids-font-weight = وزن الخط
setting-reading-aids-font-weight-help = 400 عادي، 700 عريض.
setting-reading-aids-ruler-mode = مسطرة القراءة
setting-reading-aids-ruler-mode-help = وسم السطر الحالي، أو شريط من الأسطر.
choice-reading-aids-ruler-mode-off = إيقاف
choice-reading-aids-ruler-mode-current-line = السطر الحالي
choice-reading-aids-ruler-mode-ruler = مسطرة
setting-reading-aids-ruler-scope = تغطية المسطرة
setting-reading-aids-ruler-scope-help = صف ملتف، أو السطر كاملًا.
choice-reading-aids-ruler-scope-row = صف
choice-reading-aids-ruler-scope-line = السطر كاملًا
setting-reading-aids-ruler-rows-above = صفوف المسطرة أعلاه
setting-reading-aids-ruler-rows-above-help = صفوف الشريط أعلى الصف الحالي.
setting-reading-aids-ruler-rows-below = صفوف المسطرة أسفله
setting-reading-aids-ruler-rows-below-help = صفوف الشريط أسفل الصف الحالي.
setting-reading-aids-ruler-mask-outside = تعتيم المسطرة
setting-reading-aids-ruler-mask-outside-help = تعتيم الصفوف خارج الشريط.
setting-reading-aids-syllables = المقاطع
setting-reading-aids-syllables-help = رسم الكلمات مقسّمة إلى مقاطع بنقطة وسطى؛ الكلام لا يتغيّر.
setting-reading-aids-difficult-words = الكلمات الصعبة
setting-reading-aids-difficult-words-help = تسطير الكلمات النادرة، وذكرها عند التنقل بين الكلمات في مستوى التفصيل المرتفع.
setting-reading-aids-syllable-options-separator = فاصل المقاطع
setting-reading-aids-syllable-options-separator-help = ما يُرسم بين المقاطع.
setting-reading-aids-syllable-options-left-min = أول فاصل مقاطع
setting-reading-aids-syllable-options-left-min-help = أقل عدد حروف قبل أول فاصل.
setting-reading-aids-syllable-options-right-min = آخر فاصل مقاطع
setting-reading-aids-syllable-options-right-min-help = أقل عدد حروف بعد آخر فاصل.
setting-reading-aids-syllable-options-min-word-len = أقصر كلمة للمقاطع
setting-reading-aids-syllable-options-min-word-len-help = الكلمات الأقصر من هذا لا تُقسَّم أبدًا.
setting-reading-aids-syllable-options-skip-urls = المقاطع تتخطى العناوين
setting-reading-aids-syllable-options-skip-urls-help = تُترك عناوين الويب والبريد الإلكتروني وحدها.
setting-reading-aids-syllable-options-skip-code = المقاطع تتخطى الشيفرة
setting-reading-aids-syllable-options-skip-code-help = تُترك الشيفرة وحدها.
setting-preview-auto-reload = إعادة تحميل المعاينة
setting-preview-auto-reload-help = إعادة تحميل معاينة المتصفح بعد كل حفظ، عبر خادم صغير على هذا الحاسوب فقط.
setting-preview-live = المعاينة الحية
setting-preview-live-help = مع تفعيل إعادة التحميل، إعادة التحميل أيضًا عند توقف الكتابة.
setting-preview-pane = لوحة المعاينة
setting-preview-pane-help = في وضع التحرير، إظهار المستند كما يُقرأ بجانب المحرر. F6 ينتقل إليها؛ ويبقى المؤشر في المحرر.
setting-preview-pane-delay-ms = مهلة المعاينة
setting-preview-pane-delay-ms-help = المدة التي يجب أن تتوقف فيها الكتابة قبل إعادة كتابة معاينة تتبع كتابتك.
setting-lexicon-glossary = المسرد
setting-lexicon-glossary-help = مسردك الخاص، يُبحث فيه قبل القاموس: أسطر term: definition، أو JSON من star. عدم الضبط يستخدم glossary.txt في مجلد الإعدادات.
setting-lexicon-data-file = ملف القاموس
setting-lexicon-data-file-help = قاموس تعريف الكلمة، lexicon-en.twlex. عدم الضبط يبحث بجانب البرنامج.
setting-stats-enabled = إحصاءات القراءة
setting-stats-enabled-help = عدّ وقت القراءة بصوت عالٍ، وأبعد نقطة، والجلسات لكل مستند.
setting-interface-language = لغة الواجهة
setting-interface-language-help = لغة كلمات textweaver نفسها، تتغيّر فورًا. يتبعها الصوت عندما يوجد للمحرك صوت لها؛ وإلا يبقى الصوت.
choice-interface-language-en = English
choice-interface-language-es = Español
choice-interface-language-fr = Français
choice-interface-language-de = Deutsch
choice-interface-language-pt = Português
choice-interface-language-ar = العربية
setting-interface-rtl = العرض من اليمين إلى اليسار
setting-interface-rtl-help = هل يعيد قارئ الطرفية ترتيب النص من اليمين إلى اليسار للعرض. Automatic يتركه للطرفيات التي تفعل ذلك بنفسها. الكلام وقارئ الشاشة يحصلان دائمًا على النص بترتيب القراءة.
choice-interface-rtl-auto = تلقائي
choice-interface-rtl-on = مفعّل
choice-interface-rtl-off = متوقف
setting-gui-announce = الإعلانات
setting-gui-announce-help = كيف تصل رسائل النافذة إلى قارئ الشاشة، من بدء التشغيل التالي. منطقة حية، أو إشعارات UI Automation (لنظام Windows فقط).
choice-gui-announce-live = منطقة حية
choice-gui-announce-uia = إشعارات UI Automation
setting-gui-header = إظهار الترويسة
setting-gui-header-help = يُظهر شريط الأوامر فوق المستند. عند إيقافه تحتفظ الأوامر بمفاتيحها وعناصر قوائمها.
setting-gui-toolbar = إظهار شريط الأدوات
setting-gui-toolbar-help = يُظهر شريط أزرار القراءة. عند إيقافه تحتفظ الأوامر بمفاتيحها وعناصر قوائمها.
setting-gui-auto-hide-menu = إخفاء شريط القوائم
setting-gui-auto-hide-menu-help = Windows: يخفي شريط قوائم النافذة حتى يُظهره Alt أو F10، ويختفي مرة أخرى عند إغلاق القائمة. لا أثر له على Linux، حيث القوائم هي قائمة F10، ولا على macOS.
setting-gui-speak-messages = نطق رسائل textweaver
setting-gui-speak-messages-help = عندما يقرأ textweaver بصوت عالٍ، ينطق أيضًا رسائله والكتابة وحركات المؤشر بصوته، للقراءة بالسمع دون قارئ شاشة.
setting-gui-sidebar = اللوحة بجانب المستند
setting-gui-sidebar-help = اللوحة التي تعرضها النافذة بجانب المستند. لا شيء، أو المحتويات (العناوين)، أو الملاحظات. مفاتيح اللوحات تغيّرها، وتتذكر النافذة آخر لوحة.
choice-gui-sidebar-off = لا شيء
choice-gui-sidebar-contents = المحتويات
choice-gui-sidebar-notes = الملاحظات

## الوحدات، تُقال بعد رقم.

settings-unit-words-per-minute = كلمة في الدقيقة
settings-unit-percent = بالمئة
settings-unit-semitones = نصف نغمة
settings-unit-milliseconds =
    { $n ->
        [one] ميلي ثانية
        [two] ميلي ثانية
        [few] ميلي ثانية
        [zero] ميلي ثانية
       *[other] ميلي ثانية
    }
settings-unit-words =
    { $n ->
        [one] كلمة
        [two] كلمات
        [few] كلمات
        [zero] كلمات
       *[other] كلمة
    }
settings-unit-places =
    { $n ->
        [one] مكان
        [two] أماكن
        [few] أماكن
        [zero] أماكن
       *[other] مكان
    }
settings-unit-columns =
    { $n ->
        [one] عمود
        [two] أعمدة
        [few] أعمدة
        [zero] أعمدة
       *[other] عمود
    }
settings-unit-characters =
    { $n ->
        [one] حرف
        [two] أحرف
        [few] أحرف
        [zero] أحرف
       *[other] حرف
    }
settings-unit-lines =
    { $n ->
        [one] سطر
        [two] أسطر
        [few] أسطر
        [zero] أسطر
       *[other] سطر
    }
settings-unit-seconds =
    { $n ->
        [one] ثانية
        [two] ثوانٍ
        [few] ثوانٍ
        [zero] ثوانٍ
       *[other] ثانية
    }
settings-unit-steps =
    { $n ->
        [one] خطوة
        [two] خطوات
        [few] خطوات
        [zero] خطوات
       *[other] خطوة
    }
settings-unit-megabytes = ميغابايت
settings-unit-files =
    { $n ->
        [one] ملف
        [two] ملفات
        [few] ملفات
        [zero] ملفات
       *[other] ملف
    }
settings-unit-letters =
    { $n ->
        [one] حرف
        [two] حروف
        [few] حروف
        [zero] حروف
       *[other] حرف
    }
settings-unit-points =
    { $n ->
        [one] نقطة
        [two] نقاط
        [few] نقاط
        [zero] نقاط
       *[other] نقطة
    }
settings-unit-rows =
    { $n ->
        [one] صف
        [two] صفوف
        [few] صفوف
        [zero] صفوف
       *[other] صف
    }
settings-unit-minutes =
    { $n ->
        [one] دقيقة
        [two] دقيقتان
        [few] دقائق
        [zero] دقيقة
       *[other] دقيقة
    }

## أقسام الإعدادات.

section-speech = الكلام
section-highlight = تمييز
section-normalization = نطق النص
section-reading = القراءة
section-display = العرض
section-editing = التحرير
section-library = المكتبة
section-keyboard = لوحة المفاتيح
section-accessibility = إمكانية الوصول
section-export = التصدير
section-braille = برايل
section-reading-aids = أدوات القراءة
section-preview = معاينة
section-lexicon = تعريف كلمة
section-stats = إحصاءات القراءة
section-interface = الواجهة
section-gui = النافذة

## وضع التحرير: الدخول والمغادرة والحفظ والكتابة.

# $key makes a new document.
edit-no-document = لا مستند لتحريره. اضغط { $key } لواحد جديد.
# $line is the line at the caret, as echoed.
edit-mode-on-brief = وضع التحرير مفعّل. { $line }
# $save and $finish are the keys that save and leave edit mode; $line is
# the line at the caret.
edit-mode-on = وضع التحرير مفعّل. حفظ: { $save }. إنهاء: { $finish }. { $line }
# Keep the letters s, d and c: they are the keys that answer.
edit-unsaved-question = { $title } به تغييرات غير محفوظة. حفظ أو تجاهل أو إلغاء؟ اضغط s أو d أو c، أو لأعلى ولأسفل ثم Enter. Escape للإلغاء.
edit-save-changes-title = حفظ التغييرات في { $title }؟
edit-choice-save = حفظ، ثم متابعة
edit-choice-discard = تجاهل التغييرات
edit-choice-cancel = إلغاء، متابعة التحرير
edit-save-failed = تعذّر الحفظ: { $error } ما زلت في وضع التحرير. جرّب حفظ باسم.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = حفظ باسم، Enter لـ{ $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } موجود بالفعل. استبداله؟ y أو n
edit-mode-off = وضع التحرير متوقف.
edit-mode-off-discarded = تُجوهلت التغييرات. وضع التحرير متوقف.
# The title of a new, unsaved document.
edit-untitled = بلا عنوان
edit-new-document = مستند جديد جاهز للتحرير.
# $key turns on edit mode.
edit-nothing-to-save = لا شيء لحفظه. فعّل وضع التحرير بـ{ $key } لإجراء تغييرات.
# $key turns on edit mode; $what is what the user tried to do.
edit-not-editing =
    { $what ->
        [type] فعّل وضع التحرير بـ{ $key } للكتابة.
        [change-text] فعّل وضع التحرير بـ{ $key } لتغيير النص.
        [delete-text] فعّل وضع التحرير بـ{ $key } لحذف نص.
        [undo] فعّل وضع التحرير بـ{ $key } للتراجع.
        [redo] فعّل وضع التحرير بـ{ $key } للإعادة.
        [replace-text] فعّل وضع التحرير بـ{ $key } لاستبدال نص.
        [insert] فعّل وضع التحرير بـ{ $key } للإدراج في النص.
        [cut-text] فعّل وضع التحرير بـ{ $key } لقص نص.
        [move-cells] فعّل وضع التحرير بـ{ $key } للتنقل بين خلايا الجدول.
        [delete-words] فعّل وضع التحرير بـ{ $key } لحذف كلمات.
        [paste] فعّل وضع التحرير بـ{ $key } للصق.
        [citation] فعّل وضع التحرير بـ{ $key } لإدراج استشهاد.
        [bibliography] فعّل وضع التحرير بـ{ $key } لإدراج قائمة مراجع.
       *[format] فعّل وضع التحرير بـ{ $key } لتنسيق نص.
    }
# A paste: $n characters.
edit-pasted =
    { $n ->
        [one] لُصق حرف واحد.
        [two] لُصق حرفان.
        [few] لُصقت { $n } حروف.
        [many] لُصق { $n } حرفًا.
       *[other] لُصق { $n } حرف.
    }
# $start is how the pasted text starts.
edit-pasted-start =
    { $n ->
        [one] لُصق حرف واحد: { $start }
        [two] لُصق حرفان: { $start }
        [few] لُصقت { $n } حروف: { $start }
        [many] لُصق { $n } حرفًا: { $start }
       *[other] لُصق { $n } حرف: { $start }
    }
edit-insert-failed = تعذّر الإدراج: { $error } لم يتغيّر النص.
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = لا يمكن تغيير الحروف من { $start } إلى { $end }: النص يحتوي { $len }.
edit-change-failed = تعذّر تغيير النص: { $error } لم يتغيّر النص.
edit-delete-failed = تعذّر الحذف: { $error } لم يتغيّر النص.
edit-list-ended = انتهت القائمة.
# Said when Enter continues a bulleted list.
edit-bullet = نقطة
edit-table-divider = فاصل رأس الجدول
edit-end-of-line-stop = نهاية السطر.
edit-start-of-line-stop = بداية السطر.
edit-end-of-line-content = نهاية السطر

## وضع التحرير: التنسيق، والتراجع، والجداول، والصور، والاستبدال.

# $what names the formatting command; $level is a heading level, and
# $cols and $rows a table's size.
edit-format-done =
    { $what ->
        [bold] عريض.
        [italic] مائل.
        [underline] تسطير.
        [strikethrough] يتوسطه خط.
        [code] شيفرة.
        [code-block] كتلة شيفرة.
        [link] رابط.
        [bulleted-list] قائمة نقطية.
        [numbered-list] قائمة مرقّمة.
        [block-quote] اقتباس.
        [horizontal-rule] أُدرج خط أفقي.
        [table-row] أُضيف صف إلى الجدول.
        [heading-level] مستوى العنوان { $level }.
        [table] أُدرج جدول، { $cols } أعمدة في { $rows } صفوف.
       *[heading] عنوان.
    }
# The command toggled its markup off.
edit-format-removed =
    { $what ->
        [bold] أُزيل العريض.
        [italic] أُزيل المائل.
        [underline] أُزيل التسطير.
        [strikethrough] أُزيل الخط المتوسط.
        [code] أُزيلت الشيفرة.
        [code-block] أُزيلت كتلة الشيفرة.
        [link] أُزيل الرابط.
        [bulleted-list] أُزيلت القائمة النقطية.
        [numbered-list] أُزيلت القائمة المرقّمة.
        [block-quote] أُزيل الاقتباس.
        [horizontal-rule] أُزيل الخط الأفقي.
        [table-row] أُزيل صف الجدول.
        [heading-level] أُزيل مستوى العنوان { $level }.
        [table] أُزيل الجدول.
       *[heading] أُزيل العنوان.
    }
edit-format-unchanged =
    { $what ->
        [bold] عريض: لم يتغيّر شيء.
        [italic] مائل: لم يتغيّر شيء.
        [underline] تسطير: لم يتغيّر شيء.
        [strikethrough] يتوسطه خط: لم يتغيّر شيء.
        [code] شيفرة: لم يتغيّر شيء.
        [code-block] كتلة شيفرة: لم يتغيّر شيء.
        [link] رابط: لم يتغيّر شيء.
        [bulleted-list] قائمة نقطية: لم يتغيّر شيء.
        [numbered-list] قائمة مرقّمة: لم يتغيّر شيء.
        [block-quote] اقتباس: لم يتغيّر شيء.
        [horizontal-rule] خط أفقي: لم يتغيّر شيء.
        [table-row] صف الجدول: لم يتغيّر شيء.
        [heading-level] مستوى العنوان { $level }: لم يتغيّر شيء.
        [table] الجدول: لم يتغيّر شيء.
       *[heading] عنوان: لم يتغيّر شيء.
    }
# Added after a formatting message; $text is the start of the selection.
edit-format-selected = المحدَّد: { $text }
edit-heading-level-now = مستوى العنوان { $level }.
# $line is the line at the caret after the undo or redo.
edit-undo-redo =
    { $what ->
        [undo] تراجع.
       *[redo] إعادة.
    }
edit-undo-redo-line =
    { $what ->
        [undo] تراجع. { $line }
       *[redo] إعادة. { $line }
    }
edit-nothing-to-undo = لا شيء للتراجع عنه.
edit-nothing-to-redo = لا شيء لإعادته.
edit-not-a-table-size = ليس حجم جدول: { $text }. اكتب أعمدة وصفوفًا، مثل 3 by 2.
# $name is the image's file name.
edit-image-inserted = أُدرجت الصورة { $name }. وصفها محدَّد؛ اكتب لاستبداله.
edit-image-failed = تعذّر إدراج الصورة: { $error } لم يتغيّر النص.
# $query is the text to find.
common-no-matches = لا تطابقات لـ{ $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] تطابق واحد لـ{ $query }. الاستبدال بماذا؟
        [two] تطابقان لـ{ $query }. الاستبدال بماذا؟
        [few] { $n } تطابقات لـ{ $query }. الاستبدال بماذا؟
        [many] { $n } تطابقًا لـ{ $query }. الاستبدال بماذا؟
       *[other] { $n } تطابق لـ{ $query }. الاستبدال بماذا؟
    }

## وضع التحرير: الحفظ التلقائي واسترداد العمل غير المحفوظ.

common-recovery-write-failed = تعذّرت كتابة نسخة الاسترداد: { $error }. احفظ قريبًا؛ سيواصل { -brand } المحاولة.
common-recovery-writing-again = تجري كتابة نسخة الاسترداد مجددًا.
# $title is the document; $when is how long ago its work was saved.
edit-recovery-offer = أُغلق { -brand } وبه تغييرات غير محفوظة في { $title }، حُفظت { $when }. استرداده الآن؟ لأعلى ولأسفل للاختيار، Enter للتأكيد.
edit-recovery-title = استرداد العمل غير المحفوظ في { $title }؟
edit-recovery-yes = نعم، استرداد { $title } ومتابعة التحرير
edit-recovery-no = لا، تجاهل التغييرات غير المحفوظة
edit-recovery-discarded = تُجوهلت التغييرات غير المحفوظة في { $title }.
edit-recovered = استُرد العمل غير المحفوظ في { $title }. تذكّر الحفظ.
edit-recovery-postponed = أُجِّل الاسترداد. سيُعرض العمل غير المحفوظ مجددًا في المرة القادمة.

## البحث والاستبدال، تطابق واحد كل مرة.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. السطر: { $context }. اضغط r للاستبدال، s للتخطي، a لاستبدال الكل، Escape للإيقاف.
# The replace list's title when no match is being asked about.
replace-title = استبدال
# $n is this match's number, $total the number of matches, $line the line
# number, $found the matched text, and $result what it becomes (both
# shortened); the -removed form is for an empty replacement. $context in
# replace-match-question is the text of the match's line.
replace-match-title = التطابق { $n } من { $total }، السطر { $line }: { $found } يصبح { $result }
replace-match-title-removed = التطابق { $n } من { $total }، السطر { $line }: يُحذف { $found }
replace-item-this = استبدال هذا
replace-item-skip = تخطي هذا
replace-item-rest = استبدال كل الباقي
# $state is common-on or common-off.
replace-item-match-case = مطابقة حالة الأحرف: { $state }
replace-item-whole-words = الكلمات الكاملة فقط: { $state }
replace-failed = تعذّر الاستبدال: { $error } لم يتغيّر النص.
# Said after switching match case; $state is common-on or common-off, and
# $n is the number of matches now.
replace-match-case-now =
    { $n ->
        [one] مطابقة حالة الأحرف { $state }. تطابق واحد.
        [two] مطابقة حالة الأحرف { $state }. تطابقان.
        [few] مطابقة حالة الأحرف { $state }. { $n } تطابقات.
        [many] مطابقة حالة الأحرف { $state }. { $n } تطابقًا.
       *[other] مطابقة حالة الأحرف { $state }. { $n } تطابق.
    }
replace-whole-words-now =
    { $n ->
        [one] الكلمات الكاملة فقط { $state }. تطابق واحد.
        [two] الكلمات الكاملة فقط { $state }. تطابقان.
        [few] الكلمات الكاملة فقط { $state }. { $n } تطابقات.
        [many] الكلمات الكاملة فقط { $state }. { $n } تطابقًا.
       *[other] الكلمات الكاملة فقط { $state }. { $n } تطابق.
    }
# $query is the text that was searched for.
replace-replaced =
    { $n ->
        [one] استُبدل تطابق واحد.
        [two] استُبدل تطابقان.
        [few] استُبدلت { $n } تطابقات.
        [many] استُبدل { $n } تطابقًا.
       *[other] استُبدل { $n } تطابق.
    }
replace-replaced-skipped = استُبدل { $n }، تُخطي { $skipped }.
replace-stopped = تم الإيقاف. استُبدل { $n }، تُخطي { $skipped }.
# Find and replace with regular expressions (B1-fr). $state is common-on
# or common-off; $n is the number of matches now.
replace-item-regex = تعبير نمطي: { $state }
replace-item-across-lines = عبر الأسطر: { $state }
replace-regex-now =
    { $n ->
        [one] تعبير نمطي { $state }. تطابق واحد.
        [two] تعبير نمطي { $state }. تطابقان.
        [few] تعبير نمطي { $state }. { $n } تطابقات.
        [many] تعبير نمطي { $state }. { $n } تطابقًا.
       *[other] تعبير نمطي { $state }. { $n } تطابق.
    }
replace-across-lines-now =
    { $n ->
        [one] عبر الأسطر { $state }. تطابق واحد.
        [two] عبر الأسطر { $state }. تطابقان.
        [few] عبر الأسطر { $state }. { $n } تطابقات.
        [many] عبر الأسطر { $state }. { $n } تطابقًا.
       *[other] عبر الأسطر { $state }. { $n } تطابق.
    }
# $problem is search-invalid-pattern: switching the option would make the
# pattern invalid.
replace-option-refused = { $problem } لم يتغير الخيار.
# Asked once before replacing all the rest; $n is how many. Keep y and n.
replace-all-question =
    { $n ->
        [one] استبدال التطابق الأخير؟ y أو n
        [two] استبدال التطابقين المتبقيين؟ y أو n
        [few] استبدال { $n } تطابقات متبقية؟ y أو n
        [many] استبدال { $n } تطابقًا متبقيًا؟ y أو n
       *[other] استبدال { $n } تطابق متبقٍ؟ y أو n
    }
replace-all-declined = لم يُستبدل شيء.
# The search options list, and the options as named in search-options-on.
search-options-title = خيارات البحث
search-option-match-case = مطابقة حالة الأحرف
search-option-whole-words = الكلمات الكاملة
search-option-regex = تعبير نمطي
search-option-across-lines = عبر الأسطر
# Said as Find or Replace opens when an option is on; $list joins the
# options' names with commas.
search-options-on = الخيارات المفعّلة: { $list }.
# $at is the character where the pattern fails, counting from 1; $reason
# is the regular expression engine's own explanation (in English).
search-invalid-pattern = نمط غير صالح عند الحرف { $at }: { $reason }.
search-invalid-pattern-anywhere = نمط غير صالح: { $reason }.

## الحفظ في الخلفية.

writes-still-saving = ما زال الحفظ جاريًا. يُرجى الانتظار.
writes-not-written-in-time = تعذّرت كتابة بعض التغييرات في الوقت المناسب: القرص لا يستجيب.
# $error is the system's reason.
writes-save-failed = تعذّر الحفظ: { $error }. ما زلت في وضع التحرير.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-not-saved = الإشارة المرجعية { $name } موضوعة الآن، لكن تعذّر حفظها: { $error } تحقّق من إمكانية الكتابة في مجلد البيانات.
# $name is the saved file's name.
writes-saved = حُفظ { $name }. ما زلت في وضع التحرير.

## الملفات المتغيّرة على القرص. $name اسم ملف. أبقِ على الحرفين y وn:
## فهما المفتاحان للإجابة.

disk-replace-question = { $name } موجود بالفعل. استبداله؟ y أو n
# A prompt label, also said with a full stop after it.
disk-not-replaced = لم يُستبدل. اكتب اسمًا آخر
# $key is the key for Save As.
disk-not-saved = لم يُحفظ. ما زلت في وضع التحرير. حفظ باسم، { $key }، يبقي على النسختين.
disk-kept-open-version = أُبقي على النسخة المفتوحة.
disk-overwrite-question = { $name } تغيّر على القرص منذ فتحته. حفظ التغييرات فوق ذلك؟ y أو n
disk-reload-question = { $name } تغيّر على القرص. إعادة تحميله؟ y أو n

## علامات عُثر عليها مجددًا بعد تغيّر ملف خارج textweaver.

relocate-reading-position = موضع قراءتك
relocate-bookmarks =
    { $n ->
        [one] إشارة مرجعية واحدة
        [two] إشارتان مرجعيتان
        [few] { $n } إشارات مرجعية
        [many] { $n } إشارة مرجعية
       *[other] { $n } إشارة مرجعية
    }
relocate-notes =
    { $n ->
        [one] ملاحظة واحدة
        [two] ملاحظتان
        [few] { $n } ملاحظات
        [many] { $n } ملاحظة
       *[other] { $n } ملاحظة
    }
relocate-highlights =
    { $n ->
        [one] تمييز واحد
        [two] تمييزان
        [few] { $n } تمييزات
        [many] { $n } تمييزًا
       *[other] { $n } تمييز
    }
# Lists of relocate-* items: "a and b", and "a, b, and c", where $rest is
# every item but the last, joined by commas.
relocate-join-two = { $a } و{ $b }
relocate-join-more = { $rest }، و{ $last }
# $items is a list of the items above; $n how many marks it counts in all.
relocate-moved =
    { $n ->
        [one] نُقل { $items } لمطابقتها
       *[other] نُقلت { $items } لمطابقتها
    }
relocate-lost =
    { $n ->
        [one] تعذّر العثور على { $items } وهي موسومة
       *[other] تعذّر العثور على { $items } وهي موسومة
    }
# $clauses are relocate-moved and relocate-lost, joined by a comma.
relocate-changed = تغيّر الملف؛ { $clauses }.

## مستندات جديدة من قوالب.

# The built-in templates' names.
templates-essay = مقالة
templates-report = تقرير
templates-notes = الملاحظات
# One of the user's own templates in the list; $name is its file name.
templates-yours = { $name }، قالبك
# $folder is where the user's own templates go.
templates-intro =
    { $n ->
        [one] مستند جديد من قالب، قالب واحد. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
        [two] مستند جديد من قالب، قالبان. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
        [few] مستند جديد من قالب، { $n } قوالب. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
        [many] مستند جديد من قالب، { $n } قالبًا. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
       *[other] مستند جديد من قالب، { $n } قالب. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
    }
# The title given when none is typed.
templates-untitled = بلا عنوان
# $template is the template's name, $title the document's, $date today's date (2026-09-26).
templates-created = مستند جديد من قالب { $template }: { $title }. بتاريخ { $date }. المؤشر حيث تبدأ الكتابة. تذكّر الحفظ.

## بنية ماركداون كما تُقال في وضع التحرير، قبل نص السطر أو أثناء كتابته.

mdline-heading-level = عنوان من المستوى { $level }
mdline-bullet = نقطة
# A numbered list item; $n is its number.
mdline-item = العنصر { $n }
# $item is mdline-bullet or mdline-item.
mdline-task-done = { $item }، المهمة منجزة
mdline-task-not-done = { $item }، المهمة غير منجزة
mdline-table-row = صف جدول
mdline-quote = اقتباس
mdline-code-fence = سياج شيفرة
mdline-task = مهمة
# Said as "1. " is typed at the start of a line; $n is the number as typed.
mdline-numbered-item = العنصر المرقَّم { $n }

## التنقل في الجداول صفًا وخلية. $dir هو next (للأمام) أو previous (للخلف).

common-not-in-table = لست في جدول.
tables-edge-of-table =
    { $dir ->
        [next] نهاية الجدول.
       *[previous] بداية الجدول.
    }
tables-edge-of-row =
    { $dir ->
        [next] نهاية الصف.
       *[previous] بداية الصف.
    }
# $cell is the cell's text, after its column header and a colon when it has one.
tables-header-row = صف الرأس، { $cell }
tables-row = الصف { $row }، { $cell }
# High verbosity: $message is what the move said, then where it is.
tables-with-position = { $message }. الصف { $row } من { $rows }، العمود { $col } من { $cols }
# Say Position in a table.
tables-position = جدول، الصف { $row } من { $rows }، العمود { $col } من { $cols }.

## مكاسب سريعة للتأليف: عدد الكلمات، والروابط، والحافظة، وخلايا الجدول،
## وحذف الكلمات، والتنقل بين الإعدادات.

# Code block languages said with ordinary words; proper names such as
# Python are not translated.
authoring-language-jsx = جافاسكربت مع JSX
authoring-language-tsx = تايب سكربت مع JSX
authoring-language-shell = صَدَفة
authoring-language-batch = دفعية Windows
authoring-language-c-header = ترويسة C
authoring-language-cpp = سي بلس بلس
authoring-language-csharp = سي شارب
authoring-language-diff = فرق
authoring-language-plain-text = نص عادي
authoring-grammar-not-in-build = التدقيق النحوي غير متوفر في هذه النسخة.
# $count is $n with thousands separators.
authoring-word-count-selection =
    { $n ->
        [one] كلمة واحدة في التحديد.
        [two] كلمتان في التحديد.
        [few] { $count } كلمات في التحديد.
        [many] { $count } كلمة في التحديد.
       *[other] { $count } كلمة في التحديد.
    }
authoring-word-count-document =
    { $n ->
        [one] كلمة واحدة في المستند.
        [two] كلمتان في المستند.
        [few] { $count } كلمات في المستند.
        [many] { $count } كلمة في المستند.
       *[other] { $count } كلمة في المستند.
    }
# $text is the link's text.
authoring-link-address = عنوان الرابط: { $url }
authoring-link-named-address = الرابط { $text }، العنوان: { $url }
authoring-no-link = لا رابط عند المؤشر.
authoring-typing-echo =
    { $echo ->
        [characters-and-words] ترديد الكتابة: الحروف والكلمات.
        [characters] ترديد الكتابة: الحروف.
        [words] ترديد الكتابة: الكلمات.
       *[none] ترديد الكتابة: لا شيء.
    }
authoring-nothing-to-copy = لا شيء محدَّد لنسخه.
# $text is the first words of what was copied.
authoring-copied = نُسخ: { $text }
authoring-copied-sentence = نُسخت الجملة: { $text }
authoring-nothing-to-cut = لا شيء محدَّد لقصه.
authoring-cut = قُصّ: { $text }
# $dir is next (moving forward) or previous.
authoring-table-edge =
    { $dir ->
        [next] نهاية الجدول.
       *[previous] بداية الجدول.
    }
# A column with no header text.
authoring-table-column = العمود { $n }
# Moving into a new row: $header is the column's header, $content the cell.
authoring-table-cell-row = الصف { $row }. { $header }: { $content }
authoring-nothing-to-select = لا شيء لتحديده.
authoring-selected-all =
    { $n ->
        [one] حُدِّد الكل، كلمة واحدة.
        [two] حُدِّد الكل، كلمتان.
        [few] حُدِّد الكل، { $count } كلمات.
        [many] حُدِّد الكل، { $count } كلمة.
       *[other] حُدِّد الكل، { $count } كلمة.
    }
authoring-space-deleted = حُذفت المسافة.
# $text is the word deleted.
authoring-deleted = حُذفت { $text }.
# $key is the terminal's own paste key.
authoring-nothing-copied = لا شيء منسوخ في { -brand } بعد. استخدم لصق طرفيتك، مثل { $key }.
authoring-verbosity =
    { $level ->
        [low] مستوى التفصيل: منخفض.
        [high] مستوى التفصيل: مرتفع.
       *[normal] مستوى التفصيل: عادي.
    }
authoring-punctuation =
    { $level ->
        [none] علامات الترقيم: لا شيء.
        [all] علامات الترقيم: كلها.
       *[some] علامات الترقيم: بعضها.
    }

## فحص ماركداون (وضع التحرير). تُقال المشكلة بعد "Lint: ".

lint-heading-level = مستوى العنوان { $level } بعد المستوى { $prev }؛ استخدم المستوى { $use }.
# $reference is the link reference's name.
lint-link-reference = مرجع الرابط { $reference } بلا تعريف.
lint-bare-url = عنوان ويب مجرَّد؛ ضعه بين قوسين مثلثين أو اجعله رابطًا باسم.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = علامة القائمة { $marker }؛ تستخدم هذه القائمة { $used }.
lint-marker-dash = شرطة
lint-marker-star = نجمة
lint-marker-plus = زائد
lint-marker-other = أخرى
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = علامات جدولة أو مسافات في سطر فارغ.
lint-trailing-tabs-line-end = علامات جدولة أو مسافات في نهاية السطر.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] مسافة واحدة في سطر فارغ.
        [two] مسافتان في سطر فارغ.
        [few] { $n } مسافات في سطر فارغ.
        [many] { $n } مسافة في سطر فارغ.
       *[other] { $n } مسافة في سطر فارغ.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] مسافة واحدة في نهاية السطر.
        [two] مسافتان في نهاية السطر.
        [few] { $n } مسافات في نهاية السطر.
        [many] { $n } مسافة في نهاية السطر.
       *[other] { $n } مسافة في نهاية السطر.
    }
# $key turns on edit mode.
lint-not-editing = يفحص Lint ماركداون الذي تكتبه. فعّل وضع التحرير بـ{ $key } أولًا.
lint-not-markdown = يفحص Lint ماركداون، وهذا المستند ليس ماركداون.
lint-none = لا مشكلات في Lint.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] لا مشكلة Lint أخرى. مشكلة Lint واحدة في المجموع.
       *[other] لا مشكلة Lint أخرى. { $count } مشكلات Lint في المجموع.
    }
lint-no-earlier =
    { $n ->
        [one] لا مشكلة Lint أقدم. مشكلة Lint واحدة في المجموع.
       *[other] لا مشكلة Lint أقدم. { $count } مشكلات Lint في المجموع.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
common-line = السطر { $line }.

## التدقيق النحوي (Harper). $message هي رسالة Harper نفسها، بالإنجليزية.

# $words are the words the problem is about.
grammar-said = القواعد: { $message } الكلمات: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = الإصلاح: إزالتها.
grammar-fix = الإصلاح: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = إزالة الكلمات
grammar-none = لا مشكلات نحوية موجودة.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] لا مشكلة نحوية أخرى. مشكلة نحوية واحدة في المجموع.
       *[other] لا مشكلة نحوية أخرى. { $count } مشكلات نحوية في المجموع.
    }
grammar-no-earlier =
    { $n ->
        [one] لا مشكلة نحوية أقدم. مشكلة نحوية واحدة في المجموع.
       *[other] لا مشكلة نحوية أقدم. { $count } مشكلات نحوية في المجموع.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } يسرد الإصلاحات.
# Added at high verbosity.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } لا إصلاح لعرضه.
grammar-fixes =
    { $n ->
        [one] { $words }: إصلاح واحد.
        [two] { $words }: إصلاحان.
        [few] { $words }: { $n } إصلاحات.
        [many] { $words }: { $n } إصلاحًا.
       *[other] { $words }: { $n } إصلاح.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: إصلاح واحد. Enter لإجراء التغيير.
        [two] { $words }: إصلاحان. Enter لإجراء التغيير.
        [few] { $words }: { $n } إصلاحات. Enter لإجراء التغيير.
        [many] { $words }: { $n } إصلاحًا. Enter لإجراء التغيير.
       *[other] { $words }: { $n } إصلاح. Enter لإجراء التغيير.
    }
common-left-as-is = تُركت كما هي.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. فعّل وضع التحرير بـ{ $key } لتغيير النص.
grammar-removed = أُزيلت.
grammar-changed = غُيِّرت إلى { $fix }.
grammar-change-failed = تعذّر تغيير النص: { $error } لم يتغيّر شيء.

## التدقيق الإملائي.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = فاصلة عليا
spell-not-available = التدقيق الإملائي غير متوفر: هذه النسخة بلا قائمة كلمات.
spell-none-found = لا أخطاء إملائية موجودة.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] لا خطأ إملائي آخر. خطأ إملائي محتمل واحد في المجموع.
       *[other] لا خطأ إملائي آخر. { $count } أخطاء إملائية محتملة في المجموع.
    }
spell-no-earlier =
    { $n ->
        [one] لا خطأ إملائي أقدم. خطأ إملائي محتمل واحد في المجموع.
       *[other] لا خطأ إملائي أقدم. { $count } أخطاء إملائية محتملة في المجموع.
    }
# Added at high verbosity.
spell-no-misspelled-word = لا كلمة خطأ إملائيًا عند المؤشر.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: لا اقتراحات.
        [one] { $word }: اقتراح واحد.
        [two] { $word }: اقتراحان.
        [few] { $word }: { $n } اقتراحات.
        [many] { $word }: { $n } اقتراحًا.
       *[other] { $word }: { $n } اقتراح.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: لا اقتراحات. Enter لاستبدال الكلمة.
        [one] { $word }: اقتراح واحد. Enter لاستبدال الكلمة.
        [two] { $word }: اقتراحان. Enter لاستبدال الكلمة.
        [few] { $word }: { $n } اقتراحات. Enter لاستبدال الكلمة.
        [many] { $word }: { $n } اقتراحًا. Enter لاستبدال الكلمة.
       *[other] { $word }: { $n } اقتراح. Enter لاستبدال الكلمة.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. فعّل وضع التحرير بـ{ $key } لتغيير النص.
spell-replaced = استُبدلت بـ{ $word }.
spell-replace-failed = تعذّر الاستبدال: { $error } لم تتغيّر الكلمة.
spell-added-for-session = أُضيفت { $word } إلى قائمة كلماتك لهذه الجلسة.
spell-added = أُضيفت { $word } إلى قائمة كلماتك.
spell-save-failed = تعذّر حفظ قائمة كلماتك: { $error } تبقى الكلمة معروفة حتى تخرج.
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] لا أخطاء إملائية.
        [one] خطأ إملائي محتمل واحد.
       *[other] { $count } أخطاء إملائية محتملة.
    }

## بدء تشغيل قارئ الطرفية.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = محرك الكلام { $wanted } غير متوفر؛ يُستخدم { $backend }.
tui-setup-speech-failed = تعذّر بدء الكلام ({ $error })؛ التشغيل بصمت.
speech-engine-fallback = تعذّر بدء { $failed }؛ يتحدث { $engine } بدلًا منه.
speech-engine-fallback-silent = تعذّر بدء { $failed } ولا يتوفر محرك كلام آخر؛ يبقى { -brand } صامتًا.
tui-setup-cannot-save = تعذّر حفظ الإعدادات أو المواضع: { $error } القراءة تعمل. تضيع التغييرات عند الخروج.
tui-setup-keymap-ignored = جرى تجاهل ملف خريطة المفاتيح: { $error } تُستخدم المفاتيح الافتراضية. أصلح الملف، ثم أعد التشغيل.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = مرحبًا بك في { -brand }. { $open } يفتح مستندًا. { $play } يبدأ القراءة ويوقفها مؤقتًا، و{ $stop } يوقفها. { $palette } يسرد كل الأوامر. { $help } يفتح المساعدة.
gui-setup-welcome-menus = مرحبًا بك في { -brand }. { $open } يفتح مستندًا. { $play } يقرأ ويوقف مؤقتًا، و{ $stop } يوقف. { $palette } يسرد كل الأوامر، وAlt أو { $menu } القوائم. { $help } يفتح المساعدة.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = لا يوجد مستند مفتوح. اضغط { $open } لفتح واحد، { $new } لواحد جديد، أو { $help } للمساعدة.

## إطلاق قارئ الطرفية.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = تعذّر فتح { $name }: { $error }

## النسخ في قارئ الطرفية.

tui-clip-system = نُسخ باستخدام حافظة النظام، لأن هذه الطرفية لا تقبل نصًا منسوخًا.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = تعذّر النسخ بحافظة النظام: { $error }. أُرسل إلى الطرفية بدلًا من ذلك.
tui-clip-not-available = حافظة النظام غير متوفرة
tui-clip-not-built = هذه النسخة بلا حافظة نظام

## شاشة قارئ الطرفية.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = لا يوجد مستند
# The screen without a document. $keys names the keys for the action.
tui-empty-open = فتح واحد: { $keys }.
tui-empty-help = المساعدة: { $keys }.
tui-empty-quit = إنهاء: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y نعم  n أو a لا  { $escape } لا
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = تشغيل
tui-hint-sentence = جملة
tui-hint-faster = أسرع
tui-hint-slower = أبطأ
tui-hint-close-rsvp = إغلاق العرض السريع
tui-hint-quit = إنهاء
tui-hint-save = حفظ
tui-hint-finish = إنهاء
tui-hint-undo = تراجع
tui-hint-bold = عريض
tui-hint-heading = عنوان
tui-hint-commands = أوامر
tui-hint-next-line = السطر التالي
tui-hint-previous-line = السطر السابق
tui-hint-again = مجددًا
tui-hint-read-on = متابعة القراءة
tui-hint-leave = مغادرة
tui-hint-paragraph = فقرة
tui-hint-find = بحث
tui-hint-mark = وضع علامة
tui-hint-lines = أسطر
tui-hint-keys = مفاتيح
tui-hint-choose = اختيار
tui-hint-close = إغلاق
tui-hint-back = رجوع
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } من { $count }، { $title }

text-summary =
    { $change ->
        [selected] تم تحديد { $count } من الأحرف
        [unselected] أُلغي تحديد { $count } من الأحرف
        [copied] نُسخ { $count } من الأحرف
        [cut] قُصّ { $count } من الأحرف
       *[deleted] حُذف { $count } من الأحرف
    }
text-summary-range =
    { $change ->
        [selected] تم تحديد { $count } من الأحرف، من { $first } إلى { $last }
        [unselected] أُلغي تحديد { $count } من الأحرف، من { $first } إلى { $last }
        [copied] نُسخ { $count } من الأحرف، من { $first } إلى { $last }
        [cut] قُصّ { $count } من الأحرف، من { $first } إلى { $last }
       *[deleted] حُذف { $count } من الأحرف، من { $first } إلى { $last }
    }
voice-character-keys-on = الاختصارات أحادية المفتاح مفعّلة.
voice-character-keys-off = الاختصارات أحادية المفتاح معطّلة.
goto-word-start = start
goto-word-end = end

language-voices-loading = لا تزال قائمة الأصوات قيد التحميل، لذا يستمر الصوت الحالي في التحدث.

## The window (GUI)

gui-open-title = فتح مستند
gui-open-documents = المستندات التي يقرؤها textweaver
gui-open-all-files = كل الملفات
gui-open-no-dialog = لم يُفتح منتقي الملفات في النظام. اكتب مسار المستند بدلًا من ذلك.
gui-text-size = حجم النص { $size } نقطة.
gui-text-size-largest = حجم النص { $size } نقطة، وهو الأكبر.
gui-text-size-smallest = حجم النص { $size } نقطة، وهو الأصغر.
gui-font = الخط: { $family }.
gui-font-list = الخط

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

## تصفية المكتبة والقاموس والسرعات.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = المكتبة، { $shown } من { $n } يطابق { $filter }
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] مُسحت التصفية، مستند واحد.
        [two] مُسحت التصفية، مستندان.
        [few] مُسحت التصفية، { $n } مستندات.
        [many] مُسحت التصفية، { $n } مستندًا.
       *[other] مُسحت التصفية، { $n } مستند.
    }
# No document matches the filter $query.
library-filter-none = لا مستندات تطابق { $query }. Backspace لحذف الحروف.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] مستند واحد مطابق.
        [two] مستندان مطابقان.
        [few] { $n } مستندات مطابقة.
        [many] { $n } مستندًا مطابقًا.
       *[other] { $n } مستند مطابق.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = ما زال القاموس قيد التحميل.
# إعدادات.
setting-speech-dectalk-library = مكتبة DECtalk
setting-speech-dectalk-library-help = مكتبة DECtalk المراد تحميلها؛ عدم الضبط يبحث في الأماكن المعتادة.
setting-speech-piper-voices = مجلد أصوات Piper
setting-speech-piper-voices-help = مجلد أصوات Piper؛ عدم الضبط يستخدم مجلد piper في مجلد بيانات textweaver.
setting-speech-piper-voice = صوت Piper
setting-speech-piper-voice-help = صوت Piper للبدء به، حسب المعرّف؛ عدم الضبط يأخذ أول صوت مثبَّت.
setting-speech-piper-phonemizer = المحوِّل الصوتي لـ Piper
setting-speech-piper-phonemizer-help = كيف يحوّل Piper النص إلى أصوات. مكتبة espeak-ng إن كانت مثبَّتة، أو تلك المكتبة، أو محوِّل textweaver.
choice-speech-piper-phonemizer-auto = تلقائي
choice-speech-piper-phonemizer-library = مكتبة espeak-ng
choice-speech-piper-phonemizer-rust = محوِّل textweaver
setting-speech-voice-params = السرعة وطبقة الصوت لكل صوت
setting-speech-voice-params-help = السرعة وطبقة الصوت اللتان استُخدم بهما كل صوت آخر مرة. اختيار الصوت مجددًا يعيدهما.
setting-editing-author = المؤلف
setting-editing-author-help = المؤلف الذي يُكتب في المستندات الجديدة المنشأة من قالب؛ تركه فارغًا يبقيه خاليًا.

## The window (GUI): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = نعم
gui-no = لا
gui-answer-delete = حذف
gui-answer-remove = إزالة
gui-answer-replace = استبدال
gui-question-hint = Y للإجابة بنعم، وN للإجابة بلا، وEscape للإجابة بلا.
gui-button-open = فتح…
gui-button-font = الخط…
gui-button-edit = بدء التحرير
gui-button-finish-editing = إنهاء التحرير
gui-button-settings = الإعدادات…
gui-button-commands = الأوامر…
gui-button-play = تشغيل
gui-button-pause = إيقاف مؤقت
gui-button-stop = إيقاف
gui-button-previous-sentence = الجملة السابقة
gui-button-next-sentence = الجملة التالية
gui-button-slower = أبطأ
gui-button-faster = أسرع
gui-hint-open = اختيار مستند للقراءة.
gui-hint-font = اختيار خط نص المستند.
gui-hint-edit = التبديل بين القراءة والتحرير.
gui-hint-settings = كل خيار، مع مساعدته.
gui-hint-commands = تشغيل أي أمر باسمه.
gui-hint-play = القراءة من الكلمة الحالية أو الإيقاف.
gui-button-close = إغلاق
gui-toolbar-reading = القراءة
gui-document = المستند
gui-document-titled = { $title }، المستند
gui-list-hint = Enter للاختيار، وEscape للإغلاق.
gui-sidebar-contents = المحتويات
gui-sidebar-notes = الملاحظات
gui-sidebar-open =
    { $n ->
        [one] { $panel } مفتوحة، عنصر واحد.
       *[other] { $panel } مفتوحة، { $n } عناصر.
    }
gui-sidebar-closed = { $panel } مغلقة.
gui-header-shown = الترويسة ظاهرة.
gui-header-hidden = الترويسة مخفية. تحتفظ أوامرها بمفاتيحها.
gui-toolbar-shown = شريط الأدوات ظاهر.
gui-toolbar-hidden = شريط الأدوات مخفي. تحتفظ أوامره بمفاتيحها.
# The preview pane beside the editor (edit mode). $key is the key that moves between the window's parts (F6).
gui-preview = المعاينة
gui-preview-shown = المعاينة ظاهرة بجانب المحرر. { $key } ينتقل إليها.
gui-preview-hidden = المعاينة مخفية.
gui-preview-later = المعاينة مفعّلة. تظهر بجانب المحرر في وضع التحرير.
gui-preview-empty = لا شيء للمعاينة بعد.
gui-sidebar-no-headings = لا توجد عناوين.
gui-sidebar-no-notes = لا توجد ملاحظات.
gui-sidebar-current = { $item }، الحالي
gui-sidebar-hint = Enter ينتقل إليه. { $leave } ينتقل ويعود. Escape يعود.
gui-settings-sections = الأقسام
gui-settings-form = إعدادات { $section }
gui-settings-saved-hint = تسري التغييرات وتُحفظ فورًا.
gui-settings-close-help = إغلاق الإعدادات. كل تغيير محفوظ بالفعل.
gui-settings-recent = المُغيَّرة مؤخرًا
gui-settings-matching = المطابقة لـ { $filter }
gui-settings-table = { $label } جدول. حرّره في settings.toml.
gui-setting-new-value = قيمة جديدة لـ { $label }
gui-setting-value-hint = اضغط Enter للقبول، أو Escape للرجوع.
gui-prompt-path-hint = اكتب مسار مستند، ثم اضغط Enter. يكمله Tab، ويستعيد السهمان لأعلى ولأسفل المسارات السابقة.
gui-prompt-hint = اضغط Enter للقبول، أو Escape للإلغاء. يستعيد السهمان لأعلى ولأسفل الإجابات السابقة.
gui-palette-filter = اكتب لتصفية الأوامر
gui-palette-list = الأوامر
gui-palette-hint = يشغّل Enter أول نتيجة مطابقة، وينتقل Tab إلى القائمة، ويشرح F1 الأمر.
gui-open-failed = تعذّر فتح { $name }: { $error }
gui-uia-unavailable = إشعارات UI Automation متاحة في Windows فقط؛ ستُستخدم المنطقة الحية.
gui-graphics-failed = تعذّر على النافذة تشغيل الرسوميات. قارئ الطرفية textweaver لا يحتاج إليها.
gui-crashed = توقف textweaver بعد خطأ داخلي.
gui-crashed-saved = حُفظ موضعك، وستُعرض التعديلات غير المحفوظة للاستعادة في المرة القادمة.
gui-rsvp = RSVP
gui-rsvp-playing = RSVP قيد التشغيل، الكلمة { $n } من { $total }
gui-rsvp-paused = RSVP متوقف مؤقتًا، الكلمة { $n } من { $total }
gui-rsvp-finished = RSVP انتهى، الكلمة { $n } من { $total }
gui-settings-section-item =
    { $section }، { $n ->
        [0] لا إعدادات
        [one] إعداد واحد
        [two] إعدادان
        [few] { $n } إعدادات
        [many] { $n } إعدادًا
       *[other] { $n } إعداد
    }
gui-palette-count =
    { $n ->
        [0] لا يطابق أي أمر.
        [one] أمر واحد.
        [two] أمران.
        [few] { $n } أوامر.
        [many] { $n } أمرًا.
       *[other] { $n } أمر.
    }
gui-settings-form-help = ينتقل السهمان لأعلى ولأسفل بين الإعدادات. ويغيّر السهمان لليسار ولليمين إعدادًا. ويكتب Enter قيمة جديدة. ويعيد Delete القيمة الافتراضية. ويغيّر { $next } و{ $previous } القسم. اكتب للتصفية. ويقول F1 التعليمات.
gui-settings-press-enter = اضغط Enter لكتابة قيمة جديدة لـ { $label }.
gui-font-built-in = { $family } (مضمّن)
## Summaries and difficult-word definitions.

action-summarize = تلخيص التحديد أو الفصل أو المستند: أهم جمله في قائمة؛ Enter ينتقل إلى إحداها
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
setting-summary-sentences = جمل الملخص
setting-summary-sentences-help = عدد الجمل التي يعطيها التلخيص و tw summarize، من 1 إلى 50.
setting-reading-aids-difficult-definitions = تعريفات الكلمات الصعبة
setting-reading-aids-difficult-definitions-help = عند تمييز الكلمات الصعبة، نطق التعريف الأول للكلمة الصعبة من القاموس أيضًا في مستوى التفصيل المرتفع.
section-summary = الملخصات
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

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = ليس ملف JSON قابلًا للقراءة؛ قد يكون كبيرًا جدًا.
opening-damaged-notebook = ليس دفتر Jupyter قابلًا للقراءة؛ قد يكون تالفًا أو كبيرًا جدًا.
opening-damaged-svg = ليس رسمًا بصيغة SVG قابلًا للقراءة؛ قد يكون تالفًا أو كبيرًا جدًا.
opening-damaged-mathml = ليست صيغة MathML قابلة للقراءة؛ قد تكون تالفة أو كبيرة جدًا.

## Menus, the command palette, interface announcements, colors, and settings

## Menu titles; the top menus mark their access key with &.

menu-file = ملف
menu-edit = تحرير
menu-view = عرض
menu-reading = قراءة
menu-speech = النطق
menu-tools = أدوات
menu-help = مساعدة
menu-recent = المستندات الأخيرة
menu-export-as = تصدير بصيغة
menu-preview = المعاينة
menu-settings = الإعدادات
menu-find = بحث
menu-format = تنسيق
menu-insert = إدراج
menu-proofing = التدقيق
menu-citations = الاستشهادات
menu-text-size = حجم النص
menu-reading-aids = مساعدات القراءة
menu-rsvp = RSVP
menu-say = نطق
menu-move-by = التنقل حسب
menu-headings = العناوين
menu-go-to = انتقال
menu-cursor = المؤشر والتحديد
menu-bookmarks = العلامات والملاحظات
menu-tables = الجداول
menu-speech-cursor = مؤشر النطق

## Command names, in the menus and the command palette.

name-play-pause = تشغيل أو إيقاف مؤقت
name-stop = إيقاف
name-read-from-cursor = القراءة من المؤشر
name-read-document = قراءة المستند كله
name-read-current-character = نطق الحرف
name-read-current-word = نطق الكلمة
name-read-current-sentence = نطق الجملة
name-read-current-line = نطق السطر
name-read-paragraph = نطق الفقرة
name-read-selection = قراءة التحديد
name-say-position = نطق الموضع
name-say-status = نطق الحالة
name-repeat-message = تكرار آخر رسالة
name-word-count = عدد الكلمات
name-link-address = عنوان الرابط
name-replay-sentence = إعادة قراءة الجملة
name-replay-paragraph = إعادة قراءة الفقرة
name-repeat-sentence-slower = إعادة أبطأ
name-rsvp-toggle = RSVP
name-rsvp-play-pause = بدء RSVP أو إيقافه مؤقتًا
name-rsvp-faster = RSVP أسرع
name-rsvp-slower = RSVP أبطأ
name-rsvp-position-next = نقل كلمة RSVP
name-reading-level = مستوى القراءة
name-document-overview = نظرة عامة على المستند
name-reading-pass = نمط القراءة
name-define-word = تعريف الكلمة
name-summarize = تلخيص
name-toggle-citations = قراءة الاستشهادات
name-explore-math = استكشاف الرياضيات
name-listen-rendered = الاستماع كما سيُعرض
name-next-sentence = الجملة التالية
name-previous-sentence = الجملة السابقة
name-next-paragraph = الفقرة التالية
name-previous-paragraph = الفقرة السابقة
name-next-heading = القراءة من العنوان التالي
name-previous-heading = القراءة من العنوان السابق
name-skip-next-heading = العنوان التالي
name-skip-previous-heading = العنوان السابق
name-outline = المخطط
name-next-heading-level-1 = العنوان التالي، المستوى 1
name-next-heading-level-2 = العنوان التالي، المستوى 2
name-next-heading-level-3 = العنوان التالي، المستوى 3
name-next-heading-level-4 = العنوان التالي، المستوى 4
name-next-heading-level-5 = العنوان التالي، المستوى 5
name-next-heading-level-6 = العنوان التالي، المستوى 6
name-previous-heading-level-1 = العنوان السابق، المستوى 1
name-previous-heading-level-2 = العنوان السابق، المستوى 2
name-previous-heading-level-3 = العنوان السابق، المستوى 3
name-previous-heading-level-4 = العنوان السابق، المستوى 4
name-previous-heading-level-5 = العنوان السابق، المستوى 5
name-previous-heading-level-6 = العنوان السابق، المستوى 6
name-next-table = الجدول التالي
name-previous-table = الجدول السابق
name-next-list = القائمة التالية
name-previous-list = القائمة السابقة
name-next-list-item = عنصر القائمة التالي
name-previous-list-item = عنصر القائمة السابق
name-next-link = الرابط التالي
name-previous-link = الرابط السابق
name-next-block-quote = الاقتباس التالي
name-previous-block-quote = الاقتباس السابق
name-next-separator = الفاصل التالي
name-previous-separator = الفاصل السابق
name-next-graphic = الصورة التالية
name-previous-graphic = الصورة السابقة
name-follow-link = اتباع الرابط
name-table-next-row = صف الجدول التالي
name-table-previous-row = صف الجدول السابق
name-table-next-column = عمود الجدول التالي
name-table-previous-column = عمود الجدول السابق
name-next-chapter = الفصل التالي
name-previous-chapter = الفصل السابق
name-history-back = رجوع
name-history-forward = تقدم
name-go-to = انتقال إلى
name-document-start = بداية المستند
name-document-end = نهاية المستند
name-caret-next-word = الكلمة التالية
name-caret-previous-word = الكلمة السابقة
name-caret-next-line = السطر التالي
name-caret-previous-line = السطر السابق
name-select-next-word = تحديد الكلمة التالية
name-select-previous-word = تحديد الكلمة السابقة
name-select-next-line = تحديد السطر التالي
name-select-previous-line = تحديد السطر السابق
name-page-down = صفحة للأسفل
name-page-up = صفحة للأعلى
name-scroll-down = تمرير للأسفل
name-scroll-up = تمرير للأعلى
name-speech-cursor-toggle = مؤشر النطق
name-speech-cursor-next-line = مؤشر النطق، السطر التالي
name-speech-cursor-previous-line = مؤشر النطق، السطر السابق
name-speech-cursor-reread-line = مؤشر النطق، إعادة السطر
name-speech-cursor-exit-and-read = مؤشر النطق، متابعة القراءة
name-rate-up = أسرع
name-rate-down = أبطأ
name-pitch-up = طبقة أعلى
name-pitch-down = طبقة أخفض
name-volume-up = صوت أعلى
name-volume-down = صوت أخفض
name-cycle-speed-preset = سرعة مسبقة
name-choose-voice = الأصوات
name-restart-speech = إعادة تشغيل النطق
name-cycle-verbosity = مستوى التفصيل
name-cycle-punctuation = علامات الترقيم
name-find = بحث
name-find-next = بحث عن التالي
name-find-previous = بحث عن السابق
name-search-options = خيارات البحث
name-next-misspelling = الخطأ الإملائي التالي
name-previous-misspelling = الخطأ الإملائي السابق
name-spelling-suggestions = اقتراحات الإملاء
name-next-grammar-problem = مشكلة النحو التالية
name-previous-grammar-problem = مشكلة النحو السابقة
name-next-lint-problem = مشكلة التنسيق التالية
name-previous-lint-problem = مشكلة التنسيق السابقة
name-add-bookmark = إضافة علامة
name-list-bookmarks = العلامات
name-next-bookmark = العلامة التالية
name-previous-bookmark = العلامة السابقة
name-add-note = إضافة ملاحظة
name-list-notes = الملاحظات
name-next-note = الملاحظة التالية
name-previous-note = الملاحظة السابقة
name-delete-note = حذف الملاحظة أو التمييز
name-highlight-selection = تمييز
name-export-study-sheet = تصدير ورقة الدراسة
name-self-test = اختبار ذاتي
name-open = فتح
name-open-path = فتح بالمسار
name-open-library = المكتبة
name-new-document = مستند جديد
name-save = حفظ
name-save-as = حفظ باسم
name-export-settings = تصدير الإعدادات
name-import-settings = استيراد الإعدادات
name-reading-statistics = إحصاءات القراءة
name-new-from-template = جديد من قالب
name-export-html = تصدير HTML
name-export-pdf = تصدير PDF
name-export-docx = تصدير Word
name-export-epub = تصدير EPUB
name-export-brf = تصدير برايل
name-preview-in-browser = معاينة في المتصفح
name-toggle-preview-auto-reload = إعادة تحميل المعاينة تلقائيًا
name-toggle-preview-live = معاينة مباشرة
name-browse-files = تصفح الملفات
name-batch-convert = تحويل دفعة
name-export-audio = تصدير الصوت
name-quit = خروج
name-toggle-edit-mode = وضع التحرير
name-undo = تراجع
name-redo = إعادة
name-bold = غامق
name-italic = مائل
name-underline = تسطير
name-strikethrough = يتوسطه خط
name-inline-code = رمز ضمن السطر
name-code-block = كتلة رمز
name-insert-link = إدراج رابط
name-heading = عنوان
name-bullet-list = قائمة نقطية
name-numbered-list = قائمة مرقمة
name-block-quote = اقتباس كتلة
name-horizontal-rule = خط أفقي
name-insert-table = إدراج جدول
name-add-table-row = إضافة صف إلى الجدول
name-insert-image = إدراج صورة
name-replace = استبدال
name-copy = نسخ
name-cut = قص
name-next-table-cell = الخلية التالية
name-previous-table-cell = الخلية السابقة
name-cycle-typing-echo = صدى الكتابة
name-select-all = تحديد الكل
name-delete-word-before = حذف الكلمة السابقة
name-delete-word-after = حذف الكلمة التالية
name-paste = لصق
name-insert-citation = إدراج استشهاد
name-add-reference = إضافة مرجع
name-insert-bibliography = إدراج قائمة المراجع
name-check-citations = فحص الاستشهادات
name-import-references = استيراد المراجع
name-dictate = إملاء
name-next-theme = السمة التالية
name-toggle-line-numbers = أرقام الأسطر
name-toggle-character-keys = اختصارات المفتاح الواحد
name-cycle-access-mode = وضع إمكانية الوصول
name-settings-profiles = ملفات التعريف
name-bionic-toggle = القراءة البيونية
name-ruler-cycle = مسطرة القراءة
name-syllables-toggle = المقاطع
name-difficult-words-toggle = الكلمات الصعبة
name-text-larger = نص أكبر
name-text-smaller = نص أصغر
name-text-size-reset = حجم النص القياسي
name-choose-font = الخط
name-contents-panel = لوحة المحتويات
name-notes-panel = لوحة الملاحظات
name-toggle-header = الترويسة
name-toggle-toolbar = شريط الأدوات
name-show-preview = إظهار المعاينة
name-next-region = المنطقة التالية
name-previous-region = المنطقة السابقة
name-color-settings = الألوان
name-cycle-interface-announcements = إعلانات الواجهة
name-menu = القوائم
name-command-palette = لوحة الأوامر
name-settings = الإعدادات
name-keyboard-help = اختصارات لوحة المفاتيح
name-what-does-this-key-do = ماذا يفعل هذا المفتاح
name-about = حول textweaver
name-help = مساعدة

## Menus, the palette, and interface announcements.

menu-bar = القوائم
menu-title = قائمة { $name }
menu-submenu = { $name }، قائمة فرعية
menu-checked = { $name }، محدد
menu-not-checked = { $name }، غير محدد
menu-value = { $name }: { $value }
menu-with-keys = { $item }، { $keys }
menu-recent-document = { $name }، { $pct } بالمئة
menu-recent-none = لا توجد مستندات حديثة
menu-not-available = { $name } غير متاح في هذا الإصدار.
menu-no-access-key = لا يوجد عنصر بالمفتاح { $letter }.
menu-closed = أُغلقت القوائم.
menu-press-a-key = اضغط مفتاحًا لتسمع ما يفعله.
menu-key-described = { $name }: { $help }. المفاتيح: { $keys }. في القوائم: { $path }.
menu-key-described-no-menu = { $name }: { $help }. المفاتيح: { $keys }.
menu-keys = { $item }. يفتح Enter أو السهم الأيمن قائمة أو ينفذ أمرًا، وينتقل الحرف إلى عنصره، ويعود السهم الأيسر أو Backspace، ويغلق Escape.
edit-line-continues = يستمر السطر
announce-level-changed = إعلانات الواجهة: { $level }.
announce-level-off = متوقفة
announce-level-minimal = في حدها الأدنى
announce-level-normal = عادية
announce-level-full = كاملة
setting-accessibility-interface-announcements = إعلانات الواجهة
setting-accessibility-interface-announcements-help = مقدار ما يقوله textweaver عن نفسه: مربعات الحوار والتقدم والتلميحات والتأكيدات المعتادة. تُقال الأخطاء والإجابات عما طلبته دائمًا. التلقائي هو الحد الأدنى مع قارئ الشاشة، والعادي عند النطق الذاتي.
choice-accessibility-interface-announcements-auto = تلقائي
choice-accessibility-interface-announcements-off = متوقفة
choice-accessibility-interface-announcements-minimal = في حدها الأدنى
choice-accessibility-interface-announcements-normal = عادية
choice-accessibility-interface-announcements-full = كاملة
palette-item = { $name }، { $keys }
palette-item-no-keys = { $name }
palette-item-recent = { $name }، { $keys }، حديث
palette-item-recent-no-keys = { $name }، حديث
palette-list-title = الأوامر المطابقة لـ { $query }
palette-list-title-all = الأوامر
palette-list-intro =
    { $title }، { $n ->
        [one] أمر واحد
       *[other] { $n } أوامر
    }. يشغّل Enter أحدها، ويشرحه F1.
action-browse-files = تصفح الملفات والأرشيفات: يفتح Enter مجلدًا أو أرشيفًا أو مستندًا، ويصعد Backspace مستوى
action-batch-convert = تحويل مجلد من المستندات إلى صيغة أخرى في الخلفية
action-export-audio = تصدير المستند صوتًا منطوقًا: MP3 أو FLAC أو Opus أو WAV أو كتاب صوتي M4B
action-dictate = بدء الإملاء أو إيقافه: تُكتب الكلمات المنطوقة عند المؤشر في وضع التحرير
action-color-settings = فتح إعدادات الألوان: تمييز القراءة والمسطرة والعلامات وكل جزء من الشاشة، مع تباينها
action-cycle-interface-announcements = التبديل بين مقادير ما يعلنه textweaver عن نفسه: متوقفة أو في حدها الأدنى أو عادية أو كاملة؛ تُقال الأخطاء والإجابات دائمًا
action-menu = فتح القوائم: ملف وتحرير وعرض وقراءة والنطق وأدوات ومساعدة
action-what-does-this-key-do = اضغط مفتاحًا لتسمع ما يفعله وأين يوجد في القوائم دون تشغيله
action-about = عرض المعلومات التي يحتاجها بلاغ المشكلة: الإصدار والبناء والمكونات ومحركات الكلام والمجلدات
setting-colors-ruler = لون مسطرة القراءة
setting-colors-ruler-help = شريط مسطرة القراءة والسطر الحالي المحدد. تحتفظ المسطرة بتسطيرها أو خطها الغامق. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-difficult-words = لون الكلمات الصعبة
setting-colors-difficult-words-help = تسطير الكلمات الصعبة؛ تبقى مسطرة وتُذكر عند التفصيل العالي. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-syllables = لون علامات المقاطع
setting-colors-syllables-help = النقاط الوسطى بين المقاطع. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-misspellings = لون الأخطاء الإملائية
setting-colors-misspellings-help = تسطير الكلمات الخاطئة إملائيًا في النافذة؛ وتُقال أيضًا. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-lint = لون علامات التنسيق
setting-colors-lint-help = تسطير مشكلات تنسيق Markdown والنحو في النافذة؛ وتُقال أيضًا. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-find-match = لون نتائج البحث
setting-colors-find-match-help = الشريط خلف نتائج البحث؛ تبقى مسطرة. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-selection = لون التحديد
setting-colors-selection-help = الشريط خلف النص المحدد. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-focus = لون التركيز
setting-colors-focus-help = إطار التركيز والعنصر المحدد في القائمة؛ يبقيان غامقين. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-links = لون الروابط
setting-colors-links-help = لون الروابط؛ تبقى مسطرة. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-headings = لون العناوين
setting-colors-headings-help = لون العناوين؛ تبقى غامقة. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-status-bar = لون شريط الحالة
setting-colors-status-bar-help = شريط الحالة وشريط العنوان. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-notes = لون الملاحظات
setting-colors-notes-help = الشريط خلف النص ذي الملاحظة؛ يبقى مائلًا ومسطرًا. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
setting-colors-bookmarks = لون العلامات
setting-colors-bookmarks-help = الشريط خلف الكلمة ذات العلامة؛ تبقى غامقة ومسطرة. اختر اسمًا، أو اكتب رمزًا سداسيًا عشريًا. الافتراضي: لون السمة.
color-name-theme = لون السمة
color-name-blue = أزرق
color-name-orange = برتقالي
color-name-navy = أزرق داكن
color-name-skyblue = أزرق سماوي
color-name-teal = أزرق مخضر
color-name-gold = ذهبي
color-name-yellow = أصفر
color-name-purple = بنفسجي
color-name-pink = وردي
color-name-brown = بني
color-name-gray = رمادي
color-name-black = أسود
color-name-white = أبيض
section-colors = الألوان
colors-contrast-good = جيد
colors-contrast-fair = مقبول
colors-contrast-low = منخفض
colors-item = { $label }: { $value }، التباين { $ratio } إلى 1، { $verdict }
colors-contrast = التباين { $ratio } إلى 1، { $verdict }.
colors-contrast-warning = أقل من 3 إلى 1 يصعب رؤيته؛ اختر لونًا أفتح أو أغمق.
colors-intro = الألوان، { $n } إعدادات. يختار اليسار واليمين لونًا مسمى، ويكتب Enter اسمًا أو قيمة ‎#rrggbb، ويعيد Delete لون السمة، ويقول F1 المساعدة.
settings-item-recent = { $item }، غُيّر مؤخرًا
settings-reset = عاد { $label } إلى قيمته الافتراضية، { $value }.
settings-row-help = { $label }: { $value }. الافتراضي: { $default }. { $help }
number-group-separator = ,
number-decimal-separator = .
settingsio-import-question-names =
    { $n ->
        [one] استيراد إعداد واحد متغير من { $name }: { $names }؟ y أو n
       *[other] استيراد { $n } إعدادات متغيرة من { $name }: { $names }؟ y أو n
    }
settingsio-and-more = { $names } و{ $n } غيرها


## Dictation in edit mode (ADR-0042). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $words are the
## dictated words, $key the dictate key, $dir a folder, $error and $text
## are passed on as they are.
dictation-status = إملاء: { $words }
dictation-listening = الإملاء جارٍ. تكلّم، ثم اضغط { $key } للإيقاف.
dictation-finishing = جارٍ إنهاء الإملاء.
dictation-done = انتهى الإملاء.
dictation-busy = الإملاء ينتهي. حاول مرة أخرى بعد لحظة.
dictation-needs-edit = الإملاء يكتب في وضع التحرير. هل تشغّل وضع التحرير وتملي؟ y أو n
dictation-no-model = يحتاج الإملاء إلى نموذج Whisper في { $dir }. راجع Dictation في الوثائق.
dictation-failed = فشل الإملاء: { $error } راجع Dictation في الوثائق.
dictation-no-words = لم تُعرف أي كلمات في تلك العبارة.
dictation-lost = توقف الإملاء قبل كتابة كلماته الأخيرة.
dictation-not-typed = كلمات مُملاة لم تُكتب، وضع التحرير متوقف: { $text }
setting-dictation-speak-while-recording = النطق أثناء الإملاء
setting-dictation-speak-while-recording-help = نطق الكلمات المُملاة عند وصولها. عند الإيقاف تظهر في سطر الحالة وتُنطق عند كل توقف، كي لا يسمع الميكروفون الصوت.
setting-dictation-model-dir = مجلد نموذج الإملاء
setting-dictation-model-dir-help = نموذج Whisper للإملاء. عند عدم تعيينه يُستخدم whisper/rten/base.en في مجلد البيانات.
section-dictation = الإملاء


## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }، { $k } من { $n }
browse-places-title = الأماكن
browse-places-intro =
    { $n ->
        [one] الأماكن، مكان واحد.
       *[other] الأماكن، { $n } أماكن.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }، مجلد المستند
browse-place-start = { $name }، مجلد البدء
browse-place-library = { $name }، مجلد المكتبة
browse-place-disk = { $name }، قرص
browse-place-removable = { $name }، محرك قابل للإزالة
browse-place-network = { $name }، محرك شبكة
browse-place-cd = { $name }، محرك أقراص مضغوطة أو DVD
browse-place-root = { $name }، المجلد الجذر
browse-choose-here = اختيار هذا المجلد، { $name }
browse-row-folder = { $name }، مجلد
browse-row-folder-items =
    { $name }، مجلد، { $n ->
        [one] عنصر واحد
       *[other] { $count } عناصر
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }، { $kind }، { $size }
browse-row-kind = { $name }، { $kind }
browse-row-hidden = { $row }، مخفي
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = أرشيف { $kind }
browse-kind-file = ملف
browse-kind-markdown = Markdown
browse-kind-text = نص
browse-kind-html = صفحة ويب
browse-kind-epub = كتاب EPUB
browse-kind-docx = مستند Word
browse-kind-rtf = مستند RTF
browse-kind-odt = نص OpenDocument
browse-kind-latex = LaTeX
browse-kind-eml = بريد إلكتروني
browse-kind-mhtml = أرشيف ويب
browse-kind-pdf = PDF
browse-kind-image = صورة
browse-kind-daisy = كتاب DAISY
browse-kind-pptx = شرائح PowerPoint
browse-kind-sheet = جدول بيانات
browse-kind-json = JSON
browse-kind-notebook = دفتر Jupyter
browse-kind-svg = رسم SVG
browse-kind-mathml = صيغة MathML
browse-kind-pandoc = مستند مقروء عبر Pandoc
browse-size-bytes =
    { $n ->
        [one] بايت واحد
       *[other] { $count } بايت
    }
# $size is a number, with a decimal under 10 ("3.4").
browse-size-kb = { $size } كيلوبايت
browse-size-mb = { $size } ميغابايت
browse-size-gb = { $size } غيغابايت
browse-intro =
    { $name }، { $n ->
        [one] عنصر واحد.
       *[other] { $count } عناصر.
    }
browse-intro-empty = لا شيء لعرضه في { $name }.
# $filter is what was typed.
browse-intro-filtered =
    { $name }، { $n ->
        [one] عنصر واحد يطابق
       *[other] { $count } عناصر تطابق
    } { $filter }.
browse-intro-in-archive = { $intro } في { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] ملف واحد مخفي.
       *[other] { $count } ملفات مخفية.
    }
browse-intro-cut = { $intro } تُعرض أول { $max } فقط.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } يفتح Enter، ويصعد Backspace، والكتابة تصفّي. { $preview } يعرض معاينة، و{ $choose } يختار مجلدًا، و{ $sort } يرتب، و{ $all } يعرض كل الملفات. { $item }
browse-sorted-name = مرتبة حسب الاسم.
browse-sorted-date = مرتبة حسب التاريخ، الأحدث أولًا.
browse-sorted-size = مرتبة حسب الحجم، الأكبر أولًا.
browse-showing-all = تُعرض كل الملفات.
browse-showing-readable = تُعرض الملفات المقروءة فقط.
browse-closed = أُغلق مستعرض الملفات.
browse-read-only = مستعرض الملفات يفتح الملفات ويختارها فقط؛ ولا يغيّرها أبدًا.
# $key is the Choose Folder key.
browse-choose-a-folder = اختر مجلدًا: يفتح Enter مجلدًا، ويختاره { $key }.
browse-choose-a-file = اختر ملفًا: يختار Enter ملفًا.
browse-no-archive-folder = لا يمكن اختيار مجلد داخل أرشيف؛ اختر مجلدًا على القرص.
browse-nothing-waiting = لا أمر ينتظر مجلدًا؛ يفتحه Enter.
browse-cannot-read = { $name } ليس نوع ملف يستطيع textweaver قراءته.
browse-folder-unreadable = تعذّر فتح { $name }: { $reason }
browse-archive-too-deep = { $name } داخل أرشيفات كثيرة جدًا لفتحه.
browse-archive-too-large = { $name } كبير جدًا لسرده بأمان.
browse-archive-unreadable = { $name } ليس أرشيفًا يستطيع textweaver قراءته؛ ربما يكون تالفًا.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. لا نص فيه.
browse-preview-failed = تعذّرت معاينة { $name }: { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name }: { $n ->
        [one] ملف واحد
       *[other] { $count } ملفات
    }، { $readable } مقروءة. { $names }
browse-preview-archive-folder =
    { $name }، مجلد في الأرشيف، { $n ->
        [one] عنصر واحد.
       *[other] { $count } عناصر.
    }
browse-preview-folder = { $path }: { $names }
browse-preview-folder-empty = { $path }: لا شيء للقراءة هنا.
browse-preview-other = { $name }، { $size }؛ لا يستطيع textweaver قراءة هذا النوع من الملفات.
browse-preview-path = { $path }


## Batch conversion (File, Batch convert). Keep the meaning first.
batch-choose-source = اختر المجلد المراد تحويله
batch-choose-output = اختر مجلد الملفات المحوّلة
batch-format-title = التحويل إلى
batch-format-intro = تحويل { $name } إلى: اختر صيغة، { $n } خيارات.
batch-where-title = مكان الملفات
batch-where-intro = أين توضع الملفات المحوّلة؟
batch-where-converted = في مجلد converted، { $path }
batch-where-beside = بجانب كل ملف
batch-where-choose = في مجلد آخر، يُختار بعد ذلك
batch-nothing = لا توجد مستندات للتحويل في { $path }.
batch-confirm =
    تحويل { $n ->
        [one] ملف واحد
       *[other] { $n } ملفات
    } إلى { $format } في { $path }؟ y أو n
batch-confirm-beside =
    تحويل { $n ->
        [one] ملف واحد
       *[other] { $n } ملفات
    } إلى { $format } بجانب كل ملف؟ y أو n
batch-started =
    جارٍ تحويل { $n ->
        [one] ملف واحد
       *[other] { $n } ملفات
    } إلى { $format }. يوقفه Escape.
batch-progress = تم تحويل { $percent } بالمئة، { $done } من { $total } ملفات.
batch-busy = التحويل جارٍ بالفعل، { $done } من { $total } ملفات. يوقفه Escape.
batch-stop-question = إيقاف التحويل؟ تُحفظ الملفات المنجزة. y أو n
batch-stopping = سيتوقف بعد الملفات الجاري كتابتها.
batch-still-converting = التحويل مستمر.
batch-done =
    تم تحويل { $converted ->
        [one] ملف واحد
       *[other] { $converted } ملفات
    } إلى { $format }؛ { $skipped } محدّثة؛ { $failed } فشلت.
batch-stopped =
    توقف. تم تحويل { $converted ->
        [one] ملف واحد
       *[other] { $converted } ملفات
    }؛ { $left } لم تُحوَّل؛ { $failed } فشلت.
batch-report = حُفظ التقرير في { $path }.
batch-report-failed = تعذّر حفظ التقرير: { $error } يُحتفظ بالملفات المحوّلة.
batch-inaccessible = ملفات فيها عناصر لم تُجعل متاحة: { $n }. انظر التقرير.
batch-failures-title =
    { $n ->
        [one] ملف واحد فشل
       *[other] { $n } ملفات فشلت
    }
batch-failure-item = { $name }: { $reason }
batch-start-failed = تعذّر بدء التحويل: { $error } تحقّق من المجلد والتنسيق، ثم حاول مرة أخرى.
batch-thread-stopped = توقف التحويل الجماعي على نحو غير متوقع.


## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = تصدير الصوت بصيغة
audio-format-intro = تصدير { $name } صوتًا: اختر صيغة، { $n } خيارات.
audio-no-ffmpeg = تحتاج { $formats } إلى ffmpeg، ولم يُعثر عليه.
audio-format-flac = FLAC: دون فقد، نحو نصف حجم WAV
audio-format-wav = WAV: الأكبر حجمًا، يعمل في كل مكان
audio-format-mp3 = MP3: صغير، يعمل في كل مكان
audio-format-opus = Opus: الأصغر حجمًا، مصمم للكلام
audio-format-ogg = Ogg Vorbis: صغير ومفتوح، يعمل في معظم المشغلات
audio-format-m4b = كتاب صوتي M4B، عبر ffmpeg
audio-format-mp4 = فيديو مع ترجمة مكتوبة: MP4، يحتاج إلى ffmpeg
audio-format-html = صفحة القراءة: النص والصوت في ملف واحد
audio-where-title = مكان حفظ الصوت
audio-where-intro = أين يُحفظ الصوت؟
audio-where-beside = بجانب المستند، { $path }
audio-where-choose = في مجلد آخر، يُختار بعد ذلك
audio-choose-folder = اختر مجلد الصوت
audio-no-engine = لا يوجد هنا محرك كلام يكتب ملفات صوتية. ثبّت eSpeak NG، أو اختر محركًا آخر من قائمة الكلام.
audio-confirm = تصدير { $name } بصوت { $voice } بسرعة { $wpm } كلمة في الدقيقة، في { $path }؟ y أو n
audio-started = جارٍ تصدير { $name } بصيغة { $format }. Escape يوقف.
audio-progress = جارٍ تصدير الصوت، { $percent } بالمئة.
audio-busy = جارٍ تصدير { $name } بالفعل. Escape يوقف.
audio-stop-question = إيقاف التصدير؟ لن يُحفظ أي ملف. y أو n
audio-stopping = جارٍ إيقاف التصدير.
audio-still-exporting = ما زال تصدير الصوت جاريًا.
audio-stopped = توقف تصدير الصوت؛ لم يُكتب أي ملف.
audio-done =
    كُتب { $name }: { $length }، { $chapters ->
        [one] فصل واحد
       *[other] { $chapters } فصول
    }.
audio-subtitles = الترجمة في { $name }.
audio-failed = تعذر تصدير الصوت: { $error } جرّب تنسيقًا أو صوتًا آخر.
audio-thread-stopped = توقف تصدير الصوت على نحو غير متوقع.


## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = ملفات الإعدادات
gui-settings-export-title = تصدير الإعدادات
gui-settings-import-title = استيراد الإعدادات
gui-chooser-no-dialog = لم يُفتح منتقي الملفات في النظام. اكتب مسار الملف بدلًا من ذلك.
gui-colors-value = { $value }، التباين { $ratio } إلى 1، { $verdict }
gui-colors-help = يختار اليسار واليمين لونًا مسمى، الأزرق والبرتقالي أولًا. ويكتب Enter اسمًا أو قيمة ‎#rrggbb. ويعيد Delete لون السمة. تحتفظ كل علامة بتسطيرها أو سماكتها أو رمزها، أيًّا كان لونها.
gui-colors-reset-all = إعادة كل الألوان
gui-colors-reset-all-help = إعادة لون السمة نفسه لكل جزء.
gui-colors-reset-done = عادت كل الألوان إلى ألوان السمة.
colors-reset-question = إعادة كل الألوان إلى ألوان السمة؟ y أو n
gui-colors-closed = أُغلقت الألوان.
gui-font-list-intro =
    { $n ->
        [one] { $title }، عائلة واحدة.
       *[other] { $title }، { $n } عائلات.
    }


## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = صفحة { $page }


## Sync wave, S4: sync in the reader (ADR-0049).
sync-status-off = المزامنة: متوقفة
sync-status-not-set-up = المزامنة: غير مُعدّة
sync-status-starting = المزامنة: تبدأ
sync-status-folder-missing = المزامنة: المجلد غير موجود، الحفظ هنا
sync-status-read-only = المزامنة: تنسيق أحدث، قراءة فقط
sync-status-failed = المزامنة: تعذّر استخدام المجلد
sync-status-cannot-write = المزامنة: تعذّرت الكتابة، الحفظ هنا
sync-status-clock-ahead = المزامنة: ساعة { $device } متقدمة
sync-status-damaged =
    { $n ->
        [one] المزامنة: تُخُطّي ملف تالف واحد
       *[other] المزامنة: تُخُطّيت ملفات تالفة: { $n }
    }
sync-status-up-to-date = المزامنة: محدّثة
sync-status-this-computer = هذا الحاسوب: { $name }.
sync-status-no-others = لا حواسيب أخرى بعد.
sync-status-others = الحواسيب الأخرى: { $names }.
sync-status-error = مشكلة: { $error } تحقّق من مجلد المزامنة.
sync-another-computer = حاسوب آخر
sync-untitled = مستند
sync-damaged = المزامنة: تُخُطّي ملف تالف من { $device }.
sync-newer-file = المزامنة: تُخُطّي ملف أحدث من { $device }.
sync-read-only = المزامنة: تنسيق أحدث، قراءة فقط.
sync-clock-ahead = المزامنة: ساعة { $device } متقدمة بمقدار { $hours } ساعة.
sync-fresh-id = المزامنة: إعداد منسوخ؛ مُعرّف حاسوب جديد.
sync-write-failed = المزامنة: تعذّرت الكتابة. { $error } يُحفظ على هذا الحاسوب مؤقتًا.
sync-name-refused = الاسم غير مسموح. جرّب اسمًا مثل laptop.
sync-note-replaced =
    { $n ->
        [one] { $title }: استُبدلت ملاحظة بتعديل أحدث من { $device }.
       *[other] { $title }: استُبدلت ملاحظات بتعديلات أحدث من { $device }: { $n }.
    }
sync-restored-notes =
    { $n ->
        [one] { $title }: عادت ملاحظة محذوفة، عُدّلت على { $device }.
       *[other] { $title }: عادت ملاحظات محذوفة، عُدّلت على { $device }: { $n }.
    }
sync-restored-bookmarks =
    { $n ->
        [one] { $title }: عادت علامة مرجعية محذوفة، عُدّلت على { $device }.
       *[other] { $title }: عادت علامات مرجعية محذوفة، عُدّلت على { $device }: { $n }.
    }
sync-restored-highlights =
    { $n ->
        [one] { $title }: عاد تظليل محذوف، عُدّل على { $device }.
       *[other] { $title }: عادت تظليلات محذوفة، عُدّلت على { $device }: { $n }.
    }
sync-arrived =
    { $n ->
        [one] { $title }: تغيير واحد من { $device }.
       *[other] { $title }: تغييرات من { $device }: { $n }.
    }
sync-resumed = { $title }: استُؤنفت القراءة عند { $pct } بالمئة، من { $device }.
sync-place-arrived = موضع { $device }: { $pct } بالمئة.
sync-place-question = { $device } عند { $pct } بالمئة. الانتقال إليه؟ y أو n
sync-suggestion-question =
    { $n ->
        [one] قد يكون هذا { $title } من { $device }، مع ملاحظة واحدة. استخدامها؟ y أو n
       *[other] قد يكون هذا { $title } من { $device }، مع ملاحظات: { $n }. استخدامها؟ y أو n
    }
sync-went-to-place = موضع { $device }، { $pct } بالمئة.
sync-kept-place = أُبقي هذا الموضع.
sync-suggestion-accepted = تُستخدم ملاحظات { $device }.
sync-suggestion-declined = أُبقيا منفصلين.
sync-sidecar-differed =
    { $n ->
        [one] المزامنة: اختلف موضع واحد في المكتبة.
       *[other] المزامنة: اختلفت مواضع في المكتبة: { $n }.
    }
sync-sidecar-failed = المزامنة: تعذّرت كتابة موضع في المكتبة. { $error } تحقّق من إمكانية الكتابة في مجلد المكتبة.
sync-no-state = المزامنة متوقفة في هذا التشغيل: لا يُحفظ شيء.
sync-choose-folder = اختر مجلد المزامنة
sync-group-places = المواضع
sync-group-notes = الملاحظات
sync-group-highlights = التظليلات
sync-group-bookmarks = العلامات المرجعية
sync-group-statistics = الإحصاءات
sync-group-item = { $name }: { $state }
sync-start = بدء المزامنة
sync-groups-title = ما يُزامَن
sync-groups-intro = { $title }، باسم { $name }. Enter يشغّل أو يوقف؛ بدء المزامنة ينهي.
sync-started = المزامنة تعمل، باسم { $name }. حالة المزامنة: { $key }.
sync-how-to-set-up = للإعداد: الأدوات، المزامنة، إعداد المزامنة.
sync-now-started = جارٍ المزامنة.
sync-now-done =
    { $n ->
        [one] المزامنة: محدّثة، فُحص مستند واحد.
       *[other] المزامنة: محدّثة، مستندات مفحوصة: { $n }.
    }
sync-now-changed =
    { $n ->
        [one] المزامنة: تلقّى مستند واحد تغييرات.
       *[other] المزامنة: مستندات تلقّت تغييرات: { $n }.
    }
sync-no-places = لا يملك حاسوب آخر موضعًا هنا.
sync-place-item = { $device }، { $pct } بالمئة
sync-places-title =
    { $n ->
        [one] موضع آخر واحد
       *[other] مواضع أخرى: { $n }
    }
sync-no-replaced = لا ملاحظات مستبدلة في هذا المستند.
sync-replaced-item = { $text }، استبدلها { $device }
sync-replaced-item-deleted = { $text }، حذفها { $device }
sync-replaced-title =
    { $n ->
        [one] ملاحظة مستبدلة واحدة
       *[other] ملاحظات مستبدلة: { $n }
    }
sync-replaced-intro = { $title }. Enter يعيد واحدة.
sync-note-restored = أُعيدت الملاحظة: { $text }
sync-already-off = المزامنة متوقفة هنا بالفعل.
sync-stopped = أُوقفت المزامنة هنا. يبقى المجلد كما هو.
prompt-sync-computer-name = اسم هذا الحاسوب، Enter يبقيه
menu-sync = المزامنة
name-sync-setup = إعداد المزامنة
name-sync-status = حالة المزامنة
name-sync-now = المزامنة الآن
name-sync-go-to-place = الانتقال إلى موضع حاسوب آخر
name-sync-replaced-notes = الملاحظات المستبدلة
name-sync-stop = إيقاف المزامنة على هذا الحاسوب
action-sync-setup = إعداد المزامنة: اختيار مجلد المزامنة، وتسمية هذا الحاسوب، واختيار ما يُزامَن
action-sync-status = قول حالة المزامنة (محدّثة، أو المجلد غير موجود، أو مشكلة) وتسمية الحواسيب الأخرى
action-sync-now = المزامنة الآن: إرسال تغييرات هذا الحاسوب وأخذ تغييرات الحواسيب الأخرى لكل مستند
action-sync-go-to-place = سرد مواضع الحواسيب الأخرى في هذا المستند؛ Enter ينتقل إلى أحدها
action-sync-replaced-notes = سرد الملاحظات التي استبدلها تعديل أحدث من حاسوب آخر؛ Enter يعيد واحدة
action-sync-stop = إيقاف المزامنة على هذا الحاسوب؛ يبقى مجلد المزامنة كما هو
section-sync = المزامنة
setting-sync-enabled = المزامنة
setting-sync-enabled-help = مشاركة الملاحظات والتظليلات والعلامات المرجعية والمواضع مع حواسيبك الأخرى عبر مجلد المزامنة. الأدوات، المزامنة، إعداد المزامنة يشغّلها.
setting-sync-folder = مجلد المزامنة
setting-sync-folder-help = المجلد الذي تتشاركه حواسيبك. مجلد يحدّثه Syncthing، أو مجلد سحابي، أو ذاكرة USB.
setting-sync-device-name = اسم الحاسوب
setting-sync-device-name-help = اسم هذا الحاسوب في رسائل المزامنة، مثل laptop أو lab. الفارغ يستخدم Computer 1 وComputer 2 وهكذا.
setting-sync-places = مزامنة المواضع
setting-sync-places-help = مشاركة موضعك في كل مستند.
setting-sync-notes = مزامنة الملاحظات
setting-sync-notes-help = مشاركة الملاحظات.
setting-sync-highlights = مزامنة التظليلات
setting-sync-highlights-help = مشاركة التظليلات.
setting-sync-bookmarks = مزامنة العلامات المرجعية
setting-sync-bookmarks-help = مشاركة العلامات المرجعية.
setting-sync-statistics = مزامنة الإحصاءات
setting-sync-statistics-help = مشاركة وقت القراءة والجلسات لكل حاسوب.
setting-sync-position-policy = موضع الاستئناف
setting-sync-position-policy-help = الموضع الذي يُفتح عنده المستند حين يكون لحاسوب آخر موضع أيضًا. الأحدث، أو الأبعد، أو السؤال.
choice-sync-position-policy-newest = الأحدث
choice-sync-position-policy-furthest = الأبعد
choice-sync-position-policy-ask = السؤال

## End of S4

## Sync wave, S5 (see en.ftl).
sync-settings-arrived =
    { $n ->
        [one] الإعدادات: تغيير واحد من { $device }.
       *[other] الإعدادات: تغييرات من { $device }: { $n }.
    }
sync-settings-arrived-several = الإعدادات: تغييرات من { $computers } حواسيب: { $n }.
sync-kept-keys-mac =
    { $n ->
        [one] مفاتيح ماك: 1، محفوظ، غير مستخدم هنا.
       *[other] مفاتيح ماك: { $n }، محفوظة، غير مستخدمة هنا.
    }
sync-kept-keys-pc =
    { $n ->
        [one] مفاتيح ويندوز ولينكس: 1، محفوظ، غير مستخدم هنا.
       *[other] مفاتيح ويندوز ولينكس: { $n }، محفوظة، غير مستخدمة هنا.
    }
sync-group-settings = الإعدادات
sync-group-profiles = الملفات الشخصية
sync-group-key-overrides = المفاتيح المخصصة
sync-group-words = قائمة الكلمات
sync-group-glossary = المسرد والنطق
sync-group-favorite-voices = الأصوات المفضلة
voices-missing-row = { $voice }، مفضل، غير موجود على هذا الحاسوب
voices-missing = { $voice } غير موجود على هذا الحاسوب. المسافة تزيله من المفضلة.
gui-voices-list = الأصوات
gui-voices-use = استخدام الصوت
gui-voices-use-help = استخدام الصوت المحدد وسماع عينة منه، أو تنزيله بعد سؤال.
gui-voices-preview = معاينة
gui-voices-preview-help = سماع عينة بالصوت المحدد دون اختياره.
gui-voices-favorite = مفضّل
gui-voices-favorite-help = جعل الصوت المحدد مفضّلًا، أو إلغاء ذلك. المفضّلة تأتي أولًا.
gui-voices-remove = إزالة
gui-voices-remove-help = إزالة صوت Piper المُنزَّل المحدد بعد سؤال.
gui-voices-remove-unavailable = غير متاح
gui-voices-language-help = عرض أصوات اللغة التالية فقط، ثم كل اللغات مجددًا.
gui-voices-engine-help = عرض أصوات المحرك التالي فقط، ثم كل المحركات مجددًا.
gui-voices-fetch-help = تنزيل قائمة أصوات Piper، نحو 250 كيلوبايت، بعد سؤال.
gui-voices-close-help = إغلاق مدير الأصوات.
gui-voices-hint = Enter يستخدم الصوت، وSpace يضع علامة مفضّل، وEscape يغلق.
setting-sync-settings = مزامنة الإعدادات
setting-sync-settings-help = مشاركة الإعدادات القابلة للنقل: السرعة وعلامات الترقيم والسمة ووسائل القراءة وما شابهها. يبقى الصوت والمحرك ووضع الوصول ومجموعة المفاتيح والمسارات على كل حاسوب.
setting-sync-profiles = مزامنة الملفات الشخصية
setting-sync-profiles-help = مشاركة الملفات الشخصية؛ الملف المستخدم يبقى على كل حاسوب.
setting-sync-key-overrides = مزامنة المفاتيح المخصصة
setting-sync-key-overrides-help = مشاركة keymap.toml. تُحفظ مفاتيح ماك ولا تُستخدم على ويندوز أو لينكس، والعكس.
setting-sync-words = مزامنة قائمة الكلمات
setting-sync-words-help = مشاركة قائمة كلمات التدقيق الإملائي.
setting-sync-glossary = مزامنة المسرد
setting-sync-glossary-help = مشاركة مدخلات المسرد والنطق.
setting-sync-favorite-voices = مزامنة الأصوات المفضلة
setting-sync-favorite-voices-help = مشاركة الأصوات المفضلة. الصوت غير الموجود على هذا الحاسوب يُذكر أنه غير موجود على هذا الحاسوب.

## End of S5

## Lexend downloaded on first choice.
font-download-question = تنزيل الخط { $font }، { $kb } كيلوبايت، { $licence }؟ y أو n
font-downloading = تنزيل { $font }.
font-downloaded = تم تنزيل { $font } وهو جاهز.
font-download-failed = تعذّر تنزيل { $font }: { $error } يُستخدم خط آخر.
font-download-declined = لم يُنزَّل. يُستخدم خط آخر.
font-download-busy = ما زال { $font } قيد التنزيل.
font-download-no-folder = لا يوجد مجلد بيانات لحفظ { $font }.
font-download-not-in-build = تنزيل الخطوط غير متاح في هذا الإصدار.
gui-font-to-download = { $family } (للتنزيل، { $kb } كيلوبايت)
gui-font-downloaded = { $family } (منزّل)

## منتقيات الملفات والمجلدات.
prompt-browse-hint = { $label }. { $key } للتصفح.
prompt-browse-file = اختر الملف: { $label }
prompt-browse-folder = اختر المجلد: { $label }
prompt-browse-filled = { $name } مختار. يؤكده Enter.
chooser-type-files = ملفات { $type }
chooser-image-title = إدراج صورة
chooser-images = الصور
chooser-references-title = استيراد المراجع
chooser-reference-files = ملفات المراجع
chooser-profiles-import-title = استيراد ملفات الإعدادات
chooser-profiles-export-title = تصدير ملفات الإعدادات
chooser-profile-files = ملفات تصدير الإعدادات
gui-folder-no-dialog = لم يُفتح منتقي المجلدات في النظام. اختر المجلد من هذه القائمة.
gui-prompt-browse-hint = يفتح { $key } متصفح الملفات.

## المكونات الاختيارية.
name-manage-components = إدارة المكونات الاختيارية…
name-download-dictation-model = تنزيل نموذج الإملاء
action-manage-components = إدارة المكونات الاختيارية: النماذج والخطوط والأصوات التي يمكن لـ textweaver تنزيلها، مع حجمها وترخيصها
action-download-dictation-model = تنزيل نموذج الإملاء المختار في الإعدادات، بعد ذكر حجمه وترخيصه
component-feature-dictation = الإملاء
component-feature-ocr = قراءة الصفحات الممسوحة
component-feature-reading-font = خط للقراءة
component-feature-voice = صوت للنطق
component-state-installed = مثبّت
component-state-not-installed = غير مثبّت
component-state-partial = مثبّت جزئيًا
component-state-damaged = تالف
component-state-downloading = قيد التنزيل
components-title = المكونات الاختيارية
components-intro =
    { $n ->
        [one] مكوّن اختياري واحد. Enter للإجراءات.
       *[other] { $n } مكونات اختيارية. Enter للإجراءات.
    }
components-item = { $title }: { $state }، { $size }، الترخيص { $license }، من أجل { $features }
components-actions-intro = { $title }: { $state }.
components-action-download = تنزيل، { $size }
components-action-verify = التحقق من الملفات
components-action-remove = إزالة
components-action-install-zip = التثبيت من ملف zip…
components-action-install-folder = التثبيت من مجلد…
components-install-purpose = تثبيت المكوّن من هنا
component-question = تنزيل { $title }، { $size }، الترخيص { $license }؟ y أو n
component-remove-question = إزالة { $title }؟ y أو n
component-downloading = جارٍ التنزيل. Escape يوقفه.
component-installing = جارٍ التثبيت من الملف.
component-verifying = جارٍ التحقق من الملفات.
component-progress = تم تنزيل { $percent } بالمئة.
component-ready = جاهز: { $title }.
component-verified = الملفات سليمة: { $title }.
component-verify-failed =
    { $n ->
        [one] ملف واحد غير مطابق: { $files }.
       *[other] { $n } ملفات غير مطابقة: { $files }.
    }
component-removed = أُزيل: { $title }.
component-refused =
    { $n ->
        [one] ملف واحد مستبعد: { $files }.
       *[other] { $n } ملفات مستبعدة: { $files }.
    }
component-already = مثبّت بالفعل: { $title }.
component-not-there = غير مثبّت: { $title }.
component-declined = لم يُنزَّل شيء.
component-not-in-build = لا تنزيلات في هذا الإصدار.
component-no-folder = لا مجلد بيانات لحفظه.
component-error-fetch = لم يُنزَّل: فشل المصدر.
component-error-size = لم يُثبَّت: حجم ملف خاطئ.
component-error-hash = لم يُثبَّت: ملف لم يتطابق. نزّله مرة أخرى أو تحقق من المصدر.
component-error-missing = لم يُثبَّت: ملف مفقود.
component-error-cancelled = توقف التنزيل؛ يُستأنف لاحقًا.
component-error-busy = يجري تنزيل مكوّن بالفعل.
component-error-no-source = لم يُنزَّل: لا عنوان له.
component-error-name = مرفوض: اسم غير بسيط.
component-error-manifest = قائمة مكونات غير مقروءة.
component-error-io = لم يُثبَّت: تعذرت الكتابة.
components-chooser-title = المكونات الاختيارية
components-chooser-intro = إضافات اختيارية، لم يُختر شيء. المسافة تختار؛ تنزيل المختار يجلبها؛ Escape يتخطى.
components-chooser-item = { $mark }: { $title }، من أجل { $features }، { $size }، الترخيص { $license }
components-chosen = مختار
components-not-chosen = غير مختار
components-chooser-download = تنزيل المختار
components-chooser-skip = تخطٍّ الآن
components-chooser-skipped = تم التخطي؛ انظر إدارة المكونات.
components-chooser-none = لم يُختر شيء، ولم يُنزَّل شيء.
dictation-model-question = يحتاج الإملاء إلى نموذج Whisper، { $size }، الترخيص { $license }. تنزيله الآن؟ y أو n
dictation-model-declined = لا نموذج، فلا إملاء الآن.
dictation-model-not-in-build = لا نموذج ولا تنزيلات في هذا الإصدار.
dictation-model-file-missing = ينقص النموذج { $file }.
dictation-model-damaged = نموذج الإملاء تالف: { $file }.
dictation-model-no-folder = لا مجلد للنموذج: { $dir }.

## إعدادات المكونات الاختيارية.
section-components = المكونات الاختيارية
setting-dictation-model = نموذج الإملاء
setting-dictation-model-help = نموذج Whisper الذي يستخدمه الإملاء حين لا يُحدَّد مجلد. «تنزيل نموذج الإملاء» في قائمة الأدوات يجلبه.
choice-dictation-model-whisper-base-en = base.en، الافتراضي
choice-dictation-model-whisper-small-en = small.en، أكبر وأدق
setting-components-source = مصدر المكونات
setting-components-source-help = مكوناتك الخاصة، تُستخدم أولًا. مستودع GitHub بصيغة المالك/الاسم، أو مجلد على هذا الحاسوب. الفارغ لا يستخدم شيئًا. لا تضع كلمة مرور هنا أبدًا.
setting-components-mirror = مرآة المكونات
setting-components-mirror-help = من أين تأتي المكونات الاختيارية أولًا: عنوان https أو مجلد على هذا الحاسوب. الفارغ يستخدم مصادرها العامة. لا تضع كلمة مرور هنا أبدًا.

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
name-quick-start = البدء السريع
name-documentation = التوثيق…
name-report-problem = الإبلاغ عن مشكلة…
name-ask-first-run-again = السؤال مجددا عن خيارات التشغيل الأول
action-quick-start = فتح دليل البدء السريع في textweaver
action-documentation = عرض عنوان التوثيق على الويب، والسؤال قبل فتحه في المتصفح
action-report-problem = عرض مكان الإبلاغ عن مشكلة، والسؤال قبل فتحه في المتصفح؛ لا يُرسل شيء
action-ask-first-run-again = السؤال مجددا عن خيارات التشغيل الأول: الوضع المختلط مع قارئ الشاشة، والمكونات الاختيارية
about-title = حول textweaver
about-intro = حول textweaver، عدد المعلومات { $n }. أرفقها ببلاغ المشكلة.
about-version = الإصدار: textweaver { $version }
about-build = البناء: { $frontend }، { $profile }، { $os } { $arch }
about-frontend-window = النافذة
about-frontend-terminal = قارئ الطرفية
about-profile-release = إصدار نهائي
about-profile-debug = تصحيح
about-license = الترخيص: { $license }
about-engine-in-use = محرك الكلام المستخدم: { $engine }
about-engines = محركات الكلام الموجودة: { $engines }
about-engines-none = محركات الكلام الموجودة: لا شيء
about-components = المكونات: { $installed } من { $known } مثبتة
about-folder-settings = مجلد الإعدادات: { $path }
about-folder-data = مجلد البيانات: { $path }
about-folder-cache = مجلد ذاكرة التخزين المؤقت: { $path }
about-no-folders = المجلدات: لا شيء؛ هذه الجلسة لا تحفظ ملفات.
about-quick-start-online = لم يوجد البدء السريع بجانب textweaver. فتح { $address } في المتصفح؟ y أو n
about-docs-question = التوثيق: { $address }. فتحه في المتصفح؟ y أو n
about-report-question = الإبلاغ عن مشكلة: { $address }. لا يُرسل شيء. فتحه في المتصفح؟ y أو n
about-first-run-again = أُعيد ضبط خيارات التشغيل الأول؛ تُسأل عند التشغيل التالي.

## W9b-x: إعدادات القراءة (عرض، إعدادات القراءة).
name-reading-form = إعدادات القراءة
action-reading-form = فتح إعدادات القراءة: السرعة والخط والتباعد وطول السطر والسمة والتمييز والمسطرة والقراءة البيونية والمقاطع
reading-form-intro = إعدادات القراءة، { $n } إعدادات. يغيّر اليسار واليمين قيمة، ويكتب Enter قيمة، ويعيد Delete القيمة الافتراضية، ويقول F1 المساعدة.
reading-form-spacing-wcag-done = ضُبط التباعد على قيم WCAG.
reading-form-spacing-generous-done = ضُبط التباعد على الواسع، أوسع من WCAG.
gui-reading-form-help = يتنقل الأعلى والأسفل، ويغيّر اليسار واليمين قيمة، ويكتب Enter قيمة، ويقول F1 المساعدة.
gui-reading-voices = الأصوات
gui-reading-voices-help = فتح مدير الأصوات.
gui-reading-wcag = تباعد WCAG
gui-reading-wcag-help = ضبط التباعدات الأربعة على قيم WCAG.
gui-reading-generous = تباعد واسع
gui-reading-generous-help = ضبط التباعدات الأربعة أوسع من WCAG.
gui-reading-closed = أُغلقت إعدادات القراءة.
## End of W9b-x

## B1-t1: التغييرات المتعقبة والتعليقات (قائمة التغييرات).
name-list-changes = التغييرات والتعليقات
action-list-changes = عرض التغييرات المتعقبة والتعليقات: Enter ينتقل إلى أحدها، A يقبل التغيير، R يرفضه
name-accept-all-changes = قبول كل التغييرات
action-accept-all-changes = قبول كل تغيير متعقب في المستند
name-reject-all-changes = رفض كل التغييرات
action-reject-all-changes = رفض كل تغيير متعقب في المستند
name-add-comment = إضافة تعليق
action-add-comment = إضافة تعليق على التحديد أو على الجملة عند المؤشر
prompt-comment-reply = الرد
prompt-comment-text = التعليق
changes-title = التغييرات والتعليقات
changes-intro =
    { $n ->
        [one] { $title }، عنصر واحد. Enter ينتقل إليه. A يقبل التغيير، R يرفضه، ومع Shift يشمل كل تغييرات المؤلف نفسه. على التعليق، F2 للرد، Space للحل، Delete للحذف. N يضيف تعليقا.
       *[other] { $title }، { $n } عناصر. Enter ينتقل إلى أحدها. A يقبل التغيير، R يرفضه، ومع Shift يشمل كل تغييرات المؤلف نفسه. على التعليق، F2 للرد، Space للحل، Delete للحذف. N يضيف تعليقا.
    }
changes-none = لا توجد تغييرات متعقبة ولا تعليقات في هذا المستند.
changes-row = { $kind }: «{ $text }»، { $who }، { $when }
changes-kind-inserted = إدراج
changes-kind-deleted = حذف
changes-kind-moved-away = نقل من هنا
changes-kind-moved-here = نقل إلى هنا
changes-by = بواسطة { $author }
changes-no-author = المؤلف غير مسجل
changes-no-date = التاريخ غير مسجل
changes-date = { $weekday }، { $day } { $month } { $year }
changes-weekday-0 = الأحد
changes-weekday-1 = الاثنين
changes-weekday-2 = الثلاثاء
changes-weekday-3 = الأربعاء
changes-weekday-4 = الخميس
changes-weekday-5 = الجمعة
changes-weekday-6 = السبت
changes-month-1 = يناير
changes-month-2 = فبراير
changes-month-3 = مارس
changes-month-4 = أبريل
changes-month-5 = مايو
changes-month-6 = يونيو
changes-month-7 = يوليو
changes-month-8 = أغسطس
changes-month-9 = سبتمبر
changes-month-10 = أكتوبر
changes-month-11 = نوفمبر
changes-month-12 = ديسمبر
changes-comment = تعليق: { $text }
changes-comment-by = تعليق من { $author }: { $text }
changes-replies =
    { $n ->
        [one] رد واحد
       *[other] { $n } ردود
    }
changes-resolved = محلول
changes-accepted = تم القبول. { $kind }: «{ $text }».
changes-rejected = تم الرفض. { $kind }: «{ $text }».
changes-accepted-all =
    { $n ->
        [one] تم قبول تغيير واحد.
       *[other] تم قبول { $n } تغييرات.
    }
changes-rejected-all =
    { $n ->
        [one] تم رفض تغيير واحد.
       *[other] تم رفض { $n } تغييرات.
    }
changes-accepted-author =
    { $n ->
        [one] تم قبول تغيير واحد من { $author }.
       *[other] تم قبول { $n } تغييرات من { $author }.
    }
changes-rejected-author =
    { $n ->
        [one] تم رفض تغيير واحد من { $author }.
       *[other] تم رفض { $n } تغييرات من { $author }.
    }
changes-none-left = لا توجد تغييرات متعقبة للقبول أو الرفض.
changes-not-a-change = هذا الصف تعليق. F2 للرد، Space للحل، Delete للحذف.
changes-not-a-comment = هذا الصف تغيير. A يقبله، R يرفضه.
changes-edit-mode = تبقى التغييرات كما هي في وضع التحرير. اخرج من وضع التحرير لقبولها أو رفضها.
changes-replied = تمت إضافة الرد.
changes-resolved-done = تم حل التعليق.
changes-reopened = أعيد فتح التعليق.
changes-comment-deleted = تم حذف التعليق وردوده.
changes-comment-added = تمت إضافة التعليق.
changes-delete-comment-question = حذف هذا التعليق وردوده؟ y أو n
changes-written-accepted =
    { $n ->
        [one] تمت كتابة { $path } مع قبول تغيير واحد.
       *[other] تمت كتابة { $path } مع قبول { $n } تغييرات.
    }
changes-written-rejected =
    { $n ->
        [one] تمت كتابة { $path } مع رفض تغيير واحد.
       *[other] تمت كتابة { $path } مع رفض { $n } تغييرات.
    }
## End of B1-t1
## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.
setting-braille-brf-code = رمز برايل لملفات BRF
setting-braille-brf-code-help = رمز برايل الذي تُقرأ به ملفات BRF. UEB للكتب المصنوعة منذ 2016، وEBAE، أي English Braille American Edition، للكتب الأقدم. قراءة ملف BRF كنص مطبوع تحتاج إلى liblouis. افتح الملف مرة أخرى بعد أي تغيير.
choice-braille-brf-code-ueb = UEB
choice-braille-brf-code-ebae = EBAE
name-show-original-braille = عرض برايل الأصلي
action-show-original-braille = عرض برايل الأصلي للصفحة عند المؤشر، في ملف BRF مقروء كنص مطبوع
brf-original-title = برايل الأصلي، الصفحة { $page }
brf-original-intro =
    { $n ->
        [one] برايل الأصلي، الصفحة { $page }، سطر واحد. Escape يغلق.
        [two] برايل الأصلي، الصفحة { $page }، سطران. Escape يغلق.
        [few] برايل الأصلي، الصفحة { $page }، { $n } أسطر. Escape يغلق.
        [many] برايل الأصلي، الصفحة { $page }، { $n } سطرًا. Escape يغلق.
       *[other] برايل الأصلي، الصفحة { $page }، { $n } سطر. Escape يغلق.
    }
brf-original-not-brf = ليس ملف برايل. عرض برايل الأصلي يعمل مع ملفات BRF.
brf-original-unreadable = تعذرت قراءة ملف برايل: { $reason }
brf-no-liblouis = برايل معروض كبرايل: liblouis غير موجود. لقراءته كنص مطبوع، ثبّت liblouis من liblouis.io أو من حزم نظامك، ثم افتح الملف مرة أخرى.
## End of B1-r5
