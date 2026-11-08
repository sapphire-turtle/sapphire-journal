# カレンダービュー、起動時の期間、終日の日付

- 日付: 2026-11-08
- 対象:
  - PR 1: `sapphire-journal-api/src/{entry,params,lib}.rs`、
    `sapphire-journal-core/src/{entry,ops,period,cache,parser,labels,text_input/fields}.rs`、
    `sapphire-journal-cli/src/commands/entry.rs`、`sapphire-journal-mcp/src/methods.rs`、
    デスクトップの追従（`screens/{entry_pane,journal_home,kanban}.rs`、`app.rs`）
  - PR 2: `desktop/Cargo.toml`、`desktop/src/{app,settings}.rs`、
    `desktop/src/screens/{journal_home,entry_pane,settings_panel}.rs`、新規 `desktop/src/period_nav.rs`、
    `desktop/src/widgets/{period_picker,month_grid,date_picker}.rs` の削除
  - PR 3: 新規 `desktop/src/screens/calendar/{mod,timeline,month,agenda}.rs`、
    `desktop/src/{app,settings,icons}.rs`
- 前提: origin/main（sapphire-framework 0.28、corundum-framework 0.11）。
  corundum-framework-gui 0.11 の `datetime` フィーチャー（alumina-datetime 0.2 の再エクスポート）。
  sapphire-framework-gui 0.28 の `Shell::with_actions` / `ShellApp::action`。

## 背景

デスクトップのビューはリストと看板の 2 つで、どちらも `HomeState.period`
（文字列、空なら全件）で絞り込んでいる。起動時の期間は空、つまり全件で、件数が
増えると GUI が重くなる。

イベントと期限は `NaiveDateTime` で持っている。日付だけで書くと、読み込み時に
開始は 00:00、終了と期限は 23:59 になり、**書き出すときは必ず
`YYYY-MM-DDTHH:MM` になる**（`naive_datetime_serde`）。そのため、GUI や
`entry modify` で一度保存すると「日付だけ（＝終日）」という情報が消える。
CLI や MCP で期間を指定する前提だったので、これまでは問題にならなかった。
しかしカレンダーで終日の項目を別枠に出すには、この区別が必要になる。

期間ピッカーと月グリッドはデスクトップが自前で持っている。同じ役割の
alumina-datetime のピッカーを corundum-framework-gui が `datetime` として
再エクスポートしている。

## 決めたこと

1. **終日は型で表す。** フラグは作らない。`DateOrTime`（日付のみか、時刻付きか）を
   導入し、書かれたとおりの形で読み書きする。終日のイベントとは start と end が
   どちらも日付のみのもの、終日のタスクとは due が日付のみのものを指す。
2. **既存のデータは移行しない。** すでに `T00:00`〜`T23:59` で保存されたものは、
   時刻付きで 1 日中続く予定として扱う。必要になったら、一括で日付のみに直す
   移行コマンドを別に作る。
3. **カレンダービューを追加する。** 表示するのはイベントと期限付きのタスクだけ。
   見た目は期間の種類で決まる。日なら時間軸、週なら 7 列×時間軸、月なら月グリッド、
   年・範囲・全件なら 1 日 1 行のアジェンダ。今回は見るだけで、編集はしない。
4. **期間は 3 つのビューで共有する**（今の `HomeState.period` をそのまま使う）。
   ツールバーの ◀ ▶ で、日・週・月・年の単位で前後に動かせる。
5. **起動時の期間を設定で選べる。** 選択肢は前回の期間（既定）、今日、今週、今月。
   全件は選択肢に入れない。
6. **レールに「新規エントリー」ボタンを置く。** どのビューからでもサイドペインに
   新しいエントリーを開ける。カレンダーで項目を作るのも、このボタンで行う。
7. **ピッカーは corundum の `datetime` を使う。** デスクトップが
   `corundum-framework-gui` 0.11 に `datetime` フィーチャー付きで直接依存する。
   すでに `corundum-framework-platform` に直接依存しているのと同じ形にする。
   バージョンはフレームワークの使うものと同じなので、Cargo は 1 つのクレートにまとめる。
   sapphire-framework 側の変更は要らない。

PR は 1 → 2 → 3 の順に積む。仕様書はこの 1 本で、計画は 3 つのフェーズに分ける。

## PR 1: `DateOrTime`（api / core / cli / mcp）

### 型

`sapphire-journal-api` に追加する。

```rust
pub enum DateOrTime {
    Date(NaiveDate),
    DateTime(NaiveDateTime),
}
```

