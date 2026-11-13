### Voices, the voice manager, and restarting speech.

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
restart-no-engine = لا محرك كلام متاح؛ يبقى صامتًا. الفحوص في موضوع Troubleshooting، قسم No speech at all.
restart-done-silent = أُعيد تشغيل الكلام، لكنه صامت: لا محرك متاح.
restart-done = أُعيد تشغيل الكلام.

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
