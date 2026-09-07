# Network and Database Extensions

## Goal

Give MLPL bounded network and persistence capabilities without turning the
language runtime into an HTTP stack, database driver collection, or general
shell. Native Rust packages own protocol and operating-system mechanics. MLPL
owns routing, validation, transformations, authorization decisions, views, and
application behavior.

The delivery order is:

1. a synchronous bounded HTTP/HTTPS client;
2. a callback-free, single-request-at-a-time HTTP server;
3. a confined SQLite extension;
4. an MLPL web-framework facade that composes the primitives.

The first client slice is implemented by `extensions/http-client`. It is proven
through both dynamic and static V1 provider registration. The executable
`demos/http-client/get.mlpl` source reaches the real provider through sw-MLPL's
public static C-descriptor hook; `just http-client` runs it against time.gov.
That public-network command is opt-in, while the mandatory acceptance test runs
the identical source against a deterministic loopback endpoint.

## Boundary and capability model

The extension ABI remains typed rather than exposing a raw `ffi.call`. Values
cross as owned scalars, UTF-8, bytes, indexed header records, response records,
and typed generational handles. Foreign pointers, Rust values, sockets,
database connections, and callbacks never cross the ABI.

Network and filesystem authority are independent:

- `network.connect` permits outbound HTTP/HTTPS connections;
- `network.listen` permits binding configured interfaces and ports;
- a configured filesystem root permits SQLite or cache persistence;
- none of these grants process execution.

A future host must grant requested capabilities when it loads a package. The
extension manifest should declare them, while deployment configuration narrows
origins, interfaces, ports, roots, and limits. Loading native code is already a
trust decision; capability declarations make its intended authority reviewable
and prevent an MLPL facade from accidentally using broader ambient authority.

## Bounded HTTP client V1

The public `_http.get(url)` convenience call is the immediately executable MLPL
path. It accepts one absolute HTTP/HTTPS URL and fixes the request to GET, no
headers or body, a 10-second timeout, a 1 MiB response limit, and three
redirects. These conservative defaults make the common teaching example usable
through the current host adapter without weakening the full API below.

```mlpl
# demos/http-client/get.mlpl defaults to this URL when no argument is supplied.
response = _http:get("https://time.gov/");
print({status: response.status, body_bytes: tally(response.body)})
```

Run it with `just http-client`, or select another endpoint with
`just http-client https://example.com/`. The command performs real network I/O;
DNS, TLS, the remote service, and local policy can make it fail. It is therefore
not part of `just check`.

The private `_http.request(request)` provider accepts one exact record:

```text
{
  method: "GET|HEAD|POST|PUT|PATCH|DELETE|OPTIONS",
  url: "https://example.test/path",
  headers: {
    count: 1,
    item_0: {name: "accept", value: "application/json"}
  },
  body: <bytes>,
  timeout_ms: 10000,
  max_response_bytes: 4194304,
  max_redirects: 3
}
```

The indexed record preserves repeated headers despite ABI V1 not carrying a
general string-list value. The result uses the same header representation:

```text
{
  status: 200,
  headers: {count: N, item_0: {name: "...", value: "..."}, ...},
  body: <bytes>,
  final_url: "https://example.test/path"
}
```

HTTP statuses, including 4xx and 5xx, are responses. URL, argument, header, and
limit failures are invalid arguments. DNS, connection, TLS, timeout, response
header, and response-body failures are extension errors. V1 permits only
absolute HTTP/HTTPS URLs, seven fixed methods, at most 128 headers / 64 KiB of
header text, request and response bodies no larger than 16 MiB, timeouts from 1
to 120,000 ms, and at most ten redirects. It has no global cookies, implicit
credentials, cache, or mutable client handle. Redirected authorization follows
the underlying client's conservative cross-host behavior.

Tests use ephemeral loopback listeners and never require the public network.
TLS support is compiled into the provider, but public certificate/network
availability is not part of the deterministic gate.

The current sw-MLPL C-provider outbound adapter can marshal scalar and string
arguments but not an MLPL request record or packed-byte body. Consequently,
`_http.request(record)` is validated at the extension ABI and loader layers but
cannot yet be called from interpreted MLPL. `_http.get(string)` is a real,
non-identity compatibility surface over the same implementation. The remaining
upstream record/bytes requirement is tracked in `docs/upstream-contract.md`.

Large model weights must not use this buffered API. A later model acquisition
facade should map a short allowlisted model name to a pinned URL, expected
length, checksum, and cache location; disclose the transfer size; stream bounded
chunks into a temporary file under an explicitly granted root; verify the
checksum; and atomically rename the completed file. General HTTP fetch and disk
write remain separate permissions.

## Middleware configuration and order

Middleware selection is configurable; security-sensitive ordering is not. The
fixed request/response plan is:

```text
limits
  -> CORS preflight
  -> token extraction
  -> token verification
  -> MLPL authorization
  -> MLPL handler
  -> response/security headers
```

Limits run before parsing attacker-controlled bodies. CORS preflight can stop
before application work. Authentication establishes an identity; MLPL then
decides whether that identity may perform the requested application action.
Response middleware runs last on every completed response. Allowing arbitrary
reordering would make configuration errors into authentication bypasses.

