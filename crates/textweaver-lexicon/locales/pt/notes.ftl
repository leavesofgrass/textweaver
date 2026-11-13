### Notes, highlights, bookmarks, relations, and find.

## Notes and highlights.

notes-nothing-to-attach = Nada aqui para anexar uma nota.
# $on is the start of the passage the note is on.
notes-added = Nota adicionada em: { $on }
# $tags are the note's tags, joined with commas.
notes-added-with-tags = Nota adicionada com as tags { $tags } em: { $on }
# An item in the notes list. $anchor is the passage; $lost is yes when the
# passage was not found after the file changed.
notes-item =
    { $lost ->
        [yes] { $note }, linha { $line }. Em: { $anchor } Não encontrada depois que o arquivo mudou.
       *[no] { $note }, linha { $line }. Em: { $anchor }
    }
notes-none = Nenhuma nota. Para adicionar uma: { $key }.
notes-list-title = Notas
notes-list-intro =
    { $n ->
        [one] Notas, 1 item. Enter vai até uma nota, Delete a exclui, F2 a edita, Espaço abre suas ligações. C cria um cartão.
       *[other] Notas, { $n } itens. Enter vai até uma nota, Delete a exclui, F2 a edita, Espaço abre suas ligações. C cria um cartão.
    }
# Said on jumping to a note: its text, then the passage it is on.
notes-note-content = { $note }. Em: { $anchor }
# $i is the note's number, $n how many notes there are.
notes-note-label = Nota { $i } de { $n }
notes-deleted = Nota excluída: { $text }.
notes-none-here = Nenhuma nota ou realce aqui.
notes-unchanged = Nota inalterada.
notes-updated = Nota atualizada.
notes-nothing-to-highlight = Nada aqui para realçar.
notes-highlight-removed = Realce removido: { $text }
notes-highlighted-at = Realçado, { $name }, em { $pct } por cento: { $text }
notes-highlighted = Realçado, { $name }: { $text }
# An item in the highlights list. $name is the highlight's palette name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $name }: { $text }, linha { $line }, não encontrado depois que o arquivo mudou
       *[no] { $name }: { $text }, linha { $line }
    }
