# Listy a rohové pečiatky

**List** je strana na tlač, od A0 po A4, s rohovou pečiatkou. Pohľady umiestnené na ňom sa kreslia z aktuálneho modelu, takže nikdy nie sú zastarané.

## Vytvorenie listu {#making-a-sheet}

*Súbor › Nový list* alebo **Nový list** na karte *Pohľad*. List otvoríte zo skupiny *Listy* v prehliadači alebo v ktorejkoľvek [oblasti](interface#view-areas) z jej ponuky druhu pohľadu. [Pracovné priestory](interface#workspaces) *Listy* a *Dokumentácia* zobrazia list vedľa výkresu, ktorý naň umiestňujete. `PageUp` a `PageDown` prechádzajú listami.

## Hlavička listu {#the-sheet-s-header}

Hlavička oblasti, ktorá zobrazuje list, obsahuje to, čo patrí k papieru, v skupinách:

- **Umiestniť pohľad…**;
- **Text**, **Severka**, **Grafická mierka**, **Revízny obláčik**, **Tabuľka revízií** a **Orientačný pôdorys**;
- **Pomocné čiary**: pravítka a pomocné čiary (nižšie);
- **PDF** tohto listu a **Ukážka tlače**: list tak, ako sa vytlačí, na bielom.

## Umiestňovanie pohľadov {#placing-views}

1. Otvorte list.
2. Kliknite na **Umiestniť pohľad…** a vyberte pohľad: pôdorys, rez, pohľad, 3D pohľad alebo výkaz.
3. Potiahnite ho na miesto na papieri.

Každý umiestnený pohľad (**výrez listu**) zobrazuje svoj pohľad v mierke pohľadu, bez toho, čo pohľad skrýva.

- **Mierka** výrezu: vyberte ho a vo *Vlastnostiach* vyberte mierku (1:1 až 1:500, alebo za *1:* zadajte ľubovoľnú inú), alebo naň kliknite pravým tlačidlom a zvoľte **Mierka**. Výrez sa zväčší alebo zmenší okolo stredu. Mierka patrí pohľadu, takže sa pohľad v tejto mierke zobrazí na každom liste, na ktorom je.
- **Orezanie** výrezu: vyberte ho a potiahnite úchopy hrán jeho čiarkovaného orezového rámu, alebo nastavte orezanie vo *Vlastnostiach*.
- **Zarovnanie pohľadov**: pri ťahaní pohľadu sa jeho stred prichytí v línii so stredom iného pohľadu, s čiarkovanou pomocnou čiarou.
- **Práca cez výrez**: dvojitým kliknutím na výrez upravujete model priamo tam, všetkými nástrojmi; zvyšok listu zošedne. Ťahaním otáčate 3D pohľad. `Esc` ukončí.

![Pracovný priestor Listy: list vedľa pôdorysu umiestneného na ňom](img/sheet.jpg)

### Názvy pohľadov {#view-titles}

Pod každým výrezom je jeho **názov**. Vo *Vlastnostiach* výrezu: zobraziť alebo skryť ho, jeho **Číslo v krúžku**, **Mierka pod ním**, jeho **Text** (názov pohľadu alebo vlastný), **Veľkosť** a **Štýl textu**. Potiahnutím názvu na liste ho umiestnite.

## Výber a presun na papieri {#selecting-and-moving-on-the-paper}

- Ťahaním obdĺžnika na prázdnom papieri vyberiete pohľady, značky a poznámky: zľava doprava zachytí to, čo je vnútri, sprava doľava to, čoho sa dotkne; `Shift` pridáva.
- **Presunúť** (`M`, alebo `G` na sledovanie ukazovateľa) presúva vybrané na liste. Prichytáva sa k papieru, značkám, poznámkam, pohľadom a pomocným čiaram. Ťahanie jednej z viacerých vybraných položiek presunie všetky.
- Stĺpec priblíženia vpravo v oblasti približuje, prispôsobuje a zobrazí výber.

## Stĺpce tabuliek {#table-columns}

Vyber umiestnenú legendu miestností alebo iný výkaz a na čiarach jeho stĺpcov sa ukážu úchopy. Ťahaním čiary medzi dvoma stĺpcami zúžiš alebo rozšíriš stĺpec vľavo od nej, po pol milimetri; stĺpce za ním sa posunú. Legenda TZB a legenda materiálov majú dva stĺpce, vzorku alebo značku a jej popis, ktorý sa zalomí na nastavenú šírku. Dvojklik na čiaru vráti stĺpcu jeho predvolenú šírku. Šírky patria pohľadu výkazu, takže vyzerá rovnako na každom liste, kde je umiestnený.

## Titulné strany {#title-pages}

**Výkresy na tlač** vytvorí titulnú stranu projektu a jednu pre každú časť dokumentácie, ktorá má výkresy (C, D.1 ASR, D.3 ZTI, D.4 VYK, D.5 ELI, D.6 VZT, D.7 PLY, D.8 MPL), s číslom pred výkresmi časti (`S01_ZTI_000_TIT`). Vypĺňajú sa samé z *Údajov o stavbe*: ateliér a jeho kontakt, nadpis podľa stupňa (alebo tvoj vlastný *Nadpis*), stavba, investor, miesto a parcely, tri mená, zákazkové číslo a dátum. Titulka časti ukazuje jej označenie a názov, titulka projektu stavbu v 3D v ráme. Na A4 sa zmestia štyri štítky A6 na zväzky, očíslované 1 až 4; vo vlastnostiach listu nastav *Štítky na liste* na 1 pre jednu cez celý list. Titulné strany sa neuvádzajú v zozname dokumentácie.

## Pravítka a pomocné čiary {#rulers-and-guides}

V hlavičke listu zapnite **Pomocné čiary**: pozdĺž horného a ľavého okraja sa zobrazia pravítka.

- Pomocnú čiaru vytiahnete z pravítka; ťahaním ju presuniete; potiahnutím späť na pravítko ju odstránite.
- Pohľady (ich stred alebo okraje), značky, poznámky a revízne obláčiky sa k pomocným čiaram prichytávajú.
- Vo *Vlastnostiach* listu: **Kopírovať pomocné čiary na všetky listy**, aby sa sada zarovnávala strana po strane, alebo **Vymazať pomocné čiary**.

Pomocné čiary sa nikdy netlačia. Keď sú vypnuté, uchovajú sa s listom.

## Text, severka a grafická mierka {#text-north-arrow-and-scale-bar}

To, čo patrí k papieru, nie k pohľadu, sa umiestňuje priamo na list:

- **Text** (`N`): kliknite na začiatok a píšte. Je to [poznámka](notes) na liste.
- **Severka**: natočená na skutočný sever projektu (*Údaje o stavbe*). Vo *Vlastnostiach* jej môžete dať vlastný uhol.
- **Grafická mierka**: v mierke pohľadov listu. Mierku, dĺžku, dielce a hrúbku nastavíte vo *Vlastnostiach*.
- **Obrázok**: fotografia, vizualizácia, logo alebo pečiatka. Vyberte naraz ľubovoľný počet súborov PNG, JPEG a SVG a uložia sa vedľa seba. Pixely obrázka sa ukladajú v projekte; SVG sa vloží ako ostré vektorové tvary. Úchopy rámu menia jeho veľkosť (rohy zachovajú pomer strán) a *Vlastnosti* obsahujú veľkosť, nepriehľadnosť a pri SVG jednu farbu.

Umiestnia sa mimo rohovej pečiatky. Potiahnite ich na miesto alebo zadajte ich **X** a **Y** na liste s presnosťou na milimeter; `Delete` ich odstráni. Kóty, značky a poznámky o budove zostávajú v pohľadoch, takže sa zobrazia na každom liste, na ktorom je pohľad umiestnený.

## Revízie {#revisions}

- **Revízny obláčik**: zúbkovaný obrys okolo zmeny s písmenom revízie v trojuholníku (písmeno za poslednou revíziou projektu). Čo a kedy sa zmenilo, uveďte vo *Vlastnostiach*. Potiahnutím úchopov rohov a hrán zmeníte veľkosť obdĺžnika, alebo ho zmeňte na **Voľný tvar**: ťahajte jeho body, potiahnutím **+** pridáte bod, pravým tlačidlom na bod ho odstránite. **Oblúčiky** nastavujú dĺžku každého oblúka; môže mať vlastnú farbu.
- **Tabuľka revízií**: riadok za každý revízny obláčik na liste. Potiahnite ju na miesto.

## Orientačný pôdorys {#key-plan}

**Orientačný pôdorys** umiestni najnižší pôdorys znova ako malý samostatný pohľad v mierke 1:500 nad rohovú pečiatku, so skrytými kótami, značkami, poznámkami, osami, značkami rezov a nábytkom. Potiahnite ho na miesto; je to pohľad v prehliadači, ktorý zmeníte ako ktorýkoľvek iný.

## Rohové pečiatky a predlohy {#title-blocks-and-masters}

Rohovú pečiatku vyplníte vo *Vlastnostiach* listu. Polia celého projektu (stavba, investor, adresa, stupeň) pochádzajú z *Správa › Údaje o stavbe* a vlastné hodnoty listu majú prednosť.

Rám a rohová pečiatka, na ktorých list leží, sú jeho **predloha**. Vyberte ju v *Rohovej pečiatke* listu alebo kliknite na **Upraviť predlohu** a otvorí sa v pohľade Predlohy, kde sa kreslí čiarami, tvarmi, textom, poľami, logami a obrázkami: čo zmeníte, zobrazí sa na každom liste, ktorý ju používa. Pozri [Predlohy: rohové pečiatky](masters).

*Súbor › Šablóny listov…* je zoznam predlôh: *Kópia zabudovanej pečiatky*, *Importovať DXF alebo SVG…* pre vlastný výkres alebo predlohy a štýly iného projektu. Akékoľvek `{name}` v texte predlohy sa stane poľom. `{sheet}`, `{number}`, `{project}`, `{scale}` a `{date}` vyplní každý list; ostatné polia dostanú hodnotu za list v jeho *Vlastnostiach*. Použite *Použiť na tomto liste*, *Použiť na každom liste* alebo *Nový list z nej*.

## Tlač a export {#printing-and-exporting}

- `Ctrl+P` alebo tlačidlo PDF v hlavičke listu: list v popredí ako PDF pomenované číslom a názvom.
- `Ctrl+Shift+P` alebo *Súbor › Export › Listy ako PDF*: okno **Export listov**. Zvoľte **Všetky**, **Vybrané** (listy vybrané v prehliadači projektu alebo zaškrtnuté v zozname) alebo **Tento** a **Jedno PDF** alebo **Každý ako PDF** (do priečinka, každý pomenovaný číslom a názvom). Listy sa tlačia na bielom vo vlastných veľkostiach.
- *Súbor › Tento pohľad alebo list ako SVG* zapíše otvorený list. Pozri [PDF, SVG a DXF](dxf-svg).
