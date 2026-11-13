### Live documents, live sources, and the broker.

## Live documents (beta 5): settings, and live sources' passwords. $source
## is a live source's name; $reason says why the credential store failed.
section-live = المستندات الحية
prompt-live-password = كلمة مرور { $source }، Enter للإلغاء
prompt-live-password-any = كلمة مرور المصدر الحي، Enter للإلغاء
live-source-unknown = لا مصدر حيًا باسم { $source }. راجع المصادر في settings.toml.
live-password-no-store = لا مخزن لبيانات الاعتماد في هذا الإصدار: لا يستطيع { $source } الدخول بكلمة مرور. استخدم حزمة إصدار.
live-password-invalid = ليست كلمة مرور: حتى 1024 حرفًا، دون فواصل أسطر أو علامات جدولة.
live-password-kept = حُفظت كلمة مرور { $source } في مخزن بيانات الاعتماد في النظام.
live-password-not-kept = لم تُحفظ كلمة مرور { $source }: { $reason }. تُستخدم في هذه الجلسة فقط.
live-password-forgotten = نُسيت كلمة مرور { $source }.
live-password-none = لا كلمة مرور محفوظة لـ { $source }.
live-password-not-forgotten = لم تُنسَ كلمة مرور { $source }: { $reason }. حاول مجددًا، أو احذفها من مخزن بيانات الاعتماد في النظام.

## Beta 5: live documents (a log fed by a live source, such as an MQTT
## broker). $name is the source's name, $topic an MQTT topic, $path and
## $folder file paths, $line a log entry as one line.
live-log-title = سجل مباشر
live-log-empty = لا توجد إدخالات بعد.
live-entry-empty = إدخال فارغ.
live-field-callsign = رمز النداء
live-field-name = الاسم
live-field-location = الموقع
live-field-status = الحالة
live-field-traffic = الرسائل
live-field-notes = ملاحظات
live-field-time = الوقت
live-traffic = رسائل
live-yes = نعم
live-no = لا
live-read-only = هذا السجل المباشر للقراءة فقط؛ لا يمكن تحريره.
live-not-open = لا يوجد سجل مباشر مفتوح.
live-view-list = عرض القائمة.
live-view-table = عرض الجدول.
live-newest-none = لا توجد في السجل إدخالات بعد.
live-paused = تم إيقاف التحديثات مؤقتًا. تنتظر الإدخالات الجديدة حتى تستأنف.
live-paused-status = التحديثات متوقفة مؤقتًا
live-stopped = توقف { $name }. يبقى السجل مفتوحًا للقراءة فقط.
live-log-saved = تم حفظ السجل باسم { $name } في { $folder }.
live-log-save-failed = تعذر حفظ السجل المباشر: { $error } تُعاد المحاولة بعد 10 ثوانٍ. السجل: { $path }.
live-log-empty-not-saved = لا توجد في السجل إدخالات بعد، لذا لم يُحفظ شيء.
live-log-nowhere = لا يوجد مجلد لحفظ السجل المباشر فيه.
live-log-saved-already = فُتح هذا السجل من { $path }؛ وهو محفوظ بالفعل.
live-binary = أُغفل، ليس نصًا: { $len } بايت على { $topic }.
live-too-large = أُغفل، كبير جدًا: { $len } بايت على { $topic }.
live-full = السجل ممتلئ عند { $max } إدخال؛ أُغفلت الإدخالات الأحدث.
live-connecting = جارٍ الاتصال بـ { $name }.
live-reconnecting = إعادة الاتصال بـ { $name }، المحاولة { $attempt }.
live-connected = متصل بـ { $name }، مشفر.
live-connected-tailscale = متصل بـ { $name } على عنوان ضمن نطاق Tailscale. إن كان هذا Tailscale فالوصلة مشفرة.
live-connected-plain = متصل بـ { $name }. الاتصال غير مشفر.
live-disconnected = انقطع الاتصال بـ { $name }: { $reason }
live-quiet = لا توجد رسائل بعد على { $topic }. إذا استمر ذلك، فتحقق من أن الوسيط يسمح لهذا الحساب بالقراءة.
live-unread =
    { $count ->
        [0] لا شيء غير مقروء
       *[other] { $count } غير مقروء
    }
live-entries =
    { $count ->
        [one] إدخال واحد
       *[other] { $count } إدخالات
    }
live-resumed =
    { $count ->
        [0] استؤنفت التحديثات.
       *[other] استؤنفت التحديثات، وطُبق { $count } تحديث.
    }
