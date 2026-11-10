# rtop

Um clone do [btop++](https://github.com/aristocratos/btop) escrito em Rust: um monitor de
recursos rápido e colorido para o terminal, com CPU, memória, rede, discos e processos.

> Projeto independente, inspirado no visual e nos atalhos do btop++. Não é afiliado ao
> btop++/aristocratos.

```
 rtop v0.1.0 │ 01:00:58 │ up 3d 21:52                                                     wagner@LOQe  7.0.0-43-generic
╭1 cpu ───────────────────────────────────────────────────────2000ms ╮╭3 net ────────────────────────────────wlp0s20f3 ╮
│usage 6%  freq 4.33GHz  temp 44°C  load 0.94 1.19 0.91  12 cores    ││▼ download 0 B/s  top 0 B/s                     │
│100%                                      C0                     3% ││1.00 KiB/s                                      │
│                                          C1                     0% ││                                                │
│                                          C2                    39% ││                                                │
│                                          C3                     0% ││▲ upload 0 B/s  top 0 B/s                       │
│                                          C4                     6% ││1.00 KiB/s                                      │
│                                          C5                     0% ││                                                │
│                                          C6                     9% ││                                                │
│                                          C7                     0% │╰────────────────────────────────────────────────╯
│                                          C8                     6% │╭2 mem ──────────────────────────────────────────╮
│                                          C9                     6% ││Total: 14.8 GiB   Swap: 14.9 GiB                │
│                                         ⢰+2 cores                  ││Used      9.49 GiB                           64%│
╰────────────────────────────────────────────────────────────────────╯│Available 5.35 GiB                           36%│
╭4 proc ──────────────────────────────────────────────────cpu ↓ 2001 ╮│Cached    4.40 GiB                           30%│
│Pid     Program      Command                         Mem      Cpu%  ││Free      973 MiB                             6%│
│289431  rtop-9ca413… /home/wagner/code/oc/oc-india/t  12.0M    19.1 ││Swap      474 MiB / 14.9 GiB                   3%│
│289432  ui::tests::… /home/wagner/code/oc/oc-india/t  12.0M    19.1 ││                                                │
│  7471  gnome-shell  /usr/bin/gnome-shell --mode=ubu   370M     9.6 ││                                                │
│270673  code         /usr/share/code/code --type=gpu   208M     9.6 ││                                                │
│270708  code         /usr/share/code/code --type=ren   406M     9.6 ││                                                │
│  7486  KMS thread   /usr/bin/gnome-shell --mode=ubu   370M     3.2 │╰────────────────────────────────────────────────╯
│257395  Chrome_Chil… /opt/brave.com/brave/brave --ty   155M     3.2 │╭ disks ─────────────────────────────────────────╮
│261687  gdbus        /usr/bin/ptyxis --gapplication-   176M     3.2 ││/           73.1G/136G                       54%│
│270623  code         /usr/share/code/code .            305M     3.2 ││/boot       334M/921M                        36%│
│271144  code         /usr/share/code/code --type=uti   131M     3.2 ││/boot/efi   14.3M/1.05G                       1%│
│271164  code         /usr/share/code/code --type=uti   131M     3.2 ││/home       97.9G/314G                       31%│
│275816  mi-scavenger opencode                          251M     3.2 ││                                                │
│278391  kworker/u49… kworker/u49:0-i915_flip             0B     3.2 ││                                                │
│     1  systemd      /usr/lib/systemd/systemd --swit  15.7M     0.0 ││↓ 0 B/s  ↑ 0 B/s                                │
╰────────────────────────────────────────────────────────────────────╯╰────────────────────────────────────────────────╯
 q quit │  m menu │  ↑↓ select │  f filter │  e tree │  ←→ sort │  t term │  k kill │  enter info │  +/- rate
```

## Recursos

- **CPU** — gráfico de histórico em braille (ou blocos), medidores por núcleo, frequência,
  temperatura, load average e modelo do processador.
- **Memória** — RAM usada, disponível, cache e livre, além de swap, com barras em gradiente.
- **Rede** — detecta a interface da rota padrão (`/proc/net/route`), mostra velocidade atual,
  pico e total, com gráficos separados de download e upload e escala automática.
- **Discos** — uso por ponto de montagem e velocidade agregada de leitura/escrita.
- **Processos** — ordenação por qualquer coluna, filtro ao vivo, visão em árvore, detalhes,
  envio de `SIGTERM`/`SIGKILL`, pausa e suporte a mouse (clique e scroll).
- **Temas** — a paleta padrão do btop++ embutida e leitura de arquivos `.theme` do
  btop/bpytop instalados (`~/.config/btop/themes`, `/usr/share/btop/themes`, ...).
- **Configuração** — persistida em `~/.config/rtop/rtop.conf`.
- **Intervalo de atualização** — de 250 ms a 10 s, ajustável em tempo real com `+`/`-`.

## Instalação

Requisitos: Linux, Rust 1.88+ (edição 2024) e um terminal UTF-8. Para o visual completo,
use um terminal com truecolor e uma fonte que inclua caracteres braille.

```bash
cargo build --release
./target/release/rtop
```

Ou, para instalar no seu `PATH`:

```bash
cargo install --path .
rtop
```

## Atalhos

| Tecla | Ação |
| --- | --- |
| `q` / `Ctrl+C` | sair |
| `m` / `Esc` | menu principal |
| `h` / `?` / `F1` | ajuda |
| `o` / `F2` | opções |
| `↑` / `↓` | selecionar processo |
| `PgUp` / `PgDn` | mover uma página |
| `Home` / `End` | primeiro / último processo |
| `Enter` / `i` | detalhes do processo |
| `f` / `/` | filtrar processos (ao vivo) |
| `Delete` | limpar filtro |
| `e` | alternar visão em árvore |
| `←` / `→` | trocar coluna de ordenação |
| `r` | inverter a ordenação |
| `u` | pausar/retomar a lista |
| `t` | enviar `SIGTERM` (com confirmação) |
| `k` | enviar `SIGKILL` (com confirmação) |
| `+` / `-` | aumentar/diminuir o intervalo de atualização |
| `+` / `-` (árvore) | expandir/recolher os filhos do processo |
| mouse | clique para selecionar, scroll para navegar |

## Configuração

O arquivo é criado automaticamente ao sair:

```ini
# ~/.config/rtop/rtop.conf
update_ms = 2000
graph_symbol = braille   # braille | block
proc_sorting = cpu       # pid | program | command | threads | user | memory | cpu
proc_reversed = false
proc_tree = false
theme = Default
```

Tudo também pode ser alterado pelo menu `o` (Options) sem editar o arquivo.

## Temas

O tema padrão reproduz as cores do btop++. rtop também carrega temas no formato do
btop/bpytop. Basta copiar um arquivo `.theme` para `~/.config/rtop/themes/` ou usar os temas
já instalados pelo btop:

```bash
mkdir -p ~/.config/rtop/themes
cp /usr/share/btop/themes/gruvbox_dark.theme ~/.config/rtop/themes/
```

Depois selecione o tema em `Options → Theme` (ou edite `theme = gruvbox_dark` no arquivo de
configuração). Campos de fundo (`main_bg`) também são respeitados; quando o tema não define
um fundo (ou usa `#00`), rtop pinta preto puro (`#000000`) em toda a tela, inclusive em
terminais com fundo transparente.

## Estrutura do projeto

```
src/
├── main.rs        # loop de eventos, setup/restore do terminal
├── app.rs         # estado, atalhos, ordenação, filtro, árvore, overlays
├── collect.rs     # coleta via sysinfo + deltas de rede/disco
├── config.rs      # configuração persistente
├── theme.rs       # paleta do btop + parser de temas .theme
├── util.rs        # formatação de bytes, percentuais, tempo
├── widgets.rs     # painéis, meters e gráficos (braille/bloco)
└── ui/
    ├── mod.rs     # layout, cabeçalho, rodapé e overlays
    ├── cpu.rs
    ├── mem.rs
    ├── net.rs
    ├── disks.rs
    └── procs.rs
```

Feito com [ratatui](https://ratatui.rs/), [crossterm](https://github.com/crossterm-rs/crossterm),
[sysinfo](https://github.com/GuillaumeGomez/sysinfo) e [chrono](https://github.com/chronotope/chrono).

## Roadmap

- [ ] GPU (NVIDIA/AMD/Intel)
- [ ] Bateria
- [ ] Temperatura por núcleo
- [ ] Troca de interface de rede pelo teclado
- [ ] Presets de layout (como no btop)
- [ ] `--update`, `--theme` e outros argumentos de linha de comando
- [ ] Suporte a macOS/BSD

## Licença

MIT. Veja [LICENSE](LICENSE).

Inspirado por [btop++](https://github.com/aristocratos/btop), de Jakob P. Liljenberg
(aristocratos), distribuído sob Apache-2.0. O formato de temas é compatível por design.
