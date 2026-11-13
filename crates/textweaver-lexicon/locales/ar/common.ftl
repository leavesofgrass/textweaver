### Common words, parts of speech, define word, and names said inside other messages.

### رسائل واجهة textweaver باللغة العربية.
###
### بصيغة Fluent (https://projectfluent.org/)، في الجزء الذي يقرؤه
### textweaver-lexicon. المعرّفات والمتغيرات هي نفسها في locales/en، وأي رسالة
### غير موجودة هنا تُقال بالإنجليزية.

-brand = textweaver

## أقسام الكلام.

pos-noun = اسم
pos-verb = فعل
pos-adjective = صفة
pos-adverb = ظرف

## تعريف كلمة: المصادر.

source-glossary = مسردك
source-wordnet = قاموس WordNet الإنجليزي المفتوح
source-cmudict = قاموس CMU للنطق

## تعريف كلمة: قائمة المعاني.

# $word is the word looked up, $n the number of senses, $source a source-* message.
define-title =
    { $n ->
        [0] نطق { $word }، من { $source }
        [one] تعريف { $word }، معنى واحد، من { $source }
        [two] تعريفا { $word }، معنيان، من { $source }
        [few] تعريفات { $word }، { $n } معانٍ، من { $source }
        [many] تعريفات { $word }، { $n } معنى، من { $source }
       *[other] تعريفات { $word }، { $n } معنى، من { $source }
    }
# $lemma is the headword, $pos a pos-* message, $i the sense number, $n how many.
define-sense-head = { $lemma }، { $pos }، { $i } من { $n }
define-sense-head-nopos = { $lemma }، { $i } من { $n }
define-sense = { $head }: { $definition }.
define-example = مثال: { $text }.
define-synonyms = مرادفات: { $words }.
define-antonyms = العكس: { $words }.
define-kind-of = نوع من: { $words }.
# $say is a respelling such as RUN-ing, with the stressed syllable in capitals.
define-pronounced = يُنطق { $say }.
define-pronounced-or = يُنطق { $say }، أو { $other }.

## كلمات شائعة.

common-cancelled = أُلغي.
# Durations: $h hours, $m minutes, $s seconds.
duration-hours =
    { $h ->
        [one] ساعة واحدة
        [two] ساعتان
        [few] { $h } ساعات
        [many] { $h } ساعة
       *[other] { $h } ساعة
    } و{ $m ->
        [one] دقيقة واحدة
        [two] دقيقتان
        [few] { $m } دقائق
        [many] { $m } دقيقة
       *[other] { $m } دقيقة
    }
duration-minutes =
    { $m ->
        [one] دقيقة واحدة
        [two] دقيقتان
        [few] { $m } دقائق
        [many] { $m } دقيقة
       *[other] { $m } دقيقة
    } و{ $s ->
        [one] ثانية واحدة
        [two] ثانيتان
        [few] { $s } ثوانٍ
        [many] { $s } ثانية
       *[other] { $s } ثانية
    }
duration-seconds =
    { $s ->
        [one] ثانية واحدة
        [two] ثانيتان
        [few] { $s } ثوانٍ
        [many] { $s } ثانية
       *[other] { $s } ثانية
    }

## تعريف كلمة في القارئ.

# $title is define-title.
define-intro = { $title }. السهمان لأعلى ولأسفل يتنقلان بين المعاني، Enter لنسخ أحدها، Escape للإغلاق.
define-nothing-here = لا توجد كلمة عند المؤشر.
define-not-found = لم يُعثر على تعريف لـ{ $word }.
define-no-dictionary = ملف القاموس غير مثبَّت، لذا بُحث في مسردك فقط. دليل القراءة يوضح كيفية تثبيته.
define-dictionary-damaged = تعذّرت قراءة ملف القاموس: { $error } يُبحث في مسردك فقط.
define-glossary-problem = تعذّرت قراءة مسردك: { $error } يُبحث في القاموس فقط.
define-glossary-skipped =
    { $n ->
        [one] سطر واحد في مسردك بلا تعريف، فتم تخطّيه.
        [two] سطران في مسردك بلا تعريف، فتم تخطّيهما.
        [few] { $n } أسطر في مسردك بلا تعريف، فتم تخطّيها.
        [many] { $n } سطرًا في مسردك بلا تعريف، فتم تخطّيها.
       *[other] { $n } سطر في مسردك بلا تعريف، فتم تخطّيها.
    }
