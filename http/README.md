# An HTTP client for forthling

The word list `HTTP` for [forthling](https://github.com/DeSchaker/forthling), through
[ureq](https://github.com/algesten/ureq) with rustls: a Forth program can fetch a page or
post to a JSON API and read the answer, with the JSON words to take it apart. Because it
reaches other machines, it is closed by default and guarded in three ways:

- **A permission.** `Permission::Network` must be there; `Forth::sandboxed()` and the
  program's `--sandbox` take it away.
- **A list of hosts.** A request goes only to a host that the host program put in the list
  (`Http::new().allow("api.example.com")`, or `--allow-host`). With no list, no request is
  sent: the words throw -259 and say which host was refused.
- **The limits of the sandbox.** A request may take no longer than the time left in the
  evaluation (30 seconds when there is no time limit), a response is at most 1 MiB (and
  never more than the Forth memory), and the bytes sent and received count as work.

## Running

`forthling-http` is forthling with these words installed. It allows the hosts of
`--allow-host` (repeat it for more), and no others:

```
cargo run --release --manifest-path http/Cargo.toml -- --allow-host example.com http/examples/page.fs
```

In a Rust program:

```rust
forth.install(&forthling_http::Http::new().allow("api.example.com"));
```

`S" FORTHLING-HTTP" ENVIRONMENT?` gives `1 true` when the words are there. They are in the
word list `HTTP`, so a program starts with `ALSO HTTP`.

## Words

| Word | Stack | |
|---|---|---|
| `HTTP-GET` | `( url-addr url-u -- body-addr body-u status )` | asks for a page |
| `HTTP-POST` | `( url-addr url-u body-addr body-u type-addr type-u -- body-addr' body-u' status )` | sends a body with a content type, such as `S" application/json"` |
| `HTTP-HEADER` | `( name-addr name-u value-addr value-u -- )` | adds a header to the next request, and only that one |
| `HTTP-RESPONSE-HEADER` | `( name-addr name-u -- value-addr value-u true \| false )` | a header of the latest response, whatever the case of the name |

The body and a header value that a word returns are `ALLOCATE`d, for the program to `FREE`.
The status is a number such as 200 or 404, and is no error: the program decides what a 404
means. In the interpreter the strings of `S"` share a buffer, so put the strings of one call
in a definition, as the examples do.

A word throws -263 for a request that fails: an address that is no `http` or `https` URL, a
connection that fails, a time that runs out (-258 when it was the time of the sandbox), a
response that is too large, more than 5 redirects. The message says which. It throws -259
for a host that is not in the list and for a network that is not permitted. Both can be
caught with `CATCH`.

Redirects (301, 302, 303, 307, 308) are followed up to 5 times, and every hop must go to an
allowed host too. A 301, 302 or 303 turns a POST into a GET, as browsers do. The headers
set with `HTTP-HEADER` are dropped when a redirect goes to another host, so a token does
not leak. There is no proxy and no decompression: the response is the bytes the server sent.

## The hosts

A host is written without a port and in any case: `api.example.com`. `*.example.com` allows
every host below `example.com` (not `example.com` itself), and `*` allows every host. An
address is compared as text, so `127.0.0.1` allows that address and not `localhost`. The
list is the guard, not the permission alone: a name that resolves to an address inside a
network is still allowed when it is in the list.

## Examples

[`examples/page.fs`](examples/page.fs) fetches a page and shows its status, size and
content type; [`examples/items.fs`](examples/items.fs) posts JSON to an API, sends a token
with a GET and reads the answers with the JSON words.

```
ALSO HTTP
: SHOW-PAGE ( -- )
  S" https://example.com/" HTTP-GET ( body u status )
  . . ." bytes, "
  S" content-type" HTTP-RESPONSE-HEADER IF 2DUP TYPE DROP FREE THROW THEN
  CR  FREE THROW ;
SHOW-PAGE
```

This printed `200 577 bytes, text/html; charset=utf-8`.

The same words are in the MCP server, when it is built with the Cargo feature `http` and
started with `--allow-host`, and in the Python module, with `Forth(allow_hosts=[…])`. The
playground has none: a WebAssembly module cannot wait for the network.
