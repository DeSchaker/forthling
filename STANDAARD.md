# Stappenplan: naar Forth-2012

Doel: deze interpreter laten voldoen aan de [Forth-2012-standaard](https://forth-standard.org/standard/words).
Eerst de verplichte CORE-wordset, daarna CORE EXT en daarna de optionele wordsets.

## Waar staan we nu

*Bijgewerkt 2026-10-30, na fase 1 stap 1 t/m 5.*

| Wordset   | Aanwezig  | Ontbreekt |
|-----------|----------:|----------:|
| CORE      | 129 / 133 | 4         |
| CORE EXT  | 13 / 49   | 36        |

Testsuite: `prelimtest.fth`, `tester.fr` en **`core.fr` draaien foutloos** (alle 638 tests).
`cargo test` draait ze alle drie (`suite_core` geeft de `ACCEPT`-test zijn invoer mee).

Ontbrekende CORE-woorden (niet getest door `core.fr`):
`ABORT ABORT" ENVIRONMENT? QUIT`

### Volgende sessie: hier verder

- **Volgende stap:** fase 1, stap 6: `ABORT ABORT" QUIT ENVIRONMENT?`. Daarmee is CORE
  compleet (mijlpaal M3). `ABORT` en `ABORT"` gooien `-1`/`-2`; `QUIT` gooit `-56` en de REPL
  leegt dan alleen de return stack. Zie fase 0.4 en stap 6 hieronder.
- **Terminalinvoer:** `Forth::input` is het enige invoerkanaal van de terminal; de REPL,
  `ACCEPT` en `KEY` lezen er allemaal uit. Tests stoppen er een `io::Cursor` in. `KEY` leest
  per regel gebufferd (de terminal staat niet in raw mode), zie de README.
- **Besloten (2026-10-30):** deling is floored. `/ MOD /MOD */ */MOD` staan in de prelude
  en gebruiken `FM/MOD`. Gedocumenteerd in de README.
- **Gerepareerd:** `WHILE` laat zijn orig nu ónder de dest achter, zodat meerdere `WHILE`s
  één `BEGIN` kunnen delen (`core.fr` GI5).
- **Meten:** draai `core.fr` in `tests/forth2012-test-suite/src` met een bestand dat
  `INCLUDE tester.fr` en `INCLUDE core.fr` bevat. Kijk dan tot welke regel hij komt en hoeveel
  `INCORRECT RESULT`/`WRONG NUMBER OF RESULTS` er zijn.
- **Repo:** https://github.com/DeSchaker/forthling (publiek, MIT). De testsuite staat lokaal
  in `tests/forth2012-test-suite` en wordt door git genegeerd.
- **Windows:** PowerShell 5.1 geeft argumenten met `"` of `/` verkeerd door aan `git` en `gh`.
  Gebruik daarvoor Bash, of een bestand voor commitboodschappen.

Ontbrekende CORE EXT-woorden:
`.R 0<> 2>R 2R> 2R@ :NONAME ACTION-OF BUFFER: C" CASE OF ENDOF ENDCASE COMPILE, DEFER
DEFER! DEFER@ IS ERASE MARKER PAD PARSE PARSE-NAME REFILL SAVE-INPUT
RESTORE-INPUT ROLL S\" SOURCE-ID TO VALUE U.R U> UNUSED WITHIN [COMPILE]`

De meeste ontbrekende woorden zijn klein. Het echte werk zit in een paar
architectuurkeuzes die nu buiten Forth in Rust-structuren zitten. Die komen eerst.

---

## Fase 0: het fundament ✅ klaar (2026-10-29)

Wat er gebouwd is:
- **Testsuite**: gecloned naar `tests/forth2012-test-suite` (vaste commit `4c87e56`; de repo
  heeft geen licentiebestand, dus hij hoort niet in de eigen broncode). Opnieuw ophalen:
  `git clone https://github.com/gerryjackson/forth2012-test-suite.git tests/forth2012-test-suite`
  en daarna `git -C tests/forth2012-test-suite checkout 4c87e56`.
  `cargo test` draait `prelimtest.fth` en controleert `tester.fr`. Ontbreekt de map,
  dan worden die twee tests overgeslagen met een melding.
- **Geheugen**: 1 MiB, adresseerbaar per byte, cellen van 8 bytes (little-endian).
  `CREATE` aligneert. Geheugenindeling (`src/main.rs`, bovenaan): systeemvariabelen
  (`BASE`, `STATE`, `>IN`) vanaf adres 0, data space vanaf 64, tijdelijke buffers en
  invoerbuffers bovenin.
- **Invoer**: elke invoerbron (terminal, en per geneste `INCLUDE`) heeft een eigen regelbuffer
  in het geheugen. `>IN` wordt na een geneste bron teruggezet. De parser leest `>IN` bij elk
  woord opnieuw, zodat een programma de positie zelf kan verzetten. Na een woord wordt ook de
  spatie erachter overgeslagen, zoals de standaard voorschrijft.
- **Nieuwe woorden**: `SOURCE >IN STATE WORD COUNT FIND`. Die zijn nodig voor `prelimtest.fth`
  en `tester.fr`, en horen bij het nieuwe invoermodel.
- **Fouten**: `Throw { code, detail, location }` met codes uit tabel 9.1. Meldingen zien eruit
  als `file.fs:3: error -13: undefined word: FOO`.
- **Tijdelijke buffers**: `S"` gebruikt in interpretatiemodus twee afwisselende buffers en laat
  `HERE` met rust. De ruimte voor `PAD`, de buffer van `WORD` en `<# #>` is gereserveerd.

Het oorspronkelijke plan voor deze fase:

### 0.1 Testsuite als meetlat
- Haal de [forth2012-test-suite](https://github.com/gerryjackson/forth2012-test-suite)
  van Gerry Jackson binnen (`prelimtest.fth`, `tester.fr`, `core.fr`, `coreexttest.fth`, enz.).
- Voeg een `cargo test` toe die de suite draait en het aantal fouten telt.
  Elke fase wordt "klaar" als de bijbehorende testfile foutloos draait.
- Let op: `tester.fr` gebruikt al `SOURCE TYPE` om fouten te melden, dus 0.3 is nodig
  voordat de suite nette foutmeldingen geeft.

### 0.2 Byte-adresseerbaar geheugen
Nu is één adres één cel van 64 bits en is een teken ook een cel. Formeel mag dat
(volgens §3.1.2 is een teken minstens één address unit en past een cel een geheel
aantal keren). Toch raad ik aan om over te stappen:
- geheugen wordt `Vec<u8>`, een cel is 8 bytes, `1 CELLS` = 8 en `1 CHARS` = 1;
- `@ !` lezen/schrijven 8 bytes (little-endian), `C@ C!` één byte;
- `ALIGN`/`ALIGNED` krijgen echte betekenis;
- strings worden compact en `MOVE`, `FILL`, `COUNT` en counted strings (`C"`, `WORD`)
  gedragen zich zoals elke andere Forth en zoals de testsuite verwacht.

Raakt: `mem`, `here`, `addr()`, `comma()`, `TYPE`, `S"`, `CELLS`/`CELL+`/`CHARS` in de prelude.

### 0.3 Interpreter-toestand in Forth-geheugen
De standaard laat programma's de toestand van de interpreter lezen en schrijven:
- `STATE`: een adres met de compileervlag (nu het Rust-veld `compiling`);
- `>IN`: een adres met de parse-positie (nu `pos`);
- `SOURCE`: het adres en de lengte van de invoerbuffer (nu een `Vec<char>` buiten het geheugen).

Plan: reserveer vaste cellen voor `STATE` en `>IN` (net als `BASE` nu) en een
invoerbuffer in het geheugen. `word()` en `parse()` lezen voortaan daaruit.
Dat maakt meteen `WORD`, `PARSE`, `PARSE-NAME`, `EVALUATE`, `REFILL`,
`SOURCE-ID` en `SAVE-INPUT`/`RESTORE-INPUT` mogelijk.
Invoerbronnen (toetsenbord, `EVALUATE`-string, bestand) worden een stapel, zodat ze genest kunnen worden.

### 0.4 Foutafhandeling met THROW-codes
Fouten zijn nu `String`s. Vervang ze door standaard-throwcodes (tabel 9.1), bijvoorbeeld
`-4` stack underflow, `-10` deling door nul, `-13` onbekend woord, `-14` compile-only.
- `Res` wordt `Result<T, Throw>` met een code en een optionele tekst.
- `ABORT` = throw `-1`, `ABORT"` = `-2`, `QUIT` = `-56` (de REPL leegt dan alleen de stacks).
- Daarmee zijn `CATCH`/`THROW` (EXCEPTION-wordset) daarna bijna gratis.

### 0.5 Tijdelijke buffers
- `S"` in interpretatiemodus schrijft nu naar `HERE` en verschuift die. Dat moet een
  tijdelijke buffer worden (minstens twee, afwisselend, zoals de FILE-wordset vraagt).
- Reserveer ook ruimte voor `PAD`, de buffer van `WORD` en de pictured-output-buffer (`<# … #>`).

---

## Fase 1: CORE afmaken

In deze volgorde, omdat latere groepen op eerdere leunen:

1. ✅ **Dubbele cellen en rekenen**: `S>D M* UM* UM/MOD FM/MOD SM/REM */MOD 2@ 2!`.
   Deling is floored; `/` en `MOD` zijn gedefinieerd via `FM/MOD`.
2. ✅ **Pictured numeric output**: `<# # #S HOLD SIGN #>`, plus `HOLDS` uit CORE EXT. `.` en `U.`
   staan nu in Forth in de prelude; `.R` en `U.R` (CORE EXT) kunnen er zo bij.
3. ✅ **Geheugen en strings**: `ALIGN ALIGNED COUNT FILL MOVE`.
4. ✅ **Parsing en invoer**: `ACCEPT KEY` (`SOURCE >IN WORD` zijn er al sinds fase 0,
   `EVALUATE` en `>NUMBER` sinds 2026-10-30. `EVALUATE` is een geneste `Source` waarvan de buffer
   de string zelf is; `>NUMBER` en de eigen getalconversie delen `to_number`).
   `EVALUATE` wordt een extra soort invoerbron waarvan de buffer de string zelf is.
   Laat de eigen getalconversie (`number()`) daarna `>NUMBER` gebruiken.
5. ✅ **Woordenboek**: `>BODY` (`STATE` en `FIND` zijn er al sinds fase 0).
6. **Systeem**: `ABORT ABORT" QUIT ENVIRONMENT?`. `ENVIRONMENT?` beantwoordt in elk geval
   `/COUNTED-STRING /HOLD /PAD ADDRESS-UNIT-BITS FLOORED MAX-CHAR MAX-D MAX-N MAX-U
   MAX-UD RETURN-STACK-CELLS STACK-CELLS`.

**Mijlpaal:** `core.fr` draait zonder fouten.

---

## Fase 2: CORE EXT

Gegroepeerd van makkelijk naar lastig:

- **Triviaal (prelude)**: `0<> U> WITHIN ERASE 2>R 2R> 2R@ ROLL UNUSED PAD .R U.R`
- **Controlestructuren**: `CASE OF ENDOF ENDCASE`, opgebouwd uit `IF`/`THEN`, met een eigen `Ctrl`-markering.
- **Compiler**: `:NONAME` (een definitie zonder naam die een xt achterlaat), `COMPILE,` en
  `[COMPILE]` (verouderd, maar goedkoop).
- **Waarden**: `VALUE TO`, en `DEFER IS DEFER! DEFER@ ACTION-OF`. Beide hebben een nieuwe `Kind` nodig;
  `TO` en `IS` zijn state-smart (bij compileren leggen ze een instructie vast).
- **Data**: `BUFFER:`, `C"` (counted string, vereist 0.2).
- **Strings met escapes**: `S\"` (`\n \t \" \\ \x41` enz.).
- **Invoer**: `PARSE PARSE-NAME REFILL SOURCE-ID SAVE-INPUT RESTORE-INPUT` (vereist 0.3).
- **Woordenboek terugdraaien**: `MARKER`. Die bewaart de lengte van `dict` en `HERE` en zet ze
  terug bij uitvoeren. Door de `Vec<Word>`-opzet is dit makkelijk.

**Mijlpaal:** `coreexttest.fth` draait zonder fouten.

---

## Fase 3: optionele wordsets (op volgorde van waarde ten opzichte van moeite)

| Prioriteit | Wordset   | Waarom / wat |
|-----------:|-----------|--------------|
| 1 | EXCEPTION | `CATCH THROW`. Na 0.4 bijna gratis en heel bruikbaar. |
| 2 | TOOLS     | `.S ? DUMP SEE WORDS` (deels aanwezig), `[IF] [ELSE] [THEN]`, `AHEAD`, `CS-PICK`, `CS-ROLL`, `BYE`, `N>R`/`NR>`, `[DEFINED]`/`[UNDEFINED]`, `SYNONYM`. De testsuite zelf gebruikt `[IF]`. |
| 3 | STRING    | `COMPARE SEARCH /STRING -TRAILING BLANK CMOVE CMOVE> SLITERAL`, plus `REPLACES`/`SUBSTITUTE`/`UNESCAPE`. |
| 4 | FILE      | `INCLUDED INCLUDE REQUIRE`, plus file-I/O (`OPEN-FILE READ-LINE WRITE-FILE …`). `INCLUDE` bestaat al in eenvoudige vorm. |
| 5 | DOUBLE    | `D+ D- D. D< DNEGATE 2CONSTANT 2VARIABLE M+ M*/ …`, en getallen met een punt (`123.`) als dubbele cel. |
| 6 | FACILITY  | `KEY? PAGE AT-XY MS TIME&DATE`, plus `BEGIN-STRUCTURE`/`FIELD:`. Structuren zijn nuttig, terminal-I/O minder. |
| 7 | MEMORY    | `ALLOCATE FREE RESIZE`: een eenvoudige heap in het bovenste deel van het geheugen. |
| 8 | LOCALS    | `{: a b | c -- :}`. Vergt een aparte locals-frame in `Frame`. |
| 9 | SEARCH    | Wordlists: `WORDLIST GET-ORDER SET-ORDER DEFINITIONS …`. Vraagt een herbouw van `find()`. |
| 10 | FLOATING | Aparte float-stack en `F+ F. F@ …` (groot, ongeveer 70 woorden). |
| 11 | XCHAR    | UTF-8-bewuste tekenwoorden. Pas zinvol na 0.2. |
| 12 | BLOCK    | Historisch. Alleen voor de volledigheid. |

---

## Doorlopend: documentatie (§4.1 van de standaard)

Een standaardsysteem moet zijn *implementation-defined* keuzes en *ambiguous conditions*
documenteren. Houd een sectie bij in de README met onder andere:
- celgrootte (64 bits), address unit (8 bits na 0.2), tekenset;
- deling: symmetrisch of floored;
- hoofdletterongevoeligheid van namen (toegestaan);
- wat er gebeurt bij stack-underflow, een onbekend woord, een ongeldig adres, enz. (throwcodes);
- maximale lengte van een naam en van counted strings, grootte van `PAD` en de pictured-output-buffer;
- extensies: `."` en `S"` werken ook in interpretatiemodus, plus de getalprefixen `$ # % 'c'`.

Tot slot kan de interpreter zich officieel noemen: *"Forth-2012 System, providing the
Core word set, the Core Extensions word set, the Exception word set, …"*.

---

## Voorgestelde mijlpalen

| # | Inhoud | Klaar als |
|---|--------|-----------|
| M1 ✅ | Fase 0.1 + 0.4 (testsuite + throwcodes) | `prelimtest.fth` draait, fouten tonen een code |
| M2 ✅ | Fase 0.2 + 0.3 + 0.5 (geheugen en invoer herbouwen) | alle bestaande tests en de demo werken nog |
| M3 | Fase 1 | `core.fr` foutloos ✅ (2026-10-30) en alle CORE-woorden aanwezig → **CORE-compliant** |
| M4 | Fase 2 | `coreexttest.fth` foutloos |
| M5 | EXCEPTION + TOOLS + STRING | bijbehorende testfiles foutloos |
| M6+ | overige wordsets naar behoefte | idem |

M2 is de grootste en riskantste stap. Die doe je het best in één keer, met de huidige
unit-tests als vangnet, voordat er nieuwe woorden bovenop komen.