- 読み込み（serde と `FromStr`）: `YYYY-MM-DD` は `Date` になる。
  `YYYY-MM-DDTHH:MM`、`…:SS`、`…:SS.fff` は `DateTime` になる（今読める形式はすべて読める）。
- 書き出し（serde と `Display`）: `Date` は `YYYY-MM-DD`。`DateTime` は
  `YYYY-MM-DDTHH:MM`（秒を落とすのは今と同じ）で、`T00:00` も省略しない。
- `start_bound()`: `Date` ならその日の 00:00:00、`DateTime` ならその値を返す。
- `end_bound()`: `Date` ならその日の 23:59:59、`DateTime` ならその値を返す。
- `is_date()` と `date()`（日付の部分）。
- `JsonSchema`: 文字列型。説明文は「`YYYY-MM-DD`（終日）または
  `YYYY-MM-DDTHH:MM`」とする。

### 置き換えるフィールド

| フィールド | 今 | 変更後 |
|---|---|---|
| `EventMeta.start` / `.end`、`EventMetaView` も | `NaiveDateTime` | `DateOrTime` |
| `TaskMeta.due`、`TaskMetaView` も | `Option<NaiveDateTime>` | `Option<DateOrTime>` |

`started_at`、`closed_at`、`created_at`、`updated_at` は自動で付く記録用の時刻なので、
`NaiveDateTime` のままにする。置き換えの後で使われなくなる `naive_datetime_serde::eod` は削除する。

### 判定と比較

- 終日のイベント: start と end がどちらも `Date`。
- 終日のタスク: due が `Date`。
- 混在しているもの（start は日付のみ、end は時刻付きなど）は時刻付きとして扱い、
  日付のみの側は bound の値で比較する。
- 期間フィルタ（`Period::matches`、`Period::overlaps_event`）、ソート、
  「end が start より前でない」という検査は、すべて `start_bound()` と `end_bound()` で比較する。
  期限は `end_bound()` で比較する。日付のみの期限は、その日の終わりまで有効という意味にする。
- 期限切れの判定（`labels.rs`）も `end_bound()` で行う。

### 入力（CLI、MCP、params）

- `EntryNewParams` と `EntryModifyParams` の `task_due` / `event_start` /
  `event_end` は、今と同じく文字列で受け取り、`DateOrTime::from_str` で解釈する。
  たとえば `--due 2026-11-08` は `Date` のまま保存される。
- `period::parse_datetime` と `parse_datetime_end` は、日付や期限の入力には使わなくなる。
  期間の別名の解釈（`parse_period`）は今のまま残す。

### エントリーキャッシュ

- 列は `TEXT` のままで、`DateOrTime` を書き出した文字列を保存する。
  SQL で日付による絞り込みはしていないので、列の追加は要らない。
- 保存している値の意味が変わるので、`SCHEMA_VERSION` を 6 から 7 に上げて、
  キャッシュを作り直させる。

### デスクトップの追従（この PR では最小限）

- 型の変更に合わせてコンパイルが通るようにする。エントリーペインは今のテキスト入力のまま、
  `DateOrTime::from_str` で解釈する。ピッカーへの置き換えは PR 2 で行う。

### バージョン

- `sapphire-journal-api` は 4.x から 5.0.0 にする（破壊的変更）。
- ファイルや IPC でやり取りする形式は文字列のまま変わらないので、既存のファイルはそのまま読める。

### テスト

- 日付と日時が、それぞれ元の形で書き戻されること（ラウンドトリップ）。`T00:00` が残ることも確認する。
- 旧形式（秒付き、小数秒付き）が読めること。
- `start_bound()` と `end_bound()` の値。
- 期間フィルタの境界: `today` で、今日が期限の `Date` のタスクが出ること。
  前日の 23:59 に終わる時刻付きのイベントが出ないこと。
- 終日のイベントが複数日にまたがる場合の `overlaps_event`。
- `entry modify --due 2026-11-08` を実行すると、ファイルに `due: 2026-11-08` と書かれること。

## PR 2: デスクトップの基盤

### 依存関係

```toml
corundum-framework-gui = { version = "0.11.0", default-features = false, features = ["datetime"] }
```

ピッカーにはどれも `datetime::config(ctx)` を渡す。これで Lucide アイコン、
翻訳された文言、強調色が付く。

### 期間の状態

- `HomeState.period` を `String` から `Option<datetime::Period>` に変える。`None` は全件。
- リストと看板は、`first_day()` の 00:00:00 から `last_day()` の 23:59:59 までの
  `sapphire_journal_core::period::Period::Range` を作り、今と同じ `EntryFilter` で絞り込む。
