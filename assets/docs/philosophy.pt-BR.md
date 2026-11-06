# A filosofia do MWP

[English](philosophy.md) · **Português (Brasil)**

O MWP não é uma rede de segurança que impede erros. É uma **linha de produção**: um jeito de trabalhar que mantém um sistema saudável ao longo de uma vida longa, com pessoas e agentes fazendo cada um a parte que fazem melhor.

Pessoas erram. Agentes erram. Isso não vai mudar. O que o MWP muda é de onde um erro pode vir, com que rapidez ele aparece e quão barato é corrigi-lo.

## A premissa: pagar uma vez, depois operar

A maior parte do custo do MWP está no começo: decidir os limites dos módulos, escrever as regras, registrar os testes, documentar as decisões que importam. É trabalho de verdade, e exige pessoas que conhecem o negócio.

Depois disso, a forma do trabalho muda. Em um sistema maduro (um ERP, um produto de longa duração), as regras de negócio são muito mais estáveis que o código. Na maioria dos dias, ninguém reescreve como um pedido é precificado ou como uma nota é fechada. O trabalho são **tasks**: um novo relatório, um campo, uma correção, uma integração. Um módulo só é alterado quando uma task muda o comportamento que ele descreve.

Assim, o estado normal é:

- **muitas tasks**, pequenas e delegadas;
- **poucas mudanças em módulos**, feitas na mesma task que muda o comportamento;
- **decisões raras**, registradas uma vez como ADRs e depois respeitadas.

Estável não significa congelado. Leis, impostos e contratos mudam, e alguns módulos vão mudar com frequência. O ponto é que um módulo muda *quando uma regra muda*, não toda vez que alguém escreve código.

## O ciclo

```
Estruturar  →  Delegar  →  Executar  →  Revisar
    ↑                                      │
    └──────────────────────────────────────┘
```

| Etapa | Quem | O que acontece | O que produz |
|---|---|---|---|
| **Estruturar** | Mantenedores | Definem módulos, regras, testes e decisões, e os mantêm verdadeiros | Módulos, ADRs, constituição |
| **Delegar** | Quem escreve a task | Transforma uma necessidade em uma task pequena, com manifesto claro e critérios verificáveis | Uma task em `tasks/open/` |
| **Executar** | Agente | Carrega só o manifesto, testa primeiro, implementa, atualiza os módulos, conclui com evidência | Código, testes, módulos atualizados, closure |
| **Revisar** | Quem revisa | Confere a evidência, os critérios manuais e as mudanças nos módulos; aceita ou abre uma nova task | Uma task concluída ou uma task de continuação |

A revisão alimenta a próxima rodada de estruturação: um módulo que se mostrou incompleto, uma regra que estava errada, uma decisão que precisa ser revista. Então o ciclo se repete.

O CLI apoia todas as etapas, mas não é dono de nenhuma: `task new` e `context` ajudam na delegação, `next` e `remap` ajudam na execução, `lint` ajuda na revisão. Ele verifica o que não exige julgamento, para que as pessoas só gastem julgamento onde ele é necessário.

## Quem faz o quê

| Papel | Responsável por | Nunca |
|---|---|---|
| **Mantenedores** (uma ou duas pessoas de engenharia que conhecem as regras de negócio) | A verdade dos módulos, aceitar ADRs, a constituição, auditar os módulos contra o código | Deixar um módulo se desatualizar porque "o código é a documentação" |
| **Quem escreve a task** (muitas vezes um mantenedor) | Objetivo, escopo, o manifesto (`modules:`, `reads:`), critérios que dizem como são verificados, o tamanho | Escrever critérios vagos ou deixar o manifesto vazio |
| **Agente** | Executar a task dentro do manifesto, escrever os testes primeiro, atualizar os módulos, concluir com evidência, propor ADRs, parar em conflitos | Mudar um teste existente para fazê-lo passar, editar uma task concluída, aceitar um ADR, resolver um conflito em silêncio |
| **Quem revisa** (uma pessoa) | Conferir a closure e os critérios manuais, aprovar planos de tamanho L | Aprovar uma closure sem evidência |
| **CLI** | IDs, o mapa, referências, closures, testes registrados | Julgar se uma regra ou um critério está certo |

