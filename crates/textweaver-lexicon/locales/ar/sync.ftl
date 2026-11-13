### Sync between computers.

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
menu-cards = بطاقات الدراسة
menu-cw-practice = تمرين شفرة مورس
section-sync = المزامنة

## End of S4

## Sync wave, S5 (see locales/en).
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

## End of S5
