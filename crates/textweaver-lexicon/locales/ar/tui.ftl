### The terminal reader.

## بدء تشغيل قارئ الطرفية.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = محرك الكلام { $wanted } غير متوفر؛ يُستخدم { $backend }.
tui-setup-speech-failed = تعذّر بدء الكلام ({ $error })؛ التشغيل بصمت.
speech-engine-fallback = تعذّر بدء { $failed }؛ يتحدث { $engine } بدلًا منه.
speech-engine-fallback-silent = تعذّر بدء { $failed } ولا يتوفر محرك كلام آخر؛ يبقى { -brand } صامتًا.
tui-setup-cannot-save = تعذّر حفظ الإعدادات أو المواضع: { $error } القراءة تعمل. تضيع التغييرات عند الخروج.
tui-setup-keymap-ignored = جرى تجاهل ملف خريطة المفاتيح: { $error } تُستخدم المفاتيح الافتراضية. أصلح الملف، ثم أعد التشغيل.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = مرحبًا بك في { -brand }. { $open } يفتح مستندًا. { $play } يبدأ القراءة ويوقفها مؤقتًا، و{ $stop } يوقفها. { $palette } يسرد كل الأوامر. { $help } يفتح المساعدة.
gui-setup-welcome-menus = مرحبًا بك في { -brand }. { $open } يفتح مستندًا. { $play } يقرأ ويوقف مؤقتًا، و{ $stop } يوقف. { $palette } يسرد كل الأوامر، وAlt أو { $menu } القوائم. { $help } يفتح المساعدة.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = لا يوجد مستند مفتوح. اضغط { $open } لفتح واحد، { $new } لواحد جديد، أو { $help } للمساعدة.

## إطلاق قارئ الطرفية.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = تعذّر فتح { $name }: { $error }

## النسخ في قارئ الطرفية.

tui-clip-system = نُسخ باستخدام حافظة النظام، لأن هذه الطرفية لا تقبل نصًا منسوخًا.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = تعذّر النسخ بحافظة النظام: { $error }. أُرسل إلى الطرفية بدلًا من ذلك.
tui-clip-not-available = حافظة النظام غير متوفرة
tui-clip-not-built = هذه النسخة بلا حافظة نظام

## شاشة قارئ الطرفية.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = لا يوجد مستند
# The screen without a document. $keys names the keys for the action.
tui-empty-open = فتح واحد: { $keys }.
tui-empty-help = المساعدة: { $keys }.
tui-empty-quit = إنهاء: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y نعم  n أو a لا  { $escape } لا
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = تشغيل
tui-hint-sentence = جملة
tui-hint-faster = أسرع
tui-hint-slower = أبطأ
tui-hint-close-rsvp = إغلاق العرض السريع
tui-hint-quit = إنهاء
tui-hint-save = حفظ
tui-hint-finish = إنهاء
tui-hint-undo = تراجع
tui-hint-bold = عريض
tui-hint-heading = عنوان
tui-hint-commands = أوامر
tui-hint-next-line = السطر التالي
tui-hint-previous-line = السطر السابق
tui-hint-again = مجددًا
tui-hint-read-on = متابعة القراءة
tui-hint-leave = مغادرة
tui-hint-find = بحث
tui-hint-mark = وضع علامة
tui-hint-lines = أسطر
tui-hint-keys = مفاتيح
tui-hint-choose = اختيار
tui-hint-close = إغلاق
tui-hint-back = رجوع
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } من { $count }، { $title }
