# Predlohy: rohové pečiatky

**Predloha** je výkres, na ktorom leží každý list: jeho papier, rám a rohová pečiatka s názvom stavby, číslom listu a dátumom vyplnenými každým listom. Predlohy sa kreslia vo vlastnom pohľade, v pohľade **Predlohy**, čiarami, tvarmi, textom, poľami, obrázkami a logami.

## Otvorenie pohľadu Predlohy {#opening-the-masters-view}

- Vyberte *Predlohy* v ponuke druhu pohľadu ktorejkoľvek oblasti alebo otvorte skupinu *Predlohy* v prehliadači projektu.
- Vo *Vlastnostiach* listu kliknite na **Upraviť predlohu**.

Hlavička vyberá predlohu a obsahuje **+** (nová predloha zo *Štandard A3*, *Minimálna A3* alebo prázdna), *Duplikovať*, *Exportovať SVG* a *Odstrániť*. **Načítať SVG alebo DXF…** prevezme rám alebo rohovú pečiatku nakreslenú v ľubovoľnom kresliacom programe.

![Predloha v pohľade Predlohy](img/masters.jpg)

## Kreslenie na predlohu {#drawing-on-a-master}

| Nástroj | Čo vytvorí |
| --- | --- |
| **Výber** | Výber, ťahanie alebo zmena veľkosti za roh (`Shift` zachová pomer strán) |
| **Čiara** | Klikajte na rohy; `Enter` alebo pravé tlačidlo ju ukončí, opätovný prvý roh ju uzavrie |
| **Tvar** | Obdĺžnik (dva rohy), kružnica (stred, potom bod na nej) alebo elipsa (dva rohy obdĺžnika okolo nej), z rozbaľovacej ponuky |
| **Text** | Kliknite, kam patrí; jeho text, veľkosť a zarovnanie sú vo *Vlastnostiach* |
| **Pole** | Text, ktorý vyplní každý list: `{sheet}`, `{number}`, `{project}`, `{scale}`, `{date}` alebo vlastné pole (investor, vypracoval, kontroloval…) |
| **Pomocná čiara** | Dve kliknutia; takmer vodorovná alebo zvislá sa narovná |
| **Obrázok** | Logo, pečiatka alebo pozadie: SVG sa vloží ako ostré vektorové logo, PNG alebo JPEG ako obrázok |

Jedno pravidlo pre všetky nástroje: `Enter` alebo pravé tlačidlo ukončí nakreslené a ponechá ho, `Esc` zahodí rozpracovaný kus a ktorékoľvek z nich, keď nič neprebieha, nástroj odloží.

Všetko sa prichytáva k nakreslenému, k okrajom, k milimetrovej mriežke a k pomocným čiaram. `M` presunie vybrané, `R` ho otočí, `Ctrl+D` ho skopíruje o 5 mm vedľa a `Delete` ho odstráni. `Ctrl+]` ho dá nad všetko ostatné, `Ctrl+[` pod. `Ctrl` + ťahanie stredným tlačidlom približuje.

## Karta Predloha vo Vlastnostiach {#the-master-tab-in-properties}

Keď je v popredí pohľad Predlohy, *Vlastnosti* zobrazujú predlohu:

- **Papier** a **Merané od**: roh, od ktorého sa rohová pečiatka meria;
- **rám** a jeho **okraje** (ľavý, dolný, pravý, horný);
- **vlastné polia**, pomenované tu a vyplnené za každý list;
- **Pomocné čiary**: počet, zobraziť alebo skryť, nakresliť jednu, vymazať všetky. Pomocné čiary sa uchovávajú s predlohou a nikdy sa netlačia;
- **Obrázky a logá**, každé s veľkosťou, nepriehľadnosťou a pri logu jednou farbou;
- **vybraná** čiara, tvar, text, pole alebo obrázok s perom, farbou výplne, nepriehľadnosťou a textom.

**Nový list s ňou** vytvorí list s touto predlohou; **Použiť na každom liste** ju priradí všetkým listom projektu v jednom kroku pre Späť. Odstránenie predlohy vráti jej listy na zabudovanú pečiatku.

Predlohy sa tlačia na listoch, v PDF aj v exporte SVG presne tak, ako sú nakreslené. Na tmavom papieri sa čierne čiary pri práci zobrazujú svetlo a aj tak sa tlačia čierne.

Pozri aj [Listy a rohové pečiatky](sheets).
