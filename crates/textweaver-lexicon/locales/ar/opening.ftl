### Opening documents, the file browser, and batch conversion.

## فتح مستند.

open-opened = فُتح { $title }.
open-resumed = فُتح { $title }. استُؤنفت القراءة عند { $pct } بالمئة.
open-resumed-synced = فُتح { $title }. استُؤنفت القراءة عند { $pct } بالمئة، من جهاز آخر.

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
open-refused-network = لم يُفتح: الملف على حاسوب آخر.
tasks-not-opened = لم يُفتح.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = فتحه؟ y أو n

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = ليس ملف JSON قابلًا للقراءة؛ قد يكون كبيرًا جدًا.
opening-damaged-notebook = ليس دفتر Jupyter قابلًا للقراءة؛ قد يكون تالفًا أو كبيرًا جدًا.
opening-damaged-canvas = ليس ملف لوحة (canvas) قابلًا للقراءة؛ قد يكون تالفًا أو كبيرًا جدًا.
opening-damaged-svg = ليس رسمًا بصيغة SVG قابلًا للقراءة؛ قد يكون تالفًا أو كبيرًا جدًا.
opening-damaged-mathml = ليست صيغة MathML قابلة للقراءة؛ قد تكون تالفة أو كبيرة جدًا.

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
