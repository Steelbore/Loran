+++
name = "xh"
category = "networking"
summary = "Friendly HTTP client. Re-implementation of HTTPie's UX in Rust with extra polish."
replaces = ["curl", "wget", "httpie"]
safe_alias_for = []
pairs_with = ["jaq", "dog"]
official = "https://github.com/ducaale/xh"
tldr_page = "xh"
written_in = "rust"
since = "bravais@0.1"
tags = ["http", "tui-friendly"]
aliases = []
+++

## Steelbore notes

`xh` is Steelbore's default HTTP CLI. It is binary-compatible with most HTTPie command lines but boots in milliseconds, statically links, and ships colourful output with `bat`-like syntax highlighting on the response body.

`safe_alias_for` is empty because `curl` / `wget` argument grammars are utterly different; scripts that parse `curl -v` output would not understand `xh`.

## Recommended usage

```sh
xh httpbin.org/get
xh POST httpbin.org/post name=Mohamed role=engineer
xh -A bearer -a "$TOKEN" GET api.example.com/me
xh --download https://example.com/file.zip      # streams to disk
xh --offline POST api.example.com hello=world   # show the request, don't send
```

## Differences from `curl`

- The first positional argument is the method (defaults to `GET`).
- `key=value` builds a JSON body; `key==value` builds a query string.
- `-A bearer -a TOKEN` is one auth flag, not `-H "Authorization: Bearer …"`.
- Coloured headers + body by default on TTY; auto-disabled on pipes.

## Pairs with

- **jaq** — `xh api.example.com/users | jaq '.[].name'`
- **dog** — when the failure is DNS rather than HTTP, switch to `dog`.
