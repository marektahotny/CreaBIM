# Obkreslenie a podklad

**Podklad** je niečo zobrazené slabo pod pôdorysom alebo rezom (alebo nad ním), na obkreslenie alebo porovnanie:

- **iný pohľad**: podlažie pod tým, ktoré kreslíte;
- **obrázok**: naskenovaný pôdorys, fotografia náčrtu (PNG, JPEG, TIFF alebo BMP);
- **výkres DXF**: geodetické zameranie alebo výkres inej profesie. DWG otvoríte po uložení ako DXF: urobí to ktorýkoľvek CAD program alebo bezplatný ODA File Converter bez pripojenia;
- **strana PDF**: výkres poslaný ako PDF, vykreslený v mierke 1:N, alebo naskenovaná strana.

Podklad sa ukladá s projektom. Jeden podklad sa môže zobrazovať vo **viacerých pohľadoch** (zameranie pozemku pod každým podlažím). Obrázky a výkresy zostávajú na disku tam, kde sú, ako odkazy: *Znova načítať* ich načíta znova po zmene.

## Pridanie {#adding-one}

**Podklad** na karte *Pohľad* otvorí kartu **Obkreslenie a podklad** vo *Vlastnostiach* pre pohľad v oblasti, ktorú ste naposledy použili. Vyberte iný pohľad v ponuke *Pridať* alebo kliknite na **Obrázok…**, **DXF výkres…** alebo **PDF…**. Pridá sa na mieste a vyberie sa. Obrázok, súbor DXF alebo PDF pustený na pôdorys či rez sa tiež stane podkladom.

Rovnako to ide z ponuky **Súbor › Import**: *Mračno bodov (XYZ, LAS…)*, *PDF ako podklad*, *Obrázok ako podklad (PNG, JPG)* a *DXF ako podklad* pridajú podklad do otvoreného pôdorysu alebo rezu. *Terén zo zameraných bodov (XYZ)* tam vytvorí terén na pracovnom podlaží.

## Umiestnenie {#placing-it}

- Potiahnutím **štvorcového úchopu** v strede ho presuniete.
- Potiahnutím **rohu** ho zmeníte mierkou okolo stredu.
- Potiahnutím **okrúhleho úchopu** nad ním ho otočíte okolo stredu (po 5°; `Alt` pre ľubovoľný uhol).
- Alebo vo *Vlastnostiach* zadajte *X*, *Y*, *Otočenie* a *Mierku*.
- **Kalibrovať…**: kliknite na dva body na ňom so známou vzdialenosťou (okótovaná stena na skene) a zadajte túto vzdialenosť. Zmení sa mierka, aby zodpovedal; prvý bod zostane na mieste.
- **Zarovnať…**: kliknite na dva body na podklade a potom na dva body modelu, na ktoré patria: posunie sa, otočí a zmení mierku na ne.
- **Späť na miesto**: tam, kde bol vložený, neposunutý, neotočený, vo vlastnej veľkosti.
- **Presunúť**, **Otočiť** a **Zmeniť mierku** z panela nástrojov fungujú na podklady ako na každý prvok.

V reze stojí podklad v rovine rezu a jeho úchopy a presuny zostávajú v tejto rovine. Jeho čiary sa prichytávajú: konce, stredy a kdekoľvek pozdĺž nich.

Jeho čiarkovaný obrys sa zobrazí, keď je vybraný alebo pod ukazovateľom, v pôdorysoch aj v 3D; kliknutím na obrys ho vyberiete. **Zamknuté** vypne úchopy.

**Kopírovať do pohľadu…** vloží samostatnú kópiu toho istého výkresu do iného pohľadu s vlastnou polohou, veľkosťou a nastaveniami.

