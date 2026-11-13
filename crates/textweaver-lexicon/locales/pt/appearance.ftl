### Themes, accessibility modes, and the Lexend font.

## Themes.

message-error = Erro: { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = Não há tema chamado { $name }; usando { $used }.
# $theme is the new theme's display name.
themes-next = Tema { $theme }.
themes-soft-dark-name = { $theme }, escuro suave
themes-choice-aa = { $theme }, atende AA
themes-choice-below-aa = { $theme }, abaixo de AA
themes-below-aa =
    { $count ->
        [one] Abaixo de AA: 1 verificação não atinge o mínimo.
       *[other] Abaixo de AA: { $count } verificações não atingem o mínimo.
    }

## Accessibility modes and the first-run question.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Modo autofala. O textweaver fala tudo.
        [screen-reader] Modo leitor de tela. O textweaver fica em silêncio; seu leitor de tela lê a linha de status.
       *[hybrid] Modo híbrido. O textweaver lê documentos em voz alta; seu leitor de tela fala mensagens e digitação.
    }
# Yes to the first-run question; $key names the keys that change the mode.
access-hybrid-chosen = Modo híbrido. O textweaver lê documentos em voz alta; seu leitor de tela fala mensagens e digitação. { $key } muda o modo.
# No to the first-run question; $key names the keys that change the mode.
access-hybrid-declined = Permanecendo no modo autofala. { $key } muda o modo.
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-hybrid-question = { $reader } está em execução. Usar o modo híbrido, em que o textweaver lê documentos em voz alta e seu leitor de tela fala mensagens e digitação? y ou n
access-a-screen-reader = Um leitor de tela
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = textweaver lê em voz alta
access-window-choice-screen-reader = meu leitor de tela lê
access-window-mode-help = Quem lê na janela: a voz do textweaver, ou só o seu leitor de tela.
access-window-mode-changed =
    { $mode ->
        [screen-reader] Meu leitor de tela lê: textweaver fica em silêncio e envia o texto ao seu leitor de tela. { $key } muda isso.
        [speaks-messages] textweaver lê em voz alta e fala suas mensagens. { $key } muda isso.
       *[reads-aloud] textweaver lê em voz alta; as mensagens vão para seu leitor de tela. { $key } muda isso.
    }

## Lexend downloaded on first choice.
font-download-question = Baixar a fonte { $font }, { $kb } KB, { $licence }? y ou n
font-downloading = Baixando { $font }.
font-downloaded = { $font } baixada e pronta.
font-download-failed = { $font } não baixada: { $error } Outra fonte é usada.
font-download-declined = Não baixada. Outra fonte é usada.
font-download-busy = { $font } ainda está baixando.
font-download-no-folder = Sem pasta de dados para { $font }.
font-download-not-in-build = Esta versão não baixa fontes.
gui-font-to-download = { $family } (baixar, { $kb } KB)
gui-font-downloaded = { $family } (baixada)