`_http.middleware_plan(config_toml)` currently validates the V1 configuration
and returns its normalized version, fixed order, token settings, origin count,
and `mlpl` authorization ownership. This makes a checked-in configuration file
or MLPL-produced TOML usable without granting the extension filesystem access.
The current implementation accepts only token verification `none`; named JWT,
PASETO, opaque-token introspection, or session verifiers require later provider
work and must fail closed until configured.

Example V1 policy:

```toml
version = 1

[limits]
max_header_bytes = 16384
max_body_bytes = 1048576

[cors]
allowed_origins = ["https://sw-ml-study.github.io"]
allowed_methods = ["GET", "POST"]

[token]
source = "authorization_bearer"
verification = "none"

[authorization]
owner = "mlpl"
```

Configuration files must contain policy, key identifiers, and verifier names,
not embedded long-lived secrets. Deployment supplies secrets through an
explicit secret capability. A later MLPL facade may construct the same policy
from records, but it must pass through the identical native validator.

## Callback-free HTTP server

The first server slice is now implemented in `extensions/http-server`. Rust
owns sockets, HTTP framing, connection deadlines, and bounded request queues.
It does not invoke MLPL functions from a Rust or async-runtime thread.
Purity of a function does not make the evaluator reentrant or thread-safe.

The first server therefore uses polling:

```text
server = web:listen(config, middleware_policy)
request = web:next_request(server, timeout_ms)
response = app:dispatch(request)       # ordinary MLPL call
web:respond(server, request.id, response)
web:close(server)
```

One active MLPL handler is sufficient for the teaching and local-tool scope.
The native listener may maintain a small bounded accept backlog, but overload
must receive a deterministic response rather than grow an unbounded queue.
Request IDs, connection state, listeners, and bodies stay behind typed handles.
Shutdown cancels pending work and invalidates every handle.

CORS, limits, token extraction, and a future configured token verifier are
mechanical native stages. Authorization, routes, controller decisions, and
response content remain MLPL. No middleware may smuggle a native-to-MLPL
callback around the polling boundary.

V1 is IPv4-loopback-only, uses one request per connection, accepts bounded
`Content-Length` bodies, rejects transfer encoding, and has no TLS termination,
keep-alive, streaming, or callbacks. See `http-server.md` for the exact record,
middleware, lifecycle, overload, and acceptance contracts.

## SQLite and framework layer

SQLite is implemented in `extensions/sqlite` because it needs neither a network
service nor a background pool. The Rust package owns confined connections, prepared
statements, transactions, type conversion, row/result budgets, busy timeouts,
and cleanup. MLPL receives records and uses explicit parameter arrays; SQL text
must never be assembled as an authentication shortcut. Connections,
statements, transactions, and cursors are typed generational handles.

V1 needs only typed generational connection handles: prepared statements are
call-scoped, explicit transaction state belongs to its connection, and bounded
queries return complete indexed records rather than persistent cursors. See
`sqlite-extension.md` for the exact limits, confinement rules, result shape,
tests, and the current upstream outbound-record limitation.

The first framework slice is now implemented in `lib/web`, with the standard
TodoMVC application in `demos/todomvc` and experiment CRUD plans in
`demos/experiment-dashboard`. The framework is primarily MLPL:

- method/path routing and named parameters;
- middleware composition around the fixed native security stages;
- request validation and application authorization;
- JSON, forms, escaped HTML, templates, cookies, and sessions;
- controllers and SQLite-backed CRUD;
- development diagnostics and confined static assets.

An experiment/model-run dashboard is the recommended acceptance application.
It can list runs, persist metrics, return JSON, render existing visualization
data, and later initiate an explicitly approved, pinned model acquisition. It
demonstrates MLPL application composition rather than hiding a domain-specific
web application inside Rust.

## Current model-weight reality

sw-MLPL's working fine-tuning demos currently construct small deterministic
weights; arbitrary pretrained checkpoint loading is not yet delivered. The
planned real SmolLM2 flow uses a named allowlist and explicit size disclosure,
then caches raw weights for CLI or connect-mode use. Safetensors/GGUF mapping,
tokenization, and the target model architecture remain separate requirements.

sw-MLPL already documents sandboxed `write_bytes`, `append_bytes`, and
`write_atomic`. Once dynamic provider import and ABI-byte bridging are proven,
small bounded HTTP responses can compose with those sinks. Large checkpoints
still require the specialized streamed, checksum-pinned acquisition path above.

## AgentRail delivery queue

1. `http-client-middleware-foundation` — deliver the bounded client, provider
   packaging, middleware-policy validation, and loopback evidence.
2. `http-server-polling` — implement bounded listen/poll/respond/close handles
   with fixed middleware ordering and no native callbacks.
3. `sqlite-extension` — delivered confined SQLite handles, parameters,
   transactions, bounded results, and deterministic cleanup.
4. `mlpl-web-framework` — delivered MLPL routes, middleware composition,
   JSON/form/HTML/session helpers, standard TodoMVC, and experiment CRUD plans;
   live provider composition awaits the recorded upstream outbound bridge.
5. `pinned-model-acquisition` — only after streaming and filesystem capability
   contracts exist, add explicit allowlisted fetch, checksum, cache, and atomic
   publication behavior.
