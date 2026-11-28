# Grafické štýly

Vzhľad každého výkresu sú údaje v projekte, nie niečo zabudované v CreaBIM: hrúbky pier pre každú mierku, spôsob kreslenia každej kategórie prvkov (jej **štýl objektu**), šrafy prerezaných plôch a **šablóny pohľadov**. Všetko je uložené spolu ako **balík štýlov**, súbor `.json`, v ktorom môže kancelária držať svoj grafický štandard. Načítajte iný balík a každý pohľad a list sa prekreslí.

## Balíky štýlov {#style-packs}

Otvorte *Súbor › Grafické štýly…*.

- Zoznam *Balík štýlov* zobrazuje balíky v projekte. Zaškrtnutý je predvolený: projekt kreslí ním.
- *Pridať dodaný balík*: CreaBIM obsahuje dva, *Štandard* (obvyklý vzhľad: čierne čiary, sivá výplň rezu, obvyklé šrafy) a *Jemné čiary* (tenké perá, bledá modrosivá výplň rezu, nábytok sivý, rezy slabnúce s hĺbkou, tiene od slnka na pôdorysoch zhora).
- *Načítať balík…* načíta balík `.json` a nastaví ho ako predvolený. Balík s rovnakým názvom sa nahradí. Štýly textu, rohové pečiatky a číslovanie značiek z balíka nahradia rovnomenné v projekte.
- *Uložiť balík ako…* zapíše balík do súboru na použitie v inom projekte alebo na zdieľanie.

Projekt bez balíka kreslí balíkom *Štandard*. Načítanie balíka je jeden krok pre Späť.

V príkazovom riadku `creabim apply-style <projekt> <balík> --pdf sheets.pdf` načíta balík (súbor alebo *Standard* či *Light linework*) do projektu a exportuje jeho listy. `creabim style-packs <priečinok>` zapíše dodané balíky ako súbory, z ktorých môžete vytvoriť vlastný.

## Štýly kresby {#drawing-styles}

Každý pôdorys a rez má **Štýl kresby**: ako sa kreslí to, čo je prerezané. Vyberte ho v hlavičke pohľadu alebo v časti *Grafika* v jeho *Vlastnostiach*. Ukladá sa s pohľadom, a teda aj v šablóne pohľadu, a obrazovka, listy aj PDF sa zhodujú.

| Štýl | Čo je prerezané |
| --- | --- |
| **Technický** (predvolený) | Šrafované podľa materiálu, ako ich nastavujú grafické štýly |
| **Plná výplň** | Steny a dosky plne vyčiernené, bez šrafy |
| **Farba a šrafa** | Každý materiál vlastnou farbou, vrstva po vrstve, so šrafou navrchu: tehla červená, izolácia žltá |
| **Farba materiálu** | Každý materiál vlastnou farbou, vrstva po vrstve, bez šrafy |
| **Farebný tón** | Jemný tón farieb materiálov na prezentáciu |
| **Sivý tón** | Rovnomerná svetlosivá, bez šrafy |
| **Čistý** | Len obrysy: bez šráf a výplní |

Vo farebných štýloch sa tónujú aj strechy viditeľné zhora pod rezom pôdorysu, najnižšia prvá, takže vyššia strecha zakryje nižšiu.

## Štýly objektov {#object-styles}

Stránka *Štýly objektov* má riadok pre každú kategóriu: steny, dosky, strechy, stĺpy, dvere, okná, nábytok atď.

- *Pohľad*: hrúbka jej čiar viditeľných pod rezom alebo v pohľade. *Rez*: hrúbka jej obrysu tam, kde ju pohľad prerezáva. Každá je *Ako nakreslené* (pero CreaBIM), jedno z pier (takže sa riadi mierkou) alebo šírka v milimetroch.
- *Farba*: vlastná farba čiary.
- *Čiara*: *Plná*, *Čiarkovaná*, *Bodkovaná*, *Osová* alebo *Čiarkobodkovaná*.
- *Výplň rezu*: farba pod šrafou rezu. *Vzor rezu*: vzor prerezaných plôch; *Podľa materiálu* ponechá vzor každého materiálu.

Súbor balíka môže nastaviť aj štýl pre jednu kategóriu v jednom materiáli, výplň a vzor povrchu (strecha viditeľná zhora alebo spevnená plocha), poradie kreslenia a priehľadnosť.

Spevnená plocha si ponecháva vzor dlažby, ktorý má nastavený. Výplň povrchu štýlu Pozemok ide pod každú plochu. Jeho vzor povrchu sa kreslí len na plochy, ktorých vlastný vzor je *Žiadny*.

