### Tracked changes and comments in Word files.

## B1-t1: التغييرات المتعقبة والتعليقات (قائمة التغييرات).
prompt-comment-reply = الرد
prompt-comment-text = التعليق
prompt-github-token = رمز GitHub لمصدر المكونات، Enter للتخطي
prompt-import-cards = استيراد بطاقات الدراسة من حزمة Anki
prompt-cw-copy = اكتب ما سمعته
changes-title = التغييرات والتعليقات
changes-intro =
    { $n ->
        [one] { $title }، عنصر واحد. Enter ينتقل إليه. A يقبل التغيير، R يرفضه، ومع Shift يشمل كل تغييرات المؤلف نفسه. على التعليق، F2 للرد، Space للحل، Delete للحذف. N يضيف تعليقا.
       *[other] { $title }، { $n } عناصر. Enter ينتقل إلى أحدها. A يقبل التغيير، R يرفضه، ومع Shift يشمل كل تغييرات المؤلف نفسه. على التعليق، F2 للرد، Space للحل، Delete للحذف. N يضيف تعليقا.
    }
changes-none = لا توجد تغييرات متعقبة ولا تعليقات في هذا المستند.
changes-row = { $kind }: «{ $text }»، { $who }، { $when }
changes-kind-inserted = إدراج
changes-kind-deleted = حذف
changes-kind-moved-away = نقل من هنا
changes-kind-moved-here = نقل إلى هنا
changes-by = بواسطة { $author }
changes-no-author = المؤلف غير مسجل
changes-no-date = التاريخ غير مسجل
changes-date = { $weekday }، { $day } { $month } { $year }
changes-weekday-0 = الأحد
changes-weekday-1 = الاثنين
changes-weekday-2 = الثلاثاء
changes-weekday-3 = الأربعاء
changes-weekday-4 = الخميس
changes-weekday-5 = الجمعة
changes-weekday-6 = السبت
changes-month-1 = يناير
changes-month-2 = فبراير
changes-month-3 = مارس
changes-month-4 = أبريل
changes-month-5 = مايو
changes-month-6 = يونيو
changes-month-7 = يوليو
changes-month-8 = أغسطس
changes-month-9 = سبتمبر
changes-month-10 = أكتوبر
changes-month-11 = نوفمبر
changes-month-12 = ديسمبر
changes-comment = تعليق: { $text }
changes-comment-by = تعليق من { $author }: { $text }
changes-replies =
    { $n ->
        [one] رد واحد
       *[other] { $n } ردود
    }
changes-resolved = محلول
changes-accepted = تم القبول. { $kind }: «{ $text }».
changes-rejected = تم الرفض. { $kind }: «{ $text }».
changes-accepted-all =
    { $n ->
        [one] تم قبول تغيير واحد.
       *[other] تم قبول { $n } تغييرات.
    }
changes-rejected-all =
    { $n ->
        [one] تم رفض تغيير واحد.
       *[other] تم رفض { $n } تغييرات.
    }
changes-accepted-author =
    { $n ->
        [one] تم قبول تغيير واحد من { $author }.
       *[other] تم قبول { $n } تغييرات من { $author }.
    }
changes-rejected-author =
    { $n ->
        [one] تم رفض تغيير واحد من { $author }.
       *[other] تم رفض { $n } تغييرات من { $author }.
    }
changes-none-left = لا توجد تغييرات متعقبة للقبول أو الرفض.
changes-not-a-change = هذا الصف تعليق. F2 للرد، Space للحل، Delete للحذف.
changes-not-a-comment = هذا الصف تغيير. A يقبله، R يرفضه.
changes-edit-mode = تبقى التغييرات كما هي في وضع التحرير. اخرج من وضع التحرير لقبولها أو رفضها.
changes-replied = تمت إضافة الرد.
changes-resolved-done = تم حل التعليق.
changes-reopened = أعيد فتح التعليق.
changes-comment-deleted = تم حذف التعليق وردوده.
changes-comment-added = تمت إضافة التعليق.
changes-delete-comment-question = حذف هذا التعليق وردوده؟ y أو n
changes-written-accepted =
    { $n ->
        [one] تمت كتابة { $path } مع قبول تغيير واحد.
       *[other] تمت كتابة { $path } مع قبول { $n } تغييرات.
    }
changes-written-rejected =
    { $n ->
        [one] تمت كتابة { $path } مع رفض تغيير واحد.
       *[other] تمت كتابة { $path } مع رفض { $n } تغييرات.
    }
## End of B1-t1

## B1-t2: حفظ المراجعة في ملف Word.
changes-accept-all-question =
    { $n ->
        [one] قبول التغيير الوحيد؟ y أو n
       *[other] قبول كل التغييرات وعددها { $n }؟ y أو n
    }
changes-reject-all-question =
    { $n ->
        [one] رفض التغيير الوحيد؟ y أو n
       *[other] رفض كل التغييرات وعددها { $n }؟ y أو n
    }
changes-save-not-word = ليس ملف Word. لا تُحفظ التغييرات إلا في ملفات .docx؛ ويكتب التصدير النص المقرر بتنسيقات أخرى.
changes-save-nothing = لا شيء للحفظ: لم يُقبل أي تغيير ولم يُرفض، ولم يتغير أي تعليق.
changes-saved-backup = حُفظت التغييرات في { $file }. الأصل محفوظ باسم { $backup }.
changes-saved = حُفظت التغييرات في { $file }.
changes-save-unplaced =
    { $n ->
        [one] تعليق جديد واحد في البداية: لم يُعثر على نصه.
       *[other] { $n } تعليقات جديدة في البداية: لم يُعثر على نصها.
    }
changes-save-failed = تعذّر حفظ التغييرات في { $file }: { $error }. الملف كما كان؛ أغلقه في Word إن كان مفتوحًا، ثم حاول مرة أخرى.
changes-save-not-in-build = حفظ التغييرات في ملف Word غير متوفر في نسخة { -brand } هذه. ما زال tw changes --in-place يقوم بذلك.
changes-in-place-accepted =
    { $n ->
        [one] قُبل تغيير واحد في { $path }. الأصل محفوظ باسم { $backup }.
       *[other] قُبلت { $n } تغييرات في { $path }. الأصل محفوظ باسم { $backup }.
    }
changes-in-place-rejected =
    { $n ->
        [one] رُفض تغيير واحد في { $path }. الأصل محفوظ باسم { $backup }.
       *[other] رُفضت { $n } تغييرات في { $path }. الأصل محفوظ باسم { $backup }.
    }

## B1-t3: edits saved into a Word file as tracked changes.
changes-tracked-saved =
    { $n ->
        [one] حُفظ تغيير متتبع واحد في { $file }.
       *[other] حُفظت { $n } تغييرات متتبعة في { $file }.
    }
changes-original-kept = الأصل محفوظ باسم { $backup }.
changes-tracked-refused =
    { $n ->
        [one] لم يُحفظ: تغيير واحد في { $file } يعبر فقرة أو يقع في رابط أو حقل، فلا يمكن تتبعه. الحفظ باسم يحتفظ بتعديلاتك بصيغة Markdown.
       *[other] لم يُحفظ: { $n } تغييرات في { $file } تعبر فقرة أو تقع في رابط أو حقل، فلا يمكن تتبعها. الحفظ باسم يحتفظ بتعديلاتك بصيغة Markdown.
    }
## End of B1-t2
