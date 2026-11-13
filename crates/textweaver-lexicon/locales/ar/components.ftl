### Optional components and updates.

## المكونات الاختيارية.
component-feature-dictation = الإملاء
component-feature-ocr = قراءة الصفحات الممسوحة
component-feature-reading-font = خط للقراءة
component-feature-voice = صوت للنطق
component-feature-audio-export = تصدير M4B وMP4
component-feature-braille = ملفات برايل كنص مطبوع، وبرايل المختصر
component-feature-pandoc = صيغ مستندات أخرى
component-state-installed = مثبّت
component-state-not-installed = غير مثبّت
component-state-partial = مثبّت جزئيًا
component-state-damaged = تالف
component-state-downloading = قيد التنزيل
component-state-installer-ready = نُزّل برنامج التثبيت
components-title = المكونات الاختيارية
components-intro =
    { $n ->
        [one] مكوّن اختياري واحد. Enter للإجراءات.
       *[other] { $n } مكونات اختيارية. Enter للإجراءات.
    }
components-item = { $title }: { $state }، { $size }، الترخيص { $license }، من أجل { $features }
components-actions-intro = { $title }: { $state }.
components-action-download = تنزيل، { $size }
components-action-verify = التحقق من الملفات
components-action-remove = إزالة
components-action-install-zip = التثبيت من ملف zip…
components-action-install-folder = التثبيت من مجلد…
components-install-purpose = تثبيت المكوّن من هنا
component-question = تنزيل { $title }، { $size }، الترخيص { $license }؟ y أو n
component-remove-question = إزالة { $title }؟ y أو n
component-downloading = جارٍ التنزيل. Escape يوقفه.
component-installing = جارٍ التثبيت من الملف.
component-verifying = جارٍ التحقق من الملفات.
component-progress = تم تنزيل { $percent } بالمئة.
component-ready = جاهز: { $title }.
component-verified = الملفات سليمة: { $title }.
component-verify-failed =
    { $n ->
        [one] ملف واحد غير مطابق: { $files }.
       *[other] { $n } ملفات غير مطابقة: { $files }.
    }
component-removed = أُزيل: { $title }.
component-refused =
    { $n ->
        [one] ملف واحد مستبعد: { $files }.
       *[other] { $n } ملفات مستبعدة: { $files }.
    }
