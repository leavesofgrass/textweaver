### Authoring: lists, citations, find and replace, tables, quick wins, and dictation.

## Authoring lists: the outline, the citation picker, spelling, grammar, and templates.

# An outline item: $text is the heading's text, $level its level.
lists-outline-item = { $text }, nível { $level }
# $n is the number of headings.
lists-outline-title =
    { $n ->
        [one] Estrutura, { $n } cabeçalho
       *[other] Estrutura, { $n } cabeçalhos
    }
# $shown headings of $n match the filter $filter typed so far.
lists-outline-title-filtered = Estrutura, { $shown } de { $n } correspondem a { $filter }
# $n is the number of references.
lists-citations-title =
    { $n ->
        [one] Inserir citação, { $n } referência
       *[other] Inserir citação, { $n } referências
    }
# $shown references of $n match the filter $filter typed so far.
lists-citations-title-filtered = Inserir citação, { $shown } de { $n } correspondem a { $filter }
# $word is the misspelled word.
lists-spelling-title = Ortografia de { $word }
lists-spelling-add = Adicionar { $word } à sua lista de palavras
lists-leave-as-is = Deixar como está
# $words are the words the grammar fixes are for.
lists-grammar-title = Correções de gramática para { $words }
# $n is the number of templates.
lists-templates-title = Novo documento a partir de um modelo, { $n } modelos
lists-no-filter = Esta lista não filtra.
# The filter was emptied: $n items are shown.
lists-filter-cleared-headings =
    { $n ->
        [one] Filtro limpo, { $n } cabeçalho.
       *[other] Filtro limpo, { $n } cabeçalhos.
    }
lists-filter-cleared-references =
    { $n ->
        [one] Filtro limpo, { $n } referência.
       *[other] Filtro limpo, { $n } referências.
    }
lists-filter-cleared-items =
    { $n ->
        [one] Filtro limpo, { $n } item.
       *[other] Filtro limpo, { $n } itens.
    }
# Nothing matches the filter $query.
lists-filter-none-headings = Nenhum cabeçalho corresponde a { $query }. Backspace remove letras.
lists-filter-none-references = Nenhuma referência corresponde a { $query }. Backspace remove letras.
lists-filter-none-items = Nenhum item corresponde a { $query }. Backspace remove letras.
# $n items match the filter.
lists-filter-matched-headings =
    { $n ->
        [one] { $n } cabeçalho corresponde.
       *[other] { $n } cabeçalhos correspondem.
    }
lists-filter-matched-references =
    { $n ->
        [one] { $n } referência corresponde.
       *[other] { $n } referências correspondem.
    }
lists-filter-matched-items =
    { $n ->
        [one] { $n } item corresponde.
       *[other] { $n } itens correspondem.
    }
lists-no-headings = Este documento não tem cabeçalhos.
# $n is the number of headings; the keys are the outline list's own.
lists-outline-intro =
    { $n ->
        [one] Estrutura, { $n } cabeçalho. Digite para filtrar, Enter vai até um cabeçalho, Escape fecha.
       *[other] Estrutura, { $n } cabeçalhos. Digite para filtrar, Enter vai até um cabeçalho, Escape fecha.
    }
# $heading is the text of the heading the cursor is under.
lists-outline-here = Você está sob { $heading }.

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Citações ligadas.
citations-off = Citações desligadas.
book-audio-on = Áudio do livro ligado. A gravação toca onde o livro tem uma.
book-audio-off = Áudio do livro desligado. A voz lê o livro.
book-audio-rate = As mudanças de velocidade não se aplicam ao áudio gravado.
# $key names the Add Reference command's keys.
citations-library-empty = Sua biblioteca de referências está vazia. Adicione uma referência por DOI, ISBN ou PMID com { $key }, ou execute importar referências na paleta de comandos.
# $n is how many references the picker lists.
citations-picker-intro =
    { $n ->
        [one] Inserir citação, { $n } referência. Digite para filtrar, Enter escolhe, Escape cancela.
       *[other] Inserir citação, { $n } referências. Digite para filtrar, Enter escolhe, Escape cancela.
    }
