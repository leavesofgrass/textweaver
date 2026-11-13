### Voices, the voice manager, and restarting speech.

## Restarting speech.

restart-silent-now = { -brand } está en silencio ahora; reinicie la voz para volver a oírla.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = { -brand } está en silencio ahora. Reinicie la voz con { $keys }.
restart-restarting = Reiniciando la voz.
restart-not-here = La voz no se puede reiniciar aquí.
restart-already = La voz ya se está reiniciando.
# $error is the system's reason, in its own words.
restart-failed = No se pudo reiniciar la voz: { $error } Espere un momento y vuelva a intentarlo.
restart-start-failed = No se pudo reiniciar la voz: no se pudo iniciar.
restart-no-engine = No hay ningún motor de voz disponible; { -brand } permanece en silencio. Consulte Troubleshooting, No speech at all, en la documentación.
restart-done-silent = Se reinició la voz, pero no hay ningún motor de voz disponible; { -brand } permanece en silencio.
restart-done = Voz reiniciada.

## The voice manager's list.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] { $n } voz: { $language }, { $engine }.
       *[other] { $n } voces: { $language }, { $engine }.
    }
voices-all-languages = todos los idiomas
voices-all-engines = todos los motores
# The filter rows at the top of the list.
voices-language-row = Idioma: { $language }
voices-engine-row = Motor: { $engine }
voices-fetch-row = Descargar la lista de voces de Piper desde internet
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = descarga { $size }
voices-licence-public-domain = dominio público
voices-licence-attribution = gratis con atribución
voices-licence-share-alike = gratis con atribución, compartir igual
voices-licence-non-commercial = no comercial
voices-licence-unknown = licencia mostrada antes de la descarga
voices-favourite = favorita
voices-current = actual

## Language names in the voice manager's language filter.

voices-language-ar = árabe
voices-language-ca = catalán
voices-language-cs = checo
voices-language-cy = galés
voices-language-da = danés
voices-language-de = alemán
voices-language-el = griego
voices-language-en = inglés
voices-language-es = español
voices-language-fa = persa
voices-language-fi = finés
voices-language-fr = francés
voices-language-hi = hindi
voices-language-hu = húngaro
voices-language-is = islandés
voices-language-it = italiano
voices-language-ja = japonés
voices-language-ka = georgiano
voices-language-kk = kazajo
voices-language-ko = coreano
voices-language-lb = luxemburgués
voices-language-lv = letón
voices-language-nl = neerlandés
voices-language-no = noruego
voices-language-pl = polaco
voices-language-pt = portugués
voices-language-ro = rumano
voices-language-ru = ruso
voices-language-sk = eslovaco
voices-language-sl = esloveno
voices-language-sr = serbio
voices-language-sv = sueco
voices-language-sw = suajili
voices-language-tr = turco
voices-language-uk = ucraniano
voices-language-vi = vietnamita
voices-language-zh = chino

## Voices: the voice manager, rate, pitch, and volume.

# Spoken by a newly chosen voice as its sample.
voice-sample = El veloz murciélago hindú comía feliz cardillo y kiwi.
voice-list-title = Elegir una voz
voice-still-loading = Las voces todavía se están cargando. La lista se abre cuando estén listas.
voice-list-failed = No se pudieron listar las voces: { $error } Elija otro motor en el menú Voz.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Gestor de voces. { $shown } Intro usa una voz y dice una muestra, o la descarga; { $preview } prueba una voz; Espacio marca una favorita; Suprimir elimina una voz descargada; Escape cierra.
voice-more-ready =
    { $n ->
        [one] Una voz más de otro motor está en la lista.
       *[other] { $n } voces más de otros motores están en la lista.
    }
voice-preview = Prueba: { $voice }.
voice-preview-sample = { $voice }. El veloz murciélago hindú comía feliz cardillo y kiwi.
voice-preview-starting = Prueba: { $voice }, iniciando { $engine }.
voice-preview-not-installed = { $voice } aún no está descargada. Intro la descarga, tras una pregunta.
voice-preview-unavailable = { $engine } no puede iniciarse aquí para una prueba. Intro cambia a ese motor.
voice-preview-engine-failed = No se pudo iniciar { $engine } para probar { $voice }.
voice-preview-failed = No se pudo probar { $voice }: { $error } Pruebe otra voz.
# $keys names the Choose Voice key.
voice-ready = Las voces están listas. { $keys } las lista.
voice-fetch-catalog-question = ¿Descargar la lista de voces de Piper, unos 250 kilobytes, desde Hugging Face? y o n
voice-fetch-catalog-question-short = ¿Descargar la lista de voces de Piper? y o n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = Voz { $voice }, en { $engine }. Cambiando de motor.
voice-download-in-progress = Ya hay una descarga de voz en curso.
voice-no-data-folder = No hay ninguna carpeta de datos donde guardar las voces de Piper.
voice-not-in-list = Esa voz ya no está en la lista de voces de Piper.
voice-download-start-failed = No se pudo iniciar la descarga.
voice-reading-licence = Leyendo la licencia de { $voice }.
voice-remove-question = ¿Quitar la voz { $voice }? y o n
voice-only-piper-removable = Solo se pueden quitar las voces de Piper descargadas.
# $plan describes the download: the voice, its size and licence.
voice-download-question = { $plan } y o n
voice-in-use = { $voice } es la voz en uso. Elija otra voz primero.
voice-removed = { $voice } quitada.
voice-remove-failed = No se pudo quitar { $voice }: { $error } Compruebe que se puede escribir en su carpeta.
voice-downloading-catalog = Descargando la lista de voces de Piper.
voice-downloading = Descargando { $voice }.
voice-downloading-percent = Descargando { $voice }, { $pct } por ciento.
voice-details-failed = No se pudieron leer los detalles de la voz: { $error } Compruebe la conexión y vuelva a intentarlo.
voice-download-stopped = La descarga se detuvo.
voice-catalog-fetched = La lista de voces de Piper tiene { $voices } voces en { $languages } idiomas. El comando Voces las lista.
voice-catalog-failed = No se pudo descargar la lista de voces: { $error } Compruebe la conexión y vuelva a intentarlo.
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } está instalada. { $licence } El comando Voces la lista.
voice-download-failed = No se pudo descargar { $voice }: { $error } Elija la voz de nuevo para reintentar.
voice-only-voice-favourite = Solo una voz puede ser favorita.
voice-favourite-added = { $voice } agregada a favoritas.
voice-favourite-removed = { $voice } quitada de favoritas.
voice-chosen = Voz { $voice }.
voice-chosen-rate = Voz { $voice }, { $wpm } palabras por minuto.
voice-fastest-rate = Velocidad máxima.
voice-slowest-rate = Velocidad mínima.
voice-rate = { $wpm } palabras por minuto.
voice-highest-pitch = Tono más alto.
voice-lowest-pitch = Tono más bajo.
voice-pitch-normal = Tono normal.
# $n is a number of semitones.
voice-pitch-plus = Tono más { $n }.
voice-pitch-minus = Tono menos { $n }.
voice-full-volume = Volumen máximo.
voice-volume-off = Volumen desactivado.
voice-volume = Volumen { $pct } por ciento.
voice-no-speed-presets = No hay velocidades preestablecidas.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = { $name }, velocidad { $wpm }.
voice-line-numbers-on = Números de línea activados.
voice-line-numbers-off = Números de línea desactivados.
