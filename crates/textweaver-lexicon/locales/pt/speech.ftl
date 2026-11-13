### Voices, the voice manager, and restarting speech.

## Restarting speech.

restart-silent-now = O { -brand } está silencioso agora; reinicie-o para ouvir a fala de novo.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = O { -brand } está silencioso agora. Reinicie a fala com { $keys }.
restart-restarting = Reiniciando a fala.
restart-not-here = A fala não pode ser reiniciada aqui.
restart-already = A fala já está reiniciando.
# $error is the system's reason, in its own words.
restart-failed = Não foi possível reiniciar a fala: { $error } Aguarde um momento e tente de novo.
restart-start-failed = Não foi possível reiniciar a fala: a inicialização falhou.
restart-no-engine = Nenhum motor de fala disponível; ficando em silêncio. Os guias Troubleshooting, No speech at all, têm as verificações.
restart-done-silent = Fala reiniciada, mas em silêncio: nenhum motor está disponível.
restart-done = Fala reiniciada.

## The voice manager's list.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] { $n } voz: { $language }, { $engine }.
       *[other] { $n } vozes: { $language }, { $engine }.
    }
voices-all-languages = todos os idiomas
voices-all-engines = todos os motores
# The filter rows at the top of the list.
voices-language-row = Idioma: { $language }
voices-engine-row = Motor: { $engine }
voices-fetch-row = Baixar a lista de vozes do Piper da internet
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = download { $size }
voices-licence-public-domain = domínio público
voices-licence-attribution = livre com crédito
voices-licence-share-alike = livre com crédito, compartilhamento igual
voices-licence-non-commercial = não comercial
voices-licence-unknown = licença mostrada antes do download
voices-favourite = favorita
voices-current = atual

## Language names in the voice manager's language filter.

voices-language-ar = Árabe
voices-language-ca = Catalão
voices-language-cs = Tcheco
voices-language-cy = Galês
voices-language-da = Dinamarquês
voices-language-de = Alemão
voices-language-el = Grego
voices-language-en = Inglês
voices-language-es = Espanhol
voices-language-fa = Persa
voices-language-fi = Finlandês
voices-language-fr = Francês
voices-language-hi = Hindi
voices-language-hu = Húngaro
voices-language-is = Islandês
voices-language-it = Italiano
voices-language-ja = Japonês
voices-language-ka = Georgiano
voices-language-kk = Cazaque
voices-language-ko = Coreano
voices-language-lb = Luxemburguês
voices-language-lv = Letão
voices-language-nl = Holandês
voices-language-no = Norueguês
voices-language-pl = Polonês
voices-language-pt = Português
voices-language-ro = Romeno
voices-language-ru = Russo
voices-language-sk = Eslovaco
voices-language-sl = Esloveno
voices-language-sr = Sérvio
voices-language-sv = Sueco
voices-language-sw = Suaíli
voices-language-tr = Turco
voices-language-uk = Ucraniano
voices-language-vi = Vietnamita
voices-language-zh = Chinês

## Voices: the voice manager, rate, pitch, and volume.

# Spoken by a newly chosen voice as its sample.
voice-sample = Um pequeno jabuti xereta viu dez cegonhas felizes.
voice-list-title = Escolher uma voz
voice-still-loading = As vozes ainda estão carregando. A lista abre quando estiverem prontas.
voice-list-failed = Não foi possível listar as vozes: { $error } Escolha outro motor no menu Fala.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Gerenciador de vozes. { $shown } Enter usa uma voz e fala uma amostra, ou baixa uma; { $preview } ouve uma prévia; Espaço marca uma favorita; Delete remove uma voz baixada; Escape fecha.
voice-more-ready =
    { $n ->
        [one] Mais uma voz de outro motor está na lista.
       *[other] Mais { $n } vozes de outros motores estão na lista.
    }
voice-preview = Prévia: { $voice }.
voice-preview-sample = { $voice }. Um pequeno jabuti xereta viu dez cegonhas felizes.
voice-preview-starting = Prévia: { $voice }, iniciando { $engine }.
voice-preview-not-installed = { $voice } ainda não foi baixada. Enter a baixa, depois de uma pergunta.
voice-preview-unavailable = { $engine } não pode iniciar aqui para uma prévia. Enter muda para esse motor.
voice-preview-engine-failed = Não foi possível iniciar { $engine } para a prévia de { $voice }.
voice-preview-failed = Não foi possível ouvir a prévia de { $voice }: { $error } Tente outra voz.
# $keys names the Choose Voice key.
voice-ready = As vozes estão prontas. { $keys } as lista.
voice-fetch-catalog-question = Baixar a lista de vozes do Piper, cerca de 250 kilobytes, do Hugging Face? y ou n
voice-fetch-catalog-question-short = Baixar a lista de vozes do Piper? y ou n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = Voz { $voice }, em { $engine }. Trocando de motor.
voice-download-in-progress = Já há um download de voz em andamento.
voice-no-data-folder = Não há pasta de dados para guardar as vozes do Piper.
voice-not-in-list = Essa voz não está mais na lista de vozes do Piper.
voice-download-start-failed = Não foi possível iniciar o download.
voice-reading-licence = Lendo a licença de { $voice }.
voice-remove-question = Remover a voz { $voice }? y ou n
voice-only-piper-removable = Somente vozes do Piper baixadas podem ser removidas.
# $plan describes the download: the voice, its size and licence.
voice-download-question = { $plan } y ou n
voice-in-use = { $voice } é a voz em uso. Escolha outra voz primeiro.
voice-removed = { $voice } removida.
voice-remove-failed = Não foi possível remover { $voice }: { $error } Verifique se a pasta dele pode ser gravada.
voice-downloading-catalog = Baixando a lista de vozes do Piper.
voice-downloading = Baixando { $voice }.
voice-downloading-percent = Baixando { $voice }, { $pct } por cento.
voice-details-failed = Não foi possível ler os detalhes da voz: { $error } Verifique a conexão e tente de novo.
voice-download-stopped = O download parou.
voice-catalog-fetched = A lista de vozes do Piper tem { $voices } vozes em { $languages } idiomas. O comando Vozes as lista.
voice-catalog-failed = Não foi possível baixar a lista de vozes: { $error } Verifique a conexão e tente de novo.
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } está instalada. { $licence } O comando Vozes a lista.
voice-download-failed = Não foi possível baixar { $voice }: { $error } Escolha a voz de novo para tentar outra vez.
voice-only-voice-favourite = Somente uma voz pode ser favorita.
voice-favourite-added = { $voice } adicionada às favoritas.
voice-favourite-removed = { $voice } removida das favoritas.
voice-chosen = Voz { $voice }.
voice-chosen-rate = Voz { $voice }, { $wpm } palavras por minuto.
voice-fastest-rate = Velocidade máxima.
voice-slowest-rate = Velocidade mínima.
voice-rate = { $wpm } palavras por minuto.
voice-highest-pitch = Tom mais alto.
voice-lowest-pitch = Tom mais baixo.
voice-pitch-normal = Tom normal.
# $n is a number of semitones.
voice-pitch-plus = Tom mais { $n }.
voice-pitch-minus = Tom menos { $n }.
voice-full-volume = Volume máximo.
voice-volume-off = Volume desligado.
voice-volume = Volume { $pct } por cento.
voice-no-speed-presets = Nenhum preset de velocidade.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = { $name }, velocidade { $wpm }.
voice-line-numbers-on = Números de linha ligados.
voice-line-numbers-off = Números de linha desligados.
