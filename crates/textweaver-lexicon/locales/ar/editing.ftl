### Edit mode: typing, formatting, saving, recovery, and templates.

## وضع التحرير: الدخول والمغادرة والحفظ والكتابة.

# $key makes a new document.
edit-no-document = لا مستند لتحريره. اضغط { $key } لواحد جديد.
# $line is the line at the caret, as echoed.
edit-mode-on-brief = وضع التحرير مفعّل. { $line }
# $save and $finish are the keys that save and leave edit mode; $line is
# the line at the caret.
edit-mode-on = وضع التحرير مفعّل. حفظ: { $save }. إنهاء: { $finish }. { $line }
# Keep the letters s, d and c: they are the keys that answer.
edit-unsaved-question = { $title } به تغييرات غير محفوظة. حفظ أو تجاهل أو إلغاء؟ اضغط s أو d أو c، أو لأعلى ولأسفل ثم Enter. Escape للإلغاء.
edit-save-changes-title = حفظ التغييرات في { $title }؟
edit-choice-save = حفظ، ثم متابعة
edit-choice-discard = تجاهل التغييرات
edit-choice-cancel = إلغاء، متابعة التحرير
edit-save-failed = تعذّر الحفظ: { $error } ما زلت في وضع التحرير. جرّب حفظ باسم.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = حفظ باسم، Enter لـ{ $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } موجود بالفعل. استبداله؟ y أو n
edit-mode-off = وضع التحرير متوقف.
edit-mode-off-discarded = تُجوهلت التغييرات. وضع التحرير متوقف.
# The title of a new, unsaved document.
edit-untitled = بلا عنوان
edit-new-document = مستند جديد جاهز للتحرير.
# $key turns on edit mode.
edit-nothing-to-save = لا شيء لحفظه. فعّل وضع التحرير بـ{ $key } لإجراء تغييرات.
# $key turns on edit mode; $what is what the user tried to do.
edit-not-editing =
    { $what ->
        [type] فعّل وضع التحرير بـ{ $key } للكتابة.
        [change-text] فعّل وضع التحرير بـ{ $key } لتغيير النص.
        [delete-text] فعّل وضع التحرير بـ{ $key } لحذف نص.
        [undo] فعّل وضع التحرير بـ{ $key } للتراجع.
        [redo] فعّل وضع التحرير بـ{ $key } للإعادة.
        [replace-text] فعّل وضع التحرير بـ{ $key } لاستبدال نص.
        [insert] فعّل وضع التحرير بـ{ $key } للإدراج في النص.
        [cut-text] فعّل وضع التحرير بـ{ $key } لقص نص.
        [move-cells] فعّل وضع التحرير بـ{ $key } للتنقل بين خلايا الجدول.
        [delete-words] فعّل وضع التحرير بـ{ $key } لحذف كلمات.
        [paste] فعّل وضع التحرير بـ{ $key } للصق.
        [citation] فعّل وضع التحرير بـ{ $key } لإدراج استشهاد.
        [bibliography] فعّل وضع التحرير بـ{ $key } لإدراج قائمة مراجع.
       *[format] فعّل وضع التحرير بـ{ $key } لتنسيق نص.
    }
# A paste: $n characters.
edit-pasted =
    { $n ->
        [one] لُصق حرف واحد.
        [two] لُصق حرفان.
        [few] لُصقت { $n } حروف.
        [many] لُصق { $n } حرفًا.
       *[other] لُصق { $n } حرف.
    }
# $start is how the pasted text starts.
edit-pasted-start =
    { $n ->
        [one] لُصق حرف واحد: { $start }
        [two] لُصق حرفان: { $start }
        [few] لُصقت { $n } حروف: { $start }
        [many] لُصق { $n } حرفًا: { $start }
       *[other] لُصق { $n } حرف: { $start }
    }
