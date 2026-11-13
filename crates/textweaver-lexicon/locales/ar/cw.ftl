### CW (Morse code): the speech engine, practice, and keying.

## Beta 4: CW (Morse code) as a speech engine.
cw-sending = CW: { $text }
cw-speed = سرعة الأحرف { $wpm }، والسرعة الفعلية { $effective } كلمة في الدقيقة.
cw-skipped =
    { $count ->
        [one] حرف واحد لم يُرسل بشفرة مورس.
       *[other] { $count } من الأحرف لم تُرسل بشفرة مورس.
    }

## Beta 4: CW practice (Koch method, random drills, copy checking).
cw-practice-koch-start = درس كوخ { $lesson } من 40، { $count } حرفًا، الجديد: { $new }.
cw-practice-characters-start = أحرف عشوائية، { $groups } مجموعات.
cw-practice-words-start = كلمات عشوائية، { $groups } مجموعات.
cw-practice-group = المجموعة { $n } من { $groups }.
cw-practice-again = مرة أخرى، المجموعة { $n } من { $groups }.
cw-practice-perfect = { $correct } من { $total }، كلها صحيحة.
cw-practice-score = { $correct } من { $total }. الأخطاء: { $missed }. المرسل: { $sent }.
cw-practice-wrong = { $sent } كُتب { $typed }
cw-practice-missing = { $sent } لم يُنسخ
cw-practice-extra = { $typed } زائد
cw-practice-round = الجولة: { $correct } من { $total }، { $percent } بالمئة.
cw-practice-added = الدرس { $lesson }: أضيف { $new }.
cw-practice-not-yet = { $threshold } بالمئة تضيف الحرف التالي.
cw-practice-all-learned = تم تعلم كل الأحرف.
cw-practice-stopped = توقف التمرين.
cw-practice-progress = درس كوخ { $lesson } من 40، الجديد: { $new }. { $recent }
cw-practice-recent = آخر الجولات: { $scores } بالمئة.
cw-practice-no-rounds = لا جولات بعد.
cw-practice-unavailable = لم يبدأ محرك CW، لذا لا يمكن للتمرين الإرسال.
cw-char-period = نقطة
cw-char-comma = فاصلة
cw-char-equals = يساوي
cw-char-slash = شرطة مائلة
cw-char-question = علامة استفهام

## Beta 4: CW keying, Morse code typed with a key or paddle. $key is a
## key name, $name a MIDI input's name.
cw-keying-on = الإبراق بـCW يعمل. أبرق بشفرة مورس لتُكتب عند المؤشر؛ { $key } يوقف الإبراق.
cw-keying-off = الإبراق بـCW متوقف.
cw-keying-ended = الإبراق بـCW متوقف، لأن وضع التحرير انتهى.
cw-keying-needs-edit = الإبراق بـCW يكتب في وضع التحرير. شغّل وضع التحرير أولًا، بـ{ $key }.
cw-keying-listening = الإبراق بـCW: يستمع إلى { $name }.
cw-keying-midi-failed = لا يستطيع الإبراق بـCW فتح { $name }: { $error } تحقق من توصيله، ثم شغّل الإبراق مرة أخرى.
cw-keying-no-midi = لم يُعثر على محوّل مفتاح MIDI؛ وصّله أو اقرنه، ثم شغّل الإبراق مرة أخرى. محوّلات لوحة المفاتيح تعمل في تطبيق { -brand }.
cw-keying-midi-not-in-build = MIDI غير متوفر في نسخة { -brand } هذه. محوّلات لوحة المفاتيح تعمل في تطبيق { -brand }.
cw-keying-no-sidetone = لا نغمة مراقبة: { $error } يستمر الإبراق من دونها؛ استخدم نغمة المراقبة في المحوّل أو جهاز الراديو.
cw-keying-unknown = ليس حرفًا من شفرة مورس.
cw-keying-erased = إشارة الخطأ: مُحي { $text }.
cw-keying-nothing-erased = إشارة الخطأ: لا شيء مُبرَق لمحوه.
cw-keying-rsvp-on = العرض السريع يُظهر الكلمات التي تبرقها.
