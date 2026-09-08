# MLPL Web Framework and TodoMVC

## Architecture

The framework is ordinary MLPL layered over the generic `_web` polling server
and `_sqlite` provider. Rust owns sockets, HTTP framing, connection queues,
SQLite connections, prepared statements, and hard resource bounds. MLPL owns
routing, named parameters, request validation, middleware composition,
authorization, encodings, sessions, controllers, persistence plans, and views.
There are no Rust-to-MLPL callbacks and no Todo or experiment semantics in Rust.

The small modules are:

- `lib/web/router.mlpl`: exact method matching, static segments, and up to two
  nonempty `:name` parameters;
- `lib/web/http.mlpl`: native-envelope validation, ordered MLPL middleware,
  authorization calls, and JSON/HTML packed-byte responses;
- `lib/web/encoding.mlpl`: five-character HTML escaping and bounded
  `application/x-www-form-urlencoded` field lookup;
- `lib/web/session.mlpl`: bounded cookie parsing, a secure `__Host-` cookie,
  and parameterized SQLite session plans.

Route matching has no implicit precedence: the MLPL application declares and
checks routes in order. Middleware is an indexed record of at most 16 ordinary
MLPL function references. The fixed native stages—limits, CORS, token
extraction/verification, and response headers—still surround the MLPL stage as
documented in `network-db-extensions.md`. MLPL maps a verified identity to its
application subject and calls its own authorizer before a controller.

JSON uses the language's bounded encoder and becomes packed UTF-8 bytes. HTML
views must pass untrusted text through `web_html_escape`. Form bodies are
limited to 16,384 characters and 64 fields; V1 decodes `+` and ASCII percent
escapes. Full percent-decoded UTF-8 needs a later byte-oriented helper. Cookie
headers are limited to 8,192 characters. Session IDs are opaque, caller-created
values rather than credentials: the framework stores subject/expiry in SQLite
and emits `Secure; HttpOnly; SameSite=Lax` host-only cookies. Cryptographic
random-ID generation and key management require a future dedicated capability;
predictable IDs must never be used in deployment.

## Standard TodoMVC demo

`demos/todomvc` implements the familiar TodoMVC behavior in MLPL:

- add and edit validated titles;
- stable IDs, toggle, and delete;
- all/active/completed filters and remaining count;
- clear completed;
- method/path controller routing with named `:id` parameters;
- form decoding and server-rendered escaped HTML;
- parameterized create/read/update/delete SQLite plans.

The deterministic model has eight slots so every operation remains visibly
bounded. Persistence uses the returned SQL/parameter records rather than the
in-memory test state. User text never appears in SQL, and rendered titles are
escaped.

## Run the persistent browser application

One command builds the two generic providers, creates the confined data
directory and missing table, starts the loopback server, and prints its URL:

```sh
just todomvc-server
# MLPL TodoMVC listening at http://127.0.0.1:3000/ (also http://localhost:3000/)
```

Browse to `http://localhost:3000/` or `http://127.0.0.1:3000/`. Both loopback
origins are explicitly allowed by the server middleware, while the listener
remains bound only to `127.0.0.1`. Stop the server with Control-C. A restart
opens the same database and uses `CREATE TABLE IF NOT EXISTS`, so existing
items remain visible to the next browser session.

The server-rendered **sw-MLPL Demo: TodoMVC** shell includes
an accessible Octocat corner linking to the public repository and a copyright
and MIT-license footer; these are emitted by `demos/todomvc/view.mlpl` without
external asset requests.

Defaults and overrides:

| Setting | Default | Contract |
|---|---|---|
| `TODOMVC_PORT` | `3000` | Integer `1..65535`; listener remains IPv4 loopback-only |
| `TODOMVC_DATA_DIR` | `<repo>/var/todomvc` | Existing/created absolute SQLite confinement root |
| `TODOMVC_DB_NAME` | `todos.sqlite3` | Relative path confined beneath the data root |

For example, `TODOMVC_PORT=4567 just todomvc-server` is available at
`http://127.0.0.1:4567/`. To remove all TodoMVC state, stop the server and run:

```sh
just todomvc-reset
```

The reset runs `DROP TABLE IF EXISTS todos` through the SQLite extension. It
does not delete unrelated files or tables. The next server start recreates an
empty `todos` table. The data directory is intentionally ignored by Git.

The separate deterministic preview remains available with:

```sh
just todomvc
```

This prints a complete server-rendered TodoMVC document and summary without
opening a socket or database.

## Experiment dashboard example

`demos/experiment-dashboard/crud.mlpl` is the second domain proof required by
the delivery plan. It defines deterministic schema, ordered read, insert,
update, and delete plans for experiment names and numeric metrics. It shares
the exact generic SQLite interface and contains no native domain behavior.

## Source ownership and acceptance

Native mlplunit suites cover route mismatches and named parameters, request
validation, two-stage middleware order, MLPL authorization, JSON, forms, HTML
escaping, cookies/session lookup plans, all TodoMVC transitions, its controller
and escaped view, and both domains' parameterized CRUD plans.

`model.mlpl` owns state transitions, SQL/schema plans, and reconstruction from
query rows. `app.mlpl` owns routes and controller behavior. `view.mlpl` owns the
list, add form, per-item toggle/editor/delete controls, filters, and escaped
HTML. `server.mlpl` composes those files with the generic `_web` and `_sqlite`
providers. Rust contains socket/HTTP/SQLite mechanics but no TodoMVC routes,
schema, HTML, or application decisions.

`scripts/check-todomvc-live` performs deterministic loopback acceptance with a
temporary database: create, edit, toggle, completed filter, stop/restart,
persistence verification, reset, and empty-schema recreation. It requires no
external network.
