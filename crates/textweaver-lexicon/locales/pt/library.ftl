### The library, links and footnotes, and the dictionary.

## The library.

# $n is how many documents the scan has found.
library-still-scanning = Ainda examinando a biblioteca: { $n } encontrados até agora.
library-scan-failed = Não foi possível examinar a biblioteca: { $error } Verifique as pastas da biblioteca nas configurações.
library-scanning = Examinando a biblioteca.
library-scan-progress = Examinando a biblioteca: { $n } encontrados até agora.
library-scan-stopped = O exame da biblioteca parou inesperadamente. Abra a biblioteca de novo para tentar outra vez.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = A biblioteca está vazia. Adicione uma pasta nas Configurações, em Pastas da biblioteca, ou abra um arquivo com { $key }.
library-add-folder-choose = Escolha a pasta a adicionar à biblioteca
library-folder-added = { $name } foi adicionada à biblioteca. Abra a biblioteca para ver os seus documentos.
library-folder-already = { $name } já está na biblioteca.
library-intro =
    { $n ->
        [one] Biblioteca, { $n } documento. Digite para filtrar, Enter abre um, F2 edita os detalhes, { $queue } põe-no na fila.
       *[other] Biblioteca, { $n } documentos. Digite para filtrar, Enter abre um, F2 edita os detalhes, { $queue } põe-no na fila.
    }
library-title = Biblioteca

## Following links and footnotes.

links-none-here = Nenhum link ou nota de rodapé no cursor.
# $text is the link's text.
common-link-no-address = O link { $text } não tem endereço.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] Abrir o link de e-mail? y ou n. { $target }
       *[web] Abrir o link da web? y ou n. { $target }
    }
# A link to a file on another computer (a network share). Keep the
# letters y and n: they are the keys that answer. $target is the link.
links-network-question = Abrir um arquivo em outro computador? y ou n. Seu login pode ser enviado a ele. { $target }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Cabeçalho
# $anchor is the heading name the link gives.
links-no-heading = Nenhum cabeçalho chamado { $anchor } neste documento.
# $file is the file the link names.
links-file-not-found = O link vai para { $file }, que não foi encontrado.
links-block-label = Bloco
links-no-block = Nenhum bloco { $id } neste documento.
links-note-not-found = Link para uma nota que não existe: { $name }.
vault-same-name = { $count } notas se chamam { $name }; a mais próxima foi aberta.
vault-properties-count = Propriedades: { $n }.
vault-properties-title = Propriedades
vault-no-properties = Este documento não tem propriedades.
vault-property-row = { $name }: { $value }
vault-property-checked = marcada
vault-property-not-checked = não marcada
vault-no-file = Este documento não está salvo numa pasta, então não tem cofre.
vault-partial = Só parte do cofre foi pesquisada; ele é muito grande.
vault-top-folder = pasta principal
vault-backlink-row =
    { $links ->
        [one] { $title }, 1 link, em { $folder }
       *[other] { $title }, { $links } links, em { $folder }
    }
vault-note-row = { $title }, em { $folder }
vault-backlinks-title = Notas com link para cá
vault-backlinks-intro =
    { $n ->
        [one] Notas com link para cá, 1 nota. Enter abre.
       *[other] Notas com link para cá, { $n } notas. Enter abre uma.
    }
vault-no-backlinks = Nenhuma nota tem link para cá.
vault-unresolved-title = Links para notas que não existem
vault-unresolved-intro =
    { $n ->
        [one] Links para notas que não existem, 1 link.
       *[other] Links para notas que não existem, { $n } links.
    }
vault-no-unresolved = Todo link desta nota leva a uma nota.
vault-note-tags-title = Tags desta nota
vault-note-tags-intro =
    { $n ->
        [one] Tags desta nota, 1 tag. Enter lista as notas com ela.
       *[other] Tags desta nota, { $n } tags. Enter lista as notas com uma.
    }
vault-no-note-tags = Esta nota não tem tags.
vault-tags-title = Tags do cofre
vault-tags-intro =
    { $n ->
        [one] Tags do cofre, 1 tag. Enter lista suas notas.
       *[other] Tags do cofre, { $n } tags. Enter lista as notas de uma tag.
    }
vault-no-tags = Nenhuma tag neste cofre.
vault-tag-row =
    { $n ->
        [one] { $tag }, 1 nota
       *[other] { $tag }, { $n } notas
    }
vault-tag-notes-title = Notas com uma tag
vault-tag-notes-intro =
    { $n ->
        [one] Tag { $tag }, 1 nota. Enter abre.
       *[other] Tag { $tag }, { $n } notas. Enter abre uma.
    }
vault-tag-no-notes = Nenhuma nota tem a tag { $tag }.
# $title is the document's title; $heading is the heading arrived at, $anchor
# the heading a link names; $key names the History Back command's keys;
# $text is the first words at the position arrived at.
links-opened = { $title } aberto. Voltar: { $key }.
links-opened-at = { $title } aberto, em { $heading }. Voltar: { $key }.
links-opened-no-heading = { $title } aberto, no início: nenhum título chamado { $anchor }. Voltar: { $key }.
links-opened-no-block = { $title } aberto, no início: nenhum bloco { $id }. Voltar: { $key }.
links-back-in = De volta em { $title }: { $text }
links-forward-in = Avançou para { $title }: { $text }
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = De volta à referência da nota de rodapé { $label }, linha { $line }.
# $text is the start of the note.
links-footnote = Nota de rodapé { $label }: { $text }
links-footnote-unreferenced = Nenhuma referência à nota de rodapé { $label } no texto.
links-footnote-no-note = A nota de rodapé { $label } não tem nota.

## O filtro da biblioteca, o dicionário e as velocidades.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = Biblioteca, { $shown } de { $n } correspondem a { $filter }
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] Filtro limpo, { $n } documento.
       *[other] Filtro limpo, { $n } documentos.
    }
# No document matches the filter $query.
library-filter-none = Nenhum documento corresponde a { $query }. Backspace remove letras.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] { $n } documento corresponde.
       *[other] { $n } documentos correspondem.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = O dicionário ainda está carregando.

## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = Página { $page }
