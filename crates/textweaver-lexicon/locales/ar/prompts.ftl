### Prompts, questions, lists, and file choosers.

## طلبات الإدخال.

prompt-define-word = تعريف أي كلمة؟
prompt-profile-name = اسم لملف الإعدادات الجديد
prompt-profile-rename = اسم جديد لملف الإعدادات، Enter للإبقاء عليه
prompt-profiles-import = استيراد ملفات الإعدادات من ملف
prompt-profiles-export = تصدير ملفات الإعدادات إلى ملف، مثل textweaver-profiles.json

## الأسئلة والأجوبة. أبقِ على الحرفين y وn: فهما المفتاحان للإجابة.

common-press-y-or-n = اضغط y أو n.
common-kept = أُبقي عليه.
confirm-quit = إنهاء textweaver؟ y أو n
confirm-delete-note = حذف هذه الملاحظة أو التمييز؟ y أو n
notes-remove-highlight-question = إزالة هذا التمييز؟ y أو n
notes-delete-note-question = حذف هذه الملاحظة؟ y أو n
list-nothing-to-mark = لا شيء لتحديده في هذه القائمة.
notes-editing = تحرير ملاحظة: { $text }
# $key opens a document.
app-no-document-open = لا يوجد مستند مفتوح. اضغط { $key } لفتح واحد.
app-window-only = يعمل هذا الأمر في تطبيق textweaver.
app-terminal-only = يعمل هذا الأمر في قارئ الطرفية.
settings-save-failed = تعذّر حفظ الإعدادات: { $error } تبقى تغييراتك سارية حتى تخرج.
settings-outside-kept = تم الاحتفاظ بالإعدادات التي تغيّرت خارج textweaver.
edit-still-editing = ما زلت في وضع التحرير.
# The reading view inside edit mode (toggle_preview). $key is its key;
# $line the line the caret is back on.
edit-preview-on = المعاينة، للقراءة فقط: عرض القراءة في المكان نفسه. { $key } يعود إلى التحرير.
edit-preview-off = عودة إلى التحرير. { $line }
edit-preview-read-only = المعاينة للقراءة فقط. { $key } يعود إلى التحرير.
edit-preview-not-editing = تعمل المعاينة في وضع التحرير. { $key } يبدأ التحرير.
edit-preview-plain = النص العادي بلا تنسيق لمعاينته؛ يُقرأ كما هو.
edit-preview-failed = تعذر عرض المعاينة: { $error } تحقق من Markdown ثم حاول مرة أخرى.
goto-not-a-target = ليس هدف انتقال: { $text }. اكتب رقم سطر، أو نسبة مئوية مثل 50%، أو start، أو end.

## الطلبات: تُعرض التسمية وتُقال عند فتح الطلب.

prompt-find = بحث
prompt-go-to = الانتقال إلى سطر أو نسبة مئوية أو start أو end
prompt-open = فتح ملف
prompt-command = أمر
# $label is prompt-command.
prompt-command-palette-intro = { $label }. اكتب جزءًا من اسم؛ Tab لإكماله، والسهمان لأعلى ولأسفل لسرد التطابقات.
prompt-save-as = حفظ باسم
prompt-export-as = تصدير باسم
prompt-table-size = حجم الجدول، أعمدة × صفوف، مثل 3 × 2
prompt-image-path = ملف صورة
prompt-replace-find = استبدال، ابحث عن
prompt-replace-with = استبدل بـ
prompt-note = ملاحظة
prompt-edit-note = تحرير ملاحظة، Enter للإبقاء عليها
prompt-rename-bookmark = اسم جديد للإشارة المرجعية، Enter للإبقاء عليه
prompt-export-settings = تصدير الإعدادات إلى ملف، مثل textweaver-settings.json
prompt-import-settings = استيراد الإعدادات من ملف
prompt-citation-locator = رقم الصفحة أو موضع آخر، مثل 12 أو chapter 2؛ Enter لعدم تحديد شيء
prompt-reference-identifier = DOI أو ISBN لإضافته
prompt-import-references = استيراد المراجع من ملف
prompt-template-title = عنوان المستند الجديد
prompt-setting-value = القيمة الجديدة، Enter للإبقاء عليها

## إكمال المسارات تلقائيًا في الطلبات.

# $folder is the folder's full path.
pathc-no-folder = لا يوجد مجلد { $folder }.
# $prefix is what was typed after the last separator.
pathc-no-match = لا يوجد ملف أو مجلد يبدأ بـ{ $prefix }.
# The one name that matched; $kind is folder or file.
pathc-one =
    { $kind ->
        [folder] { $name }، مجلد
       *[file] { $name }، ملف
    }
# $n names matched; $names are the first few, joined with commas; $more is
# yes when more matched than are read out.
pathc-many =
    { $more ->
        [yes] { $n } تطابقات: { $names }، والمزيد.
       *[no] { $n } تطابقات: { $names }.
    }

## القوائم والطلبات المشتركة بين كل واجهة.

# The focused list item: $item is its text, $k its place, $n the number of items.
listmodel-item-position = { $k } من { $n }، { $item }
# $letter is the letter or digit typed.
listmodel-no-item-starts = لا عنصر يبدأ بـ{ $letter }.
listmodel-top-of-list = أعلى القائمة.
listmodel-end-of-list = نهاية القائمة.
listmodel-no-matching-commands = لا أوامر مطابقة.
listmodel-no-earlier-entries = لا مدخلات أقدم.
# Tab in the command palette: $n commands match (always more than one); $names lists the first few, joined by commas.
listmodel-command-matches = { $n } تطابقات: { $names }.
goto-word-start = start
goto-word-end = end

language-voices-loading = لا تزال قائمة الأصوات قيد التحميل، لذا يستمر الصوت الحالي في التحدث.

## منتقيات الملفات والمجلدات.
prompt-browse-hint = { $label }. { $key } للتصفح.
prompt-browse-file = اختر الملف: { $label }
prompt-browse-folder = اختر المجلد: { $label }
prompt-browse-filled = { $name } مختار. يؤكده Enter.
chooser-type-files = ملفات { $type }
chooser-image-title = إدراج صورة
chooser-images = الصور
chooser-references-title = استيراد المراجع
chooser-reference-files = ملفات المراجع
chooser-anki-title = استيراد بطاقات Anki
chooser-anki-files = حزم Anki
chooser-profiles-import-title = استيراد ملفات الإعدادات
chooser-profiles-export-title = تصدير ملفات الإعدادات
chooser-profile-files = ملفات تصدير الإعدادات
chooser-live-source-files = ملفات المصادر المباشرة
gui-folder-no-dialog = لم يُفتح منتقي المجلدات في النظام. اختر المجلد من هذه القائمة.
gui-prompt-browse-hint = يفتح { $key } متصفح الملفات.

## A secret field's Show choice: the show key, or the Show box in
## the textweaver app, shows a password, token or join code as typed.
prompt-show-hint = { $label }. { $key } يُظهره.
prompt-secret-hidden = مخفي، عدد الأحرف { $count }
prompt-show-join-code = إظهار رمز الانضمام
prompt-showing-join-code = رمز الانضمام ظاهر
prompt-hiding-join-code = رمز الانضمام مخفي
prompt-show-password = إظهار كلمة المرور
prompt-showing-password = كلمة المرور ظاهرة
prompt-hiding-password = كلمة المرور مخفية
prompt-show-token = إظهار الرمز المميز
prompt-showing-token = الرمز المميز ظاهر
prompt-hiding-token = الرمز المميز مخفي
gui-prompt-show-hint = { $key } أو مربع الإظهار يُظهر ما كُتب.
