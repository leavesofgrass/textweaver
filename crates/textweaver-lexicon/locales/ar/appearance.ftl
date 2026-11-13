### Themes, accessibility modes, and the Lexend font.

## السمات.

message-error = خطأ: { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = لا توجد سمة باسم { $name }؛ يُستخدم { $used }.
# $theme is the new theme's display name.
themes-next = السمة { $theme }.
themes-soft-dark-name = { $theme }، داكن هادئ
themes-choice-aa = { $theme }، يستوفي AA
themes-choice-below-aa = { $theme }، دون AA
themes-below-aa =
    { $count ->
        [one] أقل من AA: فحص واحد لا يبلغ الحد الأدنى.
       *[other] أقل من AA: { $count } فحوص لا تبلغ الحد الأدنى.
    }

## أوضاع إمكانية الوصول وسؤال أول تشغيل.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] وضع النطق الذاتي. ينطق textweaver كل شيء.
        [screen-reader] وضع قارئ الشاشة. textweaver صامت؛ قارئ شاشتك يقرأ سطر الحالة.
       *[hybrid] الوضع المختلط. يقرأ textweaver المستندات بصوت عالٍ؛ قارئ شاشتك ينطق الرسائل والكتابة.
    }
# Yes to the first-run question; $key names the keys that change the mode.
access-hybrid-chosen = الوضع المختلط. يقرأ textweaver المستندات بصوت عالٍ؛ قارئ شاشتك ينطق الرسائل والكتابة. { $key } يغيّر الوضع.
# No to the first-run question; $key names the keys that change the mode.
access-hybrid-declined = البقاء في وضع النطق الذاتي. { $key } يغيّر الوضع.
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-hybrid-question = { $reader } قيد التشغيل. استخدام الوضع المختلط، حيث يقرأ textweaver المستندات بصوت عالٍ وينطق قارئ شاشتك الرسائل والكتابة؟ y أو n
access-a-screen-reader = قارئ شاشة
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = { $reader } قيد التشغيل: يقرأ textweaver المستندات بصوت عالٍ ويترك الرسائل لقارئ شاشتك. { $key } يغيّر ذلك.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = يقرأ textweaver بصوت عالٍ
access-window-choice-screen-reader = قارئ شاشتي يقرأ
access-window-mode-help = من يقرأ في النافذة: صوت textweaver، أو قارئ الشاشة وحده.
access-window-mode-changed =
    { $mode ->
        [screen-reader] قارئ شاشتي يقرأ: يصمت textweaver ويرسل النص إلى قارئ شاشتك. { $key } يغيّر ذلك.
        [speaks-messages] يقرأ textweaver بصوت عالٍ وينطق رسائله. { $key } يغيّر ذلك.
       *[reads-aloud] يقرأ textweaver بصوت عالٍ؛ تذهب الرسائل إلى قارئ شاشتك. { $key } يغيّر ذلك.
    }

## Lexend downloaded on first choice.
font-download-question = تنزيل الخط { $font }، { $kb } كيلوبايت، { $licence }؟ y أو n
font-downloading = تنزيل { $font }.
font-downloaded = تم تنزيل { $font } وهو جاهز.
font-download-failed = تعذّر تنزيل { $font }: { $error } يُستخدم خط آخر.
font-download-declined = لم يُنزَّل. يُستخدم خط آخر.
font-download-busy = ما زال { $font } قيد التنزيل.
font-download-no-folder = لا يوجد مجلد بيانات لحفظ { $font }.
font-download-not-in-build = تنزيل الخطوط غير متاح في هذا الإصدار.
gui-font-to-download = { $family } (للتنزيل، { $kb } كيلوبايت)
gui-font-downloaded = { $family } (منزّل)
