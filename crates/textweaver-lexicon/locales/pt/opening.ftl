### Opening documents, the file browser, and batch conversion.

## Opening a document.

open-opened = Abriu { $title }.
open-resumed = Abriu { $title }. Retomado em { $pct } por cento.
open-resumed-synced = Abriu { $title }. Retomado em { $pct } por cento, de outro dispositivo.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } é uma pasta, não um documento. Informe o nome de um arquivo dentro dela.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = não há nenhum arquivo chamado { $name } em { $folder }. Verifique o nome.
opening-no-file-here = não há nenhum arquivo chamado { $name } aqui. Verifique o nome.
opening-no-permission = você não tem permissão para lê-lo.
opening-damaged-rtf = não é um arquivo RTF legível; pode estar danificado.
opening-damaged-odt = não é um arquivo de texto OpenDocument legível; pode estar danificado.
opening-damaged-latex = não é um arquivo LaTeX legível; pode estar danificado ou ser grande demais.
opening-damaged-email = não é uma mensagem de e-mail legível; pode estar danificada ou ser grande demais.
opening-damaged-mhtml = não é uma página da web arquivada legível; pode estar danificada ou ser grande demais.
opening-pdf-password = protegido por senha; remova a senha antes, num programa de PDF.
opening-old-office = formato antigo do Office; salve-o como .docx e abra esse arquivo.
opening-rar = é um arquivo RAR, que não abre. Extraia-o primeiro, ou use ZIP ou 7z.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = Não foi possível abrir { $name }: { $reason }
opening-started = Abrindo { $name }. Escape cancela.
opening-stopped = Parou de abrir { $name }.
opening-still = Ainda abrindo { $name }, { $secs } segundos.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = Ainda abrindo { $name }: { $step }
opening-stopped-unexpectedly = Não foi possível abrir { $name }: o carregamento parou inesperadamente.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = As citações não estão nesta versão do { -brand }.
lean-publish-not-in-build = Exportar e pré-visualizar não estão nesta versão do { -brand }. tw convert ainda converte.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } parou inesperadamente.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = A busca por { $input } parou inesperadamente.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = abrir outros programas está desligado em uma sessão que não guarda arquivos
tasks-opening = Abrindo.
# $target is a file or a web address; $error says why.
tasks-could-not-open = Não foi possível abrir { $target }: { $error }
open-refused-scheme = Não aberto: link { $scheme } bloqueado.
open-refused-missing = Não aberto: arquivo não encontrado.
open-refused-invalid = Não aberto: endereço inválido.
open-refused-network = Não aberto: o arquivo está em outro computador.
tasks-not-opened = Não aberto.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = Abrir? y ou n

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = não é um arquivo JSON legível; pode ser grande demais.
opening-damaged-notebook = não é um notebook Jupyter legível; pode estar danificado ou ser grande demais.
opening-damaged-canvas = não é uma tela (canvas) legível; pode estar danificada ou ser grande demais.
opening-damaged-svg = não é um desenho SVG legível; pode estar danificado ou ser grande demais.
opening-damaged-mathml = não é uma fórmula MathML legível; pode estar danificada ou ser grande demais.

## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }, { $k } de { $n }
browse-places-title = Locais
browse-places-intro =
    { $n ->
        [one] Locais, 1 local.
       *[other] Locais, { $n } locais.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }, a pasta do documento
browse-place-start = { $name }, pasta inicial
browse-place-library = { $name }, pasta da biblioteca
browse-place-disk = { $name }, disco
browse-place-removable = { $name }, unidade removível
browse-place-network = { $name }, unidade de rede
browse-place-cd = { $name }, unidade de CD ou DVD
browse-place-root = { $name }, a pasta raiz
browse-choose-here = Escolher esta pasta, { $name }
browse-row-folder = { $name }, pasta
browse-row-folder-items =
    { $name }, pasta, { $n ->
        [one] 1 item
       *[other] { $count } itens
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }, { $kind }, { $size }
browse-row-kind = { $name }, { $kind }
browse-row-hidden = { $row }, oculto
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = arquivo compactado { $kind }
browse-kind-file = arquivo
browse-kind-markdown = Markdown
browse-kind-text = texto
browse-kind-html = página web
browse-kind-epub = livro EPUB
browse-kind-docx = documento do Word
browse-kind-rtf = documento RTF
browse-kind-odt = texto OpenDocument
browse-kind-latex = LaTeX
browse-kind-eml = e-mail
browse-kind-mhtml = arquivo da web
browse-kind-pdf = PDF
browse-kind-image = imagem
browse-kind-daisy = livro DAISY
browse-kind-pptx = slides do PowerPoint
browse-kind-sheet = planilha
browse-kind-json = JSON
browse-kind-notebook = caderno do Jupyter
browse-kind-svg = desenho SVG
browse-kind-mathml = fórmula MathML
browse-kind-pandoc = documento lido com o Pandoc
browse-size-bytes =
    { $n ->
        [one] 1 byte
       *[other] { $count } bytes
    }
# $size is a number, with a decimal under 10 ("3.4").
browse-size-kb = { $size } KB
browse-size-mb = { $size } MB
browse-size-gb = { $size } GB
browse-intro =
    { $name }, { $n ->
        [one] 1 item.
       *[other] { $count } itens.
    }
