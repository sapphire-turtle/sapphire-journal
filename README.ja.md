# sapphire-journal（日本語）

> 言語: [English](README.md) | **日本語**

sapphire-framework上に構築された、タスク・イベント・ノート管理アプリ。ファイルベース・ローカルファースト・人間とAIエージェントの協働のために作られました。

> **任意のMCPクライアントから使えます。** AIとの連携は汎用的なMCPサーバーとして実装しているため、名前が同じ[sapphire-agent](https://github.com/sapphire-turtle/sapphire-agent)専用ではありません。MCPクライアント機能を持つAIハーネス（Claude Code、Claude Desktopなど）であれば何でも連携できます。

## コンセプト

- **Markdownをソース・オブ・トゥルースに** — すべてのデータはプレーンな`.md`ファイルに保存され、どんなツールでも読み書きできます
- **SQLiteをキャッシュに** — Markdownファイルの上に素早いクエリとインデックスを実現（予定）
- **バレットジャーナル着想** — タスク・イベント・ノートは対等なPeerとして、それぞれが独立したentryとして存在します。バレットジャーナルがすべてのバレット（タスク・イベント・ノート）を等しく扱うように、sapphire-journalもタイプを問わずすべてのentryを同じように扱います
- **テキストエディタ／IDE互換** — YAMLフロントマター付きのプレーンな`.md`ファイル。特別なツールなしにどんなエディタでも読み書きできます
- **人間とAIの共同編集** — gitやSyncthing同期を介してAIエージェント（Claudeなど）が同じジャーナルを・作成・することを想定して設計されています

## 設計判断

### エントリID：連番ではなくcaretta-id

各entryのファイル名は[caretta-id](https://github.com/fluo10/caretta-id)（0.1秒精度の7文字BASE32識別子、例 `123abcd_my_note.md`）が前置きされます。

連番IDだと、gitやSyncthingで同期された共有ジャーナルで人間とAIエージェントが同時にentryを追加したときに衝突します。caretta-idは現在のUnix時間を0.1秒単位で値に使うため、0.1秒以上離れて作られた2つのentryは必ず異なるIDになります — 中央調停者なしで衝突なしを保証します。

### ファイル配置：`{year}/{id}_{slug}.md`

entryは年ディレクトリ（例 `2026/`）にグループ化され、ジャーナルルートが長期間で埋まりすぎるのを防ぎつつ、階層が浅すぎてナビゲートしづらくならないようにしています。タイトルから作ったスラッグにより、sapphire-journalを開かなくてもファイル名が読みやすくなります。

## データモデル

各ファイルは最も重要なデータ単位である**Entry**です。entryにはフリーフォームのノート、タスクのチェックボックス（`- [ ]`）、またはその両方を含められます。

ノートentry：

```markdown
---
id: '1a2b3c4'
title: Meeting notes
created_at: '2026-03-06T14:00:00'
tags: [work]
---

Discussion points from the team meeting.
```

task entryは`task`ブロックを追加します：

```markdown
---
id: '2b3c4d5'
title: Fix login bug
created_at: '2026-03-06T10:00:00'
tags: [work, backend]
task:
  status: open
  due: '2026-03-08T18:00:00'
---

Reproduction steps and notes here.
```

event entryは`event`ブロックを追加します：

```markdown
---
id: '3c4d5e6'
title: Team sync
created_at: '2026-03-06T09:00:00'
event:
  start: '2026-03-07T15:00:00'
  end: '2026-03-07T16:00:00'
---
```

## インストール

### インストールスクリプト

```sh
# Linux, macOS
curl -fsSL https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.sh | sh -s -- journal
```

```powershell
# Windows
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.ps1))) journal
```

`sapphire-journal` と、同期に使う `sapphire-bridge` を `~/.local/bin` にインストールします。
デスクトップアプリも入れるときは `journal-desktop` を加えてください。オプションは
[インストールスクリプトの README](https://github.com/sapphire-turtle/sapphire-framework/tree/main/install/README.ja.md) を参照してください。

### cargo-binstall

```sh
cargo binstall sapphire-journal
# デスクトップアプリは crates.io にない
cargo binstall --git https://github.com/sapphire-turtle/sapphire-journal sapphire-journal-desktop
```

### ソースから

```sh
cargo install sapphire-journal
```

## CLIの使い方

コマンドリファレンス全体は[cli/README.md](cli/README.md)を参照してください。

**ジャーナル**とは、`.sapphire-journal/`ディレクトリを含むディレクトリツリーのことです。既存のディレクトリを`sapphire-journal workspace init <dir>`でジャーナルにします。

`sapphire-journal`は単一のバイナリです。ジャーナル独自のコマンド（`entry`、`cache`など）と、[sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework)のコマンド（`serve`、`status`、`service`、`workspace`、`mcp`など）が並んでいます。引数なし（または`serve`）で起動するとアプリサーバーになります。ジャーナルを読み書きするのはこのサーバーだけで、ジャーナルを1つ提供して検索用に索引し、ホストの`sapphire-bridge`を通じて同期し、MCPに応答します。それ以外のコマンドとデスクトップアプリはすべてIPC経由のクライアントで、サーバーが提供しているジャーナルに対して働きます（`sapphire-journal workspace init|select <dir>`で選びます）。

```sh
sapphire-journal service install        # またはターミナルで `sapphire-journal serve`
mkdir -p ~/journal
sapphire-journal workspace init ~/journal
sapphire-journal entry new --title "Hello"
```

AIエージェントは`sapphire-journal mcp`（stdio）で接続します。`sapphire-journal config set http.mcp.enabled=true --restart`で有効にすれば（サーバーの`config.toml`の`[http.mcp] enabled = true`）、HTTPの`http://127.0.0.1:3172/mcp`でも接続できます。意味検索にはホストの`sapphire-bridge`が動かす埋め込みモデルを使います（`sapphire-journal embedding show`）。モデルがなければ全文検索になります。

## プロジェクト構成

```
sapphire-journal/
├── cli/                     # sapphire-journalバイナリ：CLIとアプリサーバー
├── desktop/                 # デスクトップGUI（egui）
└── crates/
    ├── sapphire-journal-core/   # データモデル、Markdownパーサ/シリアライザ、SQLiteのentryキャッシュ
    └── sapphire-journal-mcp/    # ジャーナルのサーバーメソッドと、それを呼ぶMCPツール
```

## ステータス

開発初期段階 — CLIとMCPサーバーは、階層と全文検索を含むentry管理について動作します。

## ライセンス

このリポジトリは異なるライセンスのコンポーネントを含みます：

| コンポーネント | ライセンス |
|-----------|---------|
| `sapphire-journal-api` | MIT OR Apache-2.0 |
| `sapphire-journal-core` | MIT OR Apache-2.0 |
| `sapphire-journal` | MIT OR Apache-2.0 |
| `sapphire-journal-mcp` | MIT OR Apache-2.0 |
| `sapphire-journal-desktop` | GPL-3.0-or-later、App Storeマーケットプレイス例外付き |

各コンポーネントのディレクトリにある`LICENSE`（または`LICENSE-MIT` / `LICENSE-APACHE`）ファイルに完全なライセンス文書があります。
