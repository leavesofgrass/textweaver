### Export, preview, and audio export.

## التصدير والمعاينة من القارئ. F5 هو مفتاح إعادة التحميل في المتصفح،
## وليس مفتاحًا في textweaver.

common-no-document = لا يوجد مستند مفتوح.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = تعذّرت الكتابة إلى { $path }: { $error } تحقّق من إمكانية الكتابة في المجلد.
publish-cannot-write = تعذّرت كتابة { $path }: { $error } تحقّق من إمكانية الكتابة في مجلده.
publish-start-failed = تعذّر بدء التصدير: { $error } انتظر قليلًا، ثم حاول مرة أخرى.
publish-export-error = تعذّر التصدير: { $error } أصلح ذلك، ثم صدّر مرة أخرى.
publish-export-as-label = تصدير باسم، Enter لـ{ $path }
publish-export-over-source = لم يُصدَّر: هذا هو المستند نفسه. اختر اسمًا آخر.
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
# "Browser preview follows" was chosen. $again is yes when a preview
# is open that reloads by itself only once it is opened again.
publish-follow-off = معاينة المتصفح لا تتبع شيئًا: اضغط F5 في المتصفح بعد الحفظ.
publish-follow-save =
    { $again ->
        [yes] معاينة المتصفح تتبع كل حفظ: تُعاد الصفحة تلقائيًا. افتح المعاينة في المتصفح مرة أخرى لبدء ذلك.
       *[no] معاينة المتصفح تتبع كل حفظ: تُعاد الصفحة تلقائيًا.
    }
publish-follow-typing =
    { $again ->
        [yes] معاينة المتصفح تتبع كتابتك: تُعاد الصفحة بعد كل حفظ وعند توقف الكتابة. افتح المعاينة في المتصفح مرة أخرى لبدء ذلك.
       *[no] معاينة المتصفح تتبع كتابتك: تُعاد الصفحة بعد كل حفظ وعند توقف الكتابة.
    }
# The first preview of a session: what the browser will do. $follow is
# the "Browser preview follows" setting: off, save, or typing.
publish-preview-first =
    { $follow ->
        [save] تُفتح المعاينة في متصفحك وتُعاد بعد كل حفظ.
        [typing] تُفتح المعاينة في متصفحك وتتبع كتابتك.
       *[off] تُفتح المعاينة في متصفحك. اضغط F5 هناك بعد كل حفظ.
    }
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
# Said once for a document whose preview was cleaned of raw HTML
# (scripts, event handlers) because its folder is not trusted.
publish-preview-raw-html-left-out = حُذف HTML الخام من المعاينة. المجلدات الموثوقة في الإعدادات.
publish-server-failed = تعذّر بدء خادم إعادة تحميل المعاينة ({ $error })؛ يجري فتح الملف بدلًا من ذلك.
publish-render-failed = تعذّر عرض النص: { $error } اخرج من وضع التحرير لقراءة النص.
publish-nothing-after-caret = لا شيء لقراءته بعد المؤشر.
publish-listening = الاستماع إلى النص المعروض.

## خادم إعادة تحميل المعاينة: يظهر في المتصفح.

preview-being-written = المعاينة قيد الكتابة. أعد التحميل بعد لحظة.

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
audio-video-progress =
    { $minutes ->
        [one] جارٍ ترميز الفيديو: { $fed } من { $all } إطار، بقيت دقيقة تقريبًا.
       *[other] جارٍ ترميز الفيديو: { $fed } من { $all } إطار، بقي نحو { $minutes } دقائق.
    }
audio-video-progress-soon = جارٍ ترميز الفيديو: { $fed } من { $all } إطار، بقي أقل من دقيقة.
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
