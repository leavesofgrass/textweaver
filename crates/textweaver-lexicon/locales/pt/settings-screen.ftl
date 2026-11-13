### The settings screen, profiles, units, sections, and settings files.

## Settings profiles.

profiles-title =
    { $n ->
        [0] Perfis de configurações, nenhum salvo ainda
        [one] Perfis de configurações, 1 perfil
       *[other] Perfis de configurações, { $n } perfis
    }
# $title is profiles-title.
profiles-intro = { $title }. Enter muda para um perfil, F2 renomeia, Delete exclui.
# $summary is profile-summary-* parts joined by commas.
profiles-item = { $name }: { $summary }
profiles-item-active = { $name }, em uso: { $summary }
profiles-save-new = Salvar as configurações atuais como um novo perfil
profiles-update = Salvar as configurações atuais em { $name }
profiles-import = Importar perfis de um arquivo
profiles-export = Exportar todos os perfis para um arquivo
profile-summary-voice = voz { $voice }
profile-summary-rate = velocidade { $rate }
profile-summary-theme = tema { $theme }
# $mode is self voicing, hybrid, or screen reader.
profile-summary-access = modo { $mode }
profile-summary-empty = nada salvo
profile-switched = Mudou para { $name }.
profile-switched-backend = Mudou para { $name }. O motor de fala dele é usado a partir do próximo início.
# $keys lists settings such as speech.pitch.
profile-dropped =
    { $n ->
        [one] 1 configuração dele não é usada nesta versão: { $keys }.
       *[other] { $n } configurações dele não são usadas nesta versão: { $keys }.
    }
profile-saved = Salvou as configurações atuais como { $name }.
profile-replaced = Salvou as configurações atuais em { $name }.
profile-renamed = Renomeou { $old } para { $new }.
profile-delete-question = Excluir o perfil { $name }? y ou n
profile-deleted = Excluiu { $name }.
profile-kept = Mantido.
profile-not-found = Não há nenhum perfil chamado { $name }.
profile-needs-name = Um perfil precisa de um nome.
profile-exists = Já existe um perfil chamado { $name }.
profiles-not-an-export = { $detail }
profiles-no-persistence = Os perfis não são salvos nesta sessão.
profiles-read-failed = Não foi possível ler o arquivo de perfis, então ele é tratado como vazio: { $error } Salvar um perfil substitui o arquivo.
profiles-save-failed = Não foi possível salvar os perfis: { $error } Verifique se a pasta de configurações pode ser gravada.
profiles-none-to-export = Ainda não há perfis para exportar.
profiles-exported =
    { $n ->
        [one] Exportou 1 perfil para { $file }.
       *[other] Exportou { $n } perfis para { $file }.
    }
profiles-export-failed = Não foi possível exportar os perfis: { $error } Verifique se a pasta pode ser gravada.
profiles-imported =
    { $n ->
        [0] Não havia perfis em { $file }.
        [one] Importou 1 perfil de { $file }: { $names }.
       *[other] Importou { $n } perfis de { $file }: { $names }.
    }

## Exporting and importing settings.

# $n settings differ; $name is the file's name.
settingsio-import-question =
    { $n ->
        [one] Importar { $n } configuração alterada de { $name }? y ou n
       *[other] Importar { $n } configurações alteradas de { $name }? y ou n
    }
settingsio-no-persistence = As configurações não são salvas nesta sessão, então não podem ser exportadas ou importadas.
# $path is the file written.
settingsio-exported = Configurações exportadas para { $path }.
settingsio-export-failed = Não foi possível exportar as configurações: { $error } Verifique se a pasta pode ser gravada.
# $path is the file; $error the system's reason.
settingsio-read-failed = Não foi possível ler { $path }: { $error } Verifique o arquivo e importe de novo.
settingsio-nothing-to-import = Nada para importar: suas configurações já correspondem a esse arquivo.
settingsio-cancelled-unchanged = Cancelado. Nada foi alterado.
settingsio-import-failed = Não foi possível importar as configurações: { $error } Verifique se a pasta de configurações pode ser gravada.
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Configurações importadas. { $summary }
settingsio-backend-next-start = O novo motor de fala é usado a partir do próximo início.
settingsio-backed-up = As configurações antigas foram salvas como backup.

## The settings screen. $label is a setting-* label, $value its value as
## described below.

settings-not-set = não definido
settings-none = nenhum
settings-empty = vazio
# A number and its unit (a settings-unit-* message): "300 words per minute".
settings-number-unit = { $n } { $unit }
settings-entries =
    { $n ->
        [0] nenhuma
        [one] 1 entrada
       *[other] { $n } entradas
    }
settings-type-on-or-off = Digite ligado ou desligado.
settings-type-a-number = Digite um número de { $min } a { $max }.
settings-outside = { $n } está fora de { $min } a { $max }.
# $names are the choices, joined with commas.
settings-choose-one-of = Escolha uma destas: { $names }.
settings-edit-table = Edite { $label } em settings.toml; ele guarda nomes e valores.
# $path is a key such as speech.rate, not translated.
settings-no-such-setting = Não há configuração { $path }.
settings-cannot-be = { $label } não pode ser isso: { $error } Escolha outro valor.
settings-changed = { $label }, { $value }.
settings-clamped = Fora do intervalo, então o valor mais próximo é usado.
settings-restart-speech = Reinicie a fala para usá-la.
settings-next-start = Usado a partir do próximo início.
settings-intro = Configurações, { $n } configurações. Digite para filtrar. Esquerda e Direita mudam um valor, Enter altera ou digita um, Delete restaura o padrão, Escape fecha.
settings-item = { $label }: { $value }
settings-title = Configurações
settings-title-matching = Configurações que correspondem a { $filter }
settings-closed = Configurações fechadas.
settings-filter-cleared =
    { $n ->
        [one] Filtro limpo, 1 configuração.
       *[other] Filtro limpo, { $n } configurações.
    }