- ツールバーの期間選択は `PeriodPickerButton` にする。選べる種類はすべてで、
  `clearable` を有効にし（全件に戻せる）、`day_badge` でその日のエントリー数を出す。
  `widgets/period_picker.rs` と `widgets/month_grid.rs` は削除する。
- GUI の期間入力欄では CLI の別名（`today`、`this_week`、`none` など）を使えなくなる。
  入力は alumina の書式になる。

### 前へ・次へ（新規 `period_nav.rs`）

- `shift(period, ±1) -> Option<Period>`: `Day` は 1 日、`Week` は 7 日、`Month` は 1 か月、
  `Year` は 1 年動かす。`Range` は `None` を返す（ボタンを無効にする）。
  月の計算は `datetime::calc::add_months` を使う。
- ツールバーの期間選択の左右に ◀ ▶ を置き、すべてのビューで共通に使う。
  期間が `Range` か全件のときは無効にする。今日へ戻るのは、ピッカーにある Today ボタンで行う。

### 起動時の期間（`settings.toml`）

```toml
startup_period = "last"   # last | today | this_week | this_month
last_period = "2026-W45"  # Period を文字列にしたもの。全件なら ""
```

- `StartupPeriod` の列挙型を作り、既定値を `Last` にする。
- 起動時の期間の決め方:
  - `Last`: `last_period` を解釈した値。`""` なら全件。
    値がないとき（初回起動や今の利用者）と、解釈に失敗したときは今週にする。
  - `Today`、`ThisWeek`、`ThisMonth`: 起動したときの今日を含む `Day`、`Week`
    （月曜始まり）、`Month`。
- `last_period` は、今の `view` / `list_layout` と同じく、変わったことを検知したときに保存する。
- `Settings` は `App` と Settings ページで共有するので `Rc<RefCell<Settings>>` にする。

### Settings 画面の「Desktop」ページ

- `with_app_sections` に、Journal ページと並べて `DesktopSettingsPage` を追加する。
- 中身は「起動時の期間」のコンボボックス 1 つ。選んだ時点で `settings.toml` に保存する。
  サーバーとやり取りしないので、Save ボタンは置かない。

### レールの新規エントリーボタン

- `Shell::with_actions(vec![Action::new("new_entry", icons::plus(), "new-entry")])` で置く。
- `ShellApp::action` で `"new_entry"` を受け取り、`home.new_entry(None)` を呼んでサイドペインに開く。
  どのビューからでも、Settings を開いている間でも使える。
- 開いているエントリーに未保存の変更があるときは、今と同じ未保存確認ダイアログを経由する。
- ツールバーの「New Entry」ボタンは削除する。右クリックメニューの「Add child entry」は残す。

### エントリーペインの日付入力

- 期限、イベントの開始、イベントの終了に、それぞれ「終日」のチェックボックスを付ける。
  オンなら `DatePickerButton`、オフなら `DateTimePickerButton` で入力し、
  結果を `DateOrTime::Date` か `DateTime` にする。
- `widgets/date_picker.rs` は削除する。
- イベントの「終日」は開始と終了で共通の 1 つにする。オンにすると両方を日付のみにし、
  オフにすると開始を 09:00、終了を 10:00 にする（どちらも日付は今の値のまま）。

### テスト

- `shift`: 年末を挟む ISO 週、閏年の 2 月、12 月から 1 月、`Range` で `None` になること。
- 起動時の期間の決定: 各選択肢、`last_period` がない場合、空の場合、壊れている場合。
- 旧 `settings.toml`（新しい項目がないもの）が読めること。今ある「旧形式でも読める」テストに加える。
- 期間から core の `Period::Range` への変換の境界。

## PR 3: カレンダービュー

### 組み込み

- `MainView::Calendar` を追加し、レールの List、Kanban の後ろに `icons::calendar()` を置く。
- `screens/calendar/mod.rs` で期間の種類に応じて振り分ける。

| 期間 | 表示 | ファイル |
|---|---|---|
| `Day` | 1 列の時間軸 | `timeline.rs` |
| `Week` | 7 列の時間軸 | `timeline.rs` |
| `Month` | 6 週×7 列のグリッド | `month.rs` |
| `Year`、`Range`、全件 | 1 日 1 行のアジェンダ | `agenda.rs` |

カレンダーに独自の表示切り替えは付けない。見た目は期間だけで決まる。

