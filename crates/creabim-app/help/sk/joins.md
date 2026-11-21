# Napojenia

Steny, dosky, strechy, stĺpy, nosníky, základy a podhľady sa môžu stretávať. Kde sa stretnú, pôdorysy a rezy styk vyčistia: prekryv sa nakreslí raz, s jedným obrysom, a vrstvy, ktoré prechádzajú, sa nakreslia ako prechádzajúce.

## Automatické napojenia {#automatic-joins}

Nemusíte nič nastavovať. Každá dvojica stretávajúcich sa vrstiev sa porovná a silnejšia vrstva prechádza cez slabšiu:

1. **Nosná vrstva** (tvárnica, betón, drevený rám) je najsilnejšia.
2. **Podklad** (záklop, laty) nasleduje.
3. **Tepelná izolácia**, **vzduchové medzery** a **fólie** idú potom.
4. **Povrchové úpravy** (omietka, štuk, obklad) sú najslabšie.

Funkcia vrstvy sa nastavuje v jej [skladbe](assemblies). Keď sú dve vrstvy rovnako silné, rozhodne druh prvku: stĺp pretína stenu a stena dosku. Keď sú prvky rovnakého druhu, vyhráva ten nakreslený skôr. Dve stretávajúce sa vrstvy z rovnakého materiálu sa zlúčia do jednej bez čiary medzi nimi.

Napríklad tam, kde stropná doska stretne stenu, betón dosky vnikne do tvárnice steny a skončí pri jej tepelnej izolácii. Omietka steny končí pri doske.

## Styky stien {#wall-junctions}

Steny, ktoré sa stretávajú koncami, sa napoja ako roh, T alebo kríž:

- **Dve steny** sa stretnú v rohu na pokos.
- **Tri a viac stien** končiacich v rovnakom bode tvoria uzol. Dve steny, ktoré uzlom prechádzajú najpriamejšie, ním pokračujú ako jedna stena. Ostatné steny končia pri ich povrchoch a ich nosná vrstva pokračuje do nosnej vrstvy priebežných stien. Ak žiadne dve steny nejdú rovno, prechádza najhrubšia dvojica.
- Stena, ktorá končí na boku inej steny, tvorí styk T.

Steny sa v uzle napoja len vtedy, keď sú nakreslené do rovnakého bodu. Konce, ktoré k sebe nedosahujú, zostanú nenapojené.

## Predvoľby rohov {#corner-presets}

Kde sa koniec steny stretne v rohu s jednou ďalšou stenou, zvoľte spôsob napojenia: pravým tlačidlom na stenu blízko daného konca, *Napojenie rohu*, alebo vo *Vlastnostiach › Styky* v riadku pre daný koniec.

| Predvoľba | Čo robí |
|---|---|
| Automaticky | Ako sa steny napoja samy: každá vrstva sa stretne so svojou partnerkou na pokose |
| Nosné vrstvy sa prekrývajú, povrchy končia | Povrchy sa stretnú na pokose; nosná vrstva silnejšej steny (hrubšej alebo skôr nakreslenej) pokračuje po vzdialený povrch nosnej vrstvy druhej steny a nosná vrstva druhej steny pri nej kolmo končí |
| Táto stena prechádza | Každá vrstva tejto steny ide po vzdialený povrch druhej steny; každá vrstva druhej končí pri tejto |
| Druhá stena prechádza | Naopak |
| Každá vrstva na pokos | Každá vrstva na pokose, ako pri Automaticky v rohu |
| Nenapojené | Oba konce zostanú kolmé a v rohu sa prekrývajú |

Predvoľba sa ukladá s oboma stenami, takže zostáva, aj keď sa ktorákoľvek posunie. Konce vrstiev posunuté ručne (pozri [Režim úprav](edit-mode)) sú odsadenia od miesta, kam ich dáva predvoľba.

## Ručné nastavenie styku {#setting-a-junction-by-hand}

Vyberte dva prvky (druhý so Shift+kliknutím) a kliknite pravým tlačidlom:

- **Prepnúť poradie napojenia**: v mieste prekryvu teraz prechádza druhý prvok.
- **Nečistiť styk**: oba prvky sa nakreslia celé, každý s vlastným obrysom. Opätovným výberom styk vyčistíte.
- **Napojiť automaticky**: návrat k automatickému napojeniu podľa priority vrstiev.

Ručne nastavené styky sú vo *Vlastnostiach* každého prvku v časti **Styky**, kde môžete zmeniť aj ich poradie. Každá zmena je jeden krok pre Späť.
