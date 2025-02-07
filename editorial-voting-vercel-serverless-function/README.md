# editorial-voting-vercel-serverless-function
解説投票サイトの API です。

すべての API は POST リクエストで呼ばれ、リクエストおよびレスポンスは JSON 形式です。

なお、エラーが発生した場合、以下の形式のレスポンスが返されます。

```ts
response: { status: "error", reason: string }
```

## 環境変数
以下の環境変数をすべて設定する必要があります。

- `EDITORIAL_VOTING_AFFILIATION_TOKEN_SALT`: 所属トークンを発行する際のソルト (例: `hello`)
- `EDITORIAL_VOTING_TOKEN_SALT`: トークンを発行する際のソルト (例: `world`)
- `EDITORIAL_VOTING_DATABASE_URL`: データベースファイルの場所 (例: `postgres://username:password@example.com/db?sslmode=require`)

## API

### /status

解説ページの現在のスコアおよび、自分の投票状態を返します。

- `token`: 与えた場合、自分の投票状態を `current_vote` として返します。
- `editorial`: 解説ページの URL (例: `https://atcoder.jp/contests/abc204/editorial/2027` や `https://blog.hamayanhamayan.com/entry/2021/06/07/024119` )
- `score`: 投票の総和です。
- `scores_by_rating`: レーティングの段階ごとの投票の総和です。 (例: `{"0-99":1}`)
- `current_vote`: `none` => 投票していない, `up` => +1, `down` => -1

```ts
request: { token?: string, editorial: string }
response: { status: "success", score: number, scores_by_rating: Map<string, number>, current_vote?: "none" | "up" | "down" }
```

### /create-affiliation-token
AtCoder アカウントと紐つけるための、所属欄での認証に使う所属トークンを発行します。

```ts
request: { atcoder_id: string }
response: { status: "success", affiliation_token: string }
```

### /create-token
AtCoder アカウントを認証し、投票用のトークンを発行します。

AtCoder の所属欄に `/create-affiliation-token` で発行された所属トークンが入っている必要があります。
なお、認証後は所属欄は変更しても構いません。

- `affiliation_token`: `/create-affiliation-token` で発行された所属トークンです。

```ts
request: { atcoder_id: string, affiliation_token: string }
response: { status: "success", token: string }
```

### /vote
解説に投票します。

投票対象となる解説は、 AtCoder の解説ページに登録されている必要があります。

また、連続して投票する場合、一定時間をおく必要があります。

- `token`: `/create-token` で発行されたトークン
- `editorial`: 解説ページの URL
- `vote`: `none` => 投票しない, `up` => +1, `down` => -1

```ts
request: { token: string, editorial: string, vote: "none" | "up" | "down" }
response: { status: "success" }
```