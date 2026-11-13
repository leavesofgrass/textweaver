### The textweaver app: drawn labels, hints, buttons, and dialogs.

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

## The window (GUI): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = نعم
gui-no = لا
gui-answer-delete = حذف
gui-answer-remove = إزالة
gui-answer-replace = استبدال
gui-question-hint = Y للإجابة بنعم، وN للإجابة بلا، وEscape للإجابة بلا.
gui-find-title = بحث واستبدال
gui-find-what = البحث عن
gui-find-with = الاستبدال بـ
gui-find-match-case = مطابقة حالة الأحرف
gui-find-whole-words = كلمات كاملة
gui-find-regex = تعبير نمطي
gui-find-across-lines = عبر الأسطر
gui-find-next = البحث عن التالي
gui-find-next-help = يحدد التطابق التالي.
gui-find-replace = استبدال
gui-find-replace-help = يستبدل التطابق المعروض وينتقل إلى التالي. الضغطة الأولى تبحث عن تطابق.
gui-find-replace-all = استبدال الكل
gui-find-replace-all-help = يذكر عدد التطابقات ويسأل مرة واحدة. تراجع واحد يعيدها كلها.
gui-find-hint = Enter في حقل البحث يجد التطابق التالي، وEnter في حقل الاستبدال يستبدله. السهم لأعلى يستعيد النصوص السابقة. Escape يغلق.
gui-find-empty = لا شيء للبحث عنه: اكتب النص في حقل البحث عن.
gui-button-open = فتح…
gui-button-font = الخط…
gui-button-edit = تحرير
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
gui-button-start-dictation = بدء الإملاء
gui-button-pause-dictation = إيقاف الإملاء مؤقتًا
gui-button-resume-dictation = استئناف الإملاء
gui-button-stop-dictation = إيقاف الإملاء
gui-button-live-pause = إيقاف التحديثات مؤقتًا
gui-button-live-resume = استئناف التحديثات
gui-button-live-newest = الانتقال إلى الأحدث
gui-button-live-table = عرض الجدول
gui-button-live-list = عرض القائمة
gui-button-live-save = حفظ السجل
gui-button-live-close = إغلاق المصدر
gui-button-live-show = إظهار التغييرات
gui-button-live-show-on = إظهار التغييرات: مشغّل
gui-button-live-show-off = إظهار التغييرات: متوقف
gui-button-live-speak = نطق التغييرات
gui-button-live-speak-on = نطق التغييرات: مشغّل
gui-button-live-speak-off = نطق التغييرات: متوقف
gui-button-unavailable = غير متاح
gui-hint-open = اختيار مستند للقراءة.
gui-hint-font = اختيار خط نص المستند.
gui-hint-edit = التبديل بين القراءة والتحرير.
gui-hint-settings = كل خيار، مع مساعدته.
gui-hint-commands = تشغيل أي أمر باسمه.
gui-hint-play = القراءة من الكلمة الحالية أو الإيقاف.
gui-hint-live-pause = تنتظر الإدخالات الجديدة أثناء الإيقاف.
gui-hint-live-view = يبقى موضعك في السجل.
gui-hint-live-save = حفظ السجل بصيغة JSON الآن.
gui-hint-live-close = إيقاف الاستقبال؛ يبقى السجل مفتوحًا.
gui-hint-live-field = للحقل في سطر المؤشر.
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
gui-focus-mode-on = وضع التركيز مُشغَّل. تحريك الفأرة يُظهر الأشرطة.
gui-focus-mode-off = وضع التركيز متوقف.
gui-full-screen-on = ملء الشاشة مُشغَّل.
gui-full-screen-off = ملء الشاشة متوقف.

## تخصيص الأزرار (crate::buttons).

buttons-title = تخصيص الأزرار
buttons-intro =
    { $n ->
        [one] تخصيص الأزرار، زر واحد. Enter على زر لنقله أو إزالته.
        [two] تخصيص الأزرار، زران. Enter على زر لنقله أو إزالته.
        [few] تخصيص الأزرار، { $n } أزرار. Enter على زر لنقله أو إزالته.
        [zero] تخصيص الأزرار، { $n } زر. Enter على زر لنقله أو إزالته.
       *[other] تخصيص الأزرار، { $n } زرًا. Enter على زر لنقله أو إزالته.
    }
buttons-row =
    { $bar ->
        [header] { $name }، الترويسة { $pos } من { $count }
       *[toolbar] { $name }، شريط الأدوات { $pos } من { $count }
    }