edit-insert-failed = تعذّر الإدراج: { $error } لم يتغيّر النص.
# $start is the first words of the pasted text.
edit-pasted-lines =
    { $n ->
        [one] لُصق سطر واحد: { $start }
        [two] لُصق سطران: { $start }
        [few] لُصقت { $n } أسطر: { $start }
        [many] لُصق { $n } سطرًا: { $start }
       *[other] لُصق { $n } سطر: { $start }
    }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = لا يمكن تغيير الحروف من { $start } إلى { $end }: النص يحتوي { $len }.
edit-change-failed = تعذّر تغيير النص: { $error } لم يتغيّر النص.
edit-delete-failed = تعذّر الحذف: { $error } لم يتغيّر النص.
edit-list-ended = انتهت القائمة.
# Said when Enter continues a bulleted list.
edit-bullet = نقطة
edit-table-divider = فاصل رأس الجدول
edit-end-of-line-stop = نهاية السطر.
edit-start-of-line-stop = بداية السطر.
edit-end-of-line-content = نهاية السطر

## وضع التحرير: التنسيق، والتراجع، والجداول، والصور، والاستبدال.

# $what names the formatting command; $level is a heading level, and
# $cols and $rows a table's size.
edit-format-done =
    { $what ->
        [bold] عريض.
        [italic] مائل.
        [underline] تسطير.
        [strikethrough] يتوسطه خط.
        [code] شيفرة.
        [code-block] كتلة شيفرة.
        [link] رابط.
        [bulleted-list] قائمة نقطية.
        [numbered-list] قائمة مرقّمة.
        [block-quote] اقتباس.
        [horizontal-rule] أُدرج خط أفقي.
        [table-row] أُضيف صف إلى الجدول.
        [heading-level] مستوى العنوان { $level }.
        [table] أُدرج جدول، { $cols } أعمدة في { $rows } صفوف.
       *[heading] عنوان.
    }
# The command toggled its markup off.
edit-format-removed =
    { $what ->
        [bold] أُزيل العريض.
        [italic] أُزيل المائل.
        [underline] أُزيل التسطير.
        [strikethrough] أُزيل الخط المتوسط.
        [code] أُزيلت الشيفرة.
        [code-block] أُزيلت كتلة الشيفرة.
        [link] أُزيل الرابط.
        [bulleted-list] أُزيلت القائمة النقطية.
        [numbered-list] أُزيلت القائمة المرقّمة.
        [block-quote] أُزيل الاقتباس.
        [horizontal-rule] أُزيل الخط الأفقي.
        [table-row] أُزيل صف الجدول.
        [heading-level] أُزيل مستوى العنوان { $level }.
        [table] أُزيل الجدول.
       *[heading] أُزيل العنوان.
    }
edit-format-unchanged =
    { $what ->
        [bold] عريض: لم يتغيّر شيء.
        [italic] مائل: لم يتغيّر شيء.
        [underline] تسطير: لم يتغيّر شيء.
        [strikethrough] يتوسطه خط: لم يتغيّر شيء.
        [code] شيفرة: لم يتغيّر شيء.
        [code-block] كتلة شيفرة: لم يتغيّر شيء.
        [link] رابط: لم يتغيّر شيء.
        [bulleted-list] قائمة نقطية: لم يتغيّر شيء.
        [numbered-list] قائمة مرقّمة: لم يتغيّر شيء.
        [block-quote] اقتباس: لم يتغيّر شيء.
        [horizontal-rule] خط أفقي: لم يتغيّر شيء.
        [table-row] صف الجدول: لم يتغيّر شيء.
        [heading-level] مستوى العنوان { $level }: لم يتغيّر شيء.
        [table] الجدول: لم يتغيّر شيء.
       *[heading] عنوان: لم يتغيّر شيء.
    }
# Added after a formatting message; $text is the start of the selection.
edit-format-selected = المحدَّد: { $text }
edit-heading-level-now = مستوى العنوان { $level }.
# $line is the line at the caret after the undo or redo.
edit-undo-redo =
    { $what ->
        [undo] تراجع.
       *[redo] إعادة.
    }
edit-undo-redo-line =
    { $what ->
        [undo] تراجع. { $line }
       *[redo] إعادة. { $line }
    }
