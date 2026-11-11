# AGENTS.md

## Objetivo

O `rtop` é um monitor interativo de recursos para Linux escrito em Rust. Use o btop++ como referência para a organização dos painéis, os gráficos, os atalhos e a navegação. Em mudanças de paridade, verifique o comportamento correspondente no btop++ e atualize a ajuda e o `README.md` quando a interação mudar.

## Estrutura do projeto

- `src/main.rs` inicializa e restaura o terminal e executa o loop de eventos.
- `src/app.rs` concentra o estado da aplicação, os atalhos e as operações sobre a lista de processos.
- `src/collect.rs` coleta métricas com `sysinfo` e calcula as taxas de rede e disco.
- `src/ui/` define o layout e renderiza os painéis. Reutilize os componentes de `src/widgets.rs` e a formatação de `src/util.rs`.
- `src/config.rs` carrega e salva as preferências. `src/theme.rs` gerencia as paletas e lê temas `.theme` compatíveis com o btop.

## Convenções

- Preserve a versão mínima de Rust definida no `Cargo.toml` e use as dependências existentes sempre que atenderem à mudança.
- Mantenha coleta, estado e renderização nos módulos correspondentes. Use o intervalo real entre amostras ao calcular taxas e trate métricas indisponíveis e processos que desaparecem entre atualizações.
- Preserve a confirmação antes de enviar sinais a processos. Para validar esse fluxo, use um processo descartável iniciado para o teste.
- Mantenha a compatibilidade com configurações e temas existentes. Use português do Brasil na documentação e preserve a consistência dos rótulos da interface.

## Validação

Execute na raiz do repositório:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Para mudanças na interface ou nos eventos, execute `cargo run` em um terminal UTF-8 ou pseudoterminal. Valide o fluxo alterado, redimensione a janela e confira se a saída por `q` e `Ctrl+C` restaura o terminal. Use um diretório temporário via `XDG_CONFIG_HOME` nos testes interativos para preservar as preferências locais.