live-resumed-dropped = استؤنفت التحديثات، وطُبق { $count }؛ وأُغفل { $dropped } أقدم أثناء الإيقاف المؤقت.

## Beta 5: live sources' commands, the set-up steps, and new entries said.
## $name is a source's name, $topic an MQTT topic, $line an entry as one
## line, $reason a sentence in words.
menu-live = المستندات المباشرة
menu-live-reading = القراءة
menu-live-broker = وسيطك
menu-live-invite = دعوة الآخرين
menu-live-publish = النشر
prompt-live-setup = إعداد مصدر مباشر
prompt-save-live-source-file = حفظ المصدر المباشر في ملف
live-failed = تعذر الاتصال بـ { $name }: { $reason } لا مزيد من المحاولات؛ أصلح ذلك ثم افتح المصدر مرة أخرى.
live-topic-refused = رُفض الموضوع: { $topic }. لا يحق لهذا الحساب قراءته.
live-skipped =
    { $count ->
        [one] أُغفلت رسالة واحدة: وصلت أسرع من أن تُعرض.
        [two] أُغفلت رسالتان: وصلتا أسرع من أن تُعرضا.
        [few] أُغفلت { $count } رسائل: وصلت أسرع من أن تُعرض.
        [many] أُغفلت { $count } رسالة: وصلت أسرع من أن تُعرض.
       *[other] أُغفلت { $count } رسالة: وصلت أسرع من أن تُعرض.
    }
live-new-entries = { $line } { $count } إدخالات جديدة.
live-unread-newest = { $count }. الأحدث: { $line }
live-not-in-build = المصادر المباشرة غير متاحة في هذا الإصدار من { -brand }.
live-speak-on = نطق الإدخالات الجديدة: مشغّل.
live-speak-off = نطق الإدخالات الجديدة: متوقف.
live-none-set-up = لا توجد مصادر مباشرة مُعدّة. يضيف «إعداد مصدر مباشر» مصدرًا.
live-open-title = المصادر المباشرة
live-source-item = { $name }، { $host }، { $topic }
live-forget-title = نسيان كلمة مرور أي مصدر مباشر؟
live-no-passwords = لا يسجل أي مصدر مباشر الدخول باسم مستخدم، لذا لا توجد كلمة مرور محفوظة.
live-not-receiving = لا يستقبل أي مصدر مباشر.
live-cannot-open = تعذر فتح المصدر المباشر. { $problems }
live-not-opened = لم يُفتح { $name }: لا توجد كلمة مرور.
live-setup-host-needed = يلزم اسم مضيف أو عنوان، دون مسافات.
live-setup-ca-file-needed = يحتاج هذا التشفير إلى ملف جهة إصدار الشهادات.
live-setup-ca-file-missing = لا يوجد ملف في { $path }.
live-setup-port-bad = المنفذ رقم من 1 إلى 65535.
live-setup-topic-needed = يلزم موضوع.
live-setup-saved = حُفظ المصدر المباشر { $name }.
live-setup-not-saved = لم يُحفظ المصدر المباشر { $name }: { $error } تحقق من إمكانية الكتابة في مجلد الإعدادات، ثم أعد إعداده.
live-setup-offer-title = فتح { $name } الآن؟
live-setup-open-now = فتحه الآن
live-setup-later = ليس الآن
live-setup-not-now = لم يُفتح. «فتح مصدر مباشر» يفتحه لاحقًا.
live-form-intro-new = إعداد مصدر مباشر. Tab للتنقل بين الحقول، Enter للحفظ، Escape للإلغاء.
live-form-intro-edit = تعديل { $name }. Tab للتنقل بين الحقول، Enter للحفظ، Escape للإلغاء.
live-form-intro-file = مصدر مباشر من { $file }. راجع كل حقل: Tab للتنقل، Enter لحفظه مصدرًا جديدًا، Escape للإلغاء.
live-form-name = الاسم
live-form-host = مضيف الوسيط
live-form-tls = التشفير
live-form-ca-file = ملف جهة الإصدار
live-form-port = المنفذ
live-form-topic = الموضوع
live-form-username = اسم المستخدم
live-form-view = العرض
live-form-choice-label = { $label }، { $n } من { $total }، سهم لأعلى أو لأسفل للتغيير
live-form-choice-field = { $label }: { $value }. سهم لأعلى أو لأسفل يغيّره.
live-form-tls-off = متوقف
live-form-tls-system = الجذور المرفقة
live-form-tls-ca-file = ملف جهة الإصدار
live-form-tls-off-about = متوقف: غير مشفر، المنفذ 1883؛ مناسب عبر Tailscale.
live-form-tls-system-about = الجذور المرفقة: TLS، يُتحقق منه بجهات إصدار الشهادات المرفقة مع { -brand }، المنفذ 8883.
live-form-tls-ca-file-about = ملف جهة الإصدار: TLS، يُتحقق منه بملف جهة إصدار شهادات تحدده، المنفذ 8883.
live-form-view-log = السجل
live-form-view-values = القيم
live-form-view-log-about = السجل: إدخال واحد لكل رسالة.
live-form-view-values-about = القيم: سطر واحد لكل حقل، يُحدَّث في مكانه.
live-form-name-needed = الاسم مطلوب.
live-form-not-a-choice = { $label } واحد من: { $choices }. سهم لأعلى أو لأسفل يغيّره.
live-form-saved-beside = حُفظ المصدر المباشر { $name }، بجانب مصدر آخر بالاسم نفسه.
live-form-unchanged = لم يتغير { $name }.
live-form-cancelled = أُلغي. لم يُحفظ أي مصدر مباشر.
live-form-cancelled-edit = أُلغي. لم يتغير { $name }.
live-edit-title = تعديل أي مصدر مباشر؟
live-save-file-title = حفظ أي مصدر مباشر في ملف؟
live-save-file-done = حُفظ { $name } باسم { $path }، دون كلمة مرور.
live-save-file-failed = لم يُحفظ { $name } في ملف: { $error } اختر مجلدًا آخر وحاول مرة أخرى.
live-file-not-read = لم يُفتح { $file } مصدرًا مباشرًا: { $reason }
live-file-too-large = حجمه أكبر من { $max } بايت.
live-file-not-text = ليس نصًا.
live-file-not-toml = ليس TOML صالحًا في السطر { $line }.
live-file-left-out = أُغفل من الملف: { $items }.
live-file-note-password = كلمة مرور، لا يقرؤها { -brand } من ملف أبدًا؛ تُطلب بعد الحفظ
live-file-note-unknown = المفتاح غير المعروف { $key }
live-file-note-too-many = كل شيء عدا أول { $max } من { $key }
live-values-title = قيم مباشرة
live-values-empty = لا توجد قيم بعد.
live-value-none = لا قيمة
live-value-stale = لا قيمة حديثة
live-values-more = { $count } أخرى
live-values-count =
    { $count ->
        [one] حقل واحد
       *[other] { $count } حقول
    }