Obkreslenie umiestnené v [pohľade profilu](profiles#the-profile-view) patrí k tomuto profilu: je zaškrtnuté **Len v tomto profile** a nekreslí sa v pôdorysoch, rezoch ani 3D. Zrušením zaškrtnutia zvolíte pohľady, v ktorých sa zobrazí, ako pri každom inom podklade.

## Výkres DXF {#a-dxf-drawing}

DXF sa načíta ako v PilonDesigneri: čiary, krivky s oblúkmi, kružnice, elipsy a splajny, texty (TEXT a MTEXT), šrafy (ich čiary vzoru, plné výplne ako obrysy), bloky s poliami, kóty, body a 3D plochy, po hladinách a v metroch podľa jednotiek súboru (v milimetroch, ak ich neuvádza). Výkresy uložené pred AutoCADom 2007 sa načítajú v ich kódovej stránke, takže slovenské a české znaky (ANSI 1250) sú správne.

- **Hladiny**: zaškrtnite hladiny, ktoré sa majú zobraziť; *Všetko* a *Nič* ich prepnú naraz.
- **Texty** sa zobrazujú vo svojej veľkosti a otáčajú sa, keď sú na obrazovke dosť veľké na čítanie.
- **Body**: geodetické body, malé kružnice, ktoré ich označujú, a body vloženia blokov sa prichytávajú.
- Výkres v národných súradniciach (**S-JTSK**, kilometre od počiatku) sa presunie do počiatku projektu ľavým dolným rohom; posuňte ho vo *Vlastnostiach*.
- **3D zameranie** zobrazuje rozsah svojich výšok. V 3D stoja jeho 3D čiary a plochy vo svojich výškach, vzťahované k ±0,000 projektu ako terén z neho vytvorený. **Terén z výšok výkresu** vytvorí terén cez jeho body, vrcholy 3D čiar a rohy plôch, na zobrazených hladinách a v jeho orezaní (pozri [Pozemok](site#terrain-from-surveyed-points)).

## Obrázok {#a-picture}

Obrázok sa vloží tak široký, ako je počet pixelov pri jeho **rozlíšení**, kreslený v **1:100**, ako PilonDesigner umiestňuje sken: rozlíšenie skenu sa načíta zo súboru (PNG, JPEG, TIFF), inak sa uvažuje 150 DPI. Vo *Vlastnostiach* nastavte **Rozlíšenie** na DPI skenu a **Mierku výkresu** na mierku vytlačenú na výkrese a obrázok dostane správnu veľkosť (šírka = pixely ÷ DPI × N). Keď nie je známe ani jedno, *Kalibrovať…* nastaví veľkosť podľa známej vzdialenosti.

## Strana PDF {#a-pdf-page}

PDF sa načíta ako v PilonDesigneri: čiary strany, jej plné výplne vo svojej farbe (biele plochy a pozadie strany sa vynechajú) a texty, v **mierke 1:N** výkresu: PDF meria papier, takže model je papier krát N. Vloží sa v 1:100; vo *Vlastnostiach* nastavte **Mierku výkresu** na mierku vytlačenú na výkrese a zväčší sa alebo zmenší okolo stredu. Strana otočená v PDF sa načíta otočená, ako je vidieť. **Strana** vyberie inú stranu súboru.

Čiary, výplne a texty sú v hladinách *PDF*, *PDF výplne* a *PDF texty*, takže ich *Hladiny* zapínajú a vypínajú ako pri DXF.

**Naskenovaná strana** (obrázok bez čiar) sa vloží ako obrázok, umiestnený tam, kde leží na strane. Skeny uložené ako JBIG2, CCITT alebo JPEG 2000 sa tu nedajú načítať: exportujte stranu ako PNG alebo JPEG a pridajte ten. Ak mierka nie je známa, skontrolujte veľkosť cez *Kalibrovať…*.

## Orezanie {#clip}

**Orezať…**: kliknite na dva protiľahlé rohy v pohľade; zobrazí sa len to, čo leží medzi nimi (čiary sa na okraji odrežú). **Celý** zobrazí opäť všetko.

## Vzhľad {#how-it-looks}

- **Farba**: červená, modrá, sivá alebo zelená, alebo *vlastné farby* (obrázka).
- **Sila**: ako slabo sa zobrazuje.
- **Pod** alebo **Nad** modelom.
- **Skryté**, alebo `Alt+F2` skryje či zobrazí všetky podklady v pohľade.

## V 3D {#in-3d}

V časti **V 3D** v jeho *Vlastnostiach*: **Zobrazené** kreslí podklad aj v 3D pohľade, na jeho podlaží alebo v rovine jeho rezu (obrázok zobrazí svoj obraz), kde ho možno vybrať a prichytávať sa k nemu. Môže tam mať **Vlastnú intenzitu** a kresliť sa **V perspektíve** alebo plocho.

Podklady nikdy nie sú v exportoch, výkazoch ani IFC.
