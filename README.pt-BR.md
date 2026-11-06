<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/icone/icon-dark.svg">
    <img src="assets/icone/icon.svg" alt="Ícone do Module Work Plan" width="140">
  </picture>
</p>

<h1 align="center">Module Work Plan</h1>

<p align="center"><strong>Documentação modular para agentes de IA: cada task carrega só o contexto de que precisa.</strong></p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-202B3C" alt="Licença: Apache 2.0"></a>
  <img src="https://img.shields.io/badge/rust-1.85%2B-E4572E" alt="Rust 1.85+">
</p>

<p align="center"><a href="README.md">English</a> · <strong>Português (Brasil)</strong></p>

---

**Module Work Plan (MWP)** é um formato de documentação para construir software com agentes de IA. Em vez de entregar ao agente uma pilha de documentos a cada pequena mudança, o MWP descreve o sistema **por módulo** (como ele é hoje) e registra cada mudança em uma **task** pequena e imutável, que diz exatamente quais módulos o agente precisa ler.

O MWP foi feito para sistemas de vida longa, com regras de negócio reais e trabalho que pode ser delegado. Não vale a pena para um protótipo ou um script descartável.

## Como funciona

| | Módulo | Task |
|---|---|---|
| Descreve | Como uma parte do sistema funciona **hoje** | O que **muda**, em uma unidade pequena |
| Contém | Regras (`FR-01`, `NFR-01`), os testes que as protegem, contratos, armadilhas | Objetivo, escopo, critérios de aceite, os módulos que pode ler e escrever |
| Vida útil | Atualizado a cada mudança, nunca acumula histórico | Imutável depois de concluída |

**Nasce na task, vive no módulo.** Tudo o que continua verdadeiro depois que a task termina é escrito no módulo antes de ela ser concluída.

Para executar uma task, o agente lê a **constituição**, a **task** e os **módulos que a task lista**, normalmente de 1 a 3. Cada critério de aceite diz como é verificado (`[auto: TEST-0007]`, `[manual]`, `[regression]`, `[doc]`), e a task só é concluída com evidência de cada um.

Quando as fontes discordam, a prioridade é **constituição > ADRs aceitos > módulo > task**. Se uma task contradiz uma regra, o agente para e pergunta em vez de resolver em silêncio.

```
mwp/
├── PROJECT.md        # visão geral curta: stack, comandos
├── constitution.md   # regras do agente: o que ler, o que é permitido, quando parar
├── map.md            # índice de módulos gerado
├── glossary.md       # termos do domínio (opcional)
├── modules/          # o sistema como ele é hoje
├── tasks/open/       # T-0043-order-discount.md
├── tasks/done/       # tasks concluídas, nunca editadas
├── specs/            # SP-0003: especificações grandes demais para um módulo
├── decisions/        # ADR-0002: por que uma escolha foi feita
└── content/          # opcional: ícones, imagens, arquivos de design, documentos
```

## Início rápido

Instale o CLI a partir de um clone deste repositório (requer [Rust](https://rustup.rs)) e configure seu projeto em um único passo:

**Linux/macOS**
```bash
./src/install.sh caminho/do/seu-projeto
```

**Windows (PowerShell)**
```powershell
.\src\install.ps1 caminho\do\seu-projeto
```

Os dois scripts instalam o `mwp` com `cargo install` e rodam `mwp init` na pasta, o que cria `mwp/`, `AGENTS.md`, `CLAUDE.md` e as skills do agente sem sobrescrever nada. Omita a pasta para instalar apenas o CLI.

1. Preencha `mwp/PROJECT.md` e substitua os `<placeholders>` em `mwp/constitution.md`.
2. Adicione `.mwp/*` e `!.mwp/ledger.json` ao `.gitignore`: o ledger é versionado, o resto de `.mwp/` é local.
3. Documente o módulo que você vai mexer primeiro (copie `mwp/modules/_template.md`, ou peça ao agente para usar a skill `module` no código existente) e rode `mwp remap`.
4. Crie uma task: `mwp task new "Order discount" --modules order`.
5. Peça ao agente: *"Execute a task T-0001"*.

Veja o **[guia](assets/docs/guide.pt-BR.md)** para o fluxo completo, as skills e todas as opções do CLI, e **[a filosofia](assets/docs/philosophy.pt-BR.md)** para entender quem faz o quê, e por quê.

## CLI

| Comando | O que faz |
|---|---|
| `mwp init` | Cria a estrutura `mwp/`; nunca sobrescreve |
| `mwp task new` / `mwp task-list` | Cria uma task com o próximo ID / lista as tasks |
| `mwp context T-0001` | Arquivos que o agente vai ler em uma task, com estimativa de tokens |
| `mwp next <T\|SP\|ADR\|TEST>` | Próximo ID livre, com reserva opcional em `.mwp/ledger.json` |
| `mwp adr new` / `mwp adr list` | Cria um ADR proposto (podendo substituir outro) / lista os ADRs |
| `mwp remap` | Reverifica os módulos e regenera o `map.md` |
| `mwp map` | Imprime, grava ou confere o mapa de módulos |
| `mwp lint` | Verifica o projeto inteiro; código de saída 1 se houver erros |

O CLI é opcional: tudo o que ele faz pode ser feito à mão seguindo os templates.

## Exemplo

**[The Black Ledger](examples/pirate-ship)** é um sistema de gestão funcional para um navio pirata, em Python, Angular e SQLite, documentado de ponta a ponta em MWP, por um pirata bêbado. O domínio é absurdo de propósito: o formato molda a estrutura do trabalho, não o tom. O exemplo está em inglês.

Ele tem 8 módulos, um ADR (todo o dinheiro é guardado em reales inteiros), 7 tasks concluídas com suas evidências e 5 tasks abertas que testam um agente: algumas contradizem uma decisão ou um teste registrado, e o agente precisa parar e perguntar.


## Repositório

| Caminho | Conteúdo |
|---|---|
| [`templates/`](templates) | Arquivos que o `mwp init` copia para um projeto |
| [`src/client/`](src/client) | O CLI `mwp` (Rust, sem dependências) |
| [`src/skill/mwp/`](src/skill/mwp) | Skill do agente: o fluxo de trabalho, o CLI e os recursos em `content/` |
| [`src/skill/module/`](src/skill/module) | Skill do agente: cria módulos a partir do código existente ou os audita contra o código e os ADRs |
| [`src/install.sh`](src/install.sh), [`src/install.ps1`](src/install.ps1) | Instaladores para macOS/Linux e Windows |
| [`assets/docs/`](assets/docs) | Guia e filosofia, em inglês e português do Brasil |
| [`examples/pirate-ship/`](examples/pirate-ship) | Um projeto completo feito com MWP, com exercícios para agentes |

## Licença

[Apache 2.0](LICENSE)
