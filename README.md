# forthling

Een kleine Forth-interpreter in Rust, zonder dependencies, op weg naar de
[Forth-2012-standaard](https://forth-standard.org/).

```
$ cargo run --release
forthling — WORDS shows the dictionary, BYE quits.
: SQUARE  DUP * ;
 ok
7 SQUARE .
49  ok
```

Een bestand uitvoeren: `cargo run --release -- examples/demo.fs`

## Wat het kan

- Een REPL en het uitvoeren van bestanden (ook genest met `INCLUDE`)
- Colon-definities die naar threaded code worden gecompileerd, met `RECURSE`,
  `IMMEDIATE`, `POSTPONE`, `LITERAL`, `[ ]`, `' EXECUTE` en `SEE`
- `IF ELSE THEN`, `BEGIN UNTIL/AGAIN/WHILE REPEAT`, `DO ?DO LOOP +LOOP LEAVE`
- `VARIABLE`, `CONSTANT`, `CREATE … DOES>`
- Geheugen dat per byte adresseerbaar is, met cellen van 64 bits
- `SOURCE`, `>IN`, `STATE`, `WORD` en `FIND`, die werken op buffers in het Forth-geheugen
- Fouten als standaard THROW-codes, bijvoorbeeld `demo.fs:3: error -13: undefined word: FOO`

Een deel van het woordenboek is in Forth zelf geschreven (`PRELUDE` in `src/main.rs`).

## Standaard

[`STANDAARD.md`](STANDAARD.md) beschrijft het stappenplan naar Forth-2012 en de huidige stand.

## Testen

```
cargo test
```

Voor de tests uit de Forth-2012-testsuite moet je die eerst clonen:

```
git clone https://github.com/gerryjackson/forth2012-test-suite.git tests/forth2012-test-suite
git -C tests/forth2012-test-suite checkout 4c87e56
```
