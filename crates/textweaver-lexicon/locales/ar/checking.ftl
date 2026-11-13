### Markdown lint, grammar, and spelling.

## فحص ماركداون (وضع التحرير). تُقال المشكلة بعد "Lint: ".

lint-heading-level = مستوى العنوان { $level } بعد المستوى { $prev }؛ استخدم المستوى { $use }.
# $reference is the link reference's name.
lint-link-reference = مرجع الرابط { $reference } بلا تعريف.
lint-bare-url = عنوان ويب مجرَّد؛ ضعه بين قوسين مثلثين أو اجعله رابطًا باسم.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = علامة القائمة { $marker }؛ تستخدم هذه القائمة { $used }.
lint-marker-dash = شرطة
lint-marker-star = نجمة
lint-marker-plus = زائد
lint-marker-other = أخرى
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = علامات جدولة أو مسافات في سطر فارغ.
lint-trailing-tabs-line-end = علامات جدولة أو مسافات في نهاية السطر.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] مسافة واحدة في سطر فارغ.
        [two] مسافتان في سطر فارغ.
        [few] { $n } مسافات في سطر فارغ.
        [many] { $n } مسافة في سطر فارغ.
       *[other] { $n } مسافة في سطر فارغ.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] مسافة واحدة في نهاية السطر.
        [two] مسافتان في نهاية السطر.
        [few] { $n } مسافات في نهاية السطر.
        [many] { $n } مسافة في نهاية السطر.
       *[other] { $n } مسافة في نهاية السطر.
    }
# $key turns on edit mode.
lint-not-editing = يفحص Lint ماركداون الذي تكتبه. فعّل وضع التحرير بـ{ $key } أولًا.
lint-not-markdown = يفحص Lint ماركداون، وهذا المستند ليس ماركداون.
lint-none = لا مشكلات في Lint.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] لا مشكلة Lint أخرى. مشكلة Lint واحدة في المجموع.
       *[other] لا مشكلة Lint أخرى. { $count } مشكلات Lint في المجموع.
    }
lint-no-earlier =
    { $n ->
        [one] لا مشكلة Lint أقدم. مشكلة Lint واحدة في المجموع.
       *[other] لا مشكلة Lint أقدم. { $count } مشكلات Lint في المجموع.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
common-line = السطر { $line }.

## التدقيق النحوي (Harper). $message هي رسالة Harper نفسها، بالإنجليزية.

# $words are the words the problem is about.
grammar-said = القواعد: { $message } الكلمات: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = الإصلاح: إزالتها.
grammar-fix = الإصلاح: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = إزالة الكلمات
grammar-none = لا مشكلات نحوية موجودة.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] لا مشكلة نحوية أخرى. مشكلة نحوية واحدة في المجموع.
       *[other] لا مشكلة نحوية أخرى. { $count } مشكلات نحوية في المجموع.
    }
grammar-no-earlier =
    { $n ->
        [one] لا مشكلة نحوية أقدم. مشكلة نحوية واحدة في المجموع.
       *[other] لا مشكلة نحوية أقدم. { $count } مشكلات نحوية في المجموع.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } يسرد الإصلاحات.
# Added at high verbosity.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } لا إصلاح لعرضه.
grammar-fixes =
    { $n ->
        [one] { $words }: إصلاح واحد.
        [two] { $words }: إصلاحان.
        [few] { $words }: { $n } إصلاحات.
        [many] { $words }: { $n } إصلاحًا.
       *[other] { $words }: { $n } إصلاح.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: إصلاح واحد. Enter لإجراء التغيير.
        [two] { $words }: إصلاحان. Enter لإجراء التغيير.
        [few] { $words }: { $n } إصلاحات. Enter لإجراء التغيير.
        [many] { $words }: { $n } إصلاحًا. Enter لإجراء التغيير.
       *[other] { $words }: { $n } إصلاح. Enter لإجراء التغيير.
    }
common-left-as-is = تُركت كما هي.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. فعّل وضع التحرير بـ{ $key } لتغيير النص.
grammar-removed = أُزيلت.
grammar-changed = غُيِّرت إلى { $fix }.
grammar-change-failed = تعذّر تغيير النص: { $error } لم يتغيّر شيء.

## التدقيق الإملائي.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = فاصلة عليا
spell-not-available = التدقيق الإملائي غير متوفر: هذه النسخة بلا قائمة كلمات.
spell-none-found = لا أخطاء إملائية موجودة.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] لا خطأ إملائي آخر. خطأ إملائي محتمل واحد في المجموع.
       *[other] لا خطأ إملائي آخر. { $count } أخطاء إملائية محتملة في المجموع.
    }
spell-no-earlier =
    { $n ->
        [one] لا خطأ إملائي أقدم. خطأ إملائي محتمل واحد في المجموع.
       *[other] لا خطأ إملائي أقدم. { $count } أخطاء إملائية محتملة في المجموع.
    }
# Added at high verbosity.
spell-no-misspelled-word = لا كلمة خطأ إملائيًا عند المؤشر.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: لا اقتراحات.
        [one] { $word }: اقتراح واحد.
        [two] { $word }: اقتراحان.
        [few] { $word }: { $n } اقتراحات.
        [many] { $word }: { $n } اقتراحًا.
       *[other] { $word }: { $n } اقتراح.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: لا اقتراحات. Enter لاستبدال الكلمة.
        [one] { $word }: اقتراح واحد. Enter لاستبدال الكلمة.
        [two] { $word }: اقتراحان. Enter لاستبدال الكلمة.
        [few] { $word }: { $n } اقتراحات. Enter لاستبدال الكلمة.
        [many] { $word }: { $n } اقتراحًا. Enter لاستبدال الكلمة.
       *[other] { $word }: { $n } اقتراح. Enter لاستبدال الكلمة.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. فعّل وضع التحرير بـ{ $key } لتغيير النص.
spell-replaced = استُبدلت بـ{ $word }.
spell-replace-failed = تعذّر الاستبدال: { $error } لم تتغيّر الكلمة.
spell-added-for-session = أُضيفت { $word } إلى قائمة كلماتك لهذه الجلسة.
spell-added = أُضيفت { $word } إلى قائمة كلماتك.
spell-save-failed = تعذّر حفظ قائمة كلماتك: { $error } تبقى الكلمة معروفة حتى تخرج.
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] لا أخطاء إملائية.
        [one] خطأ إملائي محتمل واحد.
       *[other] { $count } أخطاء إملائية محتملة.
    }
