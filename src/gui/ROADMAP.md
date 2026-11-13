# mwp-gui: roadmap da v0.1.1

O que entra na próxima versão do editor visual (`mwp-gui`), em ordem de prioridade. A v0.1.0 é o estado da branch `feature/cli-gui`: explorador com pastas, editor com abas, preview e busca, quadro com o ciclo de vida das tasks, criação e exclusão de módulos, tasks, SPECs, ADRs e content, aviso de conflito com o disco.

Cada item diz **o quê**, **por quê**, **por onde começar** no código e quando está **pronto**.

## Prioridade 1: entra na v0.1.1

Todos os itens foram feitos.

## Prioridade 2: entra se der tempo

### 1. Status de ADR
- **O quê:** botões **Accept** e **Reject** no cabeçalho de um ADR `proposed`.
- **Por onde começar:** `set_field` no core; seguir o padrão de `task_buttons`.

### 2. Agentes do projeto
- **O quê:** um diálogo para adicionar agentes a um projeto existente e atualizar as skills, como o `mwp init --agent` e o `--update-skills`.
- **Por onde começar:** `scaffold::installed`, `scaffold::init` (que nunca sobrescreve) e `scaffold::update_skills`.

### 3. Preview de imagens
- **O quê:** imagens de `content/` abrem numa aba de visualização, e imagens do Markdown aparecem no preview.
- **Por onde começar:** os carregadores de imagem do `egui_extras` e o recurso de imagens do `egui_commonmark` (hoje desligado no `Cargo.toml`). Avaliar o peso das dependências antes.

### 4. Renomear e mover
- **O quê:** renomear e mover arquivos e pastas pelo explorador (arrastar para outra pasta ou F2).
- **Cuidado:** renomear o arquivo de um módulo não muda o ID; mudar o ID exigiria atualizar as citações em tasks, SPECs e no `map.md`, e fica para depois.

### 5. Conflitos com o disco, parte 2
- **O quê:** na barra de conflito, mostrar a diferença linha a linha em vez da versão inteira; avisar quando o arquivo aberto foi apagado no disco (hoje salvar o recria sem aviso).
- **Por onde começar:** `conflict_bar` e `Editor::refresh` em `editor.rs`.

### 6. Filtros no quadro
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
