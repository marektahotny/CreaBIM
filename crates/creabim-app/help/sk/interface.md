# Okno programu v skratke

![Okno CreaBIM](img/interface.jpg)

## Horná lišta {#the-top-bar}

- **Ponuka CreaBIM** (logo): O programe, aktualizácie, nastavenia, záznam činnosti a Koniec.
- **Ponuky**: *Súbor*, *Upraviť*, *Pohľad*, *Vytvoriť*, *Materiály* a *Pomocník*. Kým je jedna otvorená, posunutím na ďalšiu prepnete.
- **Rýchle tlačidlá**: Späť, Znovu, Posunúť, Otočiť, Mierka, Duplikovať, Kopírovať, Vystrihnúť, Vložiť, Odstrániť, Skryť, Zobraziť iba vybrané, Zobraziť všetko. Robia to isté ako klávesy, takže všetko zvládnete aj len myšou.
- **Názov súboru**. Bodka vedľa neho znamená neuložené zmeny; aj titulok okna ukazuje •.
- **Pracovná plocha** (vpravo hore): pomenované rozloženie [oblastí pohľadov](interface#view-areas). Pozri [Pracovné plochy](interface#workspaces).

## Záložky a rad nástrojov {#the-tool-tabs-and-tool-row}

Nástroje sú na záložkách: **Architektúra** (steny, dvere, dosky, strechy, nosná konštrukcia, objekty, hmoty, miestnosti, pozemok), záložky profesií (**Kataster**, **TZB**, **Vykurovanie**, **VZT**, **Medicinálne plyny**, **Elektroinštalácia**), **Anotácie** (kóty, značky, poznámky, detaily), **Pohľad** (podlažia, rezy, pohľady, orezové kvádre, podklady, listy, výkazy, grafické štýly) a **Správa** (podlažia, osi, pomocné čiary, údaje o stavbe, typy, profily, materiály a kontroly projektu). **Výber** a **Zarovnať** sú vždy k dispozícii. Podržte ukazovateľ nad nástrojom a uvidíte jeho názov, kláves a spôsob použitia. *Vytvoriť* vypisuje tie isté nástroje podľa názvu a `Shift+A` ktorýkoľvek z nich nájde písaním.

## Lišta volieb nástroja {#the-tool-options-strip}

Riadok pod nástrojmi ukazuje používaný nástroj a čo sa preň dá nastaviť: hrúbku steny, režim kreslenia dosky, smer kóty. Polia majú krátky popis; prepínače a tlačidlá sú malé ikony. Podržte nad nimi ukazovateľ a uvidíte názov, kláves a funkciu. Keď je okno príliš úzke, čo sa nezmestí, je v **⋯ Viac** na pravom konci.

**?** vedľa názvu nástroja otvorí pár riadkov o jeho použití a **Čítať viac** jeho stránku v tomto pomocníkovi.

## Oblasti pohľadov {#view-areas}

Stred okna je rozdelený na **oblasti**, ako v Blenderi. Každá oblasť zobrazuje jednu vec pod vlastnou hlavičkou: 3D pohľad, pôdorys, rez, pohľad, detail, list alebo výkaz, [poznámkový blok](notes#the-notepad) projektu, [knižnicu profilov](profiles), [Predlohy](masters) listov alebo [Záznam](files#the-activity-log) činnosti.

Hlavička oblasti, zľava:

- **Ponuka druhu pohľadu**: čo oblasť zobrazuje. Zvoľte *3D pohľad*, druh výkresu, *Poznámky*, *Profily*, *Predlohy* alebo *Záznam*. 3D pohľad je jeden: oblasť, ktorá oň požiada, kým ho zobrazuje iná, dostane namiesto neho výkres.
- **Názov pohľadu**: výber iného pohľadu rovnakého druhu.
- **Tlačidlo režimu**: *Režim objektov* alebo *Režim úprav* (`Tab`). Pozri [Režim úprav](edit-mode). Listy a výkazy ho nemajú.
- **Vlastné nástroje pohľadu**: výška rezu, štýl kreslenia, röntgen, čo je skryté a podobne. Podržaním ukazovateľa uvidíte, čo ktorý robí.
- **⋯ Viac**: všetko ostatné o pohľade, napríklad *Priblížiť na mierku* (1 mm na papieri je 1 mm na obrazovke) a *Posúvať a približovať spolu s druhou oblasťou tohto pohľadu*.
- **Tlačidlo oblasti** na pravom konci: rozdelí oblasť vedľa seba alebo nad seba, alebo ju zavrie (miesto zaberie susedná oblasť).

Môžete tiež ťahať z rohu oblasti: do tej istej oblasti ju rozdelí, do susednej obe spojí. Ťahaním medzery medzi dvoma oblasťami rozdelíte miesto. `Ctrl+Medzerník` zobrazí samotnú oblasť pod ukazovateľom a ďalšie stlačenie vráti ostatné.

Každá oblasť sa posúva a približuje samostatne, aj dve oblasti s tým istým pôdorysom: detail zblízka vedľa celku. Ak ich chcete mať zosúladené, použite *Posúvať a približovať spolu s druhou oblasťou tohto pohľadu* v *⋯ Viac*; v hlavičke sa potom zobrazí malé prepojenie. Výber a úpravy sa vždy zobrazujú vo všetkých oblastiach.

## Pracovné plochy {#workspaces}

**Pracovná plocha** je pomenované rozloženie oblastí. Vyberiete ju v **Pracovnej ploche** vpravo hore:

| Pracovná plocha | Čo zobrazuje |
| --- | --- |
| *Model* | Jednu 3D oblasť na modelovanie |
| *Výkres* | Práve otvorený výkres vedľa 3D pohľadu |
| *Dokumentácia* | 3D pohľad, práve otvorený výkres a list, na ktorom je umiestnený |
| *Listy* | List vedľa výkresu, ktorý sa naň umiestňuje |

*Uložiť aktuálne rozloženie…* uloží vlastné usporiadanie pod názvom, ktoré potom vyberiete v tej istej ponuke a tam ho aj môžete odstrániť. `Ctrl+PgUp` a `Ctrl+PgDn` prechádzajú pracovnými plochami.

## Prehliadač projektu (vľavo) {#the-project-browser-left}

Všetko v projekte, zoskupené: pohľady podľa druhu, listy, výkazy, podlažia, potom steny, dvere, materiály, typy, profily a tak ďalej, každá skupina s vlastnou ikonou. Písaním do jeho vyhľadávacieho poľa filtrujete podľa názvu alebo označenia (`D01`, `W05`). Kliknutím na riadok ho vyberiete, alebo otvoríte pohľad či list. Dvojklikom na prvok naň priblížite, dvojklikom na typ ho uvidíte samostatne. `Shift` alebo `Ctrl` pridáva k výberu. Ťahaním pohľadu hore alebo dole meníte poradie v skupine. Tlačidlo hore zbalí alebo rozbalí všetky skupiny. Ťahaním okraja panela ho rozšírite.

Úzky úchyt na jeho okraji alebo `Ctrl+[` zbalí prehliadač na úzky **pás** ikon skupín. Kliknutím na skupinu v páse vypíšete jej položky a vyberiete jednu bez otvorenia prehliadača.

## Vlastnosti (vpravo) {#properties-right}

Všetko o tom, čo je vybrané, aj o pohľade a projekte. Horná časť je pripnutá, kým zvyšok sa posúva: čo je vybraný prvok (jeho trieda IFC, s vlastným pomocníkom), jeho názov a označenie, a tlačidlá pre to, čo sa s ním dá robiť.

Pás **záložiek** na jeho ľavom okraji zobrazuje iba záložky, ktoré sa uplatnia, v troch skupinách:

| Skupina | Záložky |
| --- | --- |
| Čo je vybrané | **Prvok** (typ, rozmery a konštrukcia), **Modifikátory** (pre hmotu), **Napojenia a booleovské operácie**, **Grafika** (jeho prepísania), **Údaje (IFC)**. Kým je zapnutý nástroj na umiestňovanie a nič nie je vybrané: **Ďalší prvok**, nastavenia toho, čo vytvorí ďalšie kliknutie. |
| Pohľad | **Pohľad** (pohľad v oblasti, ktorú ste naposledy použili), **Obkreslenie a podklad**, **3D pohľad** (čo zobrazuje, orezové kvádre) |
| Projekt | **Projekt** (údaje, podlažia, materiály), **Poznámky a úlohy** |

Otvorená záložka zostáva aj pri zmene výberu. Výber nástroja na umiestňovanie otvorí *Ďalší prvok*; prvý výber otvorí *Prvok*.

Každá záložka sa skladá zo sekcií. Kliknutím na nadpis sekcie ju zbalíte: zostane zbalená, aj nabudúce. **?** vedľa nadpisu povie v pár riadkoch, o čom sekcia je. Ťahaním popisu čísla do strany ho meníte, alebo kliknite a píšte. Dĺžky možno zadať v ľubovoľnej jednotke (`1200`, `1.2 m`, `4'`). Keď je vybraných viac prvkov jedného druhu, zmena sa uplatní na všetky, jedným krokom späť.

Ťahaním ľavého okraja panela ho rozšírite podľa potreby; v úzkom paneli sa riadky zalamujú namiesto toho, aby vybiehali. Úzky úchyt na jeho okraji alebo `Ctrl+]` ho zbalí na stĺpec záložiek; kliknutím na záložku ho tam znova otvoríte.

## Stavový riadok (dole) {#the-status-bar-bottom}

Začína tým, čo robí každé tlačidlo myši nad používaným pohľadom, a mení sa pri podržaní `Shift`, `Ctrl` alebo `Alt`. Potom slovami, čo robiť ďalej; skrátená správa sa pri ukázaní na ňu posunie a ukáže zvyšok. Potom prepínač **Prichytiť** a zoznam [prichytení](snaps), **Prichytenie k mriežke** (`Alt+S`), **Pracovná rovina**, **Stred otáčania** pre otáčanie a mierku, ponuka **Jednotky** (pozri [Jednotky a formáty čísel](units)) a počet prvkov v projekte.

## Panel Pomocník {#the-help-panel}

Tento panel. Otvára sa vpravo v okne, vedľa vašej práce. Ťahaním ľavého okraja ho rozšírite, hľadáte poľom hore, späť sa vrátite šípkou a zavriete ho **×**, `Esc` alebo `F1`.

## Nastavenia {#settings}

*Ponuka CreaBIM › Nastavenia…* (`Ctrl+,`) obsahuje to, čo je iba vaše: nikdy nemení niečie výkresy a ide s vami, nie s projektom. Kategórie:

| Kategória | Čo obsahuje |
| --- | --- |
| **Rozhranie** | Téma, **papier výkresu** (podľa témy, vždy svetlý alebo vždy tmavý; tlač a PDF sú vždy na bielom) a kde je **lišta Posun, Otočenie a Mierka**: nad pohľadom alebo pri ukazovateli |
| **Pohľad** | **Farba výberu a úchytov** (*Oranžová*, alebo vlastná farba zvýraznenia *Podľa témy*), vzhľad úchytov, šírka hrán a veľkosť vrcholov v Režime úprav a osi X a Y na mriežke |
| **Navigácia** | Myš: *CreaBIM* (stredné tlačidlo otáča, ľavé vyberá obdĺžnikom) alebo *Klasický* (ťahanie ľavým otáča) |
| **Prichytávanie** | Rozostup prichytenia k mriežke a značky vzdialenosti pri ťahaní |
| **Projekt** | Jednotky projektu, uložené s projektom pre každého, kto ho otvorí |
| **Mapa kláves** | Všetky klávesy podľa toho, čo robia. `Ctrl+/` otvorí zoznam kedykoľvek |

## Témy a paleta príkazov {#themes-and-the-command-palette}

- *Pohľad › Téma* vyberá farebnú tému: Grafit, Papier, Modrotlač, Šalvia, Atrament (vysoký kontrast), Solarized, Tmavá alebo Svetlá. `Ctrl+Shift+T` prepne na ďalšiu. Voľba sa zapamätá.
- `Ctrl+K` otvorí **paletu príkazov**: napíšte časť ľubovoľného príkazu, nástroja, pohľadu, stránky pomocníka alebo názvu prvku, potom `Enter`. Prvok sa vyberie a priblíži.

![Paleta príkazov](img/palette.jpg)
