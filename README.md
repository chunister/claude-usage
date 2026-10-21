# Claude Usage Tray

Aplicativo para macOS, Windows e Linux que mostra o uso da sessão de 5 horas e da semana de 7 dias do Claude. O núcleo do app é Rust e a janela usa a WebView nativa do sistema via Tauri. Electron e o download do Chromium embutido foram removidos.

## Desenvolvimento no macOS

Instale o Rust e as ferramentas de linha de comando do Xcode (`xcode-select --install`). Depois, na pasta do projeto:

```sh
cargo install tauri-cli --version "^2" --locked
cd src-tauri
cargo tauri dev
```

Para gerar o instalador do Mac:

```sh
cargo tauri build --bundles app
```

Os pacotes ficam em `src-tauri/target/release/bundle/`.

## Windows e Linux

O mesmo código é usado nos três sistemas. Cada pacote deve ser compilado no sistema correspondente, que fornece as ferramentas e a WebView nativa necessárias. O workflow em `.github/workflows/build.yml` compila e publica os artefatos da execução para macOS, Windows e Ubuntu; inicie-o manualmente pela aba **Actions** do GitHub ou faça push do projeto.

O workflow gera `.app` no macOS, `.msi` no Windows e `.deb` no Ubuntu. No Windows, instale o Rust, o Microsoft C++ Build Tools e o WebView2. No Ubuntu/Debian, instale as dependências do Tauri:

```sh
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Então, dentro de `src-tauri/`, use `cargo tauri dev` e `cargo tauri build`.

## Dados e credenciais

O app lê `~/.claude/.credentials.json` ou `CLAUDE_CODE_OAUTH_TOKEN`; no macOS também tenta o item `Claude Code-credentials` do Chaves. Ele consulta `GET /api/oauth/usage` diretamente em `api.anthropic.com`. Preferências e último uso ficam no diretório de configuração do aplicativo. O endpoint OAuth de uso não é uma API pública documentada e pode mudar.

## Funcionalidades

- Uso da janela de 5 horas e da semana de 7 dias.
- Atualização configurável de 5 a 60 minutos. O endpoint de uso OAuth não publica uma cota fixa de consultas; o limite mínimo de 5 minutos é uma escolha conservadora do app, não uma garantia contra respostas 429.
- Idiomas inglês, português e espanhol.
- Indicador de sessão, semana ou ambos na bandeja.
- Inicialização junto com a sessão do usuário.
- Cache local do último resultado.
