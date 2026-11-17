# Miestnosti

**Miestnosť** je priestor na podlaží: pracovňa, hala, kuchyňa. Má názov a číslo a pozná svoju **plochu**, **obvod** a **výšku**. Tie sa odvodzujú od stien okolo nej, takže sa aktualizujú hneď, ako sa stena posunie. Miestnosti nesú údaje, ktoré vypisuje výkaz miestností: nášľapnú vrstvu, povrchovú úpravu stien a stropu, užívanie a oddelenie.

## Umiestňovanie miestností {#placing-rooms}

1. Zvoľte nástroj **Miestnosť** (v *Vytvoriť › Stavebné prvky*, za Objektom).
2. Kliknite medzi steny. Miestnosť vyplní priestor po **vnútorné povrchy** stien, dostane ďalšie číslo na svojom podlaží (0.01, 0.02, … na prízemí, 1.01, … na prvom poschodí) a značku v pôdorysoch podlažia.
3. Vo *Vlastnostiach* zadajte jej názov a prípadne povrchové úpravy a užívanie, ak ich chcete vo výkaze.

Nástroj má aj spôsoby kreslenia nástroja Doska, voľené klávesmi `1`–`4` alebo vo voľbách nástroja:

- **Vyplniť miestnosť (3)** je kliknutie medzi steny opísané vyššie. Miestnosť si pamätá len svoj bod; jej obrys sa pri každom kreslení nájde znova zo stien, takže **sleduje steny**.
- **Mnohouholník (1)**, **Obdĺžnik (2)** a **Z náčrtu (4)** nakreslia obrys miestnosti ručne. Použite ich pre terasu, lodžiu alebo balkón, kde priestor neuzatvárajú steny. Nakreslená miestnosť si ponechá obrys a steny nesleduje; *Radšej sledovať steny* vo *Vlastnostiach* ju zmení späť na takú, ktorá ich sleduje.

Kliknutie tam, kde už miestnosť je, ju vyberie namiesto vytvorenia druhej.

## Deliace čiary miestností {#room-separators}

Otvorený priestor nemá medzi kuchyňou a jedálňou stenu. Nakreslite **deliacu čiaru miestností** tam, kde má byť hranica: zvoľte nástroj **Deliaca čiara miestností** a kliknite na jej dva konce. Miestnosti na oboch stranách sa potom merajú oddelene a každá ju pri presune nasleduje. Deliace čiary sa v pôdorysoch zobrazujú tenkými čiarami a v rezoch ani v 3D sa netlačia.

## Čo miestnosť meria {#what-a-room-measures}

- **Plocha**: po vnútorné povrchy stien okolo nej, mínus šachty cez jej podlahu (schodiskový priestor, inštalačná šachta). Uvádza sa v jednotkách projektu, v m² alebo ft².
- **Obvod**: dĺžka jej obrysu.
- **Výška**: po podhľad nad ňou, ak ho má, inak po podlažie nad ňou. Zaškrtnutím *Vlastná výška* vo *Vlastnostiach* nastavíte vlastnú.

Ak steny okolo bodu miestnosti prestanú priestor uzatvárať (jednu ste odstránili alebo príliš posunuli), miestnosť uvádza **Neuzavretá** v značke, vo *Vlastnostiach* aj vo výkaze, kým ju steny opäť neuzavrú alebo kým nenakreslíte jej obrys.

## Značky miestností {#room-tags}

Značky miestností sa zobrazujú v pôdorysoch a v detailoch a výrezoch sa vynechávajú, pokiaľ pohľad nemá zapnuté *Zobraziť značky miestností*.

Každá nová miestnosť dostane **značku** vo svojom bode v každom pôdoryse svojho podlažia. Obsahuje názov miestnosti veľkými písmenami, potom číslo a plochu. Potiahnutím značku presuniete; zostáva pri svojej miestnosti. Vyberte značku miestnosti a vo *Vlastnostiach › Zobrazuje* zaškrtnite polia, ktoré vypisuje:

- *Názov*, *Číslo*, *Plocha*, *Obvod*, *Výška*, *Podlažie*
- *Nášľapná vrstva*, *Povrchová úprava steny*, *Povrchová úprava stropu*, *Užívanie*, *Oddelenie*

Pole, pre ktoré miestnosť nemá hodnotu, sa vynechá. Štýl textu značky je štýl *Názov miestnosti*, ak ho projekt má; zmeníte ho ako pri ktorejkoľvek inej značke. Pozri [Značky](tags) a [Štýly popisov](styles).

Tlačidlo **Označiť miestnosti** vo voľbách pohľadu pôdorysu označí každú miestnosť v pôdoryse, ktorá tam ešte značku nemá. Nástroj **Značka** označí aj miestnosť alebo podhľad.

## Popisy miestností zo starších projektov {#room-notes-from-older-projects}

Projekty vytvorené pred zavedením miestností zapisovali názvy a plochy miestností ako **poznámky** („OBÝVAČKA / 37,2 m²“). *Vytvoriť › Miestnosti z popisov miestností* zmení každú takú poznámku na miestnosť so značkou na jej mieste, všade, kde ju uzatvárajú steny. Poznámky, ktoré zostanú (pri otvorenej terase alebo dvoch popisoch v jednom priestore), sa vypíšu v stavovom riadku: nakreslite tieto miestnosti alebo pridajte deliace čiary a spustite príkaz znova. Nástroj príkazového riadka urobí to isté príkazom `creabim rooms-from-notes <model> <výstup>`.

## Farby miestností {#room-colours}

Zaškrtnite **Farbiť miestnosti** vo *Vlastnostiach* pôdorysu alebo rezu (alebo **Farby miestností** vo voľbách pohľadu pôdorysu) a každá miestnosť sa vyplní vlastnou farbou. Miestnosť má rovnakú farbu vo všetkých pohľadoch a rez zobrazuje prerezané miestnosti ako pásy do ich výšky.

## Výkaz miestností {#the-room-schedule}

*Súbor › Nový výkaz › Miestnosti* vytvorí výkaz všetkých miestností s číslom, názvom, podlažím, plochou, obvodom, výškou, nášľapnou vrstvou a užívaním. Miestnosti sú zoskupené podľa podlaží, každé podlažie s počtom miestností a celkovou plochou, a posledný riadok sčíta celú budovu. Pozri [Výkazy](schedules). `creabim schedule <model> rooms` ho vypíše ako CSV.

## IFC

Miestnosti sa exportujú ako **IfcSpace**: číslo ako *Name*, názov ako *LongName*, obrys ako hranol do jej výšky a plocha, obvod a výška v *Qto_SpaceBaseQuantities*. Povrchové úpravy a užívanie idú do *Pset_SpaceCoveringRequirements* a *Pset_SpaceOccupancyRequirements*. Každá stena okolo nej je prepojená cez **IfcRelSpaceBoundary** (deliaca čiara cez virtuálnu) a podhľady nad ňou cez **IfcRelCoversSpaces**.

Pri otvorení súboru IFC sa z každého IfcSpace opäť stane miestnosť. Miestnosť zapísaná CreaBIM opäť sleduje svoje steny; priestor z iného programu si ponechá obrys svojho telesa. Pozri [IFC](ifc).

**Pozri aj:** [Podhľady](ceilings), [Nábytok a zariaďovacie predmety](furniture)
