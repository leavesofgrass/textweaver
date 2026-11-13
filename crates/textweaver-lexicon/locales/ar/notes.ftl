### Notes, highlights, bookmarks, relations, and find.

## الملاحظات والتمييزات.

notes-nothing-to-attach = لا شيء هنا لإرفاق ملاحظة به.
# $on is the start of the passage the note is on.
notes-added = أُضيفت ملاحظة عند: { $on }
# $tags are the note's tags, joined with commas.
notes-added-with-tags = أُضيفت ملاحظة بالوسوم { $tags } عند: { $on }
# An item in the notes list. $anchor is the passage; $lost is yes when the
# passage was not found after the file changed.
notes-item =
    { $lost ->
        [yes] { $note }، السطر { $line }. عند: { $anchor } لم يُعثر عليها بعد تغيّر الملف.
       *[no] { $note }، السطر { $line }. عند: { $anchor }
    }
notes-none = لا ملاحظات. لإضافة واحدة: { $key }.
notes-list-title = الملاحظات
notes-list-intro =
    { $n ->
        [one] الملاحظات، عنصر واحد. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها. C لإنشاء بطاقة.
        [two] الملاحظات، عنصران. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها. C لإنشاء بطاقة.
        [few] الملاحظات، { $n } عناصر. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها. C لإنشاء بطاقة.
        [many] الملاحظات، { $n } عنصرًا. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها. C لإنشاء بطاقة.
       *[other] الملاحظات، { $n } عنصر. Enter للانتقال إلى ملاحظة، Delete لحذفها، F2 لتحريرها، Space لفتح روابطها. C لإنشاء بطاقة.
    }
# Said on jumping to a note: its text, then the passage it is on.
notes-note-content = { $note }. عند: { $anchor }
# $i is the note's number, $n how many notes there are.
notes-note-label = الملاحظة { $i } من { $n }
notes-deleted = حُذفت الملاحظة: { $text }.
notes-none-here = لا ملاحظة أو تمييز هنا.
notes-unchanged = لم تتغيّر الملاحظة.
notes-updated = تحدّثت الملاحظة.
notes-nothing-to-highlight = لا شيء هنا لتمييزه.
notes-highlight-removed = أُزيل التمييز: { $text }
notes-highlighted-at = مُيِّز، { $name }، عند { $pct } بالمئة: { $text }
notes-highlighted = مُيِّز، { $name }: { $text }
# An item in the highlights list. $name is the highlight's palette name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $name }: { $text }، السطر { $line }، لم يُعثر عليه بعد تغيّر الملف
       *[no] { $name }: { $text }، السطر { $line }
    }
