# Pohľady

**Pohľad** je v CreaBIM spôsob, ako sa pozerať na model, uložený v projekte: pôdorys, rez, pohľad (fasáda), 3D pohľad alebo výkaz. Pohľady sa z modelu kreslia vždy nanovo, takže nikdy nezastarajú. Ukladajú sa, vracajú a aktualizujú spolu s ním.

## Druhy pohľadov {#kinds-of-views}

- **Pôdorysy**: budova prerezaná vo výške nad podlažím (*Výška rezu*, obvykle asi 1,2 m) pri pohľade zhora. Prerezané steny sa kreslia hrubo a so šrafou; dvere ukazujú otváranie. Pozri [Ako sa kreslia pôdorysy](views#how-plans-are-drawn).
- **Rezy** a **pohľady**: pozri [Rezy a pohľady](sections).
- **3D pohľady**: uložené rovnobežné 3D pohľady, napríklad *3D pohľad juhovýchod*, na umiestnenie na listy. Uhol nastavíte *Nadmorskou výškou* (výškou pohľadu) a vyberiete *Štýl*.
- **Výkazy**: tabuľky prvkov. Pozri [Výkazy](schedules).

## Otvorenie pohľadu {#opening-a-view}

- Kliknite naň v prehliadači projektu: pohľady sú zoskupené na *Pôdorysy*, *3D pohľady*, *Rezy*, *Pohľady* a *Výkazy*.
- Alebo ho vyberte v hlavičke oblasti: ponuka druhu pohľadu a potom názov pohľadu.
- `PageUp` a `PageDown` prejdú na predchádzajúci alebo nasledujúci pohľad rovnakého druhu.

Pohľad sa otvorí v oblasti, v ktorej ste naposledy pracovali. Viac oblastí môže zobrazovať pohľady vedľa seba; pozri [Oblasti pohľadov](interface#view-areas). V ktorejkoľvek z nich môžete kresliť a vyberať všetkými nástrojmi.

## Nastavenia pohľadu {#the-view-s-settings}

Hlavička oblasti, ktorá pohľad zobrazuje, obsahuje jeho nastavenia a *Vlastnosti* (karta *Pohľad*, keď nie je nič vybrané) obsahujú zvyšok:

- *Výška rezu* (pôdorysy), *Hĺbka* (rezy a pohľady) alebo výška pohľadu (3D pohľady);
- *Štýl* (rezy, pohľady a 3D pohľady): *Skryté čiary*, *Tieňované s hranami* alebo *Drôtový model*, ktorý kreslí všetky hrany vrátane skrytých. Listy zobrazujú pohľad rovnako. Šachty sa kreslia čiarkovane;
- *Skryté* a *Farebne*: prvky skryté vnútri iných sa kreslia čiarkovane a zafarbené svojou farbou (pozri [Viditeľnosť](visibility#hidden-elements));
- *Röntgen*: priehľadné steny a dosky (pozri [Viditeľnosť](visibility#x-ray));
- *Štýl kresby* (pôdorysy a rezy): ako sa kreslí to, čo je prerezané (pozri [Grafické štýly](graphic-styles#drawing-styles));
- *Dočasné kóty* (pôdorysy): zapnuté alebo vypnuté;
- *Označiť otvory*: označí každé dvere a okno bez značky;
- *Podklad* (pôdorysy): iné podlažie slabo zobrazené pod ním;
- *Viditeľnosť*: čo tento pohľad vynecháva.

Keď nie je nič vybrané, *Vlastnosti* zobrazujú to isté a viac, napríklad mierku pohľadu.

## Ako sa kreslia pôdorysy {#how-plans-are-drawn}

Pôdorysy dodržiavajú obvyklé zásady výkresov pre stavebné povolenie:

- **Hrúbky čiar** klesajú od hrubých k tenkým: prerezané steny, stĺpy a základy sú najhrubšie, potom to, čo je viditeľné pod rezom (hrany dosiek, schodisko), potom značky (krídla a otváranie dverí, rámy okien), potom nábytok a zariaďovacie predmety, potom šrafy. Čiary nad rezom (nosníky, horné skrinky) sú tenké a čiarkované. Hrúbky zodpovedajú mierke pohľadu: hrubšie v 1:50, tenšie v 1:100. Pochádzajú z [balíka štýlov](graphic-styles) projektu.
- **Pod rezom** dostane nábytok, vybavenie a importované siete pod svojimi čiarami jemný tón farby svojho materiálu.
- **Napojenia**: prerezané steny a základy z rovnakého materiálu, ktoré sa dotýkajú alebo prekrývajú, sa zlúčia do jedného obrysu bez vnútorných čiar.
- **Schodiská**: na vlastnom podlaží sa schodisko kreslí po lomovú čiaru, kde ho pôdorys prerezáva. Na podlaží nad ním je schodisko viditeľné cez schodiskový priestor s označením *DN* a priestor nad ním sa nepreškrtáva. Kríž otvoru vedie z rohu do rohu v každej časti otvoru, takže priestor v tvare L dostane kríž v každom ramene. Na vlastnom podlaží schodiska je priestor nad ním čiarkovaný obrys.
- **Pôdorysy striech** zobrazujú na každej šikmej ploche šípku nadol so sklonom (*35°* alebo *1:60* pri plochých sklonoch).
- **Popisy**: názvy miestností, poznámky, značky a obyčajné popisy sa odsunú z osí, čiar rezov, stien a jeden od druhého na najbližšie voľné miesto. Ďaleko posunutý popis dostane odkazovú čiaru k pôvodnému miestu. Krúžky osí a rezov a texty kót zostávajú na mieste. V príkazovom riadku `creabim label-clashes <model>` vypíše popisy, ktoré sa stále prekrývajú.

### Grafika pôdorysu {#plan-graphics}

*Vlastnosti* pôdorysu majú skupinu *Grafika*:

- *Tónovať to, čo je pod rezom*, jeho *Sila* a *V sivej, nie vo farbách materiálov*.
- *Hĺbka pohľadu*: ako hlboko pod svoju podlahu pôdorys vidí cez otvory v podlahe, napríklad schodiskový priestor. Medzi podlahou a rezom je to *pod* rezom, kreslené v pohľade. Pod podlahou až po hĺbku pohľadu je to *za*. Predvolene pôdorys vidí až na podlažie pod ním.
- *Hrúbky čiar z balíka štýlov*: zrušte zaškrtnutie a nastavte hrúbku každého pera v milimetroch na papieri (rez, pohľad, značky, nábytok a zariaďovacie predmety, šrafy, skryté) len pre tento pohľad. Listy, PDF a SVG používajú rovnaké hrúbky.
- Pero, farba, vzor a šrafa každej kategórie sú **štýly objektov** balíka štýlov projektu a pohľad ich môže prepísať: pozri [Grafické štýly](graphic-styles).
- *Rozsah pohľadu*: ktoré podlažia pôdorys kreslí a pôdorys strechy alebo situácia pri pohľade zhora: pozri [Rozsah pohľadu](view-range).

## Poradie v prehliadači {#ordering-the-browser}

Potiahnite pohľad v jeho skupine hore alebo dolu. Skupina si potom ponechá vaše poradie (uvádza *Vaše poradie*); jej tlačidlo radenia ju vráti do automatického poradia (pôdorysy podľa podlaží).
