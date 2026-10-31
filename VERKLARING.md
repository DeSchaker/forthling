# forthling: systeemdocumentatie volgens Forth-2012

Dit is de documentatie die paragraaf 4.1 van de Forth-2012-standaard van een
standaardsysteem vraagt: de keuzes die aan de implementatie zijn overgelaten, wat het
systeem doet bij elke ambigue situatie, en andere gegevens over het systeem. De volgorde
is die van de standaard: eerst de Core-wordset (4.1), dan elke optionele wordset (x.4.1).
De namen van de punten zijn korte omschrijvingen van die in de standaard; de
paragraafnummers verwijzen naar de tekst van de standaard zelf.

De Engelse versie staat in [`COMPLIANCE.md`](COMPLIANCE.md). De roadmap en de stand van
elke wordset staan in [`ROADMAP.md`](ROADMAP.md) (in het Engels).

## Vermelding (5.1.1)

forthling is een Forth-2012 Standard System,

- met de Core Extensions-wordset;
- met de Block-wordset en de Block Extensions-wordset;
- met de Double-Number-wordset en de Double-Number Extensions-wordset;
- met de Exception-wordset en de Exception Extensions-wordset;
- met de Facility-wordset en de Facility Extensions-wordset;
- met de File-Access-wordset en de File-Access Extensions-wordset;
- met de Floating-Point-wordset en de Floating-Point Extensions-wordset;
- met de Locals-wordset en de Locals Extensions-wordset;
- met de Memory-Allocation-wordset;
- met de Programming-Tools-wordset;
- met namen uit de Programming-Tools Extensions-wordset: alle behalve `;CODE`,
  `ASSEMBLER`, `CODE` en `EDITOR`;
- met de Search-Order-wordset en de Search-Order Extensions-wordset;
- met de String-Handling-wordset en de String-Handling Extensions-wordset;
- met de Extended-Character-wordset en de Extended-Character Extensions-wordset.

