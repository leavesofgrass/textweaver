### The settings screen, profiles, units, sections, and settings files.

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

## الوحدات، تُقال بعد رقم.

settings-unit-words-per-minute = كلمة في الدقيقة
settings-unit-hertz =
    { $n ->
        [one] هرتز
       *[other] هرتز
    }
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
settings-unit-groups =
    { $n ->
        [one] مجموعة
        [two] مجموعتان
        [few] مجموعات
        [zero] مجموعات
       *[other] مجموعة
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

section-highlight = تمييز
section-reading = القراءة
section-display = العرض
section-editing = التحرير
section-library = المكتبة وأدوات الدراسة
section-keyboard = لوحة المفاتيح واللغة
section-accessibility = إمكانية الوصول والإعلانات
section-export = التصدير
section-braille = برايل

## Settings by task (beta 4): sections, and when a setting applies.

section-voice = الصوت والكلام
section-pauses = الوقفات ومستوى التفصيل
section-speaking-text = نطق النص
section-documents = المستندات
section-math = الرياضيات
section-morse = شفرة مورس
section-morse-practice = التدرب على مورس
section-spacing = التباعد والخط والمسطرة
section-word-aids = أدوات الكلمات
section-rsvp = كلمة كلمة (RSVP)
section-window = الأشرطة واللوحات
# $n is the number of sections.
settings-intro-sections =
    { $n ->
        [0] الإعدادات، لا أقسام. Enter يفتح قسمًا. اكتب للبحث في كل الإعدادات. Escape للإغلاق.
        [one] الإعدادات، قسم واحد. Enter يفتح قسمًا. اكتب للبحث في كل الإعدادات. Escape للإغلاق.
        [two] الإعدادات، قسمان. Enter يفتح قسمًا. اكتب للبحث في كل الإعدادات. Escape للإغلاق.
        [few] الإعدادات، { $n } أقسام. Enter يفتح قسمًا. اكتب للبحث في كل الإعدادات. Escape للإغلاق.
        [many] الإعدادات، { $n } قسمًا. Enter يفتح قسمًا. اكتب للبحث في كل الإعدادات. Escape للإغلاق.
       *[other] الإعدادات، { $n } قسم. Enter يفتح قسمًا. اكتب للبحث في كل الإعدادات. Escape للإغلاق.
    }
# $section is a section-* title, $n its number of settings.
settings-section-row =
    { $n ->
        [0] { $section }، لا إعدادات
        [one] { $section }، إعداد واحد
        [two] { $section }، إعدادان
        [few] { $section }، { $n } إعدادات
        [many] { $section }، { $n } إعدادًا
       *[other] { $section }، { $n } إعداد
    }
settings-all = كل الإعدادات
settings-all-row =
    { $n ->
        [0] كل الإعدادات، لا إعدادات
        [one] كل الإعدادات، إعداد واحد
        [two] كل الإعدادات، إعدادان
        [few] كل الإعدادات، { $n } إعدادات
        [many] كل الإعدادات، { $n } إعدادًا
       *[other] كل الإعدادات، { $n } إعداد
    }
settings-title-section = الإعدادات: { $section }
settings-section-opened =
    { $n ->
        [0] { $section }، لا إعدادات. يسار ويمين لتغيير قيمة، Enter لتغيير قيمة أو كتابتها، Delete لإعادة القيمة الافتراضية، Escape للعودة إلى الأقسام.
        [one] { $section }، إعداد واحد. يسار ويمين لتغيير قيمة، Enter لتغيير قيمة أو كتابتها، Delete لإعادة القيمة الافتراضية، Escape للعودة إلى الأقسام.
        [two] { $section }، إعدادان. يسار ويمين لتغيير قيمة، Enter لتغيير قيمة أو كتابتها، Delete لإعادة القيمة الافتراضية، Escape للعودة إلى الأقسام.
        [few] { $section }، { $n } إعدادات. يسار ويمين لتغيير قيمة، Enter لتغيير قيمة أو كتابتها، Delete لإعادة القيمة الافتراضية، Escape للعودة إلى الأقسام.
        [many] { $section }، { $n } إعدادًا. يسار ويمين لتغيير قيمة، Enter لتغيير قيمة أو كتابتها، Delete لإعادة القيمة الافتراضية، Escape للعودة إلى الأقسام.
       *[other] { $section }، { $n } إعداد. يسار ويمين لتغيير قيمة، Enter لتغيير قيمة أو كتابتها، Delete لإعادة القيمة الافتراضية، Escape للعودة إلى الأقسام.
    }
settings-all-opened =
    { $n ->
        [0] كل الإعدادات، لا إعدادات، ومنها ما لا ينطبق الآن. Escape للعودة إلى الأقسام.
        [one] كل الإعدادات، إعداد واحد، ومنها ما لا ينطبق الآن. Escape للعودة إلى الأقسام.
        [two] كل الإعدادات، إعدادان، ومنها ما لا ينطبق الآن. Escape للعودة إلى الأقسام.
        [few] كل الإعدادات، { $n } إعدادات، ومنها ما لا ينطبق الآن. Escape للعودة إلى الأقسام.
        [many] كل الإعدادات، { $n } إعدادًا، ومنها ما لا ينطبق الآن. Escape للعودة إلى الأقسام.
       *[other] كل الإعدادات، { $n } إعداد، ومنها ما لا ينطبق الآن. Escape للعودة إلى الأقسام.
    }
settings-back = أقسام الإعدادات.
settings-section-now =
    { $n ->
        [0] يعرض { $section } الآن لا إعدادات.
        [one] يعرض { $section } الآن إعداد واحد.
        [two] يعرض { $section } الآن إعدادان.
        [few] يعرض { $section } الآن { $n } إعدادات.
        [many] يعرض { $section } الآن { $n } إعدادًا.
       *[other] يعرض { $section } الآن { $n } إعداد.
    }
settings-section-help =
    { $n ->
        [0] { $section }، لا إعدادات. Enter يفتحه.
        [one] { $section }، إعداد واحد. Enter يفتحه.
        [two] { $section }، إعدادان. Enter يفتحه.
        [few] { $section }، { $n } إعدادات. Enter يفتحه.
        [many] { $section }، { $n } إعدادًا. Enter يفتحه.
       *[other] { $section }، { $n } إعداد. Enter يفتحه.
    }
# $item is a settings-item row, $why a settings-applies-* sentence.
settings-item-not-now = { $item }. { $why }
# $engine, $first and $second are speech engine names, such as Eloquence.
settings-applies-engine = ينطبق حين يكون { $engine } محرك الكلام.
settings-applies-engines = ينطبق حين يكون { $first } أو { $second } محرك الكلام.
settings-applies-not-engine = ينطبق حين لا يكون { $engine } محرك الكلام.
# $os is Windows, macOS or Linux.
settings-applies-os = ينطبق على { $os } فقط.
