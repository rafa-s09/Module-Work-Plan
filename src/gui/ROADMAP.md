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

- **Aparência:** tema (escuro, claro ou do sistema), tamanho e fonte do texto em Settings > Preferences.
- **Configurações e idiomas:** Settings > Preferences escolhe o idioma da GUI (inglês ou português do Brasil), a partir de arquivos JSON em `<config>/mwp-gui/lang/` que se podem copiar e traduzir para outros idiomas.
- **Help > About:** a arte do projeto (`assets/icone/help_art.png`), as versões da GUI e do core, a descrição e a licença.
- **Conflitos com o disco, parte 2:** **Show changes** na barra de conflito lista só as linhas que diferem (as da aba e as do disco, marcadas); um arquivo apagado no disco ganha uma barra com **Save again** e **Close tab**, e o Ctrl+S não o recria sozinho.
- **Filtros do quadro** lembrados por projeto, junto com as abas.
- **CHANGELOG** da GUI (`CHANGELOG.md`), com a 0.1.0, a 0.1.1 e a 0.1.2. O `Cargo.toml` sobe para `0.1.2` quando a versão fechar.
- **Abrir arquivo rápido (Ctrl+P):** File > Go to file filtra os arquivos por nome, ID ou título enquanto se digita.
- **Substituir no projeto:** Replace all na aba Search, depois de confirmar; arquivos abertos mudam só na aba e ficam sem salvar, os outros são gravados.

## Prioridade 1: entra na v0.1.2

Todos os itens foram feitos. O que mudou em cada versão está no [CHANGELOG](CHANGELOG.md).

## Prioridade 2: entra se der tempo

Todos os itens foram feitos.

## Qualidade e manutenção

- **Testar fora do Windows:** Linux e macOS (o `open_external` usa `xdg-open` e `open`, nunca testados).
- **Tema claro:** visto no quadro, no explorador e no editor; ainda falta conferir os diálogos e as barras de conflito, cujas cores fixas foram escolhidas no tema escuro.
- **Testes que faltam:** Recortar / Copiar / Colar pelo menu Edit, arrastar arquivos para a janela, "Copy path", "Show in folder", e os arrastes do quadro e do explorador (hoje vistos só na janela real).
- **Clippy:** o único aviso que sobra é um tipo complexo em `editor.rs` (`type_complexity`).

## Fora da v0.1.2

- **Preview de imagens:** fica com um agente externo, à parte deste roadmap.

Ideias maiores, para uma v0.2 ou depois:

- **WYSIWYG de verdade:** editar direto no texto renderizado. O egui não tem esse componente; seria um projeto à parte.
- **Integração com git:** ver o que mudou desde o último commit, por arquivo.
- **Integração com Agentes** locais e externos via API
- **Execuçao direta** para agentes na task (`Execulte Task`)