notes-no-highlights = Nenhum realce. Para criar um: { $key }.
notes-highlights-title = Realces
notes-highlights-intro =
    { $n ->
        [one] Realces, 1 item. Enter vai até um, Delete o remove, F2 muda seu nome, Espaço mostra só seu nome. C cria um cartão.
       *[other] Realces, { $n } itens. Enter vai até um, Delete o remove, F2 muda seu nome, Espaço mostra só seu nome. C cria um cartão.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Realce, { $name }
notes-highlight-changed = Realce mudado para { $name }: { $text }
# $n is the palette entry's number, $key the keys of Highlight with a name.
notes-palette-no-entry = Não há nome de realce { $n } na paleta. Para escolher um nome: { $key }.
notes-highlights-named-title = Realces: { $name }
notes-highlights-named-intro =
    { $n ->
        [one] Realces chamados { $name }, 1 item. Espaço mostra todos os realces.
       *[other] Realces chamados { $name }, { $n } itens. Espaço mostra todos os realces.
    }
notes-palette-title = Nomes de realce
# A palette name: the name, how many highlights have it, its shape and its color.
notes-palette-row =
    { $count ->
        [one] { $name }, 1 realce, { $shape }, { $color }
       *[other] { $name }, { $count } realces, { $shape }, { $color }
    }
notes-palette-intro-highlight = Realçar com qual nome? { $n } nomes. Enter escolhe um.
notes-palette-intro-change = Mudar o realce para qual nome? { $n } nomes. Enter escolhe um.
notes-palette-intro-collect = Reunir os realces de qual nome? { $n } nomes. Enter os escreve como lista.
notes-collect-none = Nenhum realce chamado { $name }.
# The collected list's own title (Markdown).
notes-collect-title = Realces chamados { $name }: { $title }
notes-collect-line = (linha { $line })
# Keep the letters y and n: they are the keys that answer.
notes-collect-saved =
    { $n ->
        [one] 1 realce chamado { $name } salvo como { $file }. Abrir? y ou n. Em { $folder }.
       *[other] { $n } realces chamados { $name } salvos como { $file }. Abrir? y ou n. Em { $folder }.
    }
palette-shape-underline = sublinhado
palette-shape-double-underline = sublinhado duplo
palette-shape-bold = negrito
palette-shape-dotted = sublinhado pontilhado
palette-shape-brackets = colchetes
palette-shape-symbol = símbolo
# Shown while reading reaches a note's passage.
notes-signal = Nota: { $text }
# Said after moving onto a note's passage.
notes-has-note = Tem uma nota: { $text }

## Relations between notes (the knowledge graph as lists).

relations-type-conflicts-with = está em conflito com
relations-type-supports = apoia
relations-type-is-example-of = é um exemplo de
relations-type-cites = cita
relations-type-contradicts = contradiz
relations-type-defines = define
relations-type-extends = amplia
relations-type-see-also = veja também
relations-type-precedes = precede
relations-type-follows = segue
relations-count = Ligações: { $out } de saída, { $in } de entrada.
relations-note-title = Ligações de: { $note }
relations-title-filtered = { $title }, filtro: { $filter }
relations-note-intro = Ligações de { $note }: { $out } de saída, { $in } de entrada. Enter segue uma ligação, F2 a altera, Delete a remove. Digite para filtrar por tipo.
relations-out-item = { $type }: { $target }
relations-target-in = { $note }, em { $doc }
relations-target-missing = uma nota não encontrada
relations-empty-note = Nota vazia
relations-incoming-row =
    { $n ->
        [0] O que liga para cá: nada ainda
        [one] O que liga para cá: 1 nota
       *[other] O que liga para cá: { $n } notas
    }
relations-add-row = Adicionar uma ligação
relations-backlinks-title = O que liga para cá: { $note }
relations-backlinks-intro =
    { $n ->
        [one] O que liga para { $note }: 1 nota. Enter vai até ela. Digite para filtrar por tipo.
       *[other] O que liga para { $note }: { $n } notas. Enter vai até uma. Digite para filtrar por tipo.
    }
relations-backlink-item = { $type } esta, de: { $note }
relations-none-in = Nada liga para esta nota ainda.
relations-types-title = Tipo de ligação para: { $note }
relations-types-intro = Escolha o tipo de ligação, 10 tipos. Digite para filtrar.
relations-targets-title = { $type }: qual nota?
relations-targets-intro =
    { $n ->
        [0] Nenhuma outra nota aqui. Escolha uma nota de outro documento.
        [one] Escolha a nota de destino: 1 nota. Enter cria a ligação.
       *[other] Escolha a nota de destino: { $n } notas. Enter cria a ligação.
    }
relations-other-document-row = Uma nota de outro documento
relations-documents-title = Documentos com notas
relations-documents-intro = Documentos com notas: { $n }. Enter lista as notas de um documento.
relations-document-item =
    { $n ->
        [one] { $title }, 1 nota
       *[other] { $title }, { $n } notas
    }
relations-no-other-documents = Nenhum outro documento da biblioteca tem notas.
relations-linked = Ligado: { $type } { $target }.
relations-changed = Ligação alterada: { $type } { $target }.
relations-already = Já ligado: { $type } { $target }.
relations-removed = Ligação removida: { $type } { $target }.
relations-remove-question = Remover esta ligação? y ou n
relations-nothing-to-remove = Aqui só é possível remover uma ligação.
relations-note-gone = Nota não encontrada: foi excluída, ou o documento dela sumiu.
relations-document-missing = Documento não encontrado: { $file }.
relations-no-note-here = Nenhuma nota aqui. Ligações pertencem a notas; adicione uma: { $key }.
relations-filter-cleared = Filtro limpo, { $n } mostrados.
relations-filter-none = Nada corresponde a { $filter }.
relations-filter-matched = Filtro { $filter }: { $n } mostrados.

## Exportar o grafo de conhecimento (B1-g2).

graph-export-title = Exportar o grafo de conhecimento como
graph-export-intro = Grafo de conhecimento, ligações: { $links }. Escolha um formato; a lista Markdown é o texto para ler.
graph-export-empty = Não há ligações entre notas para exportar. Adicione uma na lista de ligações de uma nota.
graph-format-md = Lista Markdown, o texto para ler
graph-format-json = JSON, para Gephi e Cytoscape
graph-format-dot = DOT, para Graphviz
graph-format-graphml = GraphML, para Gephi, Cytoscape e yEd
graph-format-mermaid = Diagrama Mermaid
graph-format-plantuml = Diagrama PlantUML
graph-format-csv = Lista de arestas CSV, para planilhas
graph-export-saved = Grafo de conhecimento salvo como { $file }. Abrir? y ou n. Em { $folder }.
graph-export-failed = Não foi possível escrever o grafo de conhecimento: { $error } Verifique se a pasta pode ser gravada.

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Escolha um marcador e pressione Delete.
notes-choose-bookmark-rename = Escolha um marcador e pressione F2 para renomeá-lo.
notes-bookmark-deleted = Marcador { $name } excluído.
notes-renaming-bookmark = Renomeando o marcador { $name }.
notes-bookmark-unchanged = Marcador inalterado.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = Já existe um marcador chamado { $name }. Marcador { $old } inalterado.
notes-bookmark-renamed = Marcador { $old } renomeado para { $name }.

## Find, bookmarks, and selection.

marks-cannot-search = Não foi possível buscar: { $error } Verifique o padrão entre as barras.
# $pattern is the text searched for.
marks-no-matches = Nenhuma ocorrência de { $pattern }.
# The label of a match reached by Find, at high verbosity; $number is its place among $n matches.
marks-match-label = Ocorrência { $number } de { $n }
# Find wrapped past an end of the document; $dir is next (to the top) or previous (to the bottom); $message says the match.
marks-find-wrapped =
    { $dir ->
        [next] Voltou ao início. { $message }
       *[previous] Voltou ao fim. { $message }
    }
# $name is the bookmark's name, such as mark1.
marks-bookmark-already-here = O marcador { $name } já está aqui.
common-bookmark-set = Marcador { $name } definido em { $pct } por cento.
marks-no-bookmarks = Nenhum marcador. Para adicionar um: { $key }.
marks-bookmarks-intro =
    { $n ->
        [one] Marcadores, { $n } item. Enter vai até um, Delete o exclui, F2 o renomeia.
       *[other] Marcadores, { $n } itens. Enter vai até um, Delete o exclui, F2 o renomeia.
    }
# One line of the bookmark list; $lost is yes when the bookmark's text was not found after the file changed; $text is the start of its line.
marks-bookmark-item =
    { $lost ->
        [yes] { $name } (não encontrado depois que o arquivo mudou), linha { $line }, { $pct } por cento: { $text }
       *[no] { $name }, linha { $line }, { $pct } por cento: { $text }
    }
marks-bookmarks-title = Marcadores
# The label of a bookmark reached, at high verbosity.
marks-bookmark-label = Marcador { $name }
marks-selection-cleared = Seleção limpa.
# $text is the selected text, shortened.
marks-selected = Selecionado { $text }