edit-nothing-to-undo = لا شيء للتراجع عنه.
edit-nothing-to-redo = لا شيء لإعادته.
edit-not-a-table-size = ليس حجم جدول: { $text }. اكتب أعمدة وصفوفًا، مثل 3 by 2.
# $name is the image's file name.
edit-image-inserted = أُدرجت الصورة { $name }. وصفها محدَّد؛ اكتب لاستبداله.
edit-image-failed = تعذّر إدراج الصورة: { $error } لم يتغيّر النص.
# Edit mode's Obsidian aids: a wiki link, tasks, callouts.
edit-wikilink-title = رابط إلى ملاحظة
edit-wikilink-intro =
    { $n ->
        [one] رابط إلى ملاحظة، ملاحظة واحدة. يختارها مفتاح الإدخال.
       *[other] رابط إلى ملاحظة، { $n } ملاحظات. يختار مفتاح الإدخال واحدة.
    }
edit-wikilink-no-notes = لا توجد ملاحظات في هذا المخزن يمكن الربط إليها.
edit-wikilink-anchor-title = رابط إلى جزء من { $note }
edit-wikilink-anchor-intro = رابط إلى الملاحظة { $note } كلها، أو إلى عنوان أو كتلة فيها. يختار مفتاح الإدخال.
edit-wikilink-whole = الملاحظة كلها
edit-wikilink-heading = عنوان { $level }: { $text }
edit-wikilink-block = كتلة { $id }: { $text }
edit-wikilink-inserted = أُدرج رابط إلى { $target }.
edit-callout-title = نوع التنبيه
edit-callout-intro = أنواع التنبيه، { $n }. يدرج مفتاح الإدخال واحدًا.
edit-callout-inserted = أُدرج تنبيه من نوع { $type }. اكتب نصه.
edit-task-status =
    { $status ->
        [done] محددة.
        [open] غير محددة.
        [in-progress] قيد التنفيذ.
        [cancelled] ملغاة.
        [forwarded] محوّلة.
        [important] مهمة.
        [question] سؤال.
       *[other] معلّمة بـ { $mark }.
    }
edit-task-new = مهمة جديدة، غير محددة.
edit-tasks-changed =
    { $n ->
        [one] تغيّرت مهمة في سطر واحد.
       *[other] تغيّرت المهام في { $n } أسطر.
    }
# $query is the text to find.
common-no-matches = لا تطابقات لـ{ $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] تطابق واحد لـ{ $query }. الاستبدال بماذا؟
        [two] تطابقان لـ{ $query }. الاستبدال بماذا؟
        [few] { $n } تطابقات لـ{ $query }. الاستبدال بماذا؟
        [many] { $n } تطابقًا لـ{ $query }. الاستبدال بماذا؟
       *[other] { $n } تطابق لـ{ $query }. الاستبدال بماذا؟
    }

## وضع التحرير: الحفظ التلقائي واسترداد العمل غير المحفوظ.

common-recovery-write-failed = تعذّرت كتابة نسخة الاسترداد: { $error }. احفظ قريبًا؛ سيواصل { -brand } المحاولة.
common-recovery-writing-again = تجري كتابة نسخة الاسترداد مجددًا.
# $title is the document; $when is how long ago its work was saved.
edit-recovery-title = استرداد العمل غير المحفوظ في { $title }؟
edit-recovery-yes = نعم، استرداد { $title } ومتابعة التحرير
edit-recovery-no = لا، تجاهل التغييرات غير المحفوظة
edit-recovery-discarded = تُجوهلت التغييرات غير المحفوظة في { $title }.
edit-recovered = استُرد العمل غير المحفوظ في { $title }. تذكّر الحفظ.
edit-recovery-postponed = أُجِّل الاسترداد. سيُعرض العمل غير المحفوظ مجددًا في المرة القادمة.

## الحفظ في الخلفية.