# $text is what was typed at the locator prompt.
citations-locator-unreadable = Não foi possível ler o localizador { $text }. Digite uma página como 12, páginas como 3-5, ou capítulo 2; Enter sozinho para nenhum.
citations-insert-failed = Não foi possível inserir a citação: { $error } O texto não foi alterado.
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = Procurando { $what }.
citations-lookup-not-started = Não foi possível iniciar a busca: { $error } Aguarde um momento e tente de novo.
# $input is the DOI or ISBN as typed.
citations-lookup-failed = Não foi possível encontrar { $input }: { $error } Verifique o DOI, o ISBN ou o PMID e a conexão.
citations-no-library-to-add-to = Não há biblioteca para adicionar: o { -brand } não guarda arquivos nesta sessão.
citations-library-save-failed = Não foi possível salvar a biblioteca: { $error } Verifique se a pasta dele pode ser gravada.
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] { $n } citação encontrada. O { -brand } não guarda biblioteca nesta sessão.
       *[other] { $n } citações encontradas. O { -brand } não guarda biblioteca nesta sessão.
    }
citations-check-failed = Não foi possível verificar as citações: { $error } Verifique o arquivo da biblioteca de referências.
citations-no-library-to-import-into = Não há biblioteca para importar: o { -brand } não guarda arquivos nesta sessão.
# $file is the file's path.
citations-import-failed = Não foi possível importar { $file }: { $error } Verifique se é um arquivo .bib, .ris ou .json.
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = Não é possível usar o estilo de citação { $style }: { $error } Verifique o nome do estilo no início do documento.
citations-format-failed = Não foi possível formatar as citações: { $error } Verifique as chaves de citação e a biblioteca.
# $key names the Insert Citation command's keys.
citations-none-yet = O documento ainda não tem citações. Insira uma com { $key }.
citations-nothing-to-list = Nenhuma das obras citadas está na sua biblioteca, então não há nada para listar.
# $n is how many entries went in; $style is the style's name, such as apa.
citations-bibliography-inserted =
    { $n ->
        [one] Inseriu a bibliografia, { $n } entrada, estilo { $style }.
       *[other] Inseriu a bibliografia, { $n } entradas, estilo { $style }.
    }
# Follows citations-bibliography-inserted; $keys are citation keys joined with commas.
citations-not-in-library = Não estão na biblioteca: { $keys }.
citations-bibliography-insert-failed = Não foi possível inserir a bibliografia: { $error } O texto não foi alterado.

## Find and replace, one match at a time.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. A linha: { $context }. Pressione r para substituir, s para pular, a para substituir tudo, Escape para parar.
# The replace list's title when no match is being asked about.
replace-title = Substituir
# $n is this match's number, $total the number of matches, $line the line
# number, $found the matched text, and $result what it becomes (both
# shortened); the -removed form is for an empty replacement. $context in
# replace-match-question is the text of the match's line.
replace-match-title = Ocorrência { $n } de { $total }, linha { $line }: { $found } vira { $result }
replace-match-title-removed = Ocorrência { $n } de { $total }, linha { $line }: { $found } é removido
replace-item-this = Substituir esta
replace-item-skip = Pular esta
replace-item-rest = Substituir todo o resto
# $state is common-on or common-off.
replace-item-match-case = Diferenciar maiúsculas: { $state }
replace-item-whole-words = Somente palavras inteiras: { $state }
replace-failed = Não foi possível substituir: { $error } O texto não foi alterado.
# Said after switching match case; $state is common-on or common-off, and
# $n is the number of matches now.
replace-match-case-now =
    { $n ->
        [one] Diferenciar maiúsculas { $state }. 1 ocorrência.
       *[other] Diferenciar maiúsculas { $state }. { $n } ocorrências.
    }
replace-whole-words-now =
    { $n ->
        [one] Somente palavras inteiras { $state }. 1 ocorrência.
       *[other] Somente palavras inteiras { $state }. { $n } ocorrências.
    }
# $query is the text that was searched for.
replace-replaced =
    { $n ->
        [one] Substituiu 1 ocorrência.
       *[other] Substituiu { $n } ocorrências.
    }
