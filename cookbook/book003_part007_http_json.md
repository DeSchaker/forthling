# An HTTP call: ask a JSON API a question

*Book 3, Data and tools. Needs the program `forthling-http` (the word list `HTTP`, with the
`JSON` words) and a network. It does not run in the playground, and the tests do not run it.*

The program: [`http/examples/book003_example007_http_json.fs`](../http/examples/book003_example007_http_json.fs).
Its output is beside it, in `book003_example007_http_json.out`. It lives in `http/` because
that is the crate with the words. Run it with:

```
cargo run --release --manifest-path http/Cargo.toml -- --allow-host api.github.com http/examples/book003_example007_http_json.fs
```

It asks the GitHub API about the repository of forthling, then about a repository that is not
there, reads a header of the answer, and tries a host it was not allowed to reach. This is what
it printed:

```
DeSchaker/forthling Rust MIT main
status 404 Not Found
application/json; charset=utf-8
refused with -259
```

The tests leave it out on purpose: the answer comes from a server we do not control, and a
test that needs the internet fails for reasons that are not in the code. The output was
produced by running it (about 1500 steps; the bytes sent and received count as work too, so
the number depends on the size of the answers).

## Who may be asked

A program that can reach any host can send your data anywhere, so `forthling-http` reaches no
host unless you name it. `--allow-host api.github.com` allows that host and no other; repeat
the option for more. Without it every request throws -259 and says which host was refused.
The last line of the output is that case: `example.org` is not in the list.

## One request

```forth
ALSO HTTP  ALSO JSON

: FETCH ( url u -- body u status )
  S" User-Agent" S" forthling-cookbook" HTTP-HEADER
  S" Accept" S" application/vnd.github+json" HTTP-HEADER
  HTTP-GET ;
```

`HTTP-HEADER` adds a header to the next request, and only that one, so `FETCH` sets its two
headers each time. GitHub refuses a request without a `User-Agent`. `HTTP-GET` gives the body
and the status. The strings of `S"` share one buffer when they are typed at the top level, so
the strings of one call are put in a definition.

## The status is not an error

A 404 is an answer, not a failure of the program, so `HTTP-GET` does not throw for it. The
program looks at the number:

```forth
: SHOW-REPO ( -- )
  REPO-URL FETCH ( body u status )
  DUP 200 <> IF  ." status " . DROP FREE THROW CR EXIT  THEN
  DROP
  ...
```

In the `IF` the body is still on the stack under the status, so the program frees it before it
leaves. The body is `ALLOCATE`d by the word, and it is the program's to `FREE`, also when the
answer is an error.

What does throw is a request that cannot be made at all: no connection, a time that runs out, a
response over 1 MiB, a host that is not allowed (-259).

## Reading the answer

The body is JSON text. `JSON@` takes the value at a path out of it, and `JSON>STRING` reads
that value as text, without its escapes:

```forth
: .FIELD ( json u path u -- )
  JSON@ DROP JSON>STRING DROP  2DUP TYPE DROP FREE THROW  SPACE ;
```

`JSON@` leaves the value and a flag (false when the path is not there); `JSON>STRING` leaves
the text and a flag. The text is `ALLOCATE`d too, so `.FIELD` frees it after it typed it. `.FIELD`
uses up the JSON text it is given, so the program puts a copy of the body on the stack with
`2DUP` for each of the four fields, and frees the body once at the end. A path can go into an object (`license.spdx_id`) and into an array
(`items[0].name`); a number is read with `JSON>NUMBER`.

## A header of the answer

`HTTP-RESPONSE-HEADER` gives a header of the latest response, whatever the case of its name:

```forth
S" content-type" HTTP-RESPONSE-HEADER IF  2DUP TYPE DROP FREE THROW  ELSE ." (none)" THEN
```

It leaves the value and `true`, or only `false`. The value is `ALLOCATE`d once more.

## What the sandbox says

`--sandbox` takes the permission for the network away. The same program then stops at the first
request with `error -259: not permitted: the network is not permitted`. A program that you give
to someone else to run can be started that way, and it cannot reach out.

## Limits

- Only `http` and `https` addresses; no proxy, and no decompression: the response is the bytes
  the server sent.
- A response is at most 1 MiB. A request takes at most the time left in the evaluation, or 30
  seconds when there is no time limit.
- GitHub allows 60 requests an hour from an address that does not log in. To send a token, set
  `Authorization` with `HTTP-HEADER` before the request; the token is dropped when a redirect
  goes to another host.

## Try it

- Ask for another repository and print `stargazers_count` with `JSON>NUMBER`.
- Print the `description` of the repository. To see what else is in the answer, type the body
  with `TYPE` once, before you decide which paths to read.
- Wrap `SHOW-REPO` in `CATCH`, and print a short message when there is no network.
- Send JSON with `HTTP-POST`: [`http/examples/items.fs`](../http/examples/items.fs) does it
  against a small server on your own machine.
