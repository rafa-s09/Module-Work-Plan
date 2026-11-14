# mwp-gui: roadmap da v0.1.2

O que entra na próxima versão do editor visual (`mwp-gui`), em ordem de prioridade. Cada item diz **o quê**, **por quê**, **por onde começar** no código e quando está **pronto**.

## A v0.1.1

Tudo o que estava planejado entrou, e o `Cargo.toml` já diz `0.1.1`:

- **Instaladores** com `-Gui` / `--gui`.
- **Editor:** Replace (Ctrl+H) na busca, busca no projeto (Ctrl+Shift+F), IDs clicáveis no preview e com Ctrl+clique no código.
- **Abas:** Ctrl+W, Ctrl+Tab, Close others, Close to the right.
- **Quadro:** arrastar cartões entre colunas, filtros por texto, módulo, agente e tamanho.
- **Explorador:** renomear (F2) e mover arrastando entre pastas.
- **Cabeçalho:** Context da task (como o `mwp context`), Accept / Reject de ADR.
- **Projeto:** File > Project agents (adicionar agentes, atualizar skills).
- **Janela:** lembra tamanho, painéis, seções, modo do editor e abas de cada projeto.
- **Código:** o `main.rs` dividido em `explorer.rs`, `board.rs`, `tabs.rs`, `dialogs.rs`, `search.rs` e `layout.rs`.

## Já na v0.1.2

- **Help > About:** a arte do projeto (`assets/icone/help_art.png`), as versões da GUI e do core, a descrição e a licença.

## Prioridade 1: entra na v0.1.2

### 1. Conflitos com o disco, parte 2
- **O quê:** na barra de conflito, mostrar a diferença linha a linha em vez da versão inteira; avisar quando o arquivo aberto foi apagado no disco.
- **Por quê:** hoje a barra mostra o arquivo inteiro do disco, e achar o que mudou é trabalho manual; salvar um arquivo apagado o recria sem aviso.
- **Por onde começar:** `conflict_bar` e `Editor::refresh` em `editor.rs`. Um diff de linhas simples (maior subsequência comum) basta para arquivos do tamanho de um módulo.
- **Pronto quando:** as linhas só do disco e só da aba aparecem marcadas; um arquivo apagado mostra uma barra com **Save again** e **Close tab**.

### 2. Lembrar os filtros do quadro
- **O quê:** guardar os filtros do quadro por projeto, junto com as abas.
- **Por quê:** a janela já volta como foi deixada; só os filtros se perdem ao fechar.
- **Por onde começar:** `BoardFilter` em `board.rs` e o formato de linhas de `layout.rs`.
- **Pronto quando:** fechar e abrir mantém os filtros; um filtro por um módulo que sumiu é ignorado.

### 3. CHANGELOG
- **O quê:** um `CHANGELOG.md` da GUI, com a v0.1.1 (a lista acima) e o que entrar na v0.1.2.
- **Por quê:** os READMEs dizem o que a GUI faz, não o que mudou entre versões.
- **Pronto quando:** cada versão tem a sua seção, e a v0.1.2 sobe o `Cargo.toml` para `0.1.2`.

## Prioridade 2: entra se der tempo

### 4. Abrir arquivo rápido (Ctrl+P)
- **O quê:** uma caixa que filtra os arquivos de `mwp/` por nome, ID ou título enquanto se digita; Enter abre.
- **Por quê:** num projeto grande, achar um arquivo pelo explorador exige abrir seções; é o atalho mais usado em editores.
- **Por onde começar:** `View.tree` já tem todos os arquivos com ID e título; o diálogo pode seguir o padrão de `dialogs.rs`.

### 5. Substituir no projeto
- **O quê:** um campo **Replace** na aba Search, com **Replace all** em todos os arquivos encontrados.
- **Cuidado:** arquivos abertos com alterações não salvas ficam de fora ou são trocados só na aba; nada é gravado por cima de uma edição.
- **Por onde começar:** `search.rs` (os `Hit` já têm arquivo e posição) e `Editor::replace_all`.

## Qualidade e manutenção

- **Testar fora do Windows:** Linux e macOS (o `open_external` usa `xdg-open` e `open`, nunca testados).
- **Testar o tema claro:** as cores fixas dos ícones e dos destaques foram escolhidas no tema escuro.
- **Testes que faltam:** Recortar / Copiar / Colar pelo menu Edit, arrastar arquivos para a janela, "Copy path", "Show in folder", e os arrastes do quadro e do explorador (hoje vistos só na janela real).
- **Clippy:** o único aviso que sobra é um tipo complexo em `editor.rs` (`type_complexity`).

## Fora da v0.1.2

- **Preview de imagens:** fica com um agente externo, à parte deste roadmap.

Ideias maiores, para uma v0.2 ou depois:

- **WYSIWYG de verdade:** editar direto no texto renderizado. O egui não tem esse componente; seria um projeto à parte.
- **Interface em português:** hoje a GUI é toda em inglês; traduzir exige separar os textos do código.
- **Integração com git:** ver o que mudou desde o último commit, por arquivo.
- **Temas e tamanho de fonte** configuráveis.