writes-still-saving = ما زال الحفظ جاريًا. يُرجى الانتظار.
writes-not-written-in-time = تعذّرت كتابة بعض التغييرات في الوقت المناسب: القرص لا يستجيب.
# $error is the system's reason.
writes-save-failed = تعذّر الحفظ: { $error }. ما زلت في وضع التحرير.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-not-saved = الإشارة المرجعية { $name } موضوعة الآن، لكن تعذّر حفظها: { $error } تحقّق من إمكانية الكتابة في مجلد البيانات.
# $name is the saved file's name.
writes-saved = حُفظ { $name }. ما زلت في وضع التحرير.

## الملفات المتغيّرة على القرص. $name اسم ملف. أبقِ على الحرفين y وn:
## فهما المفتاحان للإجابة.

disk-replace-question = { $name } موجود بالفعل. استبداله؟ y أو n
# A prompt label, also said with a full stop after it.
disk-not-replaced = لم يُستبدل. اكتب اسمًا آخر
# $key is the key for Save As.
disk-not-saved = لم يُحفظ. ما زلت في وضع التحرير. حفظ باسم، { $key }، يبقي على النسختين.
disk-kept-open-version = أُبقي على النسخة المفتوحة.
disk-overwrite-question = { $name } تغيّر على القرص منذ فتحته. حفظ التغييرات فوق ذلك؟ y أو n
disk-reload-question = { $name } تغيّر على القرص. إعادة تحميله؟ y أو n

## علامات عُثر عليها مجددًا بعد تغيّر ملف خارج textweaver.

relocate-reading-position = موضع قراءتك
relocate-bookmarks =
    { $n ->
        [one] إشارة مرجعية واحدة
        [two] إشارتان مرجعيتان
        [few] { $n } إشارات مرجعية
        [many] { $n } إشارة مرجعية
       *[other] { $n } إشارة مرجعية
    }
relocate-notes =
    { $n ->
        [one] ملاحظة واحدة
        [two] ملاحظتان
        [few] { $n } ملاحظات
        [many] { $n } ملاحظة
       *[other] { $n } ملاحظة
    }
relocate-highlights =
    { $n ->
        [one] تمييز واحد
        [two] تمييزان
        [few] { $n } تمييزات
        [many] { $n } تمييزًا
       *[other] { $n } تمييز
    }
# Lists of relocate-* items: "a and b", and "a, b, and c", where $rest is
# every item but the last, joined by commas.
relocate-join-two = { $a } و{ $b }
relocate-join-more = { $rest }، و{ $last }
# $items is a list of the items above; $n how many marks it counts in all.
relocate-moved =
    { $n ->
        [one] نُقل { $items } لمطابقتها
       *[other] نُقلت { $items } لمطابقتها
    }
relocate-lost =
    { $n ->
        [one] تعذّر العثور على { $items } وهي موسومة
       *[other] تعذّر العثور على { $items } وهي موسومة
    }
# $clauses are relocate-moved and relocate-lost, joined by a comma.
relocate-changed = تغيّر الملف؛ { $clauses }.

## مستندات جديدة من قوالب.

# The built-in templates' names.
templates-essay = مقالة
templates-report = تقرير
templates-notes = الملاحظات
# One of the user's own templates in the list; $name is its file name.
templates-yours = { $name }، قالبك
# $folder is where the user's own templates go.
templates-intro =
    { $n ->
        [one] مستند جديد من قالب، قالب واحد. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
        [two] مستند جديد من قالب، قالبان. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
        [few] مستند جديد من قالب، { $n } قوالب. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
        [many] مستند جديد من قالب، { $n } قالبًا. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
       *[other] مستند جديد من قالب، { $n } قالب. Enter للاختيار. قوالبك الخاصة تذهب إلى { $folder }.
    }
# The title given when none is typed.
templates-untitled = بلا عنوان
# $template is the template's name, $title the document's, $date today's date (2026-09-26).
templates-created = مستند جديد من قالب { $template }: { $title }. بتاريخ { $date }. المؤشر حيث تبدأ الكتابة. تذكّر الحفظ.
