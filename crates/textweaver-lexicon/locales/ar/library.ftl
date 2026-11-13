### The library, links and footnotes, and the dictionary.

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
        [one] المكتبة، مستند واحد. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل، { $queue } لإضافته إلى قائمة القراءة.
        [two] المكتبة، مستندان. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل، { $queue } لإضافته إلى قائمة القراءة.
        [few] المكتبة، { $n } مستندات. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل، { $queue } لإضافته إلى قائمة القراءة.
        [many] المكتبة، { $n } مستندًا. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل، { $queue } لإضافته إلى قائمة القراءة.
       *[other] المكتبة، { $n } مستند. اكتب للتصفية، Enter لفتح واحد، F2 لتعديل التفاصيل، { $queue } لإضافته إلى قائمة القراءة.
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
# A link to a file on another computer (a network share). Keep the
# letters y and n: they are the keys that answer. $target is the link.
links-network-question = فتح ملف على حاسوب آخر؟ y أو n. قد تُرسل إليه بيانات تسجيل دخولك. { $target }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = عنوان
# $anchor is the heading name the link gives.
links-no-heading = لا عنوان باسم { $anchor } في هذا المستند.
# $file is the file the link names.
links-file-not-found = الرابط يذهب إلى { $file }، الذي لم يُعثر عليه.
links-block-label = كتلة
links-no-block = لا توجد كتلة { $id } في هذا المستند.
links-note-not-found = رابط إلى ملاحظة غير موجودة: { $name }.
vault-same-name = { $count } ملاحظات باسم { $name }؛ فُتحت الأقرب.
vault-properties-count = الخصائص: { $n }.
vault-properties-title = الخصائص
vault-no-properties = لا خصائص لهذا المستند.
vault-property-row = { $name }: { $value }
vault-property-checked = محدد
vault-property-not-checked = غير محدد
vault-no-file = هذا المستند غير محفوظ في مجلد، لذا لا مخزن له.
vault-partial = بُحث في جزء من المخزن فقط؛ فهو كبير جدًا.
vault-top-folder = المجلد الأعلى
vault-backlink-row =
    { $links ->
        [one] { $title }، رابط واحد، في { $folder }
       *[other] { $title }، { $links } روابط، في { $folder }
    }
vault-note-row = { $title }، في { $folder }
vault-backlinks-title = الملاحظات التي ترتبط هنا
vault-backlinks-intro =
    { $n ->
        [one] الملاحظات التي ترتبط هنا، ملاحظة واحدة. Enter لفتحها.
       *[other] الملاحظات التي ترتبط هنا، { $n } ملاحظات. Enter لفتح واحدة.
    }
vault-no-backlinks = لا توجد ملاحظات ترتبط هنا.
vault-unresolved-title = روابط إلى ملاحظات غير موجودة
vault-unresolved-intro =
    { $n ->
        [one] روابط إلى ملاحظات غير موجودة، رابط واحد.
       *[other] روابط إلى ملاحظات غير موجودة، { $n } روابط.
    }
vault-no-unresolved = كل رابط في هذه الملاحظة يقود إلى ملاحظة.
vault-note-tags-title = الوسوم في هذه الملاحظة
vault-note-tags-intro =
    { $n ->
        [one] الوسوم في هذه الملاحظة، وسم واحد. Enter يعرض الملاحظات التي تحمله.
       *[other] الوسوم في هذه الملاحظة، { $n } وسوم. Enter يعرض ملاحظات وسم.
    }
vault-no-note-tags = لا وسوم لهذه الملاحظة.
vault-tags-title = الوسوم في المخزن
vault-tags-intro =
    { $n ->
        [one] الوسوم في المخزن، وسم واحد. Enter يعرض ملاحظاته.
       *[other] الوسوم في المخزن، { $n } وسوم. Enter يعرض ملاحظات وسم.
    }
vault-no-tags = لا وسوم في هذا المخزن.
vault-tag-row =
    { $n ->
        [one] { $tag }، ملاحظة واحدة
       *[other] { $tag }، { $n } ملاحظات
    }
vault-tag-notes-title = ملاحظات تحمل وسمًا
vault-tag-notes-intro =
    { $n ->
        [one] الوسم { $tag }، ملاحظة واحدة. Enter لفتحها.
       *[other] الوسم { $tag }، { $n } ملاحظات. Enter لفتح واحدة.
    }
vault-tag-no-notes = لا ملاحظة تحمل الوسم { $tag }.
# $title is the document's title; $heading is the heading arrived at, $anchor
# the heading a link names; $key names the History Back command's keys;
# $text is the first words at the position arrived at.
links-opened = فُتح { $title }. الرجوع: { $key }.
links-opened-at = فُتح { $title }، عند { $heading }. الرجوع: { $key }.
links-opened-no-heading = فُتح { $title }، في البداية: لا عنوان باسم { $anchor }. الرجوع: { $key }.
links-opened-no-block = فُتح { $title }، في البداية: لا توجد كتلة { $id }. الرجوع: { $key }.
links-back-in = الرجوع في { $title }: { $text }
links-forward-in = التقدّم في { $title }: { $text }
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = الرجوع إلى مرجع الحاشية { $label }، السطر { $line }.
# $text is the start of the note.
links-footnote = الحاشية { $label }: { $text }
links-footnote-unreferenced = لا مرجع للحاشية { $label } في النص.
links-footnote-no-note = الحاشية { $label } بلا ملاحظة.

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

## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = صفحة { $page }