define-copied = نُسخ.

## أسماء البنية والوحدات والأوضاع (تُقال داخل رسائل أخرى).

kind-heading = عنوان
kind-paragraph = فقرة
kind-list-item = عنصر قائمة
kind-list = قائمة
kind-table = جدول
kind-row = صف
kind-cell = خلية
kind-link = رابط
kind-graphic = صورة
kind-code = شيفرة
kind-quote = اقتباس
kind-page = صفحة
kind-section = قسم
kind-bold = عريض
kind-italic = مائل
kind-underline = تسطير
kind-footnote = حاشية
kind-strikethrough = يتوسطه خط
kind-separator = فاصل
kind-math = رياضيات
unit-character = حرف
unit-word = كلمة
unit-sentence = جملة
unit-line = سطر
unit-paragraph = فقرة
unit-document = مستند
# $what is a kind-* noun, $unit its key.
unit-with-level = { $what } من المستوى { $level }
mode-browse = تصفح
mode-speech-cursor = مؤشّر الكلام
mode-edit = تحرير
mode-find = بحث
mode-command = أمر
mode-go-to = الانتقال إلى
mode-open = فتح
mode-prompt = إدخال
common-on = تشغيل
common-off = إيقاف

## لغة الواجهة. $language هو اسم اللغة بذاتها (Español)، $voice اسم صوت.

language-voice-changed = الصوت الآن { $voice }، للغة { $language }.
language-voice-kept = لا يوجد صوت للغة { $language } في محرك الكلام هذا، لذا يستمر { $voice } في التحدث.
language-list-title = اللغة
language-list-intro =
    { $n ->
        [one] اللغة، خيار واحد. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
        [two] اللغة، خياران. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
        [few] اللغة، { $n } خيارات. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
        [many] اللغة، { $n } خيارًا. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
       *[other] اللغة، { $n } خيار. السهمان لأعلى ولأسفل للتنقل، Enter للاختيار، Escape للإبقاء على اللغة.
    }

## الحروف والتحديدات، كما تُنطق.

# The name of a white-space character read on its own; $name is a fixed key.
text-char-name =
    { $name ->
        [space] مسافة
        [new-line] سطر جديد
        [tab] علامة جدولة
        [no-break-space] مسافة غير فاصلة
       *[white-space] مسافة بيضاء
    }
# Said after a selection grows ($change is selected) or shrinks (unselected); $text is the text or a character's name.
text-selection-change =
    { $change ->
        [selected] { $text } مُحدَّد
       *[unselected] { $text } غير مُحدَّد
    }

text-summary =
    { $change ->
        [selected] تم تحديد { $count } من الأحرف
        [unselected] أُلغي تحديد { $count } من الأحرف
        [copied] نُسخ { $count } من الأحرف
        [cut] قُصّ { $count } من الأحرف
       *[deleted] حُذف { $count } من الأحرف
    }
text-summary-range =
    { $change ->
        [selected] تم تحديد { $count } من الأحرف، من { $first } إلى { $last }
        [unselected] أُلغي تحديد { $count } من الأحرف، من { $first } إلى { $last }
        [copied] نُسخ { $count } من الأحرف، من { $first } إلى { $last }
        [cut] قُصّ { $count } من الأحرف، من { $first } إلى { $last }
       *[deleted] حُذف { $count } من الأحرف، من { $first } إلى { $last }
    }
voice-character-keys-on = الاختصارات أحادية المفتاح مفعّلة.
voice-character-keys-off = الاختصارات أحادية المفتاح معطّلة.
