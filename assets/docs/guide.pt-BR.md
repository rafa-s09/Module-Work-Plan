# Usando o MWP: skill e CLI

[English](guide.md) · **Português (Brasil)**

Este guia cobre o uso diário do MWP: instalar o CLI `mwp`, configurar um projeto, o fluxo de trabalho com um agente e todos os comandos. Para as ideias por trás do formato, comece pelo [README](../../README.pt-BR.md); para quem faz o quê e por quê, leia [a filosofia](philosophy.pt-BR.md).

Comandos, campos, nomes de seção (como `## Closure`) e mensagens do CLI aparecem em inglês porque é assim que existem nos arquivos e no terminal.

- [1. Instalar](#1-instalar)
- [2. Configurar um projeto](#2-configurar-um-projeto)
- [3. Conceitos em uma página](#3-conceitos-em-uma-página)
- [4. O fluxo de trabalho](#4-o-fluxo-de-trabalho)
- [5. As skills do agente](#5-as-skills-do-agente)
- [6. Referência do CLI](#6-referência-do-cli)
- [7. Integração contínua](#7-integração-contínua)
- [8. Solução de problemas](#8-solução-de-problemas)

## 1. Instalar

O CLI é escrito em Rust (1.85+) e não tem dependências. Instale o [Rust](https://rustup.rs) e rode o instalador a partir de um clone deste repositório:

**Linux/macOS**
```bash
./src/install.sh
```

**Windows (PowerShell)**
```powershell
.\src\install.ps1
```
Se o PowerShell bloquear scripts: `powershell -ExecutionPolicy Bypass -File .\src\install.ps1`.

Passe a pasta de um projeto para rodar também o `mwp init` nela (`./src/install.sh ../store`). Os dois scripts rodam `cargo install --path src/client --locked --force` e avisam se a pasta de instalação não estiver no seu `PATH`. Confira com:

```bash
mwp --version
```

Os templates e as skills são embutidos no binário durante a compilação, então rode o instalador de novo depois de alterar `templates/` ou `src/skill/`.

O CLI é opcional. Cada comando tem um equivalente manual, listado na [skill mwp](../../src/skill/mwp/SKILL.md#cli-reference).

## 2. Configurar um projeto

```bash
cd seu-projeto
mwp init --name "Store"
```

O `init` cria os arquivos abaixo e **nunca sobrescreve** um arquivo existente. Ele imprime `skip` para os que já estão lá.

```
AGENTS.md                       # aponta para a constituição, para qualquer agente
CLAUDE.md                       # o mesmo ponteiro, para o Claude Code
.claude/skills/mwp/SKILL.md     # skill de fluxo de trabalho (as duas skills são omitidas com --no-skill)
.claude/skills/module/SKILL.md  # skill de módulos
mwp/
├── PROJECT.md
├── constitution.md
├── map.md
├── glossary.md
├── modules/README.md, _template.md
├── tasks/open/.gitkeep, _template-task.md, _template-task-s.md
├── tasks/done/.gitkeep
├── specs/_template.md
├── decisions/_template.md
└── content/.gitkeep
```

Depois:

1. **Preencha `mwp/PROJECT.md`**: o que é o projeto, a stack e os comandos para rodar e testar.
2. **Adapte `mwp/constitution.md`**: substitua cada `<placeholder>` pelas regras do seu projeto (bibliotecas, comando de teste, o que o agente não pode fazer).
3. **Adicione ao `.gitignore`**: o ledger é versionado, o resto de `.mwp/` é local.

   ```gitignore
   .mwp/*
   !.mwp/ledger.json
   ```

4. **Documente o primeiro módulo**: aquele que você vai mexer primeiro, não o sistema inteiro (veja [4.1](#41-documentar-um-módulo)). Em uma base de código existente, a skill `module` pode propor e escrever os módulos para você.

## 3. Conceitos em uma página

| Arquivo | O que guarda | Quando muda |
|---|---|---|
| Módulo | Como uma parte do sistema funciona **hoje**: regras, testes, contratos, armadilhas | Atualizado a cada mudança |
| Task | O que **muda**: objetivo, escopo, critérios de aceite, módulos para ler e escrever | Nunca é editada depois de concluída |
| SPEC | Uma especificação grande demais para um módulo (durável) ou o plano de uma task (transitória) | As transitórias são incorporadas ou arquivadas na closure |
| ADR | **Por que** uma escolha foi feita | Nunca é editado depois de aceito; é substituído por um novo ADR |
| Constituição | As regras do agente | Raramente |
| Content | Recursos opcionais em `mwp/content/`: ícones, imagens, arquivos de design, documentos | Quando recursos são adicionados |

**IDs**

| ID | O que é | Escopo |
|---|---|---|
| `T-0043` | Task | Projeto |
| `SP-0003` | SPEC | Projeto |
| `ADR-0002` | Decisão | Projeto |
| `TEST-0007` | Teste | Projeto |
| `FR-01` / `NFR-01` | Regra funcional / não funcional | Módulo, citada como `order#FR-01` |
| `AC-1` | Critério de aceite | Task |

Os IDs de projeto são globais, sequenciais, têm pelo menos 4 dígitos e nunca são reutilizados. Um código de trabalho da empresa (Jira, chamado) vai no campo `external:` da task, nunca em `id:`.

O **front matter** é o contrato com o CLI. Todo módulo, task, SPEC e ADR começa com YAML, e os arquivos se citam por ID, nunca por caminho. Arquivos que começam com `_` são templates e são ignorados.

## 4. O fluxo de trabalho

### 4.1 Documentar um módulo

Para código que já existe, peça ao agente:

> Use a skill module para criar os módulos deste projeto.

Ele examina o código, **para e mostra os módulos propostos** (ID, tipo, pastas de código e de testes, dependências) e só os escreve depois que você aprovar. Ele termina com um relatório: o que criou, regras sem testes, suspeitas de bugs e tudo o que depende de uma decisão sua. Ele nunca altera código ou testes.

Para escrever um à mão, copie `mwp/modules/_template.md` para `mwp/modules/order.md` (subpastas como `domain/order.md` são aceitas) e preencha o front matter:

```yaml
---
kind: module
id: order
type: domain                 # ui | domain | infra
summary: Order creation and status rules
code: [src/order/]
tests: [tests/order/]
specs: []
depends_on: [customer]
updated: 2026-11-03
---
```

Registre cada teste na tabela `## Tests` com o seu ID. O nome do teste no código começa com o mesmo ID (`TEST-0007: 5% off over R$500`), e é assim que o CLI o encontra.

Depois, verifique o módulo e atualize o mapa:

```
$ mwp remap
ERROR mwp/modules/order.md: `depends_on` cites `customer`, no module has that id
1 error(s), 0 warning(s)
map.md updated (1 modules)
```

### 4.2 Criar uma task

```
$ mwp task new "Volume discount" --modules order --reads customer --external PROJ-1234
create mwp/tasks/open/T-0001-volume-discount.md (size M, status: open)
```

Preencha **Goal** (objetivo), **Scope** (escopo) e **Acceptance criteria** (critérios de aceite). Cada critério diz como é verificado:

| Tipo | Verificado por |
|---|---|
| `[auto: TEST-0007]` | O teste passa |
| `[manual]` | Os passos depois de `Verify:` |
| `[regression]` | Todos os testes dos módulos alterados passam |
| `[doc]` | O módulo e o changelog refletem a mudança |

Tamanhos: **S** usa o template curto; **M** escreve os testes primeiro; **L** também exige um plano aprovado por uma pessoa antes de implementar.

Antes de entregar a task, confira o que o agente vai carregar:

```
$ mwp context T-0001
mwp/constitution.md                                        1010  always
mwp/tasks/open/T-0001-volume-discount.md                    240  always
mwp/modules/order.md                                        610  modules
mwp/modules/customer.md                                     480  reads
total                                                      2340  tokens (estimate)
```

Se o total for grande, divida a task.

### 4.3 Entregar ao agente

> Execute a task T-0001

Você também pode usar o código da empresa (*"Execute PROJ-1234"*). O agente segue a skill:

1. `mwp context T-0001 --paths`, depois lê apenas esses arquivos.
2. Define `status: in_progress` e cria a branch `task/T-0001-volume-discount`.
3. Tamanho L: escreve o plano e para até a aprovação.
4. Escreve os testes primeiro (`mwp next TEST --reserve`) e confirma que falham.
5. Implementa, apenas nos módulos listados em `modules:`.
6. Verifica cada critério.
7. Atualiza os módulos e roda `mwp remap`.
8. Preenche `## Closure` com a evidência, define `status: done`, move o arquivo para `tasks/done/` e roda `mwp lint`.

Se um critério `[manual]` não pôde ser verificado, a task vai para `status: review` e uma pessoa confirma. O agente para e pergunta quando a task contradiz um módulo ou um ADR aceito, quando o código contradiz o seu módulo, ou quando um teste existente falha e parece contradizer a task.

Retrabalho é sempre uma nova task com `refs: [T-0001]`.

### 4.4 Registrar uma decisão

```
$ mwp adr new "Store money in cents" --modules order
create mwp/decisions/ADR-0001-store-money-in-cents.md (status: proposed)
```

O agente cria ADRs como `proposed`. Só uma pessoa os muda para `accepted`. Para substituir uma decisão:

```
$ mwp adr new "Store money as decimal" --supersedes ADR-0001
create mwp/decisions/ADR-0002-store-money-as-decimal.md (status: proposed)
update mwp/decisions/ADR-0001-store-money-in-cents.md (status: superseded, superseded_by: ADR-0002)
```

### 4.5 Manter os módulos em dia

Os módulos se desatualizam quando o código muda fora do fluxo. De tempos em tempos, ou antes de uma task grande, peça:

> Use a skill module para auditar os módulos.

Para cada módulo, o agente compara regras, testes, contratos e dependências com o código, e confere o código contra os ADRs aceitos. Ele corrige divergências factuais (caminhos, dependências, assinaturas, testes não registrados) e **pergunta** sobre o resto: uma regra diferente do código, um comportamento sem regra, um código que viola um ADR. Você pode limitar o alcance: *"audite os módulos order e payment"*.

### 4.6 Usar recursos

Coloque em `mwp/content/` os recursos de que o agente pode precisar: ícones, imagens, arquivos ou exportações do Figma, PDFs, documentos de referência, dados de exemplo. Subpastas por tipo facilitam a navegação:

```
mwp/content/
├── icons/
├── images/
├── design/      # arquivos .fig, exportações, links para o Figma
└── docs/        # PDFs, especificações de clientes, material de referência
```

- Cite um recurso na task pelo caminho: *"Use `mwp/content/icons/cart.svg` no botão de checkout."*
- O agente procura primeiro em `content/`, abre só o que a task precisa e copia um recurso para a pasta de assets do código quando o app o usa. O `content/` guarda a cópia de referência.
- Se um recurso estiver faltando, o agente pergunta em vez de inventar um.
- A pasta é opcional, e o CLI a ignora: `lint`, `map` e `next` nunca a leem.

### 4.7 Ver como as coisas estão

```
$ mwp task-list
T-0001   in_progress  M  Volume discount [PROJ-1234]
T-0002   open         S  Fix rounding

$ mwp adr list --status accepted
ADR-0001  accepted    2026-11-03 Store money in cents
```

## 5. As skills do agente

O `mwp init` instala duas skills em `.claude/skills/` (pule-as com `--no-skill`).

### `mwp`: o fluxo de trabalho

Fonte: [`src/skill/mwp/SKILL.md`](../../src/skill/mwp/SKILL.md). Ela ensina ao agente:

- quais arquivos existem e quais ignorar (templates, tasks concluídas, ADRs substituídos);
- as regras de IDs, incluindo buscar com limite de palavra (`\bT-0043\b`, porque `TEST-0043` contém `T-0043`);
- os passos para executar, criar e concluir uma task, e o comando do CLI para cada passo;
- como encontrar e usar recursos em `mwp/content/`;
- quando parar e perguntar;
- o equivalente manual de cada comando, para projetos sem o CLI.

### `module`: módulos a partir do código

Fonte: [`src/skill/module/SKILL.md`](../../src/skill/module/SKILL.md). Dois modos:

| Modo | Peça | O que faz |
|---|---|---|
| Criar | *"Crie os módulos deste projeto"* | Examina o código, propõe limites e para até a aprovação, escreve os módulos, roda `remap` e `lint` |
| Auditar | *"Audite os módulos"* | Compara cada módulo com seu código, testes e ADRs; corrige divergências factuais; relata o resto |

Os dois terminam com um relatório que cita um arquivo e uma linha, ou o ID de uma regra, para cada achado. A skill só escreve em `mwp/modules/`: nunca altera código, testes ou ADRs aceitos. Rode-a dentro de uma task (tamanho L) para que as linhas do changelog tenham um ID de task.

A skill explica **como** trabalhar; `mwp/constitution.md` guarda as **regras**. Se as duas discordarem, a constituição vence. Não copie regras para a skill, para o `AGENTS.md` ou para o `CLAUDE.md`: isso cria duas fontes de verdade.

**Outros agentes.** O `AGENTS.md` aponta qualquer agente para a constituição, o que basta para seguir o formato. Se o seu agente aceita skills, copie os dois arquivos `SKILL.md` para a pasta de skills dele.

## 6. Referência do CLI

O `mwp` encontra o projeto procurando `mwp/` na pasta atual e nas pastas acima dela. Todo comando termina com código 0 em caso de sucesso e 1 em caso de falha.

### `mwp init [dir]`

Cria a estrutura em `dir` (padrão: pasta atual). Nunca sobrescreve.

| Opção | Efeito |
|---|---|
| `--name <name>` | Preenche o nome do projeto no `PROJECT.md` |
| `--no-skill` | Não instala as skills em `.claude/skills/` |

### `mwp task new <title>`

Cria `mwp/tasks/open/T-XXXX-slug.md` a partir do template, com o próximo ID de task reservado no ledger e a data de hoje em `created:`. Avisa se um módulo ou uma task citados não existem.

| Opção | Efeito |
|---|---|
| `--size S\|M\|L` | Tamanho da task (padrão `M`); `S` usa o template curto |
| `--modules a,b` | Módulos que a task pode escrever |
| `--reads a,b` | Módulos que a task apenas lê |
| `--refs T-XXXX` | Tasks concluídas que o agente pode ler |
| `--external CODE` | Código(s) de trabalho da empresa |

### `mwp task-list` (ou `mwp task list`)

Lista as tasks com ID, status, tamanho, título e códigos externos. Por padrão, mostra as tasks abertas.

| Opção | Efeito |
|---|---|
| `--status <s>` | Apenas este status (`open`, `in_progress`, `blocked`, `review`, `done`) |
| `--module <id>` | Apenas tasks que escrevem neste módulo |
| `--all` | Inclui as tasks concluídas |

### `mwp context <T-XXXX | code>`

Lista os arquivos que o agente carrega para a task: a constituição, a task e o que ela cita em `modules:`, `reads:`, `specs:`, `decisions:` e `refs:`, com uma estimativa de tokens (cerca de 4 bytes por token). Imprime `MISSING` e termina com código 1 para referências que não existem.

| Opção | Efeito |
|---|---|
| `--paths` | Imprime apenas os caminhos, um por linha |

### `mwp next <T | SP | ADR | TEST>`

Imprime o próximo ID livre: o maior número encontrado nos arquivos do MWP, nas pastas de testes dos módulos e no ledger, mais um. A constituição, a skill e os templates não são examinados, porque seus IDs de exemplo não são reais.

| Opção | Efeito |
|---|---|
| `--reserve` | Registra o número em `.mwp/ledger.json`, para que nunca seja entregue de novo, mesmo se o arquivo for apagado |

O `task new` e o `adr new` reservam seus IDs automaticamente.

### `mwp adr new <title>` / `mwp adr list`

| Opção | Efeito |
|---|---|
| `--modules a,b` | (`new`) Módulos regidos pela decisão |
| `--supersedes ADR-XXXX` | (`new`) Marca esse ADR como `superseded`, com `superseded_by:` e a data de hoje |
| `--status <s>` | (`list`) Apenas este status |

### `mwp remap`

Reverifica todos os módulos e regenera a tabela em `map.md`:

- os caminhos em `code:` e `tests:` existem;
- `depends_on:` e `specs:` citam IDs existentes;
- cada TEST da tabela `## Tests` aparece em um arquivo dentro de `tests:`.

| Opção | Efeito |
|---|---|
| `--dry-run` | Apenas verifica; diz se o `map.md` mudaria |

### `mwp map`

Imprime a tabela de módulos montada a partir do front matter deles.

| Opção | Efeito |
|---|---|
| `--write` | Grava a tabela entre `<!-- mwp:map:begin -->` e `<!-- mwp:map:end -->` no `map.md`; o texto fora dos marcadores é mantido |
| `--check` | Termina com código 1 se o `map.md` estiver desatualizado |

### `mwp lint`

Verifica o projeto inteiro. Confere apenas o que não exige julgamento humano:

| Verificação | Nível |
|---|---|
| `kind` conhecido, campos obrigatórios, valores válidos de `status` / `size` / `lifetime` | erro |
| Formato de ID (`T-`, `SP-`, `ADR-` mais pelo menos 4 dígitos) e IDs duplicados | erro |
| Referências a módulos, tasks, SPECs e ADRs que não existem | erro |
| `refs:` citando uma task que não está concluída | erro |
| Status da task não bate com a pasta; task concluída com `## Closure` vazia | erro |
| ADR substituído sem um `superseded_by:` válido | erro |
| `owner:` de uma SPEC não é um módulo nem uma task | erro |
| Verificações de módulo do `remap` | erro |
| `map.md` desatualizado | erro |
| `type` do módulo fora de `ui` / `domain` / `infra`; nome de arquivo que não começa com o ID da task | aviso |
| Acima dos limites de tamanho (PROJECT ~60, constituição ~150, módulo ~200, task ~80 linhas) | aviso |

| Opção | Efeito |
|---|---|
| `--strict` | Também termina com código 1 em caso de avisos |

## 7. Integração contínua

Dois comandos bastam para manter a documentação honesta:

```bash
mwp lint
mwp map --check
```

O `lint` já inclui a verificação do mapa; rode `map --check` sozinho se quiser uma etapa rápida.

## 8. Solução de problemas

| Mensagem | Significado |
|---|---|
| `no mwp/ folder here or in a parent folder` | Rode o comando dentro do projeto, ou `mwp init` antes |
| `MISSING ADR-0009 (decisions)` | A task cita um ID que nenhum arquivo tem. Crie-o ou corrija a task |
| `TEST-0007 is in ## Tests but not found under tests paths` | O teste foi registrado, mas o código dele não contém o ID, ou `tests:` aponta para a pasta errada |
| `map.md: out of date` | Um módulo mudou. Rode `mwp remap` |
| `status is done but ## Closure is empty` | Preencha a closure com a evidência antes de concluir |
| `` `status: done` but the file is in tasks/open `` | Mova o arquivo para `tasks/done/` (ou corrija o status) |
