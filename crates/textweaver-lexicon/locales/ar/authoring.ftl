### Authoring: lists, citations, find and replace, tables, quick wins, and dictation.

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

## الاستشهادات: الإدراج، والبحث، والاستيراد، والفحص، وقائمة المراجع.

citations-on = الاستشهادات مفعّلة.
citations-off = الاستشهادات متوقفة.
book-audio-on = صوت الكتاب مفعّل. يُشغَّل التسجيل حيث يوجد في الكتاب.
book-audio-off = صوت الكتاب متوقف. الصوت المُركَّب يقرأ الكتاب.
book-audio-rate = تغييرات السرعة لا تنطبق على الصوت المسجَّل.
# $key names the Add Reference command's keys.
citations-library-empty = مكتبة مراجعك فارغة. أضف مرجعًا بـDOI أو ISBN أو PMID بـ{ $key }، أو شغّل استيراد مراجع من لوحة الأوامر.
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
citations-lookup-failed = تعذّر البحث عن { $input }: { $error } تحقّق من رقم DOI أو ISBN أو PMID ومن الاتصال.
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
authoring-language-mermaid = مخطط Mermaid
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
# $parts lists what came in, from the paste-part messages.
paste-markdown = لُصق بتنسيق Markdown: { $parts }
paste-part-heading =
    { $n ->
        [one] عنوان واحد
        [two] عنوانان
        [few] { $n } عناوين
        [many] { $n } عنوانًا
       *[other] { $n } عنوان
    }
paste-part-paragraph =
    { $n ->
        [one] فقرة واحدة
        [two] فقرتان
        [few] { $n } فقرات
        [many] { $n } فقرة
       *[other] { $n } فقرة
    }
paste-part-list =
    { $n ->
        [one] قائمة واحدة
        [two] قائمتان
        [few] { $n } قوائم
        [many] { $n } قائمة
       *[other] { $n } قائمة
    }
paste-part-table =
    { $n ->
        [one] جدول واحد
        [two] جدولان
        [few] { $n } جداول
        [many] { $n } جدولًا
       *[other] { $n } جدول
    }
paste-part-code =
    { $n ->
        [one] كتلة برمجية واحدة
        [two] كتلتان برمجيتان
        [few] { $n } كتل برمجية
        [many] { $n } كتلة برمجية
       *[other] { $n } كتلة برمجية
    }
paste-part-quote =
    { $n ->
        [one] اقتباس واحد
        [two] اقتباسان
        [few] { $n } اقتباسات
        [many] { $n } اقتباسًا
       *[other] { $n } اقتباس
    }
paste-part-link =
    { $n ->
        [one] رابط واحد
        [two] رابطان
        [few] { $n } روابط
        [many] { $n } رابطًا
       *[other] { $n } رابط
    }
paste-empty = لا شيء للصق: الحافظة فارغة.
paste-converting = جارٍ تحويل النص المنسق للصقه.
paste-failed = تعذّر اللصق: { $error } جرّب اللصق كنص عادي.
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
dictation-paused = الإملاء متوقف مؤقتًا.
dictation-resumed = استُؤنف الإملاء.
dictation-not-active = لا يوجد إملاء جارٍ.
dictation-cannot-pause = لا يمكن إيقاف هذا الإملاء مؤقتًا؛ أوقفه بدلًا من ذلك.
section-dictation = الإملاء
