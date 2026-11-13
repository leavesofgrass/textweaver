### Export, preview, and audio export.

## Export and preview from the reader. F5 is the browser's reload key,
## not textweaver's.

common-no-document = Nenhum documento está aberto.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = não é possível escrever em { $path }: { $error } Verifique se a pasta pode ser gravada.
publish-cannot-write = não é possível escrever { $path }: { $error } Verifique se a pasta dele pode ser gravada.
publish-start-failed = Não foi possível iniciar a exportação: { $error } Aguarde um momento e tente de novo.
publish-export-error = Não foi possível exportar: { $error } Corrija isso e exporte de novo.
publish-export-as-label = Exportar como, Enter para { $path }
publish-export-over-source = Não exportado: é o próprio documento. Escolha outro nome.
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Exportando para { $format }.
publish-theme-title = Tema para a página HTML
publish-theme-intro = Tema para a página HTML? { $first } primeiro, { $n } opções. Escape cancela.
publish-writing-preview = Escrevendo a pré-visualização.
publish-preview-error = Não foi possível escrever a pré-visualização: { $error } Corrija isso e abra a prévia de novo.
publish-still-exporting =
    { $secs ->
        [one] Ainda exportando para { $format }, { $secs } segundo.
       *[other] Ainda exportando para { $format }, { $secs } segundos.
    }
publish-still-previewing =
    { $secs ->
        [one] Ainda escrevendo a pré-visualização, { $secs } segundo.
       *[other] Ainda escrevendo a pré-visualização, { $secs } segundos.
    }
# "Browser preview follows" was chosen. $again is yes when a preview
# is open that reloads by itself only once it is opened again.
publish-follow-off = A prévia no navegador não acompanha nada: pressione F5 no navegador depois de salvar.
publish-follow-save =
    { $again ->
        [yes] A prévia no navegador acompanha cada salvamento: a página recarrega sozinha. Abra a pré-visualização no navegador de novo para começar.
       *[no] A prévia no navegador acompanha cada salvamento: a página recarrega sozinha.
    }
publish-follow-typing =
    { $again ->
        [yes] A prévia no navegador acompanha sua digitação: a página recarrega depois de cada salvamento e quando a digitação pausa. Abra a pré-visualização no navegador de novo para começar.
       *[no] A prévia no navegador acompanha sua digitação: a página recarrega depois de cada salvamento e quando a digitação pausa.
    }
# The first preview of a session: what the browser will do. $follow is
# the "Browser preview follows" setting: off, save, or typing.
publish-preview-first =
    { $follow ->
        [save] A pré-visualização abre no seu navegador e recarrega depois de cada salvamento.
        [typing] A pré-visualização abre no seu navegador e acompanha sua digitação.
       *[off] A pré-visualização abre no seu navegador. Pressione F5 lá depois de cada salvamento.
    }
# $error is the converter's reason.
publish-export-failed = A exportação para { $format } falhou: { $error } Tente outro formato.
publish-preview-failed = A pré-visualização falhou: { $error } Salve para tentar de novo.
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 aviso: { $first }
       *[other] { $n } avisos; o primeiro: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exportado para { $format }: { $file }. Abrir? y ou n. Em { $folder }.{ $warned }
publish-report = Relatório salvo como { $file }.
publish-report-issues =
    { $n ->
        [one] 1 item não acessível
       *[other] { $n } itens não acessíveis
    }; veja o relatório.
publish-report-failed = Não foi possível salvar o relatório: { $error } Verifique se a pasta de saída permite gravação.
publish-preview-written-served = Pré-visualização escrita. Abrindo-a no navegador. Ela recarrega por conta própria depois de cada salvamento.{ $warned }
publish-preview-written = Pré-visualização escrita. Abrindo-a no navegador. Salvar a escreve de novo; depois pressione F5 no navegador.{ $warned }
publish-preview-updated = Pré-visualização atualizada.
publish-preview-updated-press-f5 = Pré-visualização atualizada. Pressione F5 no navegador.
# Said once for a document whose preview was cleaned of raw HTML
# (scripts, event handlers) because its folder is not trusted.
publish-preview-raw-html-left-out = HTML bruto omitido na pré-visualização. As pastas confiáveis estão em Configurações.
publish-server-failed = Não foi possível iniciar o servidor de recarga da pré-visualização ({ $error }); abrindo o arquivo em vez disso.
publish-render-failed = Não foi possível renderizar o texto: { $error } Saia do modo de edição para ler o texto.
publish-nothing-after-caret = Nada para ler depois do cursor.
publish-listening = Ouvindo o texto renderizado.

## The preview's reload server: shown in the browser.

preview-being-written = A pré-visualização está sendo escrita. Recarregue em um instante.

## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = Exportar áudio como
audio-format-intro = Exportar { $name } como áudio: escolha um formato, { $n } opções.
audio-no-ffmpeg =
    { $formats } { $n ->
        [one] precisa
       *[other] precisam
    } do ffmpeg, que não foi encontrado.
audio-format-flac = FLAC: sem perdas, cerca de metade do tamanho do WAV
audio-format-wav = WAV: o maior, toca em qualquer lugar
audio-format-mp3 = MP3: pequeno, toca em qualquer lugar
audio-format-opus = Opus: o menor, feito para voz
audio-format-ogg = Ogg Vorbis: pequeno e aberto, toca na maioria dos players
audio-format-m4b = Audiolivro M4B, pelo ffmpeg
audio-format-mp4 = Vídeo com legendas: MP4, precisa do ffmpeg
audio-format-html = Página de leitura: texto e áudio, um arquivo
audio-where-title = Onde fica o áudio
audio-where-intro = Onde o áudio deve ficar?
audio-where-beside = Ao lado do documento, { $path }
audio-where-choose = Em outra pasta, escolhida a seguir
audio-choose-folder = Escolha a pasta para o áudio
audio-no-engine = Nenhum motor de voz aqui grava arquivos de áudio. Instale o eSpeak NG, ou escolha outro motor no menu Fala.
audio-confirm = Exportar { $name } com { $voice } a { $wpm } palavras por minuto, em { $path }? y ou n
audio-started = Exportando { $name } como { $format }. Escape para.
audio-progress = Exportando áudio, { $percent } por cento.
audio-video-progress =
    { $minutes ->
        [one] Codificando vídeo: { $fed } de { $all } quadros, falta cerca de 1 minuto.
       *[other] Codificando vídeo: { $fed } de { $all } quadros, faltam cerca de { $minutes } minutos.
    }
audio-video-progress-soon = Codificando vídeo: { $fed } de { $all } quadros, falta menos de um minuto.
audio-busy = Já exportando { $name }. Escape para.
audio-stop-question = Parar a exportação? Nenhum arquivo é mantido. y ou n
audio-stopping = Parando a exportação.
audio-still-exporting = Ainda exportando o áudio.
audio-stopped = Exportação de áudio parada; nenhum arquivo foi gravado.
audio-done =
    { $name } gravado: { $length }, { $chapters ->
        [one] 1 capítulo
       *[other] { $chapters } capítulos
    }.
audio-subtitles = Legendas em { $name }.
audio-failed = Não foi possível exportar o áudio: { $error } Tente outro formato ou outra voz.
audio-thread-stopped = A exportação de áudio parou inesperadamente.
