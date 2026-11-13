### The study sheet, the self-test, and study cards.

## ورقة الدراسة.

notes-nothing-to-export = لا ملاحظات أو تمييزات لتصديرها.
# Keep the letters y and n: they are the keys that answer. $file is the
# sheet's file name, $folder the folder it was saved in.
notes-study-sheet-saved-notes =
    ورقة دراسة تحتوي { $n ->
        [one] ملاحظة واحدة
        [two] ملاحظتين
        [few] { $n } ملاحظات
        [many] { $n } ملاحظة
       *[other] { $n } ملاحظة
    } حُفظت باسم { $file }. فتحها؟ y أو n. في { $folder }.
notes-study-sheet-saved-highlights =
    ورقة دراسة تحتوي { $h ->
        [one] تمييزًا واحدًا
        [two] تمييزين
        [few] { $h } تمييزات
        [many] { $h } تمييزًا
       *[other] { $h } تمييز
    } حُفظت باسم { $file }. فتحها؟ y أو n. في { $folder }.
notes-study-sheet-saved-both =
    ورقة دراسة تحتوي { $n ->
        [one] ملاحظة واحدة
        [two] ملاحظتين
        [few] { $n } ملاحظات
        [many] { $n } ملاحظة
       *[other] { $n } ملاحظة
    } و{ $h ->
        [one] تمييزًا واحدًا
        [two] تمييزين
        [few] { $h } تمييزات
        [many] { $h } تمييزًا
       *[other] { $h } تمييز
    } حُفظت باسم { $file }. فتحها؟ y أو n. في { $folder }.
notes-study-sheet-failed = تعذّرت كتابة ورقة الدراسة: { $error } تحقّق من إمكانية الكتابة في المجلد.
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = ورقة دراسة: { $title }
notes-sheet-exported = صُدِّرت من { -brand } في { $date }.
notes-sheet-before-first-heading = قبل العنوان الأول
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (الوسوم: { $tags })
# $name is the highlight's palette name.
notes-sheet-highlighted = مُيِّز، { $name }.
# On the study sheet grouped by name: the heading a highlight falls under.
notes-sheet-under = تحت: { $heading }
# The study sheet's section of notes, after the names.
notes-sheet-notes = ملاحظات

## الاختبار الذاتي: أسئلة بإجابات مخفية (crate::reveal).

reveal-self-test-title = اختبار ذاتي: { $title }
reveal-self-test-intro =
    { $n ->
        [one] اختبار ذاتي، سؤال واحد. Enter يُظهر الإجابة. مسافة للإجابة بصوت عالٍ.
        [two] اختبار ذاتي، سؤالان. Enter يُظهر كل إجابة. مسافة للإجابة بصوت عالٍ.
        [few] اختبار ذاتي، { $n } أسئلة. Enter يُظهر كل إجابة. مسافة للإجابة بصوت عالٍ.
        [zero] اختبار ذاتي، { $n } سؤال. Enter يُظهر كل إجابة. مسافة للإجابة بصوت عالٍ.
       *[other] اختبار ذاتي، { $n } سؤالًا. Enter يُظهر كل إجابة. مسافة للإجابة بصوت عالٍ.
    }
reveal-nothing-to-test = لا ملاحظات أو تمييزات للاختبار. أضف ملاحظة أو تمييزًا أولًا.
reveal-prompt-note = { $note } (في { $section })
reveal-prompt-highlight = ماذا ميّزت في { $section }؟
reveal-row-shown = { $prompt } الإجابة: { $answer }
reveal-answer = الإجابة: { $answer }
reveal-listening = أجب بصوت عالٍ الآن. مسافة للإيقاف.
reveal-you-said = قلت: { $words }. Enter يُظهر الإجابة.
reveal-heard-nothing = لم تُسمع إجابة. مسافة للمحاولة مرة أخرى.
reveal-no-dictation = الإجابة بصوت عالٍ تحتاج الإملاء، وهو غير موجود في هذا الإصدار.

## بطاقات الدراسة وجلسة الدراسة (crate::cards).

cards-blank = فراغ
cards-recall-question = ماذا يقول «{ $heading }»؟
cards-no-document = افتح مستندًا لإنشاء البطاقات أو دراستها.
cards-nothing-to-make = لا ملاحظات أو تمييزات لإنشاء البطاقات. أضف ملاحظة أو تمييزًا أولًا.
cards-made =
    { $added ->
        [0] لا بطاقات جديدة. { $total } بطاقة في المجموع.
        [one] أُنشئت البطاقات: واحدة جديدة، { $total } في المجموع.
        [two] أُنشئت البطاقات: اثنتان جديدتان، { $total } في المجموع.
       *[other] أُنشئت البطاقات: { $added } جديدة، { $total } في المجموع.
    }
cards-none-from-item = لا بطاقة من هذا العنصر: يحتاج إلى نص على مقطع.
cards-made-one = أُنشئت بطاقة: { $question }
cards-updated-one = حُدّثت بطاقة: { $question }
cards-none = لا بطاقات بعد. لإنشائها: { $key }.
cards-study-title = دراسة البطاقات: { $title }
cards-study-intro =
    { $n ->
        [one] دراسة البطاقات، بطاقة واحدة. Enter يُظهر الإجابة، ومن 1 إلى 4 لتقييمها. مسافة للإجابة بصوت عالٍ.
        [two] دراسة البطاقات، بطاقتان. Enter يُظهر كل إجابة، ومن 1 إلى 4 لتقييمها. مسافة للإجابة بصوت عالٍ.
        [few] دراسة البطاقات، { $n } بطاقات. Enter يُظهر كل إجابة، ومن 1 إلى 4 لتقييمها. مسافة للإجابة بصوت عالٍ.
        [many] دراسة البطاقات، { $n } بطاقة. Enter يُظهر كل إجابة، ومن 1 إلى 4 لتقييمها. مسافة للإجابة بصوت عالٍ.
       *[other] دراسة البطاقات، { $n } بطاقة. Enter يُظهر كل إجابة، ومن 1 إلى 4 لتقييمها. مسافة للإجابة بصوت عالٍ.
    }
