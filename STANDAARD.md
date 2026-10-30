# Stappenplan: naar Forth-2012

Doel: deze interpreter laten voldoen aan de [Forth-2012-standaard](https://forth-standard.org/standard/words).
Eerst de verplichte CORE-wordset, daarna CORE EXT en daarna de optionele wordsets.

## Waar staan we nu

*Bijgewerkt 2026-10-30: CORE, CORE EXT, EXCEPTION, TOOLS, STRING, FILE en DOUBLE zijn compleet; mijlpaal M5 is bereikt.*

| Wordset       | Aanwezig  | Ontbreekt |
|---------------|----------:|----------:|
| CORE          | 133 / 133 | 0         |
| CORE EXT      | 49 / 49   | 0         |
| EXCEPTION     | 2 / 2     | 0         |
| EXCEPTION EXT | 2 / 2     | 0         |
| TOOLS         | 5 / 5     | 0         |
| TOOLS EXT     | 13 / 22   | 9 (zie hieronder) |
| STRING        | 8 / 8     | 0         |
| STRING EXT    | 3 / 3     | 0         |
| FILE          | 21 / 21   | 0         |
| FILE EXT      | 8 / 8     | 0         |
| DOUBLE        | 20 / 20   | 0         |
| DOUBLE EXT    | 3 / 3     | 0         |

Ontbrekend in TOOLS EXT: `;CODE ASSEMBLER CODE EDITOR` (er is geen assembler en geen
editor), `FORGET` (verouderd; gebruik `MARKER`) en `NAME>COMPILE NAME>INTERPRET NAME>STRING
TRAVERSE-WORDLIST` (die hebben wordlists nodig en komen met SEARCH).

Testsuite: in de volgorde van `runtests.fth` draaien `prelimtest.fth`, `tester.fr`, `core.fr`,
`coreplustest.fth`, `utilities.fth`, `errorreport.fth`, `coreexttest.fth`, `doubletest.fth`,
`exceptiontest.fth`, `filetest.fth`, `stringtest.fth` en `toolstest.fth` **foutloos**.
`cargo test` draait ze allemaal vanuit `tests/suite.rs` (`suite_prelimtest`, `suite_core`,
`suite_core_ext`, `suite_double`, `suite_exception`, `suite_file_access`, `suite_string`,
`suite_tools`); een nieuwe wordset-test is één regel met de hulpfunctie `run_suite`, die ook
meerdere bestanden neemt (`filetest.fth` heeft `coreexttest.fth` nodig). `toolstest.fth`
slaat zijn `TRAVERSE-WORDLIST`-deel over zolang SEARCH ontbreekt.

### CI en hygiëne (sinds 2026-10-30)

GitHub Actions (`.github/workflows/ci.yml`) draait bij elke push en pull request:
`cargo fmt --check`, `cargo clippy --all-targets -D warnings` en `cargo test` op Rust 1.74
(de `rust-version` in `Cargo.toml`, gelijk aan de lokale toolchain), plus `cargo test` op de
nieuwste stable op Linux en Windows. De testsuite wordt daar gekloond op commit `4c87e56`.
Verhoog je de `rust-version`, pas dan ook de toolchain in `ci.yml` aan. `.gitattributes`
zet alle tekstbestanden op LF, ook in een Windows-checkout.

Kleine opschoonpunten om mee te nemen bij volgende wordsets: een `pop_str()` voor het
patroon `pop2` + `check` (8×), benoemde constanten voor de THROW-codes, een registratie voor
compile-only woorden in plaats van `need_def()` in elk woord (30×), en constanten als
`R/O W/O R/W` naar `prelude.fs`.

### Codeindeling (sinds 2026-10-30)

De interpreter is een library-crate (`src/lib.rs`) met een dunne binary (`src/main.rs`: de
REPL en het draaien van bestanden). Elke module begint met `use crate::*;`; `lib.rs`
her-exporteert de items van alle modules, zodat ze overal beschikbaar zijn. De methodes van
`Forth` zijn over de modules verdeeld en `pub(crate)`.

| Bestand | Inhoud |
|---|---|
| `lib.rs` | `struct Forth`, `Forth::new`, de publieke API (`interpret`, `include`, `set_input`, `output`, …) |
| `throw.rs` | `Throw` en de foutcodetabel |
| `stack.rs` | data stack, return stack, hulpjes voor dubbele cellen |
| `memory.rs` | geheugenindeling (constanten), `get`/`set`/`check`/`reserve`, strings in het geheugen |
| `dictionary.rs` | `Word`, `Kind`, `define`/`find`, `SEE` en `WORDS` |
| `parse.rs` | parsen van de invoerbuffer, getallen, pictured numeric output |
| `interpreter.rs` | buitenste interpreter, invoerbronnen (`Source`, `Origin`), terminal |
| `files.rs` | file ids, `INCLUDED`/`REQUIRED`, hulpfuncties voor de FILE-woorden |
| `run.rs` | binnenste interpreter: `Instr`, frames, loops, `CATCH` |
| `compiler.rs` | `Ctrl`, hulpjes voor definities en control flow, `TO`/`IS` |
| `words/*.rs` | de primitieven per groep (`stack`, `double`, `memory`, `console`, `define`, `control`, `string`, `tools`, `system`, `file`) |
| `prelude.fs` | woorden die in Forth zelf geschreven zijn (`include_str!`) |
| `tests.rs` | unit-tests (mogen bij de interne staat); de suites staan in `tests/suite.rs` |

Een nieuwe wordset krijgt zijn eigen `words/<naam>.rs` met een `install(f)`, aangeroepen in
`words/mod.rs`; de volgorde daar is de volgorde in het woordenboek.

### Volgende sessie: hier verder

- **Volgende stap:** fase 3, FACILITY (`facilitytest.fth`): `BEGIN-STRUCTURE FIELD: +FIELD
  END-STRUCTURE …`, `KEY? PAGE AT-XY MS TIME&DATE`.
- **DOUBLE:** de rekenwoorden zijn primitieven op `i128` (`d_unop`, `d_binop`, `d_compare`);
  `2LITERAL 2ROT D. D.R` staan in de prelude. `number()` geeft nu `(u128, bool)`: een punt
  aan het eind maakt er een dubbele cel van (alleen aan het eind, zoals de standaard zegt).
  `M*/` rekent via een product van 192 bits (`mul_div`), floored zoals de rest. `2VALUE` is
  `Kind::TwoValue`; `TO` compileert daarvoor twee keer `Instr::Store`.
- **FILE:** open bestanden staan in `Forth::files` (file id n is `files[n - 1]`, vrije ids
  worden hergebruikt). `INCLUDED` opent het bestand, krijgt een file id (dat is ook de
  `SOURCE-ID`) en roept `include_file` aan, die de rest van het bestand inleest en het
  daarna sluit, ook bij een fout. `REQUIRED` onthoudt namen via `fs::canonicalize` in
  `Forth::included`. Een relatief pad dat niet bestaat, wordt ook naast het bestand gezocht
  dat aan het includen is (zo vindt `filetest.fth` zijn `required-helper*.fth`). De ior van
  een mislukt bestandswoord is zijn eigen THROW-code (`-62` … `-76`). `READ-LINE` leest
  `u1 + 2` bytes en zet de bestandspositie daarna terug naar net na de regel. `(` loopt in
  een bestand door over regels heen. `suite_file_access` maakt `fatest*.txt` aan in de
  projectmap en ruimt die weer op (staan in `.gitignore` voor het geval een test faalt).
- **STRING:** `COMPARE SEARCH CMOVE CMOVE> SLITERAL REPLACES SUBSTITUTE UNESCAPE` zijn
  primitieven, `/STRING BLANK -TRAILING` staan in de prelude. De teksten van `REPLACES` staan
  in `Forth::substitutions` (een `HashMap`, namen in hoofdletters); `MARKER` zet die niet
  terug. `SUBSTITUTE` bouwt het resultaat eerst in een `Vec`, dus overlappende buffers
  werken; past het niet, dan geeft hij `-78`. Een naam met `%` geeft bij `REPLACES` `-79`.
- **TOOLS:** `[IF]` en `[ELSE]` slaan woorden over met `skip_conditional`, ook over regels heen
  (via `refill()`), en tellen geneste `[IF]`s mee. `CS-PICK`/`CS-ROLL` werken op de
  control-flow stack `ctrl` (`Ctrl` is nu `Copy`). `SYNONYM` kopieert `kind` en
  `immediate` van het oude woord. `N>R` zet n cellen en dan n op de return stack.
  `DUMP` toont hex en ASCII, 16 bytes per regel. `?` staat in de prelude.
- **`CATCH`:** legt diepte van data stack, return stack, loops, frames, invoerbronnen, `>IN`,
  `STATE` en de control-flow stack vast en zet die bij een `THROW` terug. Een definitie die
  binnen `CATCH` begon, wordt geschrapt. `QUIT` gaat door `CATCH` heen.
- **Rust-stack:** elke `CATCH` nest een `run`-lus in Rust. Daarom is de nesting begrensd op
  `MAX_CATCHES` (1000, dieper geeft `-5`) en draait de interpreter in een thread met 64 MiB
  stack (`main` → `run_forth`). Zonder die thread crashte 5000 keer geneste `CATCH` op Windows.
- **Meten:** volg `runtests.fth`: `utilities.fth` en `errorreport.fth` komen vóór de
  wordset-tests, en de `ACCEPT`-test in `core.fr` wil een regel invoer (via een pipe).
- **Invoerbronnen:** elke `Source` heeft een `Origin` (terminal, `EVALUATE`, bestand met al
  zijn regels) en een regelteller. `refill()` laadt de volgende regel; `include` is
  `while refill() { interpret_buffer() }`. `RESTORE-INPUT` kan in een bestand terug naar een
  eerdere regel, bij de terminal en `EVALUATE` alleen binnen de huidige regel.
- **Nieuwe soorten woorden:** `Kind::Value`, `Kind::Defer` en `Kind::Marker`. `TO`, `IS` en
  `ACTION-OF` compileren `Instr::Store`/`Instr::Fetch`; `CASE` gebruikt `Ctrl::Case/Of/EndOf`
  en `Instr::Of`/`Instr::Drop`. Een `:NONAME`-woord blijft verborgen (lege naam).
- **Fouten en `QUIT`:** `ABORT` gooit `-1`, `ABORT"` gooit `-2` met de tekst als detail
  (`Instr::Abort`), `QUIT` gooit `-56` (`QUIT` in de code). `reset(clear_stack)` laat bij
  `QUIT` de data stack staan. In bestandsmodus stopt `QUIT` de run zonder melding.
- **Stacklimieten:** data stack en return stack hebben elk 65536 cellen (`STACK_CELLS`,
  `RSTACK_CELLS`); overloop geeft `-3`/`-5`. `ENVIRONMENT?` meldt deze waarden.
- **Terminalinvoer:** `Forth::input` is het enige invoerkanaal van de terminal; de REPL,
  `ACCEPT` en `KEY` lezen er allemaal uit. Tests stoppen er een `io::Cursor` in. `KEY` leest
  per regel gebufferd (de terminal staat niet in raw mode), zie de README.
- **Besloten (2026-10-30):** deling is floored. `/ MOD /MOD */ */MOD` staan in de prelude
  en gebruiken `FM/MOD`. Gedocumenteerd in de README.
- **Gerepareerd:** `WHILE` laat zijn orig nu ónder de dest achter, zodat meerdere `WHILE`s
  één `BEGIN` kunnen delen (`core.fr` GI5).
- **Meten (algemeen):** draai de testfiles in `tests/forth2012-test-suite/src` met een bestand dat
  ze met `INCLUDE` laadt. Kijk dan tot welke regel hij komt en hoeveel
  `INCORRECT RESULT`/`WRONG NUMBER OF RESULTS` er zijn.
- **Opmaak:** de code volgt `rustfmt` (instellingen in `rustfmt.toml`). Draai `cargo fmt`
  voor elke commit; `cargo fmt -- --check` moet schoon zijn. De formatteercommit `7d16ded`
  staat in `.git-blame-ignore-revs`, zodat `git blame` hem overslaat.
- **Repo:** https://github.com/DeSchaker/forthling (publiek, MIT). De testsuite staat lokaal
  in `tests/forth2012-test-suite` en wordt door git genegeerd.
- **Windows:** PowerShell 5.1 geeft argumenten met `"` of `/` verkeerd door aan `git` en `gh`.
  Gebruik daarvoor Bash, of een bestand voor commitboodschappen.

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
  `CREATE` aligneert. Geheugenindeling (`src/memory.rs`, bovenaan): systeemvariabelen
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

## Fase 1: CORE afmaken ✅ klaar (2026-10-30)

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
6. ✅ **Systeem**: `ABORT ABORT" QUIT ENVIRONMENT?`. `ENVIRONMENT?` beantwoordt in elk geval
   `/COUNTED-STRING /HOLD /PAD ADDRESS-UNIT-BITS FLOORED MAX-CHAR MAX-D MAX-N MAX-U
   MAX-UD RETURN-STACK-CELLS STACK-CELLS`.

**Mijlpaal:** `core.fr` draait zonder fouten.

---

## Fase 2: CORE EXT ✅ klaar (2026-10-30)

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
| 1 ✅ | EXCEPTION | `CATCH THROW`, plus `ABORT ABORT"` uit EXCEPTION EXT. Klaar 2026-10-30. |
| 2 ✅ | TOOLS     | `.S ? DUMP SEE WORDS`, `[IF] [ELSE] [THEN]`, `AHEAD`, `CS-PICK`, `CS-ROLL`, `BYE`, `N>R`/`NR>`, `[DEFINED]`/`[UNDEFINED]`, `SYNONYM`. Klaar 2026-10-30; `NAME>…` en `TRAVERSE-WORDLIST` volgen met SEARCH. |
| 3 ✅ | STRING    | `COMPARE SEARCH /STRING -TRAILING BLANK CMOVE CMOVE> SLITERAL`, plus `REPLACES`/`SUBSTITUTE`/`UNESCAPE`. Klaar 2026-10-30. |
| 4 ✅ | FILE      | `INCLUDED INCLUDE REQUIRE`, plus file-I/O (`OPEN-FILE READ-LINE WRITE-FILE …`) met echte file ids. Klaar 2026-10-30. |
| 5 ✅ | DOUBLE    | `D+ D- D. D< DNEGATE 2CONSTANT 2VARIABLE M+ M*/ …`, en getallen met een punt (`123.`) als dubbele cel. Klaar 2026-10-30. |
| 6 | FACILITY  | `KEY? PAGE AT-XY MS TIME&DATE`, plus `BEGIN-STRUCTURE`/`FIELD:`. Structuren zijn nuttig, terminal-I/O minder. |
| 7 | MEMORY    | `ALLOCATE FREE RESIZE`: een eenvoudige heap in het bovenste deel van het geheugen. |
| 8 | LOCALS    | `{: a b | c -- :}`. Vergt een aparte locals-frame in `Frame`. |
| 9 | SEARCH    | Wordlists: `WORDLIST GET-ORDER SET-ORDER DEFINITIONS …`. Vraagt een herbouw van `find()`. |
| 10 | FLOATING | Aparte float-stack en `F+ F. F@ …` (groot, ongeveer 70 woorden). |
| 11 | XCHAR    | UTF-8-bewuste tekenwoorden. Pas zinvol na 0.2. |
| 12 | BLOCK    | Historisch. Alleen voor de volledigheid. |

---

## Later: assembler en editor (vastgelegd 2026-10-30)

Uit TOOLS EXT ontbreken bewust `;CODE ASSEMBLER CODE EDITOR`. `ASSEMBLER` en `EDITOR` zelf
zetten alleen een woordenlijst vooraan in de zoekvolgorde, dus beide wachten op **SEARCH**.

- **Echte assembler** (`CODE naam … END-CODE` met machinecode): vergt uitvoerbaar geheugen
  van het OS (`VirtualAlloc`/`mmap`, zelfgeschreven FFI en `unsafe`), een aanroepconventie
  (hoe machinecode bij de data stack en return stack komt en terugkeert naar de
  interpreter), een assembler-woordenschat per platform (x86-64-codering is lastig; ARM is
  een tweede set) en het risico dat een fout het hele proces laat crashen, buiten `CATCH`
  om. Veel werk, weinig winst; alleen voor de volledigheid.
- **Goedkoop alternatief:** de standaard laat open wat "machinecode" is. Een `CODE` die
  forthlings eigen `Instr`-instructies assembleert, is legitiem en goed te doen, maar voegt
  weinig toe boven `:`-definities.
- **Klassieke editor:** bewerkt *blocks* (16 × 64 tekens, woorden als `L T P`) en vergt
  dus ook de **BLOCK**-wordset. Een eigen schermeditor in de terminal vergt raw mode zonder
  crates (Windows Console API en termios): veel platformwerk.
- **Pragmatisch, kan los van de roadmap:** een extensiewoord `EDIT bestand` dat de eigen
  editor start (`$EDITOR`, anders `notepad`/`vi`) via `std::process::Command` en daarna
  het bestand `INCLUDE`t. Een paar regels code, in de praktijk erg handig.

Advies: echte assembler en block-editor pas na SEARCH en BLOCK, en alleen als volledigheid
het waard is; `EDIT` kan tussendoor.

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
| M3 ✅ | Fase 1 | `core.fr` foutloos en alle CORE-woorden aanwezig → **CORE-compliant** (2026-10-30) |
| M4 ✅ | Fase 2 | `coreexttest.fth` foutloos (2026-10-30) |
| M5 ✅ | EXCEPTION + TOOLS + STRING | bijbehorende testfiles foutloos (2026-10-30) |
| M6+ | overige wordsets naar behoefte (FILE en DOUBLE ✅ 2026-10-30) | idem |

M2 is de grootste en riskantste stap. Die doe je het best in één keer, met de huidige
unit-tests als vangnet, voordat er nieuwe woorden bovenop komen.
