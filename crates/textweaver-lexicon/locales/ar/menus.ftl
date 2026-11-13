### Menus, the command palette, and interface announcements.

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
menu-highlights = التمييزات
menu-tables = الجداول
menu-speech-cursor = مؤشر النطق

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
menu-context = قائمة السياق
menu-context-closed = أُغلقت قائمة السياق.
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
color-name-theme = لون السمة
color-name-blue = أزرق
color-name-orange = برتقالي
color-name-navy = أزرق داكن
color-name-skyblue = أزرق سماوي
color-name-teal = أزرق مخضر
color-name-gold = ذهبي
color-name-yellow = أصفر
color-name-green = أخضر
color-name-cyan = سماوي
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
colors-palette-label = اسم التمييز { $name }
colors-palette-value = { $color }، { $shape }، التباين { $text } إلى 1 مع النص و{ $page } إلى 1 مع الصفحة، { $verdict }
colors-palette-system = { $color }، { $shape }؛ تُرسم ألوان التباين العالي للنظام، ويظل الشكل يميّزه
colors-palette-unknown = { $color }، { $shape }؛ لون لا يستطيع textweaver قياسه
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
