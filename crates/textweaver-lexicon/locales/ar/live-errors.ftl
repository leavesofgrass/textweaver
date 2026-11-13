### Live documents: why a live source is down, and why a join link does not read.

## Why a live source is down (beta 6). Each is said after "Disconnected
## from NAME:" or "Cannot connect to NAME:". $target is the broker as
## host:port, $host its host, $path a CA file, $error and $detail the
## system's or the TLS library's own words, $packet an MQTT packet name
## such as PUBREC, $topic a topic filter. $why is one of the live-why-*
## or live-topic-problem-* pieces below.
live-down-bad-login = غير مصرّح: رفض الوسيط اسم المستخدم أو كلمة المرور. تحقق منهما في إعدادات المصدر.
live-down-login-refused = غير مصرّح: رفض الوسيط تسجيل الدخول. تحقق من اسم المستخدم وكلمة المرور.
live-down-protocol-version = رُفض: لا يقبل الوسيط MQTT 3.1.1.
live-down-client-id = رُفض: لم يقبل الوسيط معرّف العميل.
live-down-unavailable = تعذر الوصول: يقول الوسيط إنه غير متاح الآن.
live-down-unexpected-success = خطأ في البروتوكول: رمز نجاح غير متوقع.
live-down-tls-needed = يلزم TLS: رد { $target } بـ TLS. شغّل TLS لهذا المصدر.
live-down-cannot-connect = تعذر الوصول: لا اتصال بـ { $target }: { $why }.
live-down-connect-failed = تعذر الوصول: لا اتصال بـ { $target }.
live-down-handshake = فشل TLS: لم تكتمل المصافحة مع { $target }: { $why }.
live-down-lost-logging-in = انقطع الاتصال بـ { $target } أثناء تسجيل الدخول: { $why }.
live-down-lost-logging-in-tls = انقطع الاتصال بـ { $target } أثناء تسجيل الدخول: { $why }. إن كان هذا المنفذ يتوقع TLS، فشغّل TLS لهذا المصدر.
live-down-lost = انقطع الاتصال بـ { $target }: { $why }.
live-down-session-ended = انقطع الاتصال بـ { $target }: أنهى الوسيط الجلسة.
live-down-stopped-answering = انقطع الاتصال بـ { $target }: توقف الوسيط عن الرد.
live-down-login-timeout = تعذر الوصول: قبل { $target } الاتصال، لكنه لم يرد على تسجيل الدخول في الوقت المناسب.
live-down-no-address = تعذر الوصول: لا عنوان للمضيف { $host }.
live-down-host-not-found = تعذر الوصول: لم يُعثر على المضيف { $host }: { $why }.
live-down-thread = فشل: لا يمكن بدء خيط الاتصال: { $error } افتح المصدر مرة أخرى.
live-down-topics-refused = غير مصرّح: رفض الوسيط كل موضوع طلبه هذا المصدر.
live-down-qos2 = خطأ في البروتوكول: أرسل الوسيط QoS 2 دون أن يُطلب.
live-down-unexpected-packet = خطأ في البروتوكول: حزمة { $packet } غير متوقعة من الوسيط.
live-down-no-connack = خطأ في البروتوكول: لم يرد الوسيط على تسجيل الدخول بـ CONNACK.
live-down-no-host = خطأ في الإعدادات: لم يُحدد مضيف.
live-down-port-zero = خطأ في الإعدادات: المنفذ 0.
live-down-no-topic = خطأ في الإعدادات: لم يُحدد موضوع.
live-down-bad-topic = خطأ في الإعدادات: { $why }: { $topic }. صحح الموضوع في إعدادات المصدر.
live-down-password-without-user = خطأ في الإعدادات: كلمة المرور تحتاج إلى اسم مستخدم.
live-down-tls-on-plain = خطأ في الإعدادات: طُلب TLS على اتصال غير مشفر. أوقف TLS أو غيّر المنفذ.
live-down-ca-unreadable = خطأ في الإعدادات: تعذرت قراءة ملف جهة الإصدار { $path }: { $error } تحقق من ملف جهة الإصدار في إعدادات المصدر.
live-down-ca-not-pem = خطأ في الإعدادات: ملف جهة الإصدار { $path } ليس PEM صالحًا: { $error } اختر ملف شهادة PEM.
live-down-ca-refused = خطأ في الإعدادات: رُفضت شهادة في { $path }: { $error } تحقق من ملف جهة الإصدار في إعدادات المصدر.
live-down-ca-empty = خطأ في الإعدادات: ملف جهة الإصدار { $path } لا يحوي شهادة. اختر ملف شهادة PEM.
live-down-bad-tls-host = خطأ في الإعدادات: { $host } ليس اسم مضيف صالحًا لـ TLS. صحح المضيف في إعدادات المصدر.
live-down-unknown-issuer = فشل TLS: شهادة الوسيط غير موثوقة، وجهة إصدارها غير معروفة. عيّن ملف جهة إصدار إن كان الوسيط يستخدم شهادة خاصة.
live-down-wrong-host-name = فشل TLS: شهادة الوسيط لاسم مضيف مختلف.
live-down-expired = فشل TLS: انتهت صلاحية شهادة الوسيط.
live-down-not-valid-yet = فشل TLS: شهادة الوسيط لم تبدأ صلاحيتها بعد. تحقق من ساعة هذا الحاسوب.
live-down-revoked = فشل TLS: أُلغيت شهادة الوسيط.
live-down-pin-mismatch = فشل TLS: مفتاح الوسيط لا يطابق البصمة. تحقق من البصمة أو رمز الانضمام؛ فقد لا يكون هذا الوسيط الذي تقصده.
live-down-certificate-refused = فشل TLS: رُفضت شهادة الوسيط ({ $detail }).
live-down-handshake-alert = فشل TLS: أنهى الوسيط المصافحة ({ $detail }).
live-down-no-certificate = فشل TLS: لم يرسل الوسيط شهادة.
live-down-incompatible = فشل TLS: إعدادات TLS للوسيط غير متوافقة ({ $detail }).
live-down-not-tls = فشل TLS: لم يرد الوسيط بـ TLS ({ $detail }). هل هذا منفذ غير مشفر؟
live-down-tls = فشل TLS: { $error } تحقق من إعدادات TLS للمصدر.

