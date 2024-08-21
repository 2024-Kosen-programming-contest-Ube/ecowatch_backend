# 実行方法

example.env を.env に編集し、適切に編集する。\
ecowatch_backend を実行。

# ビルド

## データベースのセットアップ

.env で`DATABASE_URL`を定義する。
もしくは、

```
cargo sqlx prepare
```

を実行する。

sqlx-cli をインストール

```
cargo install sqlx-cli
```

database を作成し、マイグレーションを実行

```
sqlx database create
sqlx migrate run --source ./db/migrations
```