live-values-no-table = لا جدول في عرض القيم: كل حقل سطر واحد.
live-values-no-log = لا يحتفظ عرض القيم بسجل للحفظ.
live-values-full = يتسع عرض القيم لـ { $max } حقل على الأكثر؛ يتم تجاهل الحقول الأخرى.
live-values-resumed =
    { $count ->
        [0] استؤنفت التحديثات. لم يتغير أي حقل.
       *[other] استؤنفت التحديثات، وتغير { $count } حقل.
    }
live-field-none = لا يوجد حقل في هذا السطر.
live-field-show-on = تُعرض تغييرات { $field }.
live-field-show-off = لم تعد تغييرات { $field } تُعرض.
live-field-speak-on = تُنطق تغييرات { $field }.
live-field-speak-off = لم تعد تغييرات { $field } تُنطق.
live-state-connecting = جارٍ الاتصال
live-state-reconnecting = جارٍ إعادة الاتصال
live-state-encrypted = متصل، مشفّر
live-state-tailscale = متصل، نطاق Tailscale
live-state-plain = متصل، غير مشفّر
live-state-lost = انقطع الاتصال

## Beta 5, phase 5b: the broker in the app, sharing a document live,
## check-ins, pinned keys and join codes. $name is a document or live
## source; $rev a revision number; $topic an MQTT topic; $code and $pin
## are in groups of four; $reason is a sentence in words.
prompt-live-check-in = تسجيل حضور
live-form-pin = البصمة
live-form-join-code = رمز الانضمام
live-form-tls-pinned = مفتاح مثبت
live-form-tls-pinned-about = مفتاح مثبت: TLS، يثق فقط بمفتاح الوسيط الذي تكتب بصمته كما يطبعها وسيط { -brand }، المنفذ 8883.
live-form-tls-join = رمز الانضمام
live-form-tls-join-about = رمز الانضمام: TLS بالمفتاح الذي يحدده الرمز، وتسجيل الدخول بالرمز، المنفذ 8883.
live-form-pin-needed = البصمة مطلوبة: يطبعها الوسيط بعد "Certificate pin".
live-form-pin-bad = ليست بصمة: لها 52 حرفًا كما يطبعها الوسيط، أو 12 من رمز انضمام، في مجموعات من أربعة.
live-form-join-needed = رمز الانضمام مطلوب: 16 حرفًا في أربع مجموعات من أربعة.
live-form-join-bad = ليس رمز انضمام: له 16 حرفًا ورقمًا في أربع مجموعات من أربعة.
live-file-note-join-code = رمز انضمام، لا يقرؤه { -brand } من ملف أبدًا؛ اطلب الرمز نفسه
live-pin-not-read = لا يمكن فتح { $name }: بصمته ليست بصمة. حرر المصدر واكتب البصمة من جديد.
live-document-shown = { $name }، المراجعة { $rev }، منشور مباشرة.
live-document-updated = حُدّث { $name }، المراجعة { $rev }.
live-document-revision = المراجعة { $rev }
live-document-entries-left-out = تُترك إدخالات السجل من هذا المصدر: إنه يعرض مستندًا منشورًا.
live-document-one-view = هذا مستند منشور: لا إدخالات له ولا جدول ولا سجل.
live-broker-status =
    { $count ->
        [one] الوسيط يعمل، عميل واحد
       *[other] الوسيط يعمل، { $count } عملاء
    }