cards-no-session = لا جلسة دراسة. لبدء واحدة: { $key }.
cards-card-gone = أُزيلت تلك البطاقة.
cards-grade-again = مرة أخرى
cards-grade-hard = صعبة
cards-grade-good = جيدة
cards-grade-easy = سهلة
cards-graded = { $grade }. البطاقة { $i } من { $n }. السؤال: { $question }
# Said when a grade from the palette opens the session again on the next card.
cards-graded-reopen = { $grade }. دراسة البطاقات، البطاقة { $i } من { $n }.
cards-session-done =
    { $n ->
        [one] { $grade }. انتهى: قُيّمت البطاقة.
       *[other] { $grade }. انتهى: قُيّمت كل البطاقات، وعددها { $n }.
    }
cards-reversed = معكوسة. السؤال: { $question }
cards-unreversed = كما أُنشئت. السؤال: { $question }
cards-not-reversible = بطاقات الأسئلة وحدها يمكن عكسها.
cards-kind-cloze = املأ الفراغ
cards-kind-question = سؤال
cards-kind-recall = استذكار
cards-not-graded = لم تُقيَّم بعد
cards-last-grade = آخر تقييم: { $grade }
cards-item = { $kind }: { $question }، { $grade }
cards-list-title = البطاقات
cards-list-intro =
    { $n ->
        [one] البطاقات، عنصر واحد. Enter للانتقال إلى مصدرها، Delete لإزالتها.
        [two] البطاقات، عنصران. Enter للانتقال إلى مصدر بطاقة، Delete لإزالتها.
        [few] البطاقات، { $n } عناصر. Enter للانتقال إلى مصدر بطاقة، Delete لإزالتها.
        [many] البطاقات، { $n } عنصرًا. Enter للانتقال إلى مصدر بطاقة، Delete لإزالتها.
       *[other] البطاقات، { $n } عنصر. Enter للانتقال إلى مصدر بطاقة، Delete لإزالتها.
    }
cards-source-label = مصدر البطاقة
cards-remove-question = إزالة هذه البطاقة وتقييماتها؟ y أو n
cards-removed = أُزيلت البطاقة.
cards-save-failed = تعذّر حفظ البطاقات: { $error } تحقّق من إمكانية الكتابة في مجلد البيانات.
cards-anki-exported =
    { $n ->
        [one] صُدّرت بطاقة واحدة إلى Anki: { $file }، في { $folder }.
       *[other] صُدّرت { $n } بطاقة إلى Anki: { $file }، في { $folder }.
    }
cards-anki-export-failed = تعذّر تصدير البطاقات إلى Anki: { $error } تحقّق من إمكانية الكتابة في مجلد المستند.
cards-anki-import-failed = تعذّر استيراد { $file }: { $error } تحقّق من أن الملف حزمة Anki بامتداد ‎.apkg.
cards-anki-imported =
    { $added ->
        [one] استُوردت بطاقة واحدة.
       *[other] استُوردت { $added } بطاقة.
    }
cards-anki-already =
    { $n ->
        [one] بطاقة واحدة موجودة من قبل؛ دُمجت تقييماتها.
       *[other] { $n } بطاقة موجودة من قبل؛ دُمجت تقييماتها.
    }
cards-anki-skipped-cloze =
    { $n ->
        [one] تُخطّيت ملاحظة ملء فراغ واحدة.
       *[other] تُخطّيت { $n } ملاحظة ملء فراغ.
    }
cards-anki-skipped-media =
    { $n ->
        [one] تُخطّيت ملاحظة واحدة ليست إلا صورًا أو صوتًا.
       *[other] تُخطّيت { $n } ملاحظة ليست إلا صورًا أو صوتًا.
    }
cards-anki-skipped-other =
    { $n ->
        [one] تُخطّيت ملاحظة واحدة بلا نص أو بأكثر من بطاقتين.
       *[other] تُخطّيت { $n } ملاحظة بلا نص أو بأكثر من بطاقتين.
    }
cards-from-anki = جاءت هذه البطاقة من Anki؛ ليس لها موضع في المستند.
# Said first in the study session and the Cards list (B1-f2). $due is how
# many graded cards are due, $new how many were never graded.
cards-due-summary =
    { $due ->
        [one] المستحق اليوم: بطاقة واحدة، { $new } جديدة.
       *[other] المستحق اليوم: { $due } بطاقة، { $new } جديدة.
    }
# When a card is next due, after a grade. $days is whole days, at least 1.
cards-next-in =
    { $days ->
        [one] التالية غدًا
       *[other] التالية بعد { $days } يوم
    }
cards-due-now = مستحقة اليوم
# A grade or last grade, then when the card is next due: "Good, next in 3 days".
cards-grade-next = { $grade }، { $next }
# Study cards when no card is due and none is new: every card is asked.
cards-nothing-due =
    { $days ->
        [one] لا شيء مستحق اليوم؛ البطاقة التالية مستحقة غدًا. ستُدرس كل البطاقات مسبقًا.
       *[other] لا شيء مستحق اليوم؛ البطاقة التالية مستحقة بعد { $days } يوم. ستُدرس كل البطاقات مسبقًا.
    }
