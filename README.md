### データベースのセットアップ

.env で`DATABASE_URL`を定義する。

sqlx-cli をインストール

```
cargo install sqlx-cli
```

database を作成し、マイグレーションを実行

```
sqlx migrate add create
sqlx migrate run --source ./db/migrations
```