live-share-status = نشر، المراجعة { $rev }
live-broker-already = الوسيط يعمل بالفعل.
live-broker-no-folder = لا مجلد بيانات لمفتاحه وحساباته.
live-broker-not-started = لم يبدأ الوسيط: { $reason }
live-broker-nobody = لا حسابات ولا رمز انضمام، فلا يمكن لأحد تسجيل الدخول. أضف حسابًا بالأمر tw broker account add، أو شغّل رمز الانضمام.
live-broker-bad-address = { $address } ليس عنوانًا للاستماع. أعطِ عنوان IP مثل 127.0.0.1 أو 100.101.1.2، مع منفذ أو دونه.
live-broker-started-local = الوسيط يعمل: { $addresses }، TLS، هذا الحاسوب فقط.
live-broker-started = الوسيط يعمل: { $addresses }، TLS.
live-broker-started-plain-local = الوسيط يعمل: { $addresses }، غير مشفر، هذا الحاسوب فقط.
live-broker-started-plain = الوسيط يعمل: { $addresses }، غير مشفر.
live-broker-plain-warning = غير مشفر: يستطيع أي شخص على هذه الشبكة قراءة الرسائل وكلمات المرور. استخدم TLS، أو هذا الحاسوب أو Tailscale فقط.
live-broker-join-code = رمز الانضمام: { $code }.
live-broker-pin = بصمة الشهادة: { $pin }.
live-broker-not-running = لا وسيط يعمل. تشغيل وسيطك يبدأ واحدًا.
live-join-code-title = رمز الانضمام
live-pin-title = بصمة الشهادة
live-broker-no-join-code = لا رمز انضمام: يعمل الوسيط بالحسابات. يثبّت القراء مفتاحه.
live-broker-no-join-code-plain = لا رمز انضمام ولا بصمة: الوسيط غير مشفر.
live-code-group = المجموعة { $n } من { $total }: { $group }
live-join-code-copied = نُسخ رمز الانضمام.
live-join-link-copied = نُسخ رابط الانضمام.
live-join-code-pasted = أُلصق رمز الانضمام.
live-join-link-filled = عُبئ من رابط الانضمام: المضيف والمنفذ والرمز والموضوع.
live-join-link-filled-no-topic = عُبئ من رابط الانضمام: المضيف والمنفذ والرمز. اكتب الموضوع.
live-join-link-bad = ليس رابط انضمام: { $reason }
live-form-intro-link = الانضمام برابط. الصق رابط الانضمام في أي حقل؛ Enter للانضمام، Escape للإلغاء.
live-form-intro-link-clipboard = الانضمام بالرابط الذي في الحافظة. Enter للانضمام، Escape للإلغاء.
live-qr-title = رمز QR للانضمام
live-qr-warning = يستطيع الانضمام كل من يرى هذا الرمز.
live-qr-image = رمز QR للانضمام إلى { $host }
live-qr-save = حفظ رمز QR
live-qr-draw = رسم رمز QR بالحروف
live-qr-drawing-title = رمز QR للانضمام بالحروف
live-qr-drawing-warning = تلي ذلك حروف كتل لكاميرا؛ يقرؤها قارئ الشاشة ضجيجًا. Escape يغلق.
live-qr-failed = لا رمز QR: { $reason }
live-qr-save-type = لم يُحفظ رمز QR: يجب أن ينتهي اسم الملف بـ .png أو .svg.
live-qr-save-failed = لم يُحفظ رمز QR: { $error } اختر مجلدًا أو اسمًا آخر.
live-qr-saved = حُفظ رمز QR: { $path }
live-qr-intro-clipboard = الانضمام من رمز QR في الحافظة. Enter ينضم، Escape يلغي.
live-qr-intro-file = الانضمام من رمز QR في { $file }. Enter ينضم، Escape يلغي.
live-qr-clipboard-no-link = لا رابط انضمام في صورة الحافظة: { $reason } اختر ملف صورة.
live-qr-no-link = لا رابط انضمام: { $reason }
live-qr-other-text = يحمل رمز QR شيئًا آخر.
live-qr-not-found = لم يُعثر على رمز QR في الصورة.
live-qr-unreadable = عُثر على رمز QR لكن تعذرت قراءته. جرّب صورة أوضح أو أكبر.
live-qr-too-large = ملف الصورة كبير جدًا.
live-qr-too-long = رابط الانضمام لا يتسع في رمز QR.
live-qr-image-error = تعذرت قراءة الصورة: { $error } جرّب ملف PNG أو JPEG.
live-qr-reading = جارٍ قراءة صورة QR.
prompt-save-join-qr = حفظ رمز QR في ملف (.png أو .svg)
prompt-join-qr-image = ملف صورة فيه رمز QR
chooser-qr-files = صور PNG وSVG
live-field-menu-title = حقل رمز الانضمام
live-field-hide-join-code = إخفاء رمز الانضمام
live-field-empty = حقل رمز الانضمام فارغ.
gui-qr-link = رابط الانضمام
live-share-stopped-with-broker = توقف النشر مع الوسيط.
live-broker-stopped = توقف الوسيط. فُصل كل القراء.
live-send-lost = توقف الإرسال قبل أن يكتمل.
live-broker-joined = انضم: { $user }.
live-broker-left = غادر: { $user }.
live-broker-refused = رُفض تسجيل الدخول: { $user }، من { $address }.
live-broker-event = الوسيط: { $event }
live-broker-join-off = رمز الانضمام معطّل: رموز خاطئة كثيرة. أعد تشغيل الوسيط للحصول على رمز جديد.
live-send-too-large = حجم الرسالة { $len } بايت؛ والحد { $max }.
live-send-no-password = لم يُرسل: لا كلمة مرور محفوظة لـ { $name }. افتحه مرة لكتابة كلمة المرور.
live-checkin-sent = نُشر: { $entry }، على { $topic }.
live-share-sent = نُشر: { $name }، المراجعة { $rev }، على { $topic }.
live-checkin-not-sent = لم يُنشر تسجيل الحضور: { $reason }
live-share-not-sent = لم يُنشر { $name }: { $reason }
live-share-already = { $name } منشور بالفعل على { $topic }. أوقف النشر أولًا.
live-share-no-document = افتح مستندًا لنشره مباشرة.
live-share-live-document = لا يمكن نشر مستند مباشر مرة أخرى. افتح ملف مستند لنشره.
live-share-through-title = النشر عبر أي مصدر مباشر؟
live-send-nowhere = لا شيء للنشر عبره. شغّل وسيطك أو أعدّ مصدرًا مباشرًا يمكنه النشر.
live-share-none = لا مستند يُنشر.
live-share-stopped = توقف نشر { $name }. يحتفظ القراء بالمراجعة { $rev }.
live-checkin-through-title = نشر تسجيل الحضور عبر أي مصدر مباشر؟
live-checkin-to-broker = وسيط هذا الحاسوب
live-checkin-intro = تسجيل حضور إلى { $to }، على { $topic }. Tab ينتقل بين الحقول، وEnter ينشر، وEscape يلغي.
live-checkin-empty = املأ حقلًا واحدًا على الأقل.
live-checkin-cancelled = أُلغي. لم يُنشر أي تسجيل حضور.