replace-replaced-skipped = Substituiu { $n }, pulou { $skipped }.
replace-stopped = Parou. Substituiu { $n }, pulou { $skipped }.
# Find and replace with regular expressions (B1-fr). $state is common-on
# or common-off; $n is the number of matches now.
replace-item-regex = Expressão regular: { $state }
replace-item-across-lines = Entre linhas: { $state }
replace-regex-now =
    { $n ->
        [one] Expressão regular { $state }. 1 ocorrência.
       *[other] Expressão regular { $state }. { $n } ocorrências.
    }
replace-across-lines-now =
    { $n ->
        [one] Entre linhas { $state }. 1 ocorrência.
       *[other] Entre linhas { $state }. { $n } ocorrências.
    }
# $problem is search-invalid-pattern: switching the option would make the
# pattern invalid.
replace-option-refused = { $problem } A opção não mudou.
# Asked once before replacing all the rest; $n is how many. Keep y and n.
replace-all-question =
    { $n ->
        [one] Substituir a última ocorrência? y ou n
       *[other] Substituir todas as { $n } ocorrências restantes? y ou n
    }
replace-all-declined = Nada foi substituído.
# The search options list, and the options as named in search-options-on.
search-options-title = Opções de busca
search-option-match-case = diferenciar maiúsculas
search-option-whole-words = palavras inteiras
search-option-regex = expressão regular
search-option-across-lines = entre linhas
# Said as Find or Replace opens when an option is on; $list joins the
# options' names with commas.
search-options-on = Opções ativadas: { $list }.
# $at is the character where the pattern fails, counting from 1; $reason
# is the regular expression engine's own explanation (in English).
search-invalid-pattern = Padrão inválido no caractere { $at }: { $reason }.
search-invalid-pattern-anywhere = Padrão inválido: { $reason }.

## Markdown structure said in edit mode, before a line's text or as it
## is typed.

mdline-heading-level = cabeçalho nível { $level }
mdline-bullet = marcador
# A numbered list item; $n is its number.
mdline-item = item { $n }
# $item is mdline-bullet or mdline-item.
mdline-task-done = { $item }, tarefa concluída
mdline-task-not-done = { $item }, tarefa não concluída
mdline-table-row = linha de tabela
mdline-quote = citação
mdline-code-fence = cerca de código
mdline-task = tarefa
# Said as "1. " is typed at the start of a line; $n is the number as typed.
mdline-numbered-item = item numerado { $n }

## Moving through tables by row and cell. $dir is next (forward) or
## previous (backward).

common-not-in-table = Fora de uma tabela.
tables-edge-of-table =
    { $dir ->
        [next] Fim da tabela.
       *[previous] Início da tabela.
    }
tables-edge-of-row =
    { $dir ->
        [next] Fim da linha.
       *[previous] Início da linha.
    }
# $cell is the cell's text, after its column header and a colon when it has one.
tables-header-row = Linha de cabeçalho, { $cell }
tables-row = Linha { $row }, { $cell }
# High verbosity: $message is what the move said, then where it is.
tables-with-position = { $message }. Linha { $row } de { $rows }, coluna { $col } de { $cols }
# Say Position in a table.
tables-position = Tabela, linha { $row } de { $rows }, coluna { $col } de { $cols }.

## Authoring quick wins: word count, links, clipboard, table cells,
## deleting words, and cycling settings.

# Code block languages said with ordinary words; proper names such as
# Python are not translated.
authoring-language-jsx = JavaScript com JSX
authoring-language-tsx = TypeScript com JSX
authoring-language-shell = shell
authoring-language-batch = lote do Windows
authoring-language-c-header = cabeçalho C
authoring-language-cpp = C mais mais
authoring-language-csharp = C sharp
authoring-language-diff = diff
authoring-language-plain-text = texto simples
authoring-language-mermaid = diagrama Mermaid
authoring-grammar-not-in-build = A verificação de gramática não está nesta versão.
# $count is $n with thousands separators.
authoring-word-count-selection =
    { $n ->
        [one] 1 palavra na seleção.
       *[other] { $count } palavras na seleção.
    }
authoring-word-count-document =
    { $n ->
        [one] 1 palavra no documento.
       *[other] { $count } palavras no documento.
    }
