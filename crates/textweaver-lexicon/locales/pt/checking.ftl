### Markdown lint, grammar, and spelling.

## Markdown lint (edit mode). A problem is said after "Lint: ".

lint-heading-level = nível de cabeçalho { $level } depois do nível { $prev }; use o nível { $use }.
# $reference is the link reference's name.
lint-link-reference = a referência de link { $reference } não tem definição.
lint-bare-url = endereço web solto; coloque-o entre colchetes angulares ou transforme-o em um link com nome.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = marcador de lista { $marker }; esta lista usa { $used }.
lint-marker-dash = traço
lint-marker-star = asterisco
lint-marker-plus = mais
lint-marker-other = outro
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = tabulações ou espaços em uma linha vazia.
lint-trailing-tabs-line-end = tabulações ou espaços no fim da linha.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] 1 espaço em uma linha vazia.
       *[other] { $n } espaços em uma linha vazia.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] 1 espaço no fim da linha.
       *[other] { $n } espaços no fim da linha.
    }
# $key turns on edit mode.
lint-not-editing = O Lint verifica o Markdown que você escreve. Ligue o modo de edição com { $key } primeiro.
lint-not-markdown = O Lint verifica Markdown, e este documento não é Markdown.
lint-none = Nenhum problema de Lint.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] Nenhum outro problema de Lint. 1 problema de Lint no total.
       *[other] Nenhum outro problema de Lint. { $count } problemas de Lint no total.
    }
lint-no-earlier =
    { $n ->
        [one] Nenhum problema de Lint anterior. 1 problema de Lint no total.
       *[other] Nenhum problema de Lint anterior. { $count } problemas de Lint no total.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
common-line = Linha { $line }.

## Grammar checking (Harper). $message is Harper's own message, in English.

# $words are the words the problem is about.
grammar-said = Gramática: { $message } As palavras: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = Correção: remover as palavras.
grammar-fix = Correção: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = Remover as palavras
grammar-none = Nenhum problema de gramática encontrado.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] Nenhum outro problema de gramática. 1 problema de gramática no total.
       *[other] Nenhum outro problema de gramática. { $count } problemas de gramática no total.
    }
grammar-no-earlier =
    { $n ->
        [one] Nenhum problema de gramática anterior. 1 problema de gramática no total.
       *[other] Nenhum problema de gramática anterior. { $count } problemas de gramática no total.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } lista as correções.
# Added at high verbosity.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } Nenhuma correção a oferecer.
grammar-fixes =
    { $n ->
        [one] { $words }: 1 correção.
       *[other] { $words }: { $n } correções.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: 1 correção. Enter faz a alteração.
       *[other] { $words }: { $n } correções. Enter faz a alteração.
    }
common-left-as-is = Deixado como está.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Ligue o modo de edição com { $key } para alterar o texto.
grammar-removed = Removido.
grammar-changed = Alterado para { $fix }.
grammar-change-failed = Não foi possível alterar o texto: { $error } Nada foi alterado.

## Spell checking.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = apóstrofo
spell-not-available = A verificação ortográfica não está disponível: esta versão não tem lista de palavras.
spell-none-found = Nenhum erro ortográfico encontrado.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] Nenhum outro erro ortográfico. 1 possível erro ortográfico no total.
       *[other] Nenhum outro erro ortográfico. { $count } possíveis erros ortográficos no total.
    }
spell-no-earlier =
    { $n ->
        [one] Nenhum erro ortográfico anterior. 1 possível erro ortográfico no total.
       *[other] Nenhum erro ortográfico anterior. { $count } possíveis erros ortográficos no total.
    }
# Added at high verbosity.
spell-no-misspelled-word = Nenhuma palavra com erro ortográfico no cursor.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: sem sugestões.
        [one] { $word }: 1 sugestão.
       *[other] { $word }: { $n } sugestões.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: sem sugestões. Enter substitui a palavra.
        [one] { $word }: 1 sugestão. Enter substitui a palavra.
       *[other] { $word }: { $n } sugestões. Enter substitui a palavra.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. Ligue o modo de edição com { $key } para alterar o texto.
spell-replaced = Substituída por { $word }.
spell-replace-failed = Não foi possível substituir: { $error } A palavra não foi alterada.
spell-added-for-session = Adicionou { $word } à sua lista de palavras para esta sessão.
spell-added = Adicionou { $word } à sua lista de palavras.
spell-save-failed = Não foi possível salvar sua lista de palavras: { $error } A palavra é reconhecida até você sair.
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] Nenhum erro ortográfico.
        [one] 1 possível erro ortográfico.
       *[other] { $count } possíveis erros ortográficos.
    }
