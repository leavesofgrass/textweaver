### Braille: BRF files read as print, and the Braille view.

## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.

## Beta 4: the Braille view (braille dots at the bottom of the app)
## and the braille grade. $n is the part shown, $total how many.
braille-view-shown = عرض برايل ظاهر.
braille-view-hidden = عرض برايل مخفي.
braille-view-not-in-build = عرض برايل غير متوفر في نسخة { -brand } هذه.
braille-grade-uncontracted = برايل غير مختصر، الدرجة 1.
braille-grade-contracted = برايل مختصر، الدرجة 2.
braille-grade-contracted-no-liblouis = برايل مختصر، الدرجة 2. مكتبة liblouis غير موجودة، لذا يظهر برايل غير المختصر حتى تُثبَّت.
settings-unit-cells =
    { $n ->
        [one] خلية
        [two] خليتان
        [few] خلايا
        [zero] خلايا
       *[other] خلية
    }
gui-braille-view = عرض برايل
gui-braille-view-description = مرئي فقط: السطر الحالي بنقاط برايل، للمبصرين الذين يتعلمون برايل أو يعلّمونه. يعرض قارئ الشاشة برايل على سطر برايل لديك.
gui-braille-caption-uncontracted = برايل غير مختصر، { $n } من { $total }
gui-braille-caption-contracted = برايل مختصر، { $n } من { $total }
gui-braille-caption-fell-back = برايل غير مختصر (المختصر يحتاج إلى liblouis)، { $n } من { $total }
brf-original-title = برايل الأصلي، الصفحة { $page }
brf-original-intro =
    { $n ->
        [one] برايل الأصلي، الصفحة { $page }، سطر واحد. Escape يغلق.
        [two] برايل الأصلي، الصفحة { $page }، سطران. Escape يغلق.
        [few] برايل الأصلي، الصفحة { $page }، { $n } أسطر. Escape يغلق.
        [many] برايل الأصلي، الصفحة { $page }، { $n } سطرًا. Escape يغلق.
       *[other] برايل الأصلي، الصفحة { $page }، { $n } سطر. Escape يغلق.
    }
brf-original-not-brf = ليس ملف برايل. عرض برايل الأصلي يعمل مع ملفات BRF.
brf-original-unreadable = تعذرت قراءة ملف برايل: { $reason }
brf-no-liblouis = برايل معروض كبرايل: liblouis غير موجود. لقراءته كنص مطبوع، ثبّت liblouis من liblouis.io أو من حزم نظامك، ثم افتح الملف مرة أخرى.
brf-no-liblouis-short = يُعرض برايل كبرايل: liblouis غير موجود. افتح الملف مرة أخرى بعد تثبيته.
daisy-headings-only = العناوين فقط: لا يحتوي كتاب DAISY هذا على نص، بل على عناوين وصوت فقط، لذا تُقرأ عناوينه.
## End of B1-r5