component-already = مثبّت بالفعل: { $title }.
component-not-there = غير مثبّت: { $title }.
component-declined = لم يُنزَّل شيء.
component-not-in-build = لا تنزيلات في هذا الإصدار.
component-no-folder = لا مجلد بيانات لحفظه.
component-error-fetch = لم يُنزَّل: فشل المصدر.
component-error-size = لم يُثبَّت: حجم ملف خاطئ.
component-error-hash = لم يُثبَّت: ملف لم يتطابق. نزّله مرة أخرى أو تحقق من المصدر.
component-error-missing = لم يُثبَّت: ملف مفقود.
component-error-cancelled = توقف التنزيل؛ يُستأنف لاحقًا.
component-error-busy = يجري تنزيل مكوّن بالفعل.
component-error-no-source = لم يُنزَّل: لا عنوان له.
component-error-name = مرفوض: اسم غير بسيط.
component-error-manifest = قائمة مكونات غير مقروءة.
component-error-io = لم يُثبَّت: تعذرت الكتابة.
component-error-unpack = لم يُثبَّت: تعذّر فك الأرشيف.
component-launch-question = تشغيل برنامج التثبيت: { $title }، الإصدار { $version }، الترخيص { $license }؟ يسأل النظام بعدها. y أو n
component-version-unknown = غير مذكور
component-launched = بدأ برنامج التثبيت: { $title }. اتبع أسئلة النظام.
component-launch-failed = لم يبدأ برنامج التثبيت: { $reason }
component-launch-declined = لم يُشغَّل؛ احتُفظ ببرنامج التثبيت.
components-token-invalid = ليس رمزًا. تحقق منه والصقه مرة أخرى.
components-token-kept = حُفظ الرمز في مخزن بيانات الاعتماد في النظام.
components-token-not-kept = استُخدم الرمز ولم يُحفظ: { $reason }.
components-token-forgotten = نُسي الرمز.
components-token-none = لم يكن هناك رمز محفوظ.
components-token-not-forgotten = لم يُنسَ: { $reason }.
components-token-gh-still = ما زال تسجيل دخول GitHub CLI مستخدمًا؛ ينهيه gh auth logout.
components-chooser-title = المكونات الاختيارية
components-chooser-intro = إضافات اختيارية، لم يُختر شيء. المسافة تختار؛ تنزيل المختار يجلبها؛ Escape يتخطى.
components-chooser-item = { $mark }: { $title }، من أجل { $features }، { $size }، الترخيص { $license }
components-chosen = مختار
components-not-chosen = غير مختار
components-chooser-download = تنزيل المختار
components-chooser-skip = تخطٍّ الآن
components-chooser-skipped = تم التخطي؛ انظر إدارة المكونات.
components-chooser-none = لم يُختر شيء، ولم يُنزَّل شيء.
dictation-model-question = يحتاج الإملاء إلى نموذج Whisper، { $size }، الترخيص { $license }. تنزيله الآن؟ y أو n
dictation-model-declined = لا نموذج، فلا إملاء الآن.
helper-question = يحتاج { $need } إلى { $title }، { $size }، الترخيص { $license }. تنزيله الآن؟ y أو n
helper-need-audio = تصدير M4B وMP4
helper-need-brf = قراءة برايل كنص مطبوع
helper-run-command = للحصول على { $title }، شغّل: { $command }
dictation-model-not-in-build = لا نموذج ولا تنزيلات في هذا الإصدار.
dictation-model-file-missing = ينقص النموذج { $file }.
dictation-model-damaged = نموذج الإملاء تالف: { $file }.
dictation-model-no-folder = لا مجلد للنموذج: { $dir }.

## إعدادات المكونات الاختيارية.

## التحديثات (B1-u1).
components-chooser-updates = { $mark }: البحث عن تحديثات تلقائيًا، مرة يوميًا، من GitHub
update-checking = جارٍ البحث عن تحديثات.
update-none = لا تحديث: { $version } هو الأحدث.
update-found = تحديث متاح: textweaver { $version }، { $size }. تنزيله؟ y أو n
update-available-elsewhere = تحديث متاح: textweaver { $version }. حدّث هذه النسخة بالطريقة التي ثُبّتت بها.
update-declined = لم يُنزَّل. سيُعرض إصدار أحدث.
update-downloading = جارٍ تنزيل التحديث، { $size }.
update-progress = التحديث: تم تنزيل { $percent } بالمئة.
update-installed = تم التحقق من التحديث وتثبيته. أعد تشغيل textweaver لاستخدامه.
update-on-close = تم التحقق من التحديث. يُثبَّت عند إغلاق textweaver.
update-busy = البحث عن تحديثات جارٍ بالفعل.
update-not-in-build = لا تحديثات في هذا الإصدار.
update-automatic-question = البحث عن تحديثات تلقائيًا، مرة يوميًا؟ y أو n
update-automatic-on = التحديثات: يُبحث عنها مرة يوميًا.
update-automatic-off = التحديثات: لا يُبحث عنها. المساعدة، البحث عن تحديثات يبحث في أي وقت.
update-error-check = فشل البحث عن تحديثات: { $reason }
update-error-no-package = لا حزمة تحديث لهذا الحاسوب في { $version }.
update-error-no-checksum = رُفض التحديث: لا مجموع تحقق له. لم يتغير شيء.
update-error-mismatch = رُفض التحديث: مجموع التحقق لا يطابق. لم يتغير شيء.
update-error-cancelled = توقف تنزيل التحديث؛ يُستأنف في المرة القادمة.
update-error-download = لم يُنزَّل التحديث: { $reason }
update-error-install = لم يُثبَّت التحديث: { $reason }. لم يتغير شيء.
update-error-not-package = لم يُحدَّث: هذه النسخة ليست من حزمة إصدار.
section-updates = التحديثات والمكونات
