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

Výkaz výmer vypisuje aj, každé v samostatnej kategórii: **dvere** a **okná** (kus, s rozmerom ako plochou), **objekty** (kus), **schodiská** (rameno ako dĺžka, pôdorysná plocha a betón, ako ho odhadne stavbár), **zábradlia** (dĺžka) a **objemy** (objem tak, ako sú modelované, vrátane modifikátorov a mierky).

Množstvá sú v metroch, štvorcových metroch a kubických metroch.

## Sadzby {#rates}

Sadzba je cena za štvorcový meter, kubický meter, meter alebo kus materiálu, v mene, v ktorej projekt oceňujete. Nastavíte ju:

- v okne *Materiály*: *Sadzba*, číslo a jednotka *za*; *Vymazať* ju odstráni;
- alebo priamo v tabuľke *Výkaz výmer*, v stĺpcoch *Sadzba* a *Za*.

Materiál bez sadzby sa meria, ale nezapočíta sa do oceneného súčtu.

## Tabuľka výkazu výmer {#the-take-off-table}

*Súbor › Nový výkaz › Výkaz výmer* (alebo *Vytvoriť › Výkaz › Výkaz výmer*). Otvorí sa ako výkaz s riadkom za každý materiál: prvky, v ktorých je, jeho plocha, objem a dĺžka, sadzba a cena. Celková cena je dole.

- Zaškrtnite *Zoskupiť podľa podlaží* v jeho *Vlastnostiach* a materiály každého podlažia sa vypíšu pod nadpisom so súčtami podlažia.
- Kliknutím na názov materiálu vyberiete všetko, čo je z neho vyrobené.
- Umiestnite ho na list ako ktorýkoľvek výkaz. Pozri [Listy](sheets).

## Do Excelu {#to-excel}

*Súbor › Export › Výkazy do Excelu (.xlsx)* alebo **Exportovať Excel** pri ktoromkoľvek výkaze zapíše zošit: *Súhrn*, hárok pre každý výkaz, potom *Výkaz výmer* (súčty podľa materiálu a podlažia, ocenené) a *Riadky výkazu výmer* (materiály každého prvku, vrstva po vrstve, s filtrom na každom stĺpci). Čísla sa zapíšu ako čísla, takže ich tabuľka môže sčítať a triediť.

> **Tip:** Z príkazového riadka zapíše rovnaký zošit `creabim xlsx house.creabim house.xlsx`.