settings-filter-none = Nenhuma configuração corresponde a { $query }. Backspace remove letras.
settings-filter-match =
    { $n ->
        [one] 1 configuração corresponde.
       *[other] { $n } configurações correspondem.
    }
settings-largest = Maior valor, { $value }.
settings-smallest = Menor valor, { $value }.
settings-press-enter = { $label }: pressione Enter para digitar um novo valor.
settings-table-item = { $label }: { $value }. Edite-o em settings.toml.
# $help is the setting's help (setting-*-help), which may be empty.
settings-editing = { $label }, agora { $value }. { $help }

## Units, said after a number.

settings-unit-words-per-minute =
    { $n ->
        [one] palavra por minuto
       *[other] palavras por minuto
    }
settings-unit-hertz =
    { $n ->
        [one] hertz
       *[other] hertz
    }
settings-unit-percent = por cento
settings-unit-semitones =
    { $n ->
        [one] semitom
       *[other] semitons
    }
settings-unit-milliseconds =
    { $n ->
        [one] milissegundo
       *[other] milissegundos
    }
settings-unit-words =
    { $n ->
        [one] palavra
       *[other] palavras
    }
settings-unit-places =
    { $n ->
        [one] casa
       *[other] casas
    }
settings-unit-columns =
    { $n ->
        [one] coluna
       *[other] colunas
    }
settings-unit-groups =
    { $n ->
        [one] grupo
       *[other] grupos
    }
settings-unit-characters =
    { $n ->
        [one] caractere
       *[other] caracteres
    }
settings-unit-lines =
    { $n ->
        [one] linha
       *[other] linhas
    }
settings-unit-seconds =
    { $n ->
        [one] segundo
       *[other] segundos
    }
settings-unit-steps =
    { $n ->
        [one] passo
       *[other] passos
    }
settings-unit-megabytes =
    { $n ->
        [one] megabyte
       *[other] megabytes
    }
settings-unit-files =
    { $n ->
        [one] arquivo
       *[other] arquivos
    }
settings-unit-letters =
    { $n ->
        [one] letra
       *[other] letras
    }
settings-unit-points =
    { $n ->
        [one] ponto
       *[other] pontos
    }
settings-unit-rows =
    { $n ->
        [one] linha
       *[other] linhas
    }
settings-unit-minutes =
    { $n ->
        [one] minuto
       *[other] minutos
    }

## Settings sections.

section-highlight = Realce
section-reading = Leitura
section-display = Exibição
section-editing = Edição
section-library = Biblioteca e ferramentas de estudo
section-keyboard = Teclado e idioma
section-accessibility = Acessibilidade e anúncios
section-export = Exportar
section-braille = Braille

## Settings by task (beta 4): sections, and when a setting applies.

section-voice = Voz e fala
section-pauses = Pausas e detalhamento
section-speaking-text = Leitura do texto
section-documents = Documentos
section-math = Matemática
section-morse = Código Morse
section-morse-practice = Prática de Morse
section-spacing = Espaçamento, fonte e régua
section-word-aids = Recursos para palavras
section-rsvp = Palavra por palavra (RSVP)
section-window = Barras e painéis
# $n is the number of sections.
settings-intro-sections =
    { $n ->
        [one] Configurações, 1 seção. Enter abre uma seção. Digite para buscar em todas as configurações. Escape fecha.
       *[other] Configurações, { $n } seções. Enter abre uma seção. Digite para buscar em todas as configurações. Escape fecha.
    }
# $section is a section-* title, $n its number of settings.
settings-section-row =
    { $n ->
        [one] { $section }, 1 configuração
       *[other] { $section }, { $n } configurações
    }
settings-all = Todas as configurações
settings-all-row =
    { $n ->
        [one] Todas as configurações, 1 configuração
       *[other] Todas as configurações, { $n } configurações
    }
settings-title-section = Configurações: { $section }
settings-section-opened =
    { $n ->
        [one] { $section }, 1 configuração. Esquerda e Direita mudam um valor, Enter muda ou digita um, Delete restaura o padrão, Escape volta às seções.
       *[other] { $section }, { $n } configurações. Esquerda e Direita mudam um valor, Enter muda ou digita um, Delete restaura o padrão, Escape volta às seções.
    }
settings-all-opened =
    { $n ->
        [one] Todas as configurações, 1 configuração, inclusive as que não se aplicam agora. Escape volta às seções.
       *[other] Todas as configurações, { $n } configurações, inclusive as que não se aplicam agora. Escape volta às seções.
    }
settings-back = Seções das configurações.
settings-section-now =
    { $n ->
        [one] { $section } agora mostra 1 configuração.
       *[other] { $section } agora mostra { $n } configurações.
    }
settings-section-help =
    { $n ->
        [one] { $section }, 1 configuração. Enter a abre.
       *[other] { $section }, { $n } configurações. Enter a abre.
    }
# $item is a settings-item row, $why a settings-applies-* sentence.
settings-item-not-now = { $item }. { $why }
# $engine, $first and $second are speech engine names, such as Eloquence.
settings-applies-engine = Aplica-se quando { $engine } é o motor de fala.
settings-applies-engines = Aplica-se quando { $first } ou { $second } é o motor de fala.
settings-applies-not-engine = Aplica-se quando { $engine } não é o motor de fala.
# $os is Windows, macOS or Linux.
settings-applies-os = Aplica-se só no { $os }.