### 表示する項目と索引

- 対象は、`event` があるエントリーと、`task.due` があるエントリー。
  両方を持つエントリーは 2 つの項目になる。
- ツールバーの検索欄の絞り込みは適用する。期間による絞り込みはしない
  （カレンダーは自分で範囲を描くので）。
- `CalendarItem { path, title, kind: Event { start, end } | Due(DateOrTime), all_day, done }`。
  `done` はタスクが完了か取消の状態であること。
- 日付ごとの索引 `BTreeMap<NaiveDate, Vec<CalendarItem>>` は、エントリー一覧が
  更新されたとき（`Reply::Entries`）か、検索文字列が変わったときにだけ作り直す。
  毎フレーム作ることはしない。
- 複数日にまたがるイベントは、`start_bound()` の日から `end_bound()` の日までの各日に登録する。

### 日・週（`timeline.rs`）

- 上に終日の帯を置き、終日の項目を 1 行ずつ並べる。週表示では列をまたいで帯にはせず、
  各日の列にそれぞれ置く。
- その下に、0〜24 時の縦の時間軸を `ScrollArea` で置く。1 時間あたり 48px で、左に時刻の目盛りを付ける。
- 時刻付きのイベントは、その日の範囲に切り詰めた開始から終了までの高さのブロックで描く。
  最小の高さは 20px にする。
- 重なるイベントは、開始順に、空いている最初の列に割り当てる。
  重なりのまとまりごとに、列数で横幅を等分する。
- 時刻付きの期限は、その時刻の位置に高さ 20px のラベルとして描く。重なりの計算にも加える。
- 今日の列には現在時刻の線を引く。最初に開いたときは 08:00 の位置までスクロールする。
- 週の見出しには曜日と日付を出し、今日を強調する。見出しをクリックすると期間をその日にする。

### 月（`month.rs`）

- 期間の月の 1 日を含む週の月曜から、42 日分を並べる。
  当月以外の日は弱い色、今日は強調色にする。
- 各マスには日付と、収まる数だけのラベルを入れる。終日の項目を先に、
  時刻付きを `HH:MM タイトル` の形でその後に並べる。あふれた分は `+n` と出す。
- 日付か `+n` をクリックすると、期間をその日にする（日表示に切り替わる）。

### アジェンダ（`agenda.rs`）

- `ScrollArea::show_rows` を使い、1 日 1 行（固定の高さ）で、画面に見えている行だけを描く。
- 行の範囲:
  - `Year` と `Range`: `first_day()` から `last_day()` まで。
  - 全件: 今日の前後 36500 日。最初に開いたときは今日の行までスクロールする。
- 各行は左に日付と曜日（例: `11-08 (日)`、年が変わる行には年も付ける）を置き、
  右にラベルを横に並べる。あふれた分は `+n` にする。
  週末の行は弱い背景、今日の行は強調にする。
- 日付をクリックすると、期間をその日にする。

### 項目の見た目と操作

- イベントは強調色の塗りつぶしにする。タスクは枠線とチェックのアイコンで描く。
  完了したタスクは弱い色に取り消し線を付ける。サイドペインで開いている項目には選択の枠を付ける。
- クリックすると `home.open(path)` でサイドペインに開く。
- ホバーすると、タイトルと日時（終日なら日付だけ）を出す。
- この PR では、ドラッグで日時を変える操作や、空いた枠から新しく作る操作は入れない。

### テスト（描画から切り離した計算部分）

- 項目の抽出: イベントのみ、期限のみ、両方、どちらもないエントリー。終日の判定（混在を含む）。
- 複数日のイベントを各日に展開する処理。終日のものと、時刻付きで日をまたぐもの。
- 重なりの列割り当て: 重ならない場合、2 つ重なる場合、連鎖して重なる場合。
- 時刻から縦位置への変換、その日の範囲への切り詰め。
- 月グリッドの 42 日分の先頭（1 日が月曜の月と日曜の月）。
- アジェンダの行番号と日付の相互変換（全件の中心、`Range` の両端）。
- 索引が作り直されるのは、一覧の更新と検索文字列の変更のときだけであること。

## やらないこと

- 既存の `T00:00`〜`T23:59` を日付のみに直す移行コマンド。
- カレンダー上の編集（ドラッグで動かす・伸ばす、空いた枠から作る）。
- 日曜始まりの週（alumina#36 待ち）。
- GUI の期間入力欄で CLI の別名を受け付けること。
- 期間の種類と関係なく、カレンダーの表示を切り替えること。