### Por que tão poucos mantenedores

Os módulos são a única fonte de verdade sobre como o sistema se comporta. Uma fonte de verdade com muitos donos deixa de ser uma: as regras acabam escritas duas vezes, com palavras diferentes e exceções diferentes.

Uma ou duas pessoas de engenharia que dominam as regras de negócio conseguem manter coerente um sistema grande, porque não escrevem a maior parte do código. Elas escrevem e protegem as **regras**. Todos os outros, pessoas e agentes, trabalham por meio de tasks em cima dessas regras.

É também aqui que o esforço precisa de disciplina. A estrutura inicial só compensa o custo se continuar verdadeira. Um módulo desatualizado é pior que nenhum módulo, porque as pessoas confiam nele.

## Erros têm endereço

O MWP não tenta tornar os erros impossíveis. Ele torna cada erro **rastreável até o lugar onde nasceu**:

| O erro veio de | Como aparece | Onde é corrigido |
|---|---|---|
| **A task** | Objetivo errado, módulo faltando no manifesto, critérios que não capturaram a necessidade | Quem escreve a task: uma nova task com `refs:` |
| **O módulo** | Uma regra errada ou desatualizada, uma armadilha faltando | Mantenedor: o módulo, em uma nova task |
| **A execução** | O agente não cumpriu um critério, ou a evidência não se sustenta | Revisão: volta para o agente, ou uma nova task |
| **Uma decisão** | Um ADR que não serve mais | Mantenedores: um novo ADR que substitui o anterior |

Como as tasks são imutáveis, as decisões são versionadas e cada critério carrega sua evidência, sempre dá para responder *"por que o sistema faz isso?"* e *"quando isso mudou, e quem pediu?"*. É essa resposta que torna o sistema confiável, não a ausência de erros.

Uma correção é sempre um **novo** registro (uma task, um ADR), nunca a reescrita de um registro antigo. O histórico continua honesto, e a próxima pessoa ou o próximo agente pode aprender com ele.

## A confiança vem da linha, não da perfeição

Quando uma task dá errado, a pergunta nunca é "quem falhou?", e sim "qual elo falhou?":

- A task foi passada corretamente?
- O módulo era verdadeiro?
- A execução cumpriu os critérios, com evidência?
- A revisão conferiu o que tinha de conferir?

Cada elo tem um dono e um artefato visível, então cada falha aponta para um único lugar a melhorar. Com o tempo a linha fica mais forte: as tasks ficam mais claras, os módulos mais precisos e os agentes recebem contexto melhor. É assim que um sistema mantém uma vida longa.

## O que o MWP não tenta fazer

- **Impedir todos os erros.** Ele torna os erros visíveis, localizados e baratos de corrigir.
- **Substituir o julgamento de engenharia.** Limites, regras e decisões são trabalho humano. Agentes propõem; pessoas decidem.
- **Documentar tudo de antemão.** Comece pelo módulo que você vai mexer primeiro. A estrutura cresce com o trabalho.
- **Compensar na primeira semana.** O retorno vem da centésima task, não da primeira.

## Quando não vale a pena

Um protótipo, um script descartável ou um projeto que vai durar poucas semanas não precisa de módulos e ADRs. O MWP compensa quando um sistema tem vida longa, regras de negócio reais e trabalho que pode ser delegado. É exatamente o tipo de sistema em que o esforço inicial é pequeno comparado ao que ele economiza.

## A versão curta

1. **Estruture** com cuidado, com poucos donos que conhecem o negócio.
2. **Delegue** tasks pequenas, com manifestos claros e critérios verificáveis.
3. **Execute** dentro do manifesto, com testes primeiro e evidência no final.
4. **Revise** a evidência, e transforme cada problema em uma nova task.
5. **Repita.** Mantenha os módulos verdadeiros, e a linha se sustenta.
