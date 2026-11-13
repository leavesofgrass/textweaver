### The help, key names, About, and the packaged guides.

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
help-intro = مساعدة. اكتب للبحث في المساعدة كلها؛ السهمان لأعلى ولأسفل للتنقل، Escape للإغلاق.

## قائمة المساعدة. كل قيمة هي مفتاح أو مفاتيح من خريطة المفاتيح.

help-about = يقرأ textweaver المستندات بصوت عالٍ. المفاتيح أدناه هي الارتباطات الحالية.
help-search = ابحث في المساعدة كلها بالكتابة هنا: الأوامر والمفاتيح والإعدادات والأدلة.
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
help-highlights = تمييز التحديد أو الجملة، أو إزالة تمييز: { $highlight }. باسم: من { $first } إلى { $fifth }، أو القائمة: { $named }. سرد أشرطة التمييز: { $list }.
help-bookmarks-list = قائمة الإشارات المرجعية: Delete لحذف إشارة، F2 لإعادة تسميتها.
help-edit = تحرير المستند: { $edit }. حفظ: { $save }. حفظ باسم: { $saveas }. مستند جديد: { $new }.
help-editing = أثناء التحرير: تراجع { $undo }، إعادة { $redo }، عريض { $bold }. كل أمر تنسيق موجود في اختصارات لوحة المفاتيح.
help-outline = مخطط العناوين، اكتب للتصفية: { $outline }. اتباع رابط أو حاشية: { $follow }.
help-tables = الجداول: { $nextrow } و{ $previousrow } للتنقل صفًا صفًا، { $nextcell } و{ $previouscell } خلية خلية.
help-citations = الاستشهادات أثناء التحرير: إدراج { $insert }، إضافة مرجع بـDOI أو ISBN أو PMID { $reference }. التدقيق الإملائي: الخطأ التالي والسابق { $next } و{ $previous }، الاقتراحات { $suggestions }.
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

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
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

## B1-hp: الأدلة المرفقة والبحث في المساعدة.
docs-not-found = لم يُعثر على التوثيق بجانب textweaver. هل تفتح { $address } في المتصفح؟ y أو n
docs-read-only = هذا الدليل مرفق مع textweaver وهو للقراءة فقط. لتغييره، انسخ الملف إلى مكان آخر وافتح النسخة.
docs-guide-missing =
    { $n ->
        [one] دليل مفقود: { $guides }. تُفتح الأدلة الأخرى.
       *[other] أدلة مفقودة ({ $n }): { $guides }. تُفتح الأدلة الأخرى.
    }
helpsearch-title = البحث في المساعدة
helpsearch-title-matching = مساعدة عن { $filter }
helpsearch-intro = البحث في المساعدة، { $n } موضوعًا. اكتب للبحث في الأوامر والمفاتيح والإعدادات والأدلة؛ Enter يفتح موضوعًا، وF1 يشرحه.
helpsearch-match =
    { $n ->
        [one] نتيجة واحدة.
        [two] نتيجتان.
        [few] { $n } نتائج.
        [many] { $n } نتيجة.
       *[other] { $n } نتيجة.
    }
helpsearch-none = لا مساعدة عن { $query }. Backspace يحذف الحروف.
helpsearch-cleared =
    { $n ->
        [one] مُسح عامل التصفية، موضوع واحد.
        [two] مُسح عامل التصفية، موضوعان.
        [few] مُسح عامل التصفية، { $n } مواضيع.
        [many] مُسح عامل التصفية، { $n } موضوعًا.
       *[other] مُسح عامل التصفية، { $n } موضوع.
    }
helpsearch-command = { $row }، أمر
helpsearch-setting = { $label }، إعداد في { $section }
helpsearch-guide = { $heading }، في { $guide }
helpsearch-at-heading = عند { $heading }.
## End of B1-hp
