# mwp-gui: roadmap da v0.1.1

O que entra na próxima versão do editor visual (`mwp-gui`), em ordem de prioridade. A v0.1.0 é o estado da branch `feature/cli-gui`: explorador com pastas, editor com abas, preview e busca, quadro com o ciclo de vida das tasks, criação e exclusão de módulos, tasks, SPECs, ADRs e content, aviso de conflito com o disco.

Cada item diz **o quê**, **por quê**, **por onde começar** no código e quando está **pronto**.

## Prioridade 1: entra na v0.1.1

### 1. Instaladores com a GUI
- **O quê:** `src/install.ps1` e `src/install.sh` instalarem também o `mwp-gui`, com uma opção (`-Gui` / `--gui`) ou uma pergunta quando rodam num terminal.
- **Por quê:** hoje só o CLI é instalado; a GUI exige um `cargo install --path src/gui --locked` à parte, que ninguém descobre sozinho.
- **Por onde começar:** a linha `cargo install --path ... client` dos dois scripts.
- **Pronto quando:** `install.ps1 -Gui caminho` deixa `mwp` e `mwp-gui` no PATH; sem a opção, o comportamento atual não muda. Os READMEs citam a opção.

### 2. Arrastar cartões no quadro
- **O quê:** arrastar um cartão de uma coluna para outra muda o status: para **In progress** abre o diálogo de Start; para **Open**, **Blocked** ou **Review** faz o release; para **Done** faz o Close.
- **Por quê:** o ciclo de vida já existe no cabeçalho e no botão direito; arrastar é o gesto esperado num quadro.
- **Por onde começar:** `card()` e o laço das colunas no `ui()`; `TaskCmd` já executa os três passos. O arraste das abas (`tab()`) mostra como manter o id estável.
- **Pronto quando:** soltar fora de uma coluna não faz nada; os mesmos avisos de hoje aparecem (Closure vazio, task com alterações não salvas).

### 3. Lembrar o estado da janela
- **O quê:** guardar tamanho e posição da janela, largura do explorador, altura do Problems, modo do editor (Source / Split / Preview), seções recolhidas e as abas abertas por projeto.
- **Por quê:** hoje tudo volta ao padrão a cada abertura.
- **Por onde começar:** o recurso `persistence` do `eframe` guarda a janela e a memória do egui; as abas por projeto podem ir num arquivo ao lado de `recent.txt` (`<config>/mwp-gui/`).
- **Pronto quando:** fechar e abrir a GUI volta ao mesmo lugar, com as mesmas abas, sem perder nada se o arquivo de estado estiver corrompido.

## Prioridade 2: entra se der tempo

### 4. Contexto da task no cabeçalho
- **O quê:** um botão **Context** no cabeçalho da task, com a lista do que o agente vai ler (constituição, task, módulos, SPECs, ADRs, refs), a estimativa de tokens e os itens que faltam em vermelho, como o `mwp context`.
- **Por onde começar:** a função `context` em `src/client/src/main.rs`; mover a montagem da lista para o core (`ops`) e usar nos dois.

### 5. Status de ADR
- **O quê:** botões **Accept** e **Reject** no cabeçalho de um ADR `proposed`.
- **Por onde começar:** `set_field` no core; seguir o padrão de `task_buttons`.

### 6. Agentes do projeto
- **O quê:** um diálogo para adicionar agentes a um projeto existente e atualizar as skills, como o `mwp init --agent` e o `--update-skills`.
- **Por onde começar:** `scaffold::installed`, `scaffold::init` (que nunca sobrescreve) e `scaffold::update_skills`.

### 7. Preview de imagens
- **O quê:** imagens de `content/` abrem numa aba de visualização, e imagens do Markdown aparecem no preview.
- **Por onde começar:** os carregadores de imagem do `egui_extras` e o recurso de imagens do `egui_commonmark` (hoje desligado no `Cargo.toml`). Avaliar o peso das dependências antes.

### 8. Renomear e mover
- **O quê:** renomear e mover arquivos e pastas pelo explorador (arrastar para outra pasta ou F2).
- **Cuidado:** renomear o arquivo de um módulo não muda o ID; mudar o ID exigiria atualizar as citações em tasks, SPECs e no `map.md`, e fica para depois.

### 9. Conflitos com o disco, parte 2
- **O quê:** na barra de conflito, mostrar a diferença linha a linha em vez da versão inteira; avisar quando o arquivo aberto foi apagado no disco (hoje salvar o recria sem aviso).
- **Por onde começar:** `conflict_bar` e `Editor::refresh` em `editor.rs`.

### 10. Filtros no quadro
- **O quê:** filtrar os cartões por módulo, agente e tamanho, e buscar pelo título.

## Qualidade e manutenção

- **Testar fora do Windows:** Linux e macOS (o `open_external` usa `xdg-open` e `open`, nunca testados).
- **Testar o tema claro:** as cores fixas dos ícones e do destaque da busca foram escolhidas no tema escuro.
- **Testes que faltam:** Recortar / Copiar / Colar pelo menu Edit, arrastar arquivos para a janela, "Copy path" e "Show in folder".
- **Versão:** subir o `mwp-gui` para 0.1.1 no `Cargo.toml` e registrar as mudanças num `CHANGELOG.md`.

## Fora da v0.1.1

Ideias maiores, para uma v0.2 ou depois:

- **WYSIWYG de verdade:** editar direto no texto renderizado. O egui não tem esse componente; seria um projeto à parte.
- **Interface em português:** hoje a GUI é toda em inglês; traduzir exige separar os textos do código.
- **Integração com git:** ver o que mudou desde o último commit, por arquivo.
- **Temas e tamanho de fonte** configuráveis.
- **Vários cursores** e outras edições avançadas no código.