## Why a connection step failed, lower case, to follow "cannot connect to
## host:port:" and the like.
live-why-nothing-listening = لا شيء يستمع على ذلك المنفذ
live-why-no-answer = لا رد في الوقت المناسب
live-why-reset = أُعيد ضبط الاتصال
live-why-closed = أغلق الوسيط الاتصال
live-why-no-route = لا مسار إلى المضيف
live-why-lookup-failed = فشل البحث

## Why a topic filter cannot be followed, lower case, to follow a lead-in.
live-topic-problem-empty = موضوع فارغ
live-topic-problem-too-long = موضوع أطول من 65535 بايت
live-topic-problem-nul = موضوع يحوي محرف NUL
live-topic-problem-hash = لا يجوز # إلا مستوى أخيرًا كاملًا من الموضوع
live-topic-problem-plus = لا يجوز + إلا مستوى كاملًا من الموضوع

## Why a join link does not read (beta 6), lower case, said after "Not a
## join link:" or "No join link:". $scheme is twjoin, $key what followed
## the question mark, $character a character as typed, $code a
## character's number, $group its group of four in the code, $count the
## characters a code has, $needed the 16 it needs. $why is one of the
## live-topic-problem-* pieces.
live-link-bad-too-long = النص أطول من اللازم بكثير.
live-link-bad-scheme = رابط الانضمام يبدأ بـ { $scheme }://.
live-link-bad-spaces = لا مسافات ولا فواصل أسطر في رابط الانضمام.
live-link-bad-no-code = لم يُعطَ رمز انضمام بعد المضيف والمنفذ.
live-link-bad-no-host = لم يُعطَ مضيف.
live-link-bad-bracket = عنوان IPv6 يحتاج إلى قوس مربع إغلاق.
live-link-bad-not-ipv6 = العنوان بين القوسين المربعين ليس عنوان IPv6.
live-link-bad-port = المنفذ ليس رقمًا من 1 إلى 65535.
live-link-bad-host-too-long = اسم المضيف طويل جدًا.
live-link-bad-host-character = { $character } غير مستخدم في أسماء المضيفين. تحقق من المضيف.
live-link-bad-host-character-code = حرف رمزه { $code } غير مستخدم في أسماء المضيفين. تحقق من المضيف.
live-link-bad-key = { $key } ليس جزءًا من رابط الانضمام. لا يجوز بعد علامة الاستفهام إلا topic.
live-link-bad-topic-twice = الموضوع مذكور مرتين.
live-link-bad-topic-control = في الموضوع محرف تحكم.
live-link-bad-topic = { $why }.
live-link-bad-percent = في الموضوع علامة نسبة مئوية لا يتبعها رقمان سداسيان عشريان.
live-link-bad-utf8 = الموضوع ليس نص UTF-8.
live-link-bad-code-empty = لم يُعطَ رمز.
live-link-bad-code-length =
    { $count ->
        [one] في الرمز حرف واحد؛ ويلزم { $needed }، في أربع مجموعات من أربعة.
        [two] في الرمز حرفان؛ ويلزم { $needed }، في أربع مجموعات من أربعة.
        [few] في الرمز { $count } أحرف؛ ويلزم { $needed }، في أربع مجموعات من أربعة.
        [many] في الرمز { $count } حرفًا؛ ويلزم { $needed }، في أربع مجموعات من أربعة.
       *[other] في الرمز { $count } حرف؛ ويلزم { $needed }، في أربع مجموعات من أربعة.
    }
live-link-bad-code-character = { $character } في المجموعة { $group } غير مستخدم في رموز الانضمام. تحقق من ذلك الحرف.
live-link-bad-code-character-code = حرف رمزه { $code } في المجموعة { $group } غير مستخدم في رموز الانضمام. تحقق من ذلك الحرف.
