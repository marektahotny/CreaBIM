# Výkaz výmer

**Výkaz výmer** meria, z čoho je model vyrobený: koľko štvorcových metrov omietky, kubických metrov betónu, metrov nosníka. Priraďte materiálu **sadzbu** a výkaz výmer ho ocení. Ako výkaz sa číta živo z modelu, takže sa mení pri kreslení.

## Čo meria {#what-it-measures}

Steny, dosky, strechy, podhľady, stĺpy, nosníky a základy dávajú po jednom riadku za každý materiál, z ktorého sú postavené:

- prvok so [skladbou](assemblies) dá riadok za každú vrstvu (vzduchová medzera bez materiálu sa vynechá), v hrúbke vrstvy;
- čokoľvek iné dá jeden riadok za svoj materiál.

| Prvok | Plocha | Objem | Dĺžka |
|---|---|---|---|
| Stena | jej plocha mínus dvere a okná | plocha × hrúbka | pozdĺž steny |
| Doska, podhľad | pôdorysná plocha mínus otvory | plocha × hrúbka | - |
| Strecha | plocha jej rovín | plocha × hrúbka | - |
| Stĺp | - | prierez × výška | jeho výška |
| Nosník | - | prierez × dĺžka | jeho dĺžka |
| Základ | jeho pôdorysná plocha | plocha × hrúbka | dĺžka pásu |

Steny a pásové základy sa merajú na spojenom obryse, takže roh, kde sa stretnú dve steny, sa započíta raz a stena končiaca pri povrchu inej sa do nej nezapočíta.

**Objemy sú začistené a orezané.** Objem stien, dosiek, striech, podhľadov, stĺpov, nosníkov, základov a schodísk sa meria priamo na 3D telesách tak, ako ich ukazuje 3D pohľad a výkresy: po napojeniach (doska prechádzajúca stenou, stĺp alebo systémový preklad v stene), po výrezoch a so štítom steny pod strechou. Plocha steny, dosky či strechy je potom tento objem delený hrúbkou vrstvy. Kým sa 3D model po zmene prepočíta, tabuľka ukazuje posledné zmerané hodnoty a v hlavičke píše *merám orezané 3D telesá…*.

Výkaz výmer vypisuje aj, každé v samostatnej kategórii: **dvere** a **okná** (kus, s rozmerom ako plochou), **objekty** (kus), **schodiská** (rameno ako dĺžka, pôdorysná plocha a betón, ako ho odhadne stavbár), **zábradlia** (dĺžka) a **objemy** (objem tak, ako sú modelované, vrátane modifikátorov a mierky).

Množstvá sú v metroch, štvorcových metroch a kubických metroch.

## Sadzby {#rates}

Sadzba je cena za štvorcový meter, kubický meter, meter alebo kus materiálu, v mene, v ktorej projekt oceňujete. Nastavíte ju:

- v okne *Materiály*: *Sadzba*, číslo a jednotka *za*; *Vymazať* ju odstráni;
- alebo priamo v tabuľke *Výkaz výmer*, v stĺpcoch *Sadzba* a *Za*.

Materiál bez sadzby sa meria, ale nezapočíta sa do oceneného súčtu.

## Tabuľka výkazu výmer {#the-take-off-table}

*Súbor › Nový výkaz › Výkaz výmer* (alebo *Vytvoriť › Výkaz › Výkaz výmer*). Otvorí sa ako tabuľka ako v Exceli: mriežka, hlavička, ktorá pri posúvaní ostáva hore, skupiny s medzisúčtami a súčet dole.

- **Každý materiál samostatne.** Zložená konštrukcia (stena s omietkou, murivom, izoláciou a omietkou, podlaha, strecha) dá riadok za každú svoju vrstvu. Materiály rovnakej hrúbky v jednej skupine sa sčítajú do jedného riadku.
- **Zoskupiť podľa** v lište tabuľky: až tri úrovne z *Typ prvku* (steny, dosky, strechy…), *Podlažie*, *Konštrukcia* (typ alebo skladba prvku) a *Materiál*. Predvolene je výkaz rozdelený podľa typu prvku; napríklad *Podlažie › Typ prvku* dá steny, dosky… každého podlažia zvlášť. Skupina má medzisúčet objemu a ceny (pri skupine podľa materiálu aj plochy a dĺžky); trojuholníkom ju zbalíte, *Zbaliť všetko* / *Rozbaliť všetko* naraz.
- **Každý prvok samostatne** vypíše každú vrstvu každého prvku vo vlastnom riadku, s prvkom, konštrukciou, podlažím a vrstvou.
- **Triedenie:** klik na hlavičku stĺpca zoradí riadky v skupinách vzostupne, druhý klik zostupne, tretí zruší zoradenie.
- **Hľadať** ponechá len riadky, ktorých materiál, prvok, konštrukcia alebo typ prvku obsahuje zadaný text.
- **Ukázať, kde to je:** kliknite na riadok (s Ctrl pridáte ďalšie, klik na skupinu vyberie celú skupinu) a stlačte **Ukázať v 3D** alebo **Ukázať v pôdoryse**. Prvky sa označia a priblížia; ak 3D pohľad nie je otvorený, otvorí sa vedľa tabuľky, pôdorys podlažia pod ňou. Dvojklik na riadok ich ukáže v 3D.
- *Sadzbu* a *Za* materiálu meníte priamo v riadku.
- Zoskupenie, zobrazenie po prvkoch a triedenie sa ukladajú s výkazom a rovnako sa vytlačí na liste. Pozri [Listy](sheets).

## Do Excelu {#to-excel}

*Súbor › Export › Výkazy do Excelu (.xlsx)* alebo **Exportovať Excel** pri ktoromkoľvek výkaze zapíše zošit: *Súhrn*, hárok pre každý výkaz, potom *Materials by element* (výkaz rozdelený podľa typov prvkov, každý materiál samostatne), *Výkaz výmer* (súčty podľa materiálu a podlažia, ocenené) a *Riadky výkazu výmer* (materiály každého prvku, vrstva po vrstve, s filtrom na každom stĺpci). Čísla sa zapíšu ako čísla, takže ich tabuľka môže sčítať a triediť.

> **Tip:** Z príkazového riadka zapíše rovnaký zošit `creabim xlsx house.creabim house.xlsx`.
