# Callback-Free HTTP Server

The `web` extension is a deliberately small local HTTP server for teaching,
visualization, experiments, and later SQLite-backed MLPL applications. It does
not target public production hosting or high concurrency.

## Execution model

Rust owns the socket and HTTP framing but never invokes an MLPL function. MLPL
polls one owned request, calls an ordinary MLPL handler, and supplies one owned
response:

```text
server = _web.listen(config, middleware_toml)
request = _web.next_request(server, 100)
response = u:web_dispatch_request(request, handler)
_web.respond(server, request.id, response)
_web.close(server)
```

The underscore calls show the private provider contract; the public `web`
facade will own their final spelling when sw-MLPL package import lands. The
checked-in facade already proves that dispatch is an ordinary MLPL `call`, not
a callback retained or invoked by Rust.

The provider runs no worker thread. Its listener is nonblocking; one poll waits
up to the caller's bounded timeout, parses at most one request, and returns. A
server handle may retain a bounded number of accepted requests until MLPL
responds. Closing the typed generational handle drops the listener and every
pending connection. Stale, wrong-type, and foreign handles fail closed.

## API contract

`listen({address, port, max_pending}, middleware_toml)` accepts only
`127.0.0.1` in V1. Port zero asks the OS for an ephemeral port;
`local_address(server)` reports it. At most 16 server handles and 64 pending
requests are permitted.

`next_request(server, timeout_ms)` accepts 0 through 120,000 ms and returns nil
on timeout. A returned request contains:

```text
{
  id,
  method,
  path,
  query,
  headers,
  body,
  origin,
  bearer_token,
  token_verified: false,
  authorization_owner: "mlpl"
}
```

Headers use the HTTP client's indexed `{count, item_0, ...}` representation so
repeated values survive ABI V1. Bodies are owned bytes. Request IDs are
monotonic per listener and can be consumed exactly once.

`respond(server, request_id, {status, headers, body})` accepts final status
codes 200 through 599, at most 128 / 64 KiB of application headers, and at most
16 MiB of body bytes. The host owns `content-length`, `connection`, and CORS
origin headers so applications cannot emit conflicting framing. V1 closes the
connection after every response.

## Middleware and security

The shared `mlpl-http-contract` crate validates the identical TOML policy for
the client planner and server. Its order cannot be configured:

```text
limits -> CORS preflight -> token extraction -> token verification
       -> MLPL authorization -> handler -> response headers
```

Header/body limits apply before MLPL sees a request. Allowed CORS preflight is
answered natively with 204; a disallowed origin or method is rejected before
dispatch. Allowed application responses receive the matching origin and
`Vary: origin`. CORS is browser policy, not authentication.

Bearer extraction is mechanical. The only V1 verification mode is `none`, so
the request explicitly reports `token_verified: false`. MLPL must not treat the
unverified token as an identity. Future named verification providers may
produce a verified principal before MLPL performs application authorization;
secrets remain deployment capabilities rather than checked-in TOML values.

## Deliberate limits

- IPv4 loopback only; no public bind, TLS termination, proxy trust, or Unix
  socket.
- HTTP/1.0 and HTTP/1.1 request lines only; no HTTP/2 or HTTP/3.
- One request per connection; no keep-alive or pipelining.
- `Content-Length` bodies only; transfer encoding is rejected.
- No multipart, WebSocket, SSE, streaming body, or file-serving shortcut.
- No native-to-MLPL callbacks, evaluator reentry, or MLPL threads.
- Pending requests are capacity-bounded but have no independent expiry in V1;
  applications must respond or close the server.
- Malformed accepted requests become extension errors and their connection is
  dropped. A later hardening slice may turn selected parse failures into native
  4xx responses without exposing partial data.

These limits keep the server auditable and sufficient for a local Rails-like
learning application. Higher throughput can later preserve the same polling
and owned-value contract while replacing socket internals.

## Evidence and upstream boundary

Headless Rust tests use ephemeral loopback ports to prove polling, request/body
ownership, query separation, bearer extraction, response serialization, CORS
preflight, pending capacity, one-shot request IDs, close behavior, and stale
handles. The same five-function descriptor is loaded dynamically and
statically through the public extension registry. MLPL unit tests prove the
public facade performs handler dispatch in MLPL.

This is not yet `use web` execution in the stock sw-MLPL REPL. Package/facade
dynamic-provider integration remains the upstream blocker. The next queued
step adds the closest honest MLPL client execution path and opt-in HTTPS time
example rather than disguising an identity facade as native invocation.