## Perá {#pens}

Stránka *Perá* uvádza hrúbky pier v milimetroch na papieri pre každý rozsah mierok. Výkres použije prvú tabuľku, do ktorej jeho mierka patrí: s tabuľkami *Do 1:75* a *Do 1:100000* použije pôdorys 1:50 prvú a pôdorys 1:100 druhú.

## Vzory {#patterns}

Šrafovací vzor je jedna alebo viac skupín rovnobežných čiar (uhol, rozostup, odsadenie, posun a čiarky, kde čiarka dĺžky 0 je bodka) alebo malá opakovaná dlaždica, na farbe pozadia.

- *na papieri (mm)*: vzor má rovnakú veľkosť v každej mierke, ako bodky pevnej veľkosti.
- *skutočná veľkosť (m)*: vzor sa kreslí vo veľkosti modelu, ako väzba muriva alebo dlaždice.

Prerezané plochy materiálu dostanú vzor, ktorý uvádza jeho *Šrafa* (*Tehla*, *Betón*…). Vzor balíka s rovnakým názvom nahradí tento zabudovaný pre každý materiál, ktorý ho používa. Materiál môže uvádzať aj ľubovoľný vzor balíka.

## Šablóny pohľadov {#view-templates}

Šablóna pohľadu je pomenovaný súbor nastavení: mierka, štýl, kategórie, ktoré jej pohľady vynechávajú, vlastné perá, slabnutie so vzdialenosťou a tiene od slnka. Každý pohľad, ktorý ju používa, sa zmení, keď sa zmení ona.

- Šablónu priradíte pohľadu v časti *Šablóna a prepísania* v jeho *Vlastnostiach*, alebo vyberte pohľady v prehliadači projektu a na stránke *Šablóny pohľadov* použite *Priradiť N vybraným pohľadom*.
- *Pohľady bez šablóny dostanú* určuje šablónu, ktorú pohľad každého druhu dostane z balíka, ak nemá vlastnú.
- Pohľady uvádzajú názov svojej šablóny, takže načítanie iného balíka s rovnomennými šablónami ich prekreslí.

## Značky a označenia {#tags-and-marks}

Stránka *Značky a označenia* nastavuje, ako sa kreslí každá značka miestnosti a značka pohľadu v projekte. Každá voľba je riadok s obrázkom: ukážte naň a uvidíte ho vo výkresoch, odíďte a vráti sa späť, kliknutím ho ponecháte.

- **Značky miestností:** *Pod sebou* (názov, plocha, potom malé číslo), *Číslo v rámčeku* (číslo v malom rámčeku navrchu), *V rámčeku* (rámček s názvom nad číslom a plochou) alebo *V riadku* (číslo a plocha na jednom riadku).
- **Značky pohľadov:** *Klasické*, *Minimálne*, *Výrazné* alebo *Jednoduché*, spoločne pre značky rezov, pohľadov a výrezov.
- **Vlastné značky:** *+ Nový štýl z …* otvorí návrhára značiek. Vyberte hlavu, výplň, text, šípku, čiaru, okraj výrezu a farbu, sledujte náhľad, pomenujte ho a *Uložte ako štýl*. Zobrazí sa v časti *Vaše štýly* s tlačidlami *Upraviť…* a *Odstrániť*. Vaše štýly sú súčasťou balíka štýlov, takže ich *Uložiť balík ako…* a *Načítať balík…* prenesú do iných projektov.

## Prepísania {#overrides}

Pohľad môže zmeniť, ako sa v ňom kreslí kategória alebo jeden prvok, nad rámec štýlov objektov.

- Vo *Vlastnostiach* pohľadu v časti *Šablóna a prepísania* pridá *Pridať kategóriu…* prepísanie: *Polotón*, *Bez šrafy*, *Hrúbka*, vzor *Čiary* alebo *Farba*.
- Pre jeden prvok ho vyberte pri otvorenom pohľade: *V <pohľad>* v jeho *Vlastnostiach* má rovnaké polia, napríklad ohradový múr čiarkovaný len v situácii.

## Hĺbkové efekty {#depth-cues}

V časti *Šablóna a prepísania*:

- *Slabnutie so vzdialenosťou*: v rezoch, pohľadoch a pôdorysoch zhora sa čiary a výplne vzdialenejšie od rezu alebo pozorovateľa kreslia svetlejšie.
- *Tiene od slnka*: v pôdorysoch zhora vrhá každý prvok tieň na terén, *Smerom* k zvolenému smeru, s *Dĺžkou* na každý meter výšky a *Intenzitou*.

Prerezané prvky dostávajú vždy hrubé pero rezu.