browse-intro-empty = { $name } não tem nada a mostrar.
# $filter is what was typed.
browse-intro-filtered =
    { $name }, { $n ->
        [one] 1 item corresponde a
       *[other] { $count } itens correspondem a
    } { $filter }.
browse-intro-in-archive = { $intro } Em { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] 1 arquivo oculto.
       *[other] { $count } arquivos ocultos.
    }
browse-intro-cut = { $intro } Só os primeiros { $max } são mostrados.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } Enter abre, Backspace sobe, digitar filtra. { $preview } mostra uma prévia, { $choose } escolhe uma pasta, { $sort } ordena, { $all } mostra todos os arquivos. { $item }
browse-sorted-name = Ordenado por nome.
browse-sorted-date = Ordenado por data, o mais recente primeiro.
browse-sorted-size = Ordenado por tamanho, o maior primeiro.
browse-showing-all = Mostrando todos os arquivos.
browse-showing-readable = Mostrando só os arquivos legíveis.
browse-closed = Navegador de arquivos fechado.
browse-read-only = O navegador de arquivos só abre e escolhe arquivos; nunca os altera.
# $key is the Choose Folder key.
browse-choose-a-folder = Escolha uma pasta: Enter abre uma, { $key } a escolhe.
browse-choose-a-file = Escolha um arquivo: Enter escolhe um.
browse-no-archive-folder = Não é possível escolher uma pasta dentro de um arquivo compactado; escolha uma pasta do disco.
browse-nothing-waiting = Nenhum comando espera uma pasta; Enter a abre.
browse-cannot-read = { $name } não é um tipo de arquivo que o textweaver consegue ler.
browse-folder-unreadable = Não foi possível abrir { $name }: { $reason }
browse-archive-too-deep = { $name } está dentro de arquivos compactados demais para ser aberto.
browse-archive-too-large = { $name } é grande demais para ser listado com segurança.
browse-archive-unreadable = { $name } não é um arquivo compactado que o textweaver consegue ler; pode estar danificado.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. Não tem texto.
browse-preview-failed = Não foi possível mostrar uma prévia de { $name }: { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name }: { $n ->
        [one] 1 arquivo
       *[other] { $count } arquivos
    }, { $readable } legíveis. { $names }
browse-preview-archive-folder =
    { $name }, pasta do arquivo compactado, { $n ->
        [one] 1 item.
       *[other] { $count } itens.
    }
browse-preview-folder = { $path }: { $names }
browse-preview-folder-empty = { $path }: nada para ler aqui.
browse-preview-other = { $name }, { $size }; o textweaver não consegue ler este tipo de arquivo.
browse-preview-path = { $path }

## Batch conversion (File, Batch convert). Keep the meaning first.
batch-choose-source = Escolha a pasta a converter
batch-choose-output = Escolha a pasta para os arquivos convertidos
batch-format-title = Converter para
batch-format-intro = Converter { $name } para: escolha um formato, { $n } opções.
batch-where-title = Para onde vão os arquivos
batch-where-intro = Para onde devem ir os arquivos convertidos?
batch-where-converted = Numa pasta converted, { $path }
batch-where-beside = Ao lado de cada arquivo
batch-where-choose = Em outra pasta, escolhida a seguir
batch-nothing = Não há documentos a converter em { $path }.
batch-confirm =
    Converter { $n ->
        [one] 1 arquivo
       *[other] { $n } arquivos
    } para { $format } em { $path }? y ou n
batch-confirm-beside =
    Converter { $n ->
        [one] 1 arquivo
       *[other] { $n } arquivos
    } para { $format } ao lado de cada arquivo? y ou n
batch-started =
    Convertendo { $n ->
        [one] 1 arquivo
       *[other] { $n } arquivos
    } para { $format }. Escape interrompe.
batch-progress = { $percent } por cento convertido, { $done } de { $total } arquivos.
batch-busy = Já convertendo, { $done } de { $total } arquivos. Escape interrompe.
batch-stop-question = Interromper a conversão? Os arquivos já feitos são mantidos. y ou n
batch-stopping = Interrompe depois dos arquivos sendo gravados.
batch-still-converting = A conversão continua.
batch-done =
    { $converted ->
        [one] 1 arquivo convertido
       *[other] { $converted } arquivos convertidos
    } para { $format }; { $skipped } em dia; { $failed } com falha.
batch-stopped =
    Interrompido. { $converted ->
        [one] 1 arquivo convertido
       *[other] { $converted } arquivos convertidos
    }; { $left } não convertidos; { $failed } com falha.
batch-report = Relatório salvo em { $path }.
batch-report-failed = Não foi possível salvar o relatório: { $error } Os arquivos convertidos são mantidos.
batch-inaccessible =
    { $n ->
        [one] 1 arquivo tem
       *[other] { $n } arquivos têm
    } itens não acessíveis; veja o relatório.
batch-failures-title =
    { $n ->
        [one] 1 arquivo com falha
       *[other] { $n } arquivos com falha
    }
batch-failure-item = { $name }: { $reason }
batch-start-failed = Não foi possível começar a converter: { $error } Verifique a pasta e o formato e tente de novo.
batch-thread-stopped = A conversão em lote parou inesperadamente.
