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
bounded. Production persistence uses the returned SQL/parameter records rather
than the in-memory test state. User text never appears in SQL, and rendered
titles are escaped. Run the executable HTML preview with:

```sh
just todomvc
```

This prints a complete server-rendered TodoMVC document and summary. It does
not claim to start a live server yet; see the host limitation below.

## Experiment dashboard example

`demos/experiment-dashboard/crud.mlpl` is the second domain proof required by
the delivery plan. It defines deterministic schema, ordered read, insert,
update, and delete plans for experiment names and numeric metrics. It shares
the exact generic SQLite interface and contains no native domain behavior.

## Acceptance and limitation

Native mlplunit suites cover route mismatches and named parameters, request
validation, two-stage middleware order, MLPL authorization, JSON, forms, HTML
escaping, cookies/session lookup plans, all TodoMVC transitions, its controller
and escaped view, and both domains' parameterized CRUD plans. `just todomvc` is
also an executable no-network preview.

The currently pinned sw-MLPL static-provider adapter can receive native records
but cannot send MLPL records or packed bytes to an extension. `_web.listen`,
`_web.respond`, `_sqlite.open`, and parameterized database calls therefore
cannot yet be composed by the interpreter. The accepted Rust providers cover
their mechanics independently; this step does not insert identity shims or move
application logic into a special Rust host. When recursive outbound record and
bytes marshaling lands upstream, these MLPL modules already target the public
provider contracts and should require no application redesign.
