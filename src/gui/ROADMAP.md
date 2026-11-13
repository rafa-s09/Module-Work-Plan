# mwp-gui: roadmap da v0.1.1

O que entra na próxima versão do editor visual (`mwp-gui`), em ordem de prioridade. A v0.1.0 é o estado da branch `feature/cli-gui`: explorador com pastas, editor com abas, preview e busca, quadro com o ciclo de vida das tasks, criação e exclusão de módulos, tasks, SPECs, ADRs e content, aviso de conflito com o disco.

Cada item diz **o quê**, **por quê**, **por onde começar** no código e quando está **pronto**.

## Prioridade 1: entra na v0.1.1

### 1. Instaladores com a GUI
- **O quê:** `src/install.ps1` e `src/install.sh` instalarem também o `mwp-gui`, com uma opção (`-Gui` / `--gui`) ou uma pergunta quando rodam num terminal.
- **Por quê:** hoje só o CLI é instalado; a GUI exige um `cargo install --path src/gui --locked` à parte, que ninguém descobre sozinho.
- **Por onde começar:** a linha `cargo install --path ... client` dos dois scripts.
- **Pronto quando:** `install.ps1 -Gui caminho` deixa `mwp` e `mwp-gui` no PATH; sem a opção, o comportamento atual não muda. Os READMEs citam a opção.

### 2. Substituir na busca
- **O quê:** um campo **Replace** na barra de busca, com **Replace** (a ocorrência atual) e **Replace all**. Ctrl+H abre a barra já com o campo.
- **Por quê:** é o par natural do Find; hoje renomear um termo exige editar ocorrência por ocorrência.
- **Por onde começar:** `find_bar` e `Find` em `editor.rs`; `matches()` já devolve os intervalos. Substituir de trás para frente para os índices não mudarem.
- **Pronto quando:** Replace all troca todas as ocorrências de uma vez e pode ser desfeito com um único Ctrl+Z; o contador se atualiza.

### 3. Busca no projeto
- **O quê:** Ctrl+Shift+F abre um painel com os resultados em todos os arquivos de `mwp/` (arquivo, linha, trecho); um clique abre o arquivo na linha, com a busca da aba já posicionada.
- **Por quê:** achar onde um ID (`T-0007`, `FR-03`) ou um termo é citado é a pergunta mais comum num projeto MWP.
- **Por onde começar:** `Project.docs` já tem todos os documentos lidos; reaproveitar `matches()`. O painel pode ficar no lugar do Problems, como uma segunda aba dele.
- **Pronto quando:** a busca respeita "Match case", ignora `content/` binário e abre o resultado com a ocorrência destacada.

### 4. Atalhos e ações de abas
- **O quê:** Ctrl+W fecha a aba, Ctrl+Tab / Ctrl+Shift+Tab trocam, Ctrl+PageUp / Ctrl+PageDown também; no botão direito, **Close others** e **Close to the right**.
- **Por quê:** é o que todo editor tem; hoje só dá para fechar uma a uma com o mouse.
- **Por onde começar:** `Action::CloseTab` já passa pelo aviso de alterações não salvas; fechar várias pode reaproveitar o `pending` com uma lista.
- **Pronto quando:** fechar várias abas com alterações pergunta uma vez só, listando os arquivos.

### 5. Arrastar cartões no quadro
- **O quê:** arrastar um cartão de uma coluna para outra muda o status: para **In progress** abre o diálogo de Start; para **Open**, **Blocked** ou **Review** faz o release; para **Done** faz o Close.
- **Por quê:** o ciclo de vida já existe no cabeçalho e no botão direito; arrastar é o gesto esperado num quadro.
- **Por onde começar:** `card()` e o laço das colunas no `ui()`; `TaskCmd` já executa os três passos. O arraste das abas (`tab()`) mostra como manter o id estável.
- **Pronto quando:** soltar fora de uma coluna não faz nada; os mesmos avisos de hoje aparecem (Closure vazio, task com alterações não salvas).

### 6. IDs clicáveis
- **O quê:** no preview, `T-0007`, `SP-0001`, `ADR-0002` e IDs de módulos viram links que abrem o arquivo; no código, Ctrl+clique faz o mesmo.
- **Por quê:** uma task cita módulos, SPECs e outras tasks o tempo todo; hoje é preciso procurar no explorador.
- **Por onde começar:** `Project::by_id`; no preview, tratar os links antes do `CommonMarkViewer` ou interceptar o clique; no código, a posição do clique já sai do `galley`.
- **Pronto quando:** um ID inexistente não vira link.

### 7. Lembrar o estado da janela
- **O quê:** guardar tamanho e posição da janela, largura do explorador, altura do Problems, modo do editor (Source / Split / Preview), seções recolhidas e as abas abertas por projeto.
- **Por quê:** hoje tudo volta ao padrão a cada abertura.
- **Por onde começar:** o recurso `persistence` do `eframe` guarda a janela e a memória do egui; as abas por projeto podem ir num arquivo ao lado de `recent.txt` (`<config>/mwp-gui/`).
- **Pronto quando:** fechar e abrir a GUI volta ao mesmo lugar, com as mesmas abas, sem perder nada se o arquivo de estado estiver corrompido.

## Prioridade 2: entra se der tempo

### 8. Contexto da task no cabeçalho
- **O quê:** um botão **Context** no cabeçalho da task, com a lista do que o agente vai ler (constituição, task, módulos, SPECs, ADRs, refs), a estimativa de tokens e os itens que faltam em vermelho, como o `mwp context`.
- **Por onde começar:** a função `context` em `src/client/src/main.rs`; mover a montagem da lista para o core (`ops`) e usar nos dois.

### 9. Status de ADR
- **O quê:** botões **Accept** e **Reject** no cabeçalho de um ADR `proposed`.
- **Por onde começar:** `set_field` no core; seguir o padrão de `task_buttons`.

### 10. Agentes do projeto
- **O quê:** um diálogo para adicionar agentes a um projeto existente e atualizar as skills, como o `mwp init --agent` e o `--update-skills`.
- **Por onde começar:** `scaffold::installed`, `scaffold::init` (que nunca sobrescreve) e `scaffold::update_skills`.

### 11. Preview de imagens
- **O quê:** imagens de `content/` abrem numa aba de visualização, e imagens do Markdown aparecem no preview.
- **Por onde começar:** os carregadores de imagem do `egui_extras` e o recurso de imagens do `egui_commonmark` (hoje desligado no `Cargo.toml`). Avaliar o peso das dependências antes.

### 12. Renomear e mover
- **O quê:** renomear e mover arquivos e pastas pelo explorador (arrastar para outra pasta ou F2).
- **Cuidado:** renomear o arquivo de um módulo não muda o ID; mudar o ID exigiria atualizar as citações em tasks, SPECs e no `map.md`, e fica para depois.

### 13. Conflitos com o disco, parte 2
- **O quê:** na barra de conflito, mostrar a diferença linha a linha em vez da versão inteira; avisar quando o arquivo aberto foi apagado no disco (hoje salvar o recria sem aviso).
- **Por onde começar:** `conflict_bar` e `Editor::refresh` em `editor.rs`.

### 14. Filtros no quadro
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