notes-no-highlights = لا تمييزات. لإنشاء تمييز: { $key }.
notes-highlights-title = التمييزات
notes-highlights-intro =
    { $n ->
        [one] التمييزات، عنصر واحد. Enter للانتقال إلى أحدها، Delete لإزالته، F2 لتغيير اسمه، Space لعرض اسمه فقط. C لإنشاء بطاقة.
        [two] التمييزات، عنصران. Enter للانتقال إلى أحدها، Delete لإزالته، F2 لتغيير اسمه، Space لعرض اسمه فقط. C لإنشاء بطاقة.
        [few] التمييزات، { $n } عناصر. Enter للانتقال إلى أحدها، Delete لإزالته، F2 لتغيير اسمه، Space لعرض اسمه فقط. C لإنشاء بطاقة.
        [many] التمييزات، { $n } عنصرًا. Enter للانتقال إلى أحدها، Delete لإزالته، F2 لتغيير اسمه، Space لعرض اسمه فقط. C لإنشاء بطاقة.
       *[other] التمييزات، { $n } عنصر. Enter للانتقال إلى أحدها، Delete لإزالته، F2 لتغيير اسمه، Space لعرض اسمه فقط. C لإنشاء بطاقة.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = تمييز، { $name }
notes-highlight-changed = تغيّر التمييز إلى { $name }: { $text }
# $n is the palette entry's number, $key the keys of Highlight with a name.
notes-palette-no-entry = لا يوجد اسم تمييز رقم { $n } في اللوحة. لاختيار اسم: { $key }.
notes-highlights-named-title = التمييزات: { $name }
notes-highlights-named-intro =
    { $n ->
        [one] التمييزات باسم { $name }، عنصر واحد. Space لعرض كل التمييزات.
        [two] التمييزات باسم { $name }، عنصران. Space لعرض كل التمييزات.
        [few] التمييزات باسم { $name }، { $n } عناصر. Space لعرض كل التمييزات.
        [many] التمييزات باسم { $name }، { $n } عنصرًا. Space لعرض كل التمييزات.
       *[other] التمييزات باسم { $name }، { $n } عنصر. Space لعرض كل التمييزات.
    }
notes-palette-title = أسماء التمييز
# A palette name: the name, how many highlights have it, its shape and its color.
notes-palette-row =
    { $count ->
        [one] { $name }، تمييز واحد، { $shape }، { $color }
        [two] { $name }، تمييزان، { $shape }، { $color }
        [few] { $name }، { $count } تمييزات، { $shape }، { $color }
        [many] { $name }، { $count } تمييزًا، { $shape }، { $color }
       *[other] { $name }، { $count } تمييز، { $shape }، { $color }
    }
notes-palette-intro-highlight = التمييز بأي اسم؟ عدد الأسماء { $n }. Enter لاختيار اسم.
notes-palette-intro-change = تغيير التمييز إلى أي اسم؟ عدد الأسماء { $n }. Enter لاختيار اسم.
notes-palette-intro-collect = جمع تمييزات أي اسم؟ عدد الأسماء { $n }. Enter لكتابتها كقائمة.
notes-collect-none = لا توجد تمييزات باسم { $name }.
# The collected list's own title (Markdown).
notes-collect-title = التمييزات باسم { $name }: { $title }
notes-collect-line = (السطر { $line })
# Keep the letters y and n: they are the keys that answer.
notes-collect-saved =
    { $n ->
        [one] تمييز واحد باسم { $name } حُفظ بعنوان { $file }. فتحه؟ y أو n. في { $folder }.
        [two] تمييزان باسم { $name } حُفظا بعنوان { $file }. فتحه؟ y أو n. في { $folder }.
        [few] { $n } تمييزات باسم { $name } حُفظت بعنوان { $file }. فتحه؟ y أو n. في { $folder }.
        [many] { $n } تمييزًا باسم { $name } حُفظت بعنوان { $file }. فتحه؟ y أو n. في { $folder }.
       *[other] { $n } تمييز باسم { $name } حُفظت بعنوان { $file }. فتحه؟ y أو n. في { $folder }.
    }
palette-shape-underline = تسطير
palette-shape-double-underline = تسطير مزدوج
palette-shape-bold = غامق
palette-shape-dotted = تسطير منقط
palette-shape-brackets = أقواس
palette-shape-symbol = رمز
# Shown while reading reaches a note's passage.
notes-signal = ملاحظة: { $text }
# Said after moving onto a note's passage.
notes-has-note = تحتوي على ملاحظة: { $text }

## Relations between notes (the knowledge graph as lists).

relations-type-conflicts-with = يتعارض مع
relations-type-supports = يدعم
relations-type-is-example-of = مثال على
relations-type-cites = يستشهد بـ
relations-type-contradicts = يناقض
relations-type-defines = يعرّف
relations-type-extends = يوسّع
relations-type-see-also = انظر أيضًا
relations-type-precedes = يسبق
relations-type-follows = يلي
relations-count = الروابط: { $out } صادرة، { $in } واردة.
relations-note-title = روابط: { $note }
relations-title-filtered = { $title }، التصفية: { $filter }
relations-note-intro = روابط { $note }: { $out } صادرة، { $in } واردة. Enter لاتباع رابط، F2 لتغييره، Delete لإزالته. اكتب للتصفية حسب النوع.
relations-out-item = { $type }: { $target }
relations-target-in = { $note }، في { $doc }
relations-target-missing = ملاحظة غير موجودة
relations-empty-note = ملاحظة فارغة
relations-incoming-row =
    { $n ->
        [0] ما يرتبط هنا: لا شيء بعد
        [one] ما يرتبط هنا: ملاحظة واحدة
        [two] ما يرتبط هنا: ملاحظتان
       *[other] ما يرتبط هنا: { $n } ملاحظات
    }
relations-add-row = إضافة رابط
relations-backlinks-title = ما يرتبط هنا: { $note }
relations-backlinks-intro =
    { $n ->
        [one] ما يرتبط بـ { $note }: ملاحظة واحدة. Enter للانتقال إليها. اكتب للتصفية حسب النوع.
       *[other] ما يرتبط بـ { $note }: { $n } ملاحظات. Enter للانتقال إلى إحداها. اكتب للتصفية حسب النوع.
    }
relations-backlink-item = { $type } هذه، من: { $note }
relations-none-in = لا شيء يرتبط بهذه الملاحظة بعد.
relations-types-title = نوع الرابط لـ: { $note }
relations-types-intro = اختر نوع الرابط، 10 أنواع. اكتب للتصفية.
relations-targets-title = { $type }: أي ملاحظة؟
relations-targets-intro =
    { $n ->
        [0] لا توجد ملاحظة أخرى هنا. اختر ملاحظة في مستند آخر.
        [one] اختر الملاحظة المراد الربط بها: ملاحظة واحدة. Enter للربط.
       *[other] اختر الملاحظة المراد الربط بها: { $n } ملاحظات. Enter للربط.
    }
relations-other-document-row = ملاحظة في مستند آخر
relations-documents-title = مستندات بها ملاحظات
relations-documents-intro = مستندات بها ملاحظات: { $n }. Enter لعرض ملاحظات مستند.
relations-document-item =
    { $n ->
        [one] { $title }، ملاحظة واحدة
       *[other] { $title }، { $n } ملاحظات
    }
relations-no-other-documents = لا يوجد مستند آخر في المكتبة به ملاحظات.
relations-linked = تم الربط: { $type } { $target }.
relations-changed = تم تغيير الرابط: { $type } { $target }.
relations-already = مرتبط من قبل: { $type } { $target }.
relations-removed = تمت إزالة الرابط: { $type } { $target }.
relations-remove-question = إزالة هذا الرابط؟ y أو n
relations-nothing-to-remove = يمكن إزالة رابط فقط هنا.
relations-note-gone = الملاحظة غير موجودة: حُذفت، أو اختفى مستندها.
relations-document-missing = المستند غير موجود: { $file }.
relations-no-note-here = لا توجد ملاحظة هنا. الروابط تخص الملاحظات؛ أضف واحدة: { $key }.
relations-filter-cleared = تم مسح التصفية، { $n } معروضة.
relations-filter-none = لا شيء يطابق { $filter }.
relations-filter-matched = التصفية { $filter }: { $n } معروضة.

## تصدير مخطط المعرفة (B1-g2).

graph-export-title = تصدير مخطط المعرفة بصيغة
graph-export-intro = مخطط المعرفة، الروابط: { $links }. اختر صيغة؛ قائمة ماركداون هي النص المخصّص للقراءة.
graph-export-empty = لا روابط بين الملاحظات للتصدير. أضف رابطًا من قائمة روابط إحدى الملاحظات.
graph-format-md = قائمة ماركداون، النص المخصّص للقراءة
graph-format-json = JSON، لبرنامجي Gephi وCytoscape
graph-format-dot = DOT، لبرنامج Graphviz
graph-format-graphml = GraphML، لبرامج Gephi وCytoscape وyEd
graph-format-mermaid = مخطط Mermaid
graph-format-plantuml = مخطط PlantUML
graph-format-csv = قائمة حواف CSV، لجداول البيانات
graph-export-saved = حُفظ مخطط المعرفة باسم { $file }. فتحه؟ y أو n. في { $folder }.
graph-export-failed = تعذّرت كتابة مخطط المعرفة: { $error } تحقّق من إمكانية الكتابة في المجلد.

## الإشارات المرجعية: إعادة التسمية والحذف.

notes-choose-bookmark-delete = اختر إشارة مرجعية واضغط Delete.
notes-choose-bookmark-rename = اختر إشارة مرجعية واضغط F2 لإعادة تسميتها.
notes-bookmark-deleted = حُذفت الإشارة المرجعية { $name }.
notes-renaming-bookmark = إعادة تسمية الإشارة المرجعية { $name }.
notes-bookmark-unchanged = لم تتغيّر الإشارة المرجعية.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = توجد بالفعل إشارة مرجعية باسم { $name }. لم تتغيّر الإشارة المرجعية { $old }.
notes-bookmark-renamed = أُعيدت تسمية { $old } إلى { $name }.

## البحث، والإشارات المرجعية، والتحديد.

marks-cannot-search = تعذّر البحث: { $error } تحقّق من النمط بين الشرطتين المائلتين.
# $pattern is the text searched for.
marks-no-matches = لا تطابقات لـ{ $pattern }.
# The label of a match reached by Find, at high verbosity; $number is its place among $n matches.
marks-match-label = التطابق { $number } من { $n }
# Find wrapped past an end of the document; $dir is next (to the top) or previous (to the bottom); $message says the match.
marks-find-wrapped =
    { $dir ->
        [next] التف إلى الأعلى. { $message }
       *[previous] التف إلى الأسفل. { $message }
    }
# $name is the bookmark's name, such as mark1.
marks-bookmark-already-here = الإشارة المرجعية { $name } موجودة هنا بالفعل.
common-bookmark-set = وُضعت الإشارة المرجعية { $name } عند { $pct } بالمئة.
marks-no-bookmarks = لا إشارات مرجعية. لإضافة واحدة: { $key }.
marks-bookmarks-intro =
    { $n ->
        [one] الإشارات المرجعية، عنصر واحد. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
        [two] الإشارات المرجعية، عنصران. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
        [few] الإشارات المرجعية، { $n } عناصر. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
        [many] الإشارات المرجعية، { $n } عنصرًا. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
       *[other] الإشارات المرجعية، { $n } عنصر. Enter للانتقال إلى أحدها، Delete لحذفها، F2 لإعادة تسميتها.
    }
# One line of the bookmark list; $lost is yes when the bookmark's text was not found after the file changed; $text is the start of its line.
marks-bookmark-item =
    { $lost ->
        [yes] { $name } (لم يُعثر عليها بعد تغيّر الملف)، السطر { $line }، { $pct } بالمئة: { $text }
       *[no] { $name }، السطر { $line }، { $pct } بالمئة: { $text }
    }
marks-bookmarks-title = الإشارات المرجعية
# The label of a bookmark reached, at high verbosity.
marks-bookmark-label = الإشارة المرجعية { $name }
marks-selection-cleared = مُسح التحديد.
# $text is the selected text, shortened.
marks-selected = حُدِّد { $text }