De wordsets met een testbestand in de
[Forth-2012-testsuite](https://github.com/gerryjackson/forth2012-test-suite) slagen
daarvoor, de floating-point-tests en de paranoia-test inbegrepen. De suite heeft geen
tests voor de Extended-Character-wordset, dus forthling heeft eigen tests:
[`tests/xchartest.fth`](tests/xchartest.fth).

## Core: keuzes van de implementatie (4.1.1)

- **Uitgelijnde adressen** (3.1.3.3): `ALIGN` en `ALIGNED` lijnen uit op 8 adreseenheden,
  de grootte van een cel. Het systeem eist geen uitlijning: `@`, `!` en de andere
  geheugenwoorden werken op elk adres.
- **`EMIT` van een niet-grafisch teken**: de byte gaat ongewijzigd naar de standaarduitvoer;
  de terminal bepaalt wat hij toont.
- **Tekens bewerken in `ACCEPT`**: forthling doet dat zelf niet. Standaardinvoer wordt per
  regel gelezen, dus het regelbewerken van de terminal geldt (backspace bijvoorbeeld).
  `ACCEPT` houdt hoogstens het gevraagde aantal tekens en laat de rest van de regel vallen.
- **Tekenset** (3.1.2, `EMIT`, `KEY`): een teken is een byte (0 tot en met 255), en tekst
  wordt als bytes gelezen en geschreven. Uitgebreide tekens zijn UTF-8 (zie XCHAR).
- **Op tekens uitgelijnde adressen** (3.1.3.3): elk adres is op een teken uitgelijnd.
- **Vergelijken van tekens buiten ASCII** (3.4.2): namen worden vergeleken nadat de
  ASCII-letters `a` tot en met `z` hoofdletters zijn geworden; alle andere bytes, ook die
  van UTF-8, moeten precies gelijk zijn.
- **Stuurtekens als spatie** (3.4.1.1): als de tekstinterpreter, `PARSE-NAME` of
  `BL WORD` een naam zoekt, scheidt elk teken tot en met de spatie (0 tot en met 32) die
  af. `PARSE` met een spatie als scheidingsteken herkent alleen de spatie.
- **Vorm van de control-flow-stack** (3.2.3.2): een eigen stack in de compiler, niet de
  datastack. Elke origin of destination neemt één plaats in, en `CS-PICK` en `CS-ROLL`
  werken erop.
- **Cijfers boven de vijfendertig** (3.2.1.2): komen niet voor, want `BASE` is hoogstens
  36. Met `BASE` buiten 2 tot en met 36 wordt geen woord als getal herkend en worden
  getallen decimaal getoond.
- **Wat er getoond wordt als `ACCEPT` klaar is**: niets; de cursor staat waar de echo van
  de Enter-toets hem liet, aan het begin van de volgende regel.
- **Afbreken met `ABORT"`**: `ABORT"` gooit -2. Als niets die vangt, verschijnt de
  melding als `error -2: aborted: melding` (in een bestand met `bestand:regel:` ervoor),
  worden de stacks leeggemaakt en gaat de besturing terug naar de terminal. Bij bestanden
  die vanaf de opdrachtregel draaien, stopt de run met exitcode 1.
- **Einde van een invoerregel** (3.2.4.1): de Enter-toets, die een regel afsluit met LF
  (of met CR LF, wat hetzelfde wordt behandeld).
- **Grootste counted string** (3.1.3.4, `WORD`): 255 tekens.
- **Grootste geparste string** (3.4.1): 1024 tekens, de grootte van een invoerregel.
  `WORD` gooit -18 bij een string van meer dan 255 tekens.
- **Langste naam van een definitie** (3.3.1.2): forthling stelt zelf geen grens; de
  invoerregel (1024 tekens) is de grens.
- **Langste string voor `ENVIRONMENT?`**: geen grens; een string die geen bekende vraag
  is, geeft onwaar. De naam van een geïnstalleerde extensie is ook een vraag (zie Extensies).
- **Kiezen van het invoerapparaat** (3.2.4.1): de terminal is de standaardinvoer, dus die
  kies je bij het starten van forthling, door invoer om te leiden of te pipen. Bestanden
  op de opdrachtregel worden uitgevoerd in plaats van de REPL.
- **Kiezen van het uitvoerapparaat** (3.2.4.2): de standaarduitvoer, op dezelfde manier
  gekozen.
- **Hoe het woordenboek wordt opgebouwd** (3.3): koppen en gecompileerde code staan
  buiten het Forth-geheugen, dus de dataruimte bevat alleen wat een programma erin zet
  (`,` `ALLOT` `CREATE` `VARIABLE` enzovoort). Een colon-definitie wordt vertaald naar een
  lijst instructies (aanroepen, literals, sprongen), die een binnenste interpreter
  uitvoert.
- **Bits in een adreseenheid** (3.1.3.3): 8.
- **Getalvoorstelling en rekenen** (3.2.1.1): two's complement; cellen zijn 64 bits, in het
  geheugen little-endian opgeslagen.
- **Bereik van n, +n, u, d, +d en ud** (3.1.3, 3.1.4): n van -2^63 tot 2^63-1, +n van 0
  tot 2^63-1, u van 0 tot 2^64-1, d van -2^127 tot 2^127-1, +d van 0 tot 2^127-1 en ud
  van 0 tot 2^128-1.
- **Alleen-lezen-delen van de dataruimte** (3.3.3): geen.
- **Grootte van de buffer van `WORD`** (3.3.3.6): 255 tekens plus de lengte.
- **Grootte van een cel** (3.1.3): 8 adreseenheden.
- **Grootte van een teken** (3.1.2): 1 adreseenheid.
- **Grootte van de invoerbuffer van de terminal** (3.3.3.5): 1024 tekens; een langere
  regel gooit -18.
- **Grootte van de buffer voor getalopmaak** (3.3.3.6): 256 tekens; daarbuiten gooit
  -17.
- **Grootte van `PAD`** (3.3.3.6): 256 tekens.
- **Hoofdlettergevoeligheid** (3.4.2): namen zijn niet hoofdlettergevoelig voor de
  letters `a` tot en met `z`; cijfers in getallen mogen ook beide vormen hebben (`$ff`).
- **Prompt** (3.3, `QUIT`): ` ok` na een regel, of ` compiled` zolang een definitie open
  staat.
- **Afronden bij delen** (3.2.2.1): naar beneden (floored), voor `/ MOD /MOD */ */MOD` en
  `M*/`. `SM/REM` is er voor symmetrisch delen.
- **Waarde van `STATE` als die waar is**: -1.
- **Waarden na een overloop bij rekenen** (3.2.2.2): het resultaat loopt rond, modulo
  2^64 (voor `+ - * 2* NEGATE` en dergelijke). De woorden die delen, gooien -11 als het
  quotiënt niet past.
- **Of de huidige definitie na `DOES>` te vinden is**: nee; een definitie is pas na haar
  `;` te vinden.

## Core: ambigue situaties (4.1.2)

Algemene situaties:

- **Een naam die geen woord en geen getal is**: gooit -13. Als een woordenlijst met een
  naam buiten de zoekvolgorde het woord heeft, zegt de melding welke en hoe die te
  doorzoeken is (`ALSO JSON finds it`).
- **Een te lange naam voor een definitie**: kan niet voorkomen zolang hij op een
  invoerregel past.
- **Een adres buiten de gebieden van 3.3.3**: elk adres binnen het Forth-geheugen kan
  gelezen en beschreven worden, ook de eigen buffers en variabelen van het systeem; een
  adres daarbuiten gooit -9.
- **Een argument van het verkeerde type**: wordt niet opgemerkt; een cel is een cel.
- **Het execution token van een woord zonder interpretatiegedrag** (`'`, `FIND`): komt
  gewoon terug. Zo'n woord buiten een definitie uitvoeren gooit -14.
- **Delen door nul**: gooit -10, ook in `M*/`, `FM/MOD`, `SM/REM` en `UM/MOD`.
- **Overloop van de datastack of de returnstack**: gooit -3 of -5. De data- en de
  returnstack hebben elk 65536 cellen, en aanroepen nesten hoogstens 10000 diep (daarna
  -5).
- **Geen ruimte voor lusparameters**: lussen hebben een eigen stack die meegroeit; de
  aanroepdiepte hierboven begrenst die.
- **Geen ruimte in het woordenboek**: gooit -8.
- **Een woord zonder interpretatiegedrag interpreteren**: gooit -14.
- **De invoerbuffer of een stringliteral veranderen**: mag; het verandert gewoon wat
  daarna geparst wordt, of wat de literal daarna bevat.
- **Overloop van de getalopmaak**: gooit -17.
- **Overloop van een geparste string**: gooit -18.
- **Een resultaat buiten het bereik**: `*` loopt rond. `*/`, `*/MOD`, `FM/MOD`, `SM/REM`,
  `UM/MOD` en `M*/` gooien -11. `>NUMBER` loopt rond, modulo 2^128.
- **Te weinig op de datastack of de returnstack**: gooit -4 of -6.
- **Een naam van lengte nul aan het eind van de invoerbuffer**: gooit -16.

Situaties bij bepaalde woorden:

- **`>IN` voorbij de invoerbuffer**: de invoerbuffer geldt als opgebruikt; het parsen gaat
  verder met de volgende regel.
- **`RECURSE` na `DOES>`**: compileert een aanroep van het hele definiërende woord, niet
  van het `DOES>`-deel.
- **`RESTORE-INPUT` met een andere invoerbron**: er verandert niets en de vlag is waar
  (mislukt).
- **Dataruimte met definities erin wordt vrijgegeven**: koppen en code staan niet in de
  dataruimte, dus een definitie blijft werken; haar data (een `VARIABLE`-cel bijvoorbeeld)
  kan overschreven worden.
- **Lezen of schrijven in de dataruimte zonder de juiste uitlijning**: werkt.
- **De dataruimtewijzer niet uitgelijnd** (`,`): werkt; de cel wordt niet-uitgelijnd
  opgeslagen.
- **Minder dan u+2 elementen voor `PICK` of `ROLL`**: gooit -4.
- **Geen lusparameters** (`I`, `J`, `LEAVE`, `UNLOOP`, `LOOP`, `+LOOP`): gooit -26.
- **`IMMEDIATE` na een definitie zonder naam**: de `:NONAME`-definitie wordt immediate,
  wat niets verandert.
- **`TO` niet gevolgd door een value of een local**: gooit -32.
- **Naam niet gevonden** (`'`, `POSTPONE`, `[']`, `[COMPILE]`): gooit -13.
- **`DO`, `?DO` of `WITHIN` met parameters van verschillend type**: wordt niet opgemerkt.
- **`POSTPONE`, `[COMPILE]`, `'` of `[']` op `TO`**: werkt; het gecompileerde woord
  gedraagt zich dan als `TO` op de plek waar het draait.
- **`WORD` met een string langer dan een counted string**: gooit -18.
- **Een schuif van een cel of meer** (`LSHIFT`, `RSHIFT`): alle bits schuiven eruit, dus
  `1 64 LSHIFT` geeft 0. Een negatief aantal doet hetzelfde.
- **Een woord dat niet met `CREATE` gemaakt is** (`>BODY`, `DOES>`): `>BODY` gooit -31 en
  `DOES>` gooit -21.
- **`#`, `#S`, `HOLD`, `HOLDS` of `SIGN` buiten `<#` en `#>`**: ze werken op de buffer
  voor getalopmaak zoals die is, en gooien -17 als die vol is.
- **Een deferred woord zonder actie**: gooit -21.
- **`IS` of `ACTION-OF` op een woord dat niet met `DEFER` gemaakt is**: gooit -32.
- **`POSTPONE`, `[COMPILE]`, `[']` of `'` op `ACTION-OF` of `IS`**: werkt zoals bij `TO`
  hierboven.
- **`\x` niet gevolgd door twee hexadecimale cijfers** (`S\"`): de cijfers die er zijn,
  worden gebruikt, dus `\x4z` geeft teken 4 en dan `z`.
- **Een `\` voor een ander teken** (`S\"`): het teken staat voor zichzelf.

## Core: overige systeemdocumentatie (4.1.3)

- **Niet-standaardwoorden die `PAD` gebruiken**: geen; het systeem gebruikt `PAD` nooit.
- **Mogelijkheden van de terminal**: standaardinvoer, per regel gelezen (`KEY` geeft pas
  iets als er een regel is ingevoerd), en standaarduitvoer. `AT-XY` en `PAGE` schrijven
  ANSI-escapereeksen. Er is geen raw mode, dus speciale toetsen komen nooit binnen.
- **Dataruimte voor programma's**: het geheugen is 1 MiB, tenzij `--memory` of het
  config-bestand een andere grootte kiest (64 KiB tot 4 GiB). Van de standaard 1 MiB zijn
  bij het starten 1.021.112 adreseenheden vrij (`UNUSED`); de dataruimte en de heap van
  `ALLOCATE` delen die.
- **Ruimte op de returnstack**: 65536 cellen, en een aanroepdiepte van hoogstens 10000.
  Locals en lusparameters staan er los van.
- **Ruimte op de datastack**: 65536 cellen. De floating-point-stack heeft plaats voor 1024
  floats.
- **Ruimte die het woordenboek van het systeem nodig heeft**: niets van de dataruimte. De
  vaste buffers en variabelen van het systeem nemen de eerste 27.464 adreseenheden van het
  Forth-geheugen in; het woordenboek zelf staat buiten het Forth-geheugen.

## Block (7.4.1)

Keuzes van de implementatie:

- **Opmaak van `LIST`**: een regel `Screen u`, dan de 16 regels van het block, elk met zijn
  nummer (0 tot en met 15) ervoor, stuurtekens als spaties en zonder spaties aan het eind.
- **Regellengte voor `\`**: 64 tekens; in een block springt `\` naar het eind van zijn
  regel van 64.

Ambigue situaties:

- **Een block dat niet gelezen kan worden**: gooit -33.
- **I/O-fouten bij het overzetten van blocks**: gooit -33 bij lezen en -34 bij schrijven.
- **Een ongeldig blocknummer** (`BLOCK`, `BUFFER`, `LOAD`): nummers lopen van 1 tot en met
  1048576; elk ander nummer gooit -35.
- **Een programma dat `BLK` verandert**: dat verandert de invoerbron niet. `BLK` wordt bij
  de volgende wissel van invoerbron opnieuw gezet.
- **`UPDATE` zonder actuele blockbuffer**: doet niets.

Overige systeemdocumentatie:

- **Beperkingen van multiprogramming op bufferadressen**: forthling is geen
  multiprogramming-systeem. Er zijn 8 blockbuffers; een adres van `BLOCK` of `BUFFER`
  blijft geldig tot de buffer voor een ander block wordt gebruikt.
- **Beschikbare blocks**: 1 tot en met 1048576, in één blockbestand (`blocks.fb` in de
  huidige map, of het bestand dat `--blocks` of het config-bestand noemt). Elk block kan
  brontekst of data bevatten. Blocks voorbij het eind van het bestand lezen als spaties.

## Double-Number (8.4.1)

- **`D>S` met een d buiten het bereik van n**: geeft de lage cel, de waarde modulo 2^64.

## Exception (9.4.1)

`THROW`-codes: forthling gooit de codes uit tabel 9.1 van de standaard, met deze meldingen
als niets ze vangt (de meldingen zelf zijn Engels):

| Code | Melding | Code | Melding |
|---:|---|---:|---|
| -1 | aborted | -38 | non-existent file |
| -2 | aborted (en de melding van `ABORT"`) | -39 | unexpected end of file |
| -3 | stack overflow | -44 | floating-point stack overflow |
| -4 | stack underflow | -45 | floating-point stack underflow |
| -5 | return stack overflow | -49 | search-order overflow |
| -6 | return stack underflow | -50 | search-order underflow |
| -8 | dictionary overflow | -56 | `QUIT` |
| -9 | invalid memory address | -59 | `ALLOCATE` |
| -10 | division by zero | -60 | `FREE` |
| -11 | result out of range | -61 | `RESIZE` |
| -13 | undefined word | -62 | `CLOSE-FILE` exception |
| -14 | interpreting a compile-only word | -63 | `CREATE-FILE` exception |
| -15 | invalid `FORGET` | -64 | `DELETE-FILE` exception |
| -16 | attempt to use zero-length string as a name | -65 | `FILE-POSITION` exception |
| -17 | pictured numeric output string overflow | -66 | `FILE-SIZE` exception |
| -18 | parsed string overflow | -67 | `FILE-STATUS` exception |
| -21 | unsupported operation | -68 | `FLUSH-FILE` exception |
| -22 | control structure mismatch | -69 | `OPEN-FILE` exception |
| -24 | invalid numeric argument | -70 | `READ-FILE` exception |
| -26 | loop parameters unavailable | -71 | `READ-LINE` exception |
| -29 | compiler nesting | -72 | `RENAME-FILE` exception |
| -31 | `>BODY` used on non-`CREATE`d definition | -73 | `REPOSITION-FILE` exception |
| -32 | invalid name argument | -74 | `RESIZE-FILE` exception |
| -33 | block read exception | -75 | `WRITE-FILE` exception |
| -34 | block write exception | -76 | `WRITE-LINE` exception |
| -35 | invalid block number | -78 | `SUBSTITUTE` |
| -37 | file I/O exception | -79 | `REPLACES` |

forthling heeft drie eigen codes: -256, invoer te diep genest (meer dan 15 bestanden,
`EVALUATE`-strings of ge-`LOAD`-e blocks in elkaar); -258, een limiet van de zandbak
bereikt (de melding noemt welke: `steps`, `time`, `output` of `space`); en -259, iets wat de
zandbak niet toestaat (zie Zandbak). De Python-module voegt -257 toe, een
Python-exceptie in een woord dat in Python geschreven is, en de SQLite-woorden -260, een
fout die SQLite meldt, en -261, een ongeldige handle van een database of statement (zie
Extensies). Een positieve code, of een die niet
in de tabel staat, verschijnt als `exception`. Wat forthling bij elke ambigue situatie
doet, staat per wordset in dit document.

## Facility (10.4.1)

Keuzes van de implementatie:

- **Codering van toetsaanslagen** (`EKEY`): `EKEY` geeft het Unicode-codepunt van het
  volgende getypte teken, dat binnenkomt zodra er een regel is ingevoerd. De speciale
  toetsen (`K-UP`, `K-F1` en dergelijke) hebben nummers vanaf $110000, boven Unicode, en
  komen nooit binnen; de maskers voor de modifiers zijn $1000000 (shift), $2000000
  (control) en $4000000 (alt).
- **Lengte van een kloktik**: `MS` vraagt het besturingssysteem om hele milliseconden te
  slapen; de werkelijke resolutie is die van het besturingssysteem. `TIME&DATE` telt hele
  seconden, in UTC.
- **Herhaalbaarheid van `MS`**: wacht minstens het gegeven aantal milliseconden, en kan
  langer wachten als het besturingssysteem druk is.

Ambigue situaties:

- **`AT-XY` op een apparaat dat het niet kan**: de escapereeks wordt toch geschreven, dus
  die komt in omgeleide uitvoer terecht.
- **Een structuurnaam gebruikt vóór zijn `END-STRUCTURE`**: geeft 0, omdat de grootte nog
  niet bekend is.

## File-Access (11.4.1)

Keuzes van de implementatie:

- **Toegangsmethoden** (`BIN`, `CREATE-FILE`, `OPEN-FILE`, `R/O`, `R/W`, `W/O`): `R/O` is
  0, `W/O` is 1 en `R/W` is 2. `BIN` telt er 4 bij op en verandert niets: bestanden worden
  altijd als bytes gelezen en geschreven, zonder regeleinden te vertalen. `CREATE-FILE`
  maakt het bestand, of maakt het leeg als het al bestaat.
- **Uitzonderingen bij bestanden**: elk woord geeft zijn eigen ior uit tabel 9.1 (bijvoorbeeld
  -69 voor `OPEN-FILE` en -70 voor `READ-FILE`), ook bij een ongeldige file id.
  `INCLUDE-FILE` en `INCLUDED` gooien: -37 bij een ongeldige file id of een leesfout, -38
  bij een bestand dat niet te openen is.
- **Regeleinde voor `READ-LINE`**: LF of CR LF. Een regel die langer is dan de buffer, gaat
  verder bij de volgende `READ-LINE`. `WRITE-LINE` sluit regels af met LF.
- **Vorm van bestandsnamen**: de paden van het besturingssysteem, ten opzichte van de
  huidige map. `INCLUDED` en `REQUIRED` zoeken een relatieve naam ook naast het bestand
  dat wordt ingelezen.
- **Wat `FILE-STATUS` geeft**: x is altijd 0; de ior is 0 als het bestand bestaat, anders
  -67.
- **Invoer na een uitzondering in `INCLUDE-FILE` of `INCLUDED`**: het bestand wordt
  gesloten. Als niets de uitzondering vangt, verschijnt de fout met bestandsnaam en
  regelnummer en gaat de invoer terug naar de terminal; `CATCH` zet de invoerbron terug
  die gold toen het werd aangeroepen.
- **Grootste nesting van bestanden**: 15 bronnen in elkaar (bestanden, `EVALUATE` en
  `LOAD` samen); één meer gooit -256.
- **Langste invoerregel**: 1024 tekens; een langere regel gooit -18.
- **Blocks en bestanden**: alle blocks staan in één bestand, block u op byte-offset
  (u-1)×1024.
- **Aantal stringbuffers**: twee, om beurten gebruikt door `S"` en `S\"` buiten een
  definitie.
- **Grootte van die buffers**: elk 1024 tekens.

Ambigue situaties:

- **`REPOSITION-FILE` voorbij het eind van het bestand**: mag. Een latere schrijfactie
  verlengt het bestand, en het gat bevat nulbytes.
- **Lezen op posities die nog niet beschreven zijn**: voorbij het eind leest `READ-FILE`
  0 tekens en geeft `READ-LINE` een onware vlag, beide met ior 0; een gat leest als
  nulbytes.
- **Een ongeldige file id voor `INCLUDE-FILE`**: gooit -37.
- **Een I/O-fout terwijl `INCLUDE-FILE` of `INCLUDED` een bestand leest of sluit**: een
  leesfout gooit -37; het bestand wordt gesloten, en een fout bij het sluiten wordt
  genegeerd.
- **`INCLUDED` met een bestand dat niet te openen is**: gooit -38.
- **Een blocknummer zonder bestand**: elk geldig blocknummer (1 tot en met 1048576) hoort
  bij het blockbestand; andere gooien -35.
- **`SOURCE-ID` terwijl `BLK` niet nul is**: geeft 0, zoals voor de terminal.
- **`REQUIRED` voor een bestand dat net wordt ingelezen**: een bestand geldt als ingelezen
  zodra het inlezen begint, dus `REQUIRED` doet niets. `INCLUDED` leest het wel opnieuw
  in.
- **Een marker en `REQUIRED`**: een marker vergeet niet welke bestanden zijn ingelezen,
  dus nadat hij gedraaid heeft, laadt `REQUIRED` een bestand niet opnieuw waarvan de marker
  de woorden heeft verwijderd.
- **Eén bestand onder verschillende namen, of verschillende bestanden onder één naam**
  (`REQUIRED`): bestanden worden onderscheiden door hun volledige pad op het moment dat ze
  worden ingelezen. Verschillende namen voor hetzelfde bestand (relatief en absoluut
  bijvoorbeeld) gelden als één bestand; een nieuw bestand op een pad dat al eerder is
  ingelezen, geldt als ingelezen.
- **Een `REQUIRED`-bestand met een ander stackeffect dan ( i*x -- i*x )**: wordt niet
  gecontroleerd.

## Floating-Point (12.4.1)

Keuzes van de implementatie:

- **Vorm en bereik van floats** (12.3.1, `REPRESENT`): IEEE 754 binary64 (dubbele
  precisie). De grootste float is ongeveer 1.7976931348623157E308, de kleinste normale
  ongeveer 2.2250738585072014E-308, en subnormale getallen gaan tot ongeveer 4.9E-324.
  Oneindig, NaN en -0 bestaan.
- **`REPRESENT` met oneindig of NaN**: flag2 is onwaar, n is 0, flag1 geeft het teken, en
  de buffer bevat `inf` of `nan`, aangevuld met spaties.
- **Afronden** (12.3.1.2): naar het dichtstbijzijnde getal, bij gelijke afstand naar even,
  bij rekenen en bij het omzetten van tekst naar floats. `FROUND` rondt bij gelijke afstand
  ook naar even af.
- **Grootte van de floating-point-stack** (12.3.3): 1024 floats. Te vol gooit -44 en te
  leeg -45.
- **Breedte van de floating-point-stack** (12.3.3): 64 bits. Het is een aparte stack, en
  een float neemt in het geheugen 8 adreseenheden in (`1 FLOATS`), net als een DFLOAT;
  een SFLOAT neemt er 4 in.

Ambigue situaties:

- **`F@`, `F!`, `DF@`, `DF!`, `SF@` of `SF!` op een niet-uitgelijnd adres**: werkt.
- **Een resultaat buiten het bereik** (`F/`): geeft oneindig, of 0 of een subnormaal
  getal bij underflow.
- **`BASE` niet decimaal** (`REPRESENT`, `F.`, `FE.`, `FS.`): die werken altijd decimaal.
  De tekstinterpreter herkent float-literals alleen als `BASE` decimaal is; `>FLOAT` is
  altijd decimaal.
- **Beide argumenten nul** (`FATAN2`): geeft ±0 of ±π, naar de tekens van de nullen, zoals
  IEEE dat doet.
- **Cosinus van het argument nul** (`FTAN`): kan niet precies voorkomen; vlak bij π/2 is
  het resultaat groot maar eindig.
- **d of n niet precies voor te stellen** (`D>F`, `S>F`): afgerond op de dichtstbijzijnde
  float.
- **Exponent te groot** (`DF!`, `DF@`, `SF!`, `SF@`): `SF!` slaat oneindig op (of 0 of
  een subnormaal getal bij een heel kleine waarde); `DF!` en `DF@` passen altijd.
- **`FACOSH` onder één**: NaN.
- **`FLNP1` op of onder min één**: -inf bij -1, NaN daaronder.
- **`FLN` of `FLOG` op of onder nul**: -inf bij 0, NaN daaronder.
- **`FSQRT` onder nul**: NaN.
- **`FACOS`, `FASIN` of `FATANH` voorbij één**: NaN; `FATANH` van ±1 is ±inf.
- **Het gehele deel past niet** (`F>D`, `F>S`): gooit -11, ook bij oneindig of NaN.
- **Uitvoer langer dan de buffer voor getalopmaak** (`F.`, `FE.`, `FS.`): die gebruiken
  die buffer niet, dus elke lengte wordt geschreven (`1E300 F.` schrijft 301 cijfers).

## Locals (13.4.1)

Keuzes van de implementatie:

- **Meeste locals in een definitie**: 256 (`ENVIRONMENT? #LOCALS`); één meer gooit -21.

Ambigue situaties:

- **Een local uitvoeren tijdens het interpreteren**: locals worden alleen tijdens het
  compileren gevonden, dus de naam is onbekend (-13), ook tussen `[` en `]`.
- **Een localnaam die eindigt op `:`, `[` of `^`**: een gewone naam.
- **Een localnaam van één niet-alfabetisch teken**: een gewone naam. Binnen de definitie
  verbergt hij het woord met die naam (een local `+` bijvoorbeeld).
- **`{: ... :}` over meer dan één regel**: mag, in bestanden en in de terminal.
- **`{: ... :}` meer dan eens in een definitie**: mag; de locals van elke declaratie zijn
  te gebruiken.

## Memory-Allocation (14.4.1)

De standaard vraagt geen extra documentatie. Volledigheidshalve: de heap groeit van de
bovenkant van het Forth-geheugen omlaag naar `HERE`, blokken zijn op een cel uitgelijnd,
en de iors zijn -59 (`ALLOCATE`), -60 (`FREE`) en -61 (`RESIZE`).

## Programming-Tools (15.4.1)

Keuzes van de implementatie:

- **Eindmarkering na `;CODE` en `CODE`, en hoe die invoer verwerkt wordt**: er is geen
  assembler, dus `;CODE` en `CODE` zijn er niet.
- **Zoekvolgorde van `EDITOR` en `ASSEMBLER`**: zijn er niet.
- **Bron en opmaak van `SEE`**: `SEE` toont een colon-definitie als `: NAAM`, dan één
  genummerde regel per gecompileerde instructie (een woord, een getal, een sprong met zijn
  doel), dan `;` en `IMMEDIATE` als het woord dat is. Andere woorden krijgen één regel,
  zoals `DUP is a primitive`, `IF is a compile-only primitive IMMEDIATE`, `5 CONSTANT X`
  of `CREATE BUF ( address 27464 )`; een woord van een extensie toont als
  `NAME is a host word`.

Ambigue situaties:

- **`FORGET` dat de compilatiewoordenlijst verwijdert**: `FORGET` verwijdert alleen
  woorden, nooit woordenlijsten, dus dat kan niet gebeuren. (Een marker verwijdert wel de
  woordenlijsten die na hem zijn gemaakt, en zet de compilatiewoordenlijst terug zoals die
  was.)
- **`CS-PICK` of `CS-ROLL` met te weinig op de control-flow-stack**: gooit -22.
- **`FORGET` of `SYNONYM` met een naam die niet te vinden is**: gooit -13.
- **`;CODE` op een woord dat niet met `CREATE` gemaakt is**: `;CODE` is er niet.
- **`POSTPONE` op `[IF]`**: werkt; het gecompileerde woord gedraagt zich dan als `[IF]` op
  de invoer die erna komt.
- **Het eind van de invoer vóór de bijbehorende `[ELSE]` of `[THEN]`**: in een bestand of
  een string stopt het overslaan gewoon aan het eind; in de terminal gaat het verder met de
  volgende regels.
- **`FORGET` dat een definitie verwijdert die nog nodig is**: `FORGET` verwijdert het woord
  en elk later woord, dus geen overgebleven definitie kan ernaar verwijzen. Een execution
  token dat elders bewaard is (in een `DEFER` of een variabele), verwijst dan naar het woord
  dat later zijn plaats inneemt, of gooit -24 als er geen is.
- **`IMMEDIATE` op een woord dat met `SYNONYM` gemaakt is**: het synoniem wordt immediate;
  het oorspronkelijke woord verandert niet.
- **`NR>` zonder bijbehorende `N>R`**: het bovenste element van de returnstack geldt als
  het aantal; -6 als er te weinig elementen zijn.
- **Woorden toevoegen of verwijderen tijdens `TRAVERSE-WORDLIST`**: toegevoegde woorden
  worden niet bezocht. Nadat woorden verwijderd zijn (door een marker bijvoorbeeld), gaat
  de wandeling verder onder het nieuwe eind van het woordenboek; als het execution token
  zelf verwijderd is, gooit de volgende aanroep -24.

## Search-Order (16.4.1)

Keuzes van de implementatie:

- **Meeste woordenlijsten in de zoekvolgorde**: 16 (`ENVIRONMENT? WORDLISTS`).
- **Kleinste zoekvolgorde**: alleen `FORTH-WORDLIST`, wat `ONLY` instelt.

Ambigue situaties:

- **De compilatiewoordenlijst veranderen tijdens een definitie**: de definitie komt in de
  woordenlijst die de compilatiewoordenlijst was toen ze begon.
- **Een lege zoekvolgorde**: dan zijn alleen de woorden te vinden die een nieuwe
  zoekvolgorde instellen: `FORTH`, `FORTH-WORDLIST`, `GET-ORDER`, `ONLY`, `ORDER` en
  `SET-ORDER`. Elk ander woord is onbekend (-13). `PREVIOUS` met een lege zoekvolgorde
  gooit -50.
- **Te veel woordenlijsten in de zoekvolgorde** (`ALSO`): gooit -49.

## String-Handling (17.4.1)

Ambigue situaties:

- **Een substitutie die niet gemaakt kan worden** (`REPLACES`): substituties worden buiten
  het Forth-geheugen bewaard, dus alleen het geheugen van de computer zelf begrenst ze.
- **Een substitutienaam met een `%` erin** (`REPLACES`): gooit -79.
- **Een resultaat te groot voor de buffer** (`SUBSTITUTE`, `UNESCAPE`): `SUBSTITUTE` laat
  de buffer met rust en geeft 0 tekens en -78. `UNESCAPE` krijgt geen lengte van de
  buffer mee en neemt daarom het eind van de buffer waar de bestemming in ligt: een van de
  buffers van het systeem (zoals `PAD`, 256 bytes), de data space tot aan de heap, of een
  blok van `ALLOCATE`. Een resultaat dat daar voorbij loopt, gooit -9 en schrijft niets.
- **Dezelfde buffer als bron en bestemming** (`SUBSTITUTE`): werkt, want het hele resultaat
  wordt gemaakt voordat het gekopieerd wordt.

## Extended-Character (18.4.1)

Keuzes van de implementatie:

- **Invoer en weergave van Unicode**: xchars zijn UTF-8 (`ENVIRONMENT? XCHAR-ENCODING`
  geeft `UTF-8`), tot en met $10FFFF in hoogstens 4 bytes. Invoermethoden, de
  schrijfrichting (links naar rechts of rechts naar links) en de glyphs zijn aan de
  terminal; forthling geeft de bytes ongewijzigd door. `XC-WIDTH` en `X-WIDTH` kennen de
  gangbare bereiken van Unicode (0 kolommen voor stuurtekens en combinerende tekens, 2
  voor brede Oost-Aziatische tekens en emoji, 1 voor de rest), niet de volledige tabellen.
- **Xchars in bestanden en strings**: bestanden worden als bytes gelezen en geschreven, dus
  UTF-8 gaat er ongewijzigd doorheen. Een teken blijft één byte (`MAX-CHAR` is 255):
  `KEY`, `EMIT`, `C@` en de stringwoorden werken op bytes, en `EKEY>CHAR` is alleen waar
  voor ASCII. `CHAR`, `[CHAR]`, `'x'` en `PARSE` nemen hele xchars.

Ambigue situaties:

- **Geheugen zonder geldig xchar** (`X-SIZE` en de andere woorden): kapotte UTF-8 wordt
  byte voor byte gelezen; een byte die geen compleet xchar begint, is er zelf een, met de
  bytewaarde als waarde.
- **Een xchar buiten de toegestane codepunten**: een xchar boven $10FFFF schrijven
  (`XEMIT`, `XC!+`, `XC,`, `XHOLD`) schrijft U+FFFD, het vervangteken.
- **`XHOLD` buiten `<#` en `#>`**: zoals bij `HOLD` (zie Core).

## Zandbak

Een host kan beperken wat een Forth-programma mag (`Forth::sandboxed()`, `set_limits`,
`permit`; in het programma `--sandbox` en `--max-steps`, `--max-time`, `--max-output`).
Standaard zijn er geen limieten en is alles toegestaan, zoals de rest van dit document
beschrijft. In de zandbak:

- **Limieten** gelden per evaluatie, een aanroep van `interpret` of `include` door de host
  (in de REPL elke regel). Een stap is een instructie van gecompileerde code of een woord
  dat de tekstinterpreter uitvoert; getallen die hij op de stack zet, zijn geen stap.
  Woorden die veel data in één keer verwerken (`MOVE`, `CMOVE`, `CMOVE>`, `FILL`,
  `COMPARE`, `SEARCH`, `TYPE`, `DUMP`, `EVALUATE`, `REPLACES`, `SUBSTITUTE`, `UNESCAPE`,
  `X-WIDTH`, `ALLOCATE`, `RESIZE` en de JSON-woorden) tellen één stap extra per 64 bytes,
  zodat de tijdlimiet ook voor hen geldt. `SEARCH` kost tijd lineair in de lengtes. De
  limiet op **space** geldt niet per evaluatie maar in totaal: wat de eigen definities
  van het programma, hun code, woordenlijsten, `REPLACES`-teksten en JSON-strings buiten
  het Forth-geheugen innemen, min wat `MARKER` en `FORGET` weghalen (in de zandbak 4 MiB;
  een programma als de core-tests van de testsuite neemt zo'n 100 KiB). Een definitie die
  erover zou gaan, vervalt. Een bereikte limiet gooit -258; elke stap daarna gooit opnieuw, zodat een `CATCH` het wel
  ziet maar niet verder kan. Uitvoer voorbij de limiet vervalt. `MS` dat de tijdlimiet
  zou passeren, gooit meteen. In WebAssembly bestaat de tijdlimiet niet.
- **Zonder `Permission::Files`** geven de FILE-woorden die een bestand noemen
  (`OPEN-FILE`, `CREATE-FILE`, `DELETE-FILE`, `RENAME-FILE`, `FILE-STATUS`) de ior -259;
  `INCLUDE`, `INCLUDED`, `REQUIRE`, `REQUIRED` en de BLOCK-woorden, die geen ior hebben,
  gooien -259. Een bestand dat de host zelf inleest, wordt nog wel gelezen.

## Extensies

Een Rust-programma dat forthling inbouwt, kan eigen woorden toevoegen via de host-API
(`Extension`, `Forth::wordlist` en `Forth::add_word`; zie de README). De standaardwoorden
blijven daarbij zoals ze zijn:

- Een extensie zet haar woorden in een eigen woordenlijst, die een programma doorzoekt met
  `ALSO NAAM`. Het enige woord dat ze aan `FORTH-WORDLIST` toevoegt, is `NAAM`, dat net als
  `FORTH` de eerste woordenlijst van de zoekvolgorde door die lijst vervangt.
- `ENVIRONMENT?` kent elke geïnstalleerde extensie bij haar naam, zoals
  `FORTHLING-SQLITE`, en geeft dan haar versie en waar. `[DEFINED]` test op één woord.
- De woorden van een extensie horen bij het systeem: `FORGET` gooit er -15 voor.

Het programma forthling installeert twee extensies die bij forthling zelf horen. De
eerste zijn de JSON-woorden (`FORTHLING-JSON`, versie 1, woordenlijst `JSON`; zie de README). Een
JSON-waarde is haar tekst in het geheugen; de leeswoorden geven stukken daarvan terug, de
strings van `JSON>STRING` en `JSON-NAME` zijn ge-`ALLOCATE`d. Ze lezen arrays en objecten
tot 512 diep genest; dieper is voor hen geen geldige JSON. Schrijven gooit -22 bij een
structuur die niet klopt of dieper dan 512 nest, ook met `<JSON` (en na een fout is wat
half geschreven was vergeten), en -59 als de strings tussen `<JSON` en `JSON>`, alle
geneste samen, groter zouden worden dan het geheugen. `JSON-NUMBER`
schrijft decimaal, wat `BASE` ook is, `JSON-FLOAT` schrijft `null` voor oneindig of NaN,
en tekst die geen UTF-8 is, wordt geschreven met U+FFFD. De tweede zijn de tracewoorden
(`FORTHLING-DEBUG`, versie 1, woordenlijst `DEBUG`: `TRACE-ON` en `TRACE-OFF`; zie de
README); hun regels zijn uitvoer, net als die van `.S`. `Forth::new()` installeert niets.
De Python-module (`python/`)
installeert `FORTHLING-PYTHON` (versie 1), zonder eigen woorden: de woorden die een
Python-programma toevoegt, komen in `FORTH-WORDLIST` of in een woordenlijst die het noemt.
Een Python-exceptie in zo'n woord gooit -257, wat `CATCH` vangt; ongevangen krijgt het
Python-programma de exceptie zelf terug.

De MCP-server (`mcp/`, het programma `forthling-mcp`) draait elke sessie in de zandbak,
met de JSON- en SQLite-woorden; zijn eigen limieten kunnen niet boven vaste plafonds, en
hij heeft geen optie voor bestanden. Hij leest geen terminalinvoer: `KEY` en `ACCEPT` zien
het einde van de invoer. Hij voegt het woord `DATA ( -- c-addr u )` toe aan
`FORTH-WORDLIST` (`FORTHLING-MCP`): de `data` van de laatste aanroep die die meestuurde,
ge-`ALLOCATE`d.

In WebAssembly (`web/`, de speelomgeving in de browser) draait forthling in de zandbak met
de JSON-woorden. Daar doet `MS` niets, omdat WebAssembly niet kan slapen, en is er geen
tijdlimiet, omdat het geen klok heeft; `TIME&DATE` heeft de tijd die de pagina geeft
(`Forth::set_clock`), en het geheugen kan hooguit 1 GiB zijn.

De SQLite-woorden (`sqlite/`, in het programma `forthling-sqlite`) installeren
`FORTHLING-SQLITE` (versie 1) en de woordenlijst `SQLITE`. Ze volgen de FILE-wordset:
databases en statements zijn handles, fouten zijn een ior (-260 voor een fout van SQLite,
met de melding via `DB-ERROR`; -261 voor een ongeldige handle), en de strings die ze
teruggeven zijn ge-`ALLOCATE`d, zodat het programma ze met `FREE` vrijgeeft. De woorden
zonder ior (`DB-COLUMN` en dergelijke, `DB-CHANGES`, `DB-LAST-ROWID`) gooien -261 bij een
ongeldige handle en -24 bij een ontbrekende rij of kolom. Zonder `Permission::Files` opent
`DB-OPEN` alleen `:memory:` (anders de ior -259), en kan SQL geen databasebestand
koppelen, zodat `ATTACH` en `VACUUM INTO` mislukken met -260, en kan een database niet
groter worden dan het Forth-geheugen. Met een tijdlimiet wordt SQL dat erover gaat
gestopt en gooit het woord -258; het resultaat van een query mag hooguit zo groot zijn als
het Forth-geheugen (anders de ior -260).