buttons-add-header = إضافة زر إلى الترويسة
buttons-add-toolbar = إضافة زر إلى شريط الأدوات
buttons-reset = إعادة الشريطين إلى الافتراضي
buttons-move-up = نقل إلى أعلى
buttons-move-down = نقل إلى أسفل
buttons-remove = إزالة
buttons-add-intro =
    { $n ->
        [one] أمر واحد للإضافة. Enter يضيفه في النهاية.
        [two] أمران للإضافة. Enter يضيف أحدهما في النهاية.
        [few] { $n } أوامر للإضافة. Enter يضيف واحدًا في النهاية.
        [zero] { $n } أمر للإضافة. Enter يضيف واحدًا في النهاية.
       *[other] { $n } أمرًا للإضافة. Enter يضيف واحدًا في النهاية.
    }
buttons-added =
    { $bar ->
        [header] أُضيف { $name }، الترويسة { $pos } من { $count }.
       *[toolbar] أُضيف { $name }، شريط الأدوات { $pos } من { $count }.
    }
buttons-removed =
    { $bar ->
        [header] أُزيل { $name } من الترويسة.
       *[toolbar] أُزيل { $name } من شريط الأدوات.
    }
buttons-moved = نُقل { $name }، { $pos } من { $count }.
buttons-first = { $name } هو الأول أصلًا.
buttons-last = { $name } هو الأخير أصلًا.
buttons-already =
    { $bar ->
        [header] { $name } موجود في الترويسة أصلًا.
       *[toolbar] { $name } موجود في شريط الأدوات أصلًا.
    }
buttons-no-name = ليس لهذا الأمر اسم قصير، فلا يمكن أن يكون زرًا.
buttons-no-button = لا زر هناك.
buttons-reset-done = أُعيدت الأزرار إلى الافتراضي.
buttons-none-to-add = لا أوامر أخرى للإضافة.
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
gui-sidebar-hint-notes = Enter ينتقل إليه. Space يعرض روابطها. { $leave } ينتقل ويعود. Escape يعود.
gui-sidebar-notes-keys = Space يعرض روابط الملاحظة.
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

## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = ملفات الإعدادات
gui-settings-export-title = تصدير الإعدادات
gui-settings-import-title = استيراد الإعدادات
gui-chooser-no-dialog = لم يُفتح منتقي الملفات في النظام. اكتب مسار الملف بدلًا من ذلك.
gui-colors-value = { $value }، التباين { $ratio } إلى 1، { $verdict }
gui-colors-help = يختار اليسار واليمين لونًا مسمى، الأزرق والبرتقالي أولًا. ويكتب Enter اسمًا أو قيمة ‎#rrggbb. ويعيد Delete لون السمة. تحتفظ كل علامة بتسطيرها أو سماكتها أو رمزها، أيًّا كان لونها.
gui-colors-value-system = { $value }؛ لا يُرسم ما دامت ألوان التباين العالي للنظام مفعّلة
gui-colors-help-system = ألوان التباين العالي للنظام مفعّلة، فتُرسم بدلًا من هذه الألوان التي تعود عند إيقافها. يحتفظ كل اسم تمييز بشكله الخاص.
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

## W9b-x: إعدادات القراءة (عرض، إعدادات القراءة).
reading-form-intro = إعدادات القراءة، { $n } إعدادات. يغيّر اليسار واليمين قيمة، ويكتب Enter قيمة، ويعيد Delete القيمة الافتراضية، ويقول F1 المساعدة.
reading-form-spacing-wcag-done = ضُبط التباعد على قيم WCAG.
reading-form-spacing-generous-done = ضُبط التباعد على الواسع، أوسع من WCAG.
spacing-letter-without-word = زِد تباعد الكلمات مع تباعد الحروف.
gui-reading-form-help = يتنقل الأعلى والأسفل، ويغيّر اليسار واليمين قيمة، ويكتب Enter قيمة، ويقول F1 المساعدة.
gui-reading-voices = الأصوات
gui-reading-voices-help = فتح مدير الأصوات.
gui-reading-wcag = تباعد WCAG
gui-reading-wcag-help = ضبط التباعدات الأربعة على قيم WCAG.
gui-reading-generous = تباعد واسع
gui-reading-generous-help = ضبط التباعدات الأربعة أوسع من WCAG.
gui-reading-closed = أُغلقت إعدادات القراءة.
## End of W9b-x