# $text is the link's text.
authoring-link-address = Endereço do link: { $url }
authoring-link-named-address = Link { $text }, endereço: { $url }
authoring-no-link = Nenhum link no cursor.
authoring-typing-echo =
    { $echo ->
        [characters-and-words] Eco de digitação: caracteres e palavras.
        [characters] Eco de digitação: caracteres.
        [words] Eco de digitação: palavras.
       *[none] Eco de digitação: nenhum.
    }
authoring-nothing-to-copy = Nada selecionado para copiar.
# $text is the first words of what was copied.
authoring-copied = Copiado: { $text }
authoring-copied-sentence = Copiou a frase: { $text }
authoring-nothing-to-cut = Nada selecionado para recortar.
authoring-cut = Recortado: { $text }
# $dir is next (moving forward) or previous.
authoring-table-edge =
    { $dir ->
        [next] Fim da tabela.
       *[previous] Início da tabela.
    }
# A column with no header text.
authoring-table-column = coluna { $n }
# Moving into a new row: $header is the column's header, $content the cell.
authoring-table-cell-row = Linha { $row }. { $header }: { $content }
authoring-nothing-to-select = Nada para selecionar.
authoring-selected-all =
    { $n ->
        [one] Selecionou tudo, 1 palavra.
       *[other] Selecionou tudo, { $count } palavras.
    }
authoring-space-deleted = Espaço excluído.
# $text is the word deleted.
authoring-deleted = { $text } excluída.
# $key is the terminal's own paste key.
authoring-nothing-copied = Nada copiado no { -brand } ainda. Use a colagem do seu terminal, por exemplo { $key }.
# $parts lists what came in, from the paste-part messages.
paste-markdown = Colado como Markdown: { $parts }
paste-part-heading =
    { $n ->
        [one] 1 título
       *[other] { $n } títulos
    }
paste-part-paragraph =
    { $n ->
        [one] 1 parágrafo
       *[other] { $n } parágrafos
    }
paste-part-list =
    { $n ->
        [one] 1 lista
       *[other] { $n } listas
    }
paste-part-table =
    { $n ->
        [one] 1 tabela
       *[other] { $n } tabelas
    }
paste-part-code =
    { $n ->
        [one] 1 bloco de código
       *[other] { $n } blocos de código
    }
paste-part-quote =
    { $n ->
        [one] 1 citação
       *[other] { $n } citações
    }
paste-part-link =
    { $n ->
        [one] 1 link
       *[other] { $n } links
    }
paste-empty = Nada para colar: a área de transferência está vazia.
paste-converting = Convertendo o texto formatado para colar.
paste-failed = Não foi possível colar: { $error } Tente Colar como texto simples.
authoring-verbosity =
    { $level ->
        [low] Verbosidade: baixa.
        [high] Verbosidade: alta.
       *[normal] Verbosidade: normal.
    }
authoring-punctuation =
    { $level ->
        [none] Pontuação: nenhuma.
        [all] Pontuação: toda.
       *[some] Pontuação: alguma.
    }

## Dictation in edit mode (ADR-0042). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $words are the
## dictated words, $key the dictate key, $dir a folder, $error and $text
## are passed on as they are.
dictation-status = Ditando: { $words }
dictation-listening = Ditando. Fale e pressione { $key } para parar.
dictation-finishing = Terminando o ditado.
dictation-done = Ditado concluído.
dictation-busy = O ditado está terminando. Tente de novo em um momento.
dictation-needs-edit = O ditado escreve no modo de edição. Ativar o modo de edição e ditar? y ou n
dictation-no-model = O ditado precisa do modelo Whisper em { $dir }. Veja Dictation na documentação.
dictation-failed = O ditado falhou: { $error } Veja Dictation na documentação.
dictation-no-words = Nenhuma palavra reconhecida nessa frase.
dictation-lost = O ditado parou antes de escrever as últimas palavras.
dictation-not-typed = Palavras ditadas não escritas, o modo de edição está desligado: { $text }
dictation-paused = Ditado pausado.
dictation-resumed = Ditado retomado.
dictation-not-active = Nenhum ditado em andamento.
dictation-cannot-pause = Este ditado não pode ser pausado; pare-o em vez disso.
section-dictation = Ditado
