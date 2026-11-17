# Kóty

Kóty merajú model. Ich body sa držia toho, čo merajú (rohov stien, hrán dverí a okien, podlaží, osí, stĺpov, objemov), takže sa aktualizujú, keď sa to posunie.

## Dočasné kóty {#temporary-dimensions}

Vyberte v pôdoryse jednu stenu, os, dvere, okno alebo stĺp. Modré kóty ukazujú vzdialenosť k najbližším susedom: k rovnobežným stenám a osiam vedľa steny alebo osi, k najbližším osiam alebo stenám pri stĺpe a ku koncom steny alebo susednému otvoru pri dverách alebo okne.

- **Kliknite na hodnotu a zadajte novú**, potom `Enter`. Prvok sa posunie, aby platila. Stena vezme so sebou svoje dvere a okná a steny na ňu napojené; os vezme steny a stĺpy, ktoré na nej stoja. `Tab` prejde na ďalšiu hodnotu, `Backspace` vráti znak a `Esc` skončí.
- **Kliknite na zámok** vedľa hodnoty a ponecháte ju ako kótu, ktorá drží rozostup. Posun ktoréhokoľvek konca potom posunie aj druhý o rovnakú vzdialenosť. Kliknutím na zatvorený zámok ju uvoľníte; kóta zostane. Kóta, ktorá drží rozostup, má pri hodnote slabý zámok, výrazný, keď je vybraná ona alebo jeden z jej koncov; je len na obrazovke, takže listy vytlačia obyčajnú kótu.

Zadávanie funguje ako zadanie dĺžky pri kreslení steny. Do výkresu sa nič nepridá, pokiaľ hodnotu nezamknete.

Vybraná stena vždy zobrazuje vedľa seba svoju dĺžku, ktorú tiež možno zadať: posunie sa koniec bližší k stredu pohľadu a steny napojené na ňom ho nasledujú. Medzery k susedom vzdialeným viac ako 10 m sa vynechajú a hodnota nikdy neleží na úchope prvku.

### Podlažia v rezoch {#levels-in-sections}

Vyberte podlažie v reze alebo pohľade: zobrazí svoju výšku nad podlažím pod ním a pod podlažím nad ním. Kliknite na jednu a zadaním hodnoty posuniete podlažie so všetkým, čo je na ňom. Zámok drží konštrukčnú výšku podlažia. Zadanú hodnotu prijme aj umiestnená kóta, ktorej oba konce sú na podlažiach: posunie sa podlažie na konci bližšom ku kliknutiu, druhé zostane.

Dočasné kóty možno stlmiť prepínačom v hlavičke pôdorysu; keď sú vypnuté, žiadne sa nezobrazia a na žiadnu sa nedá kliknúť.

## Lineárne kóty {#linear-dimensions}

1. Zvoľte nástroj **Kóta**.
2. Kliknite na dva body, ktoré chcete merať. Prichytávajú sa k rohom a hranám.
3. Kliknite, kam má ísť kótovacia čiara.

Vo voľbách nástroja zvoľte smer:

- **Automaticky**: podľa toho, kam ťaháte čiaru. Vedľa oboch bodov meria zvislo, nad nimi alebo pod nimi vodorovne, inak pozdĺž dvoch bodov. Podržaním `Shift` ponecháte zobrazený smer.
- **Zarovnané**: skutočná vzdialenosť medzi bodmi.
- **Vodorovne** alebo **Zvislo**: vzdialenosť naprieč alebo nahor po výkrese.

*Meria* vo *Vlastnostiach* umiestnenej kóty ju neskôr zmení. Ťahaním vybranej kóty posuniete jej čiaru. **Presunúť** berie aj kóty: voľné konce idú s tým, čo sa presúva, a čiara medzi dvoma vecami sa posúva kolmo na seba.

### Reťazce a rady {#chains-and-rows}

- **Reťazovo** (voľby nástroja): po prvej kóte pridá každé kliknutie ďalšiu na tej istej čiare, až po `Esc` alebo pravé tlačidlo.
- **Rovnomerný rozostup** a **Rozostup**: kótovacie čiary sa umiestňujú v násobkoch rozostupu (v milimetroch na papieri) od toho, čo merajú, takže rady zostávajú rovnomerné. Zadaním rozostupu sa zapne. V oboch prípadoch sa čiara novej kóty prichytí k čiare blízkej rovnobežnej kóty.
- Nástroj Kóta sa prichytáva od svojho prvého bodu ako kresliace nástroje: druhý bod sa zachytí na vodorovnej a zvislej prvého.

Keď nie je nič vybrané, `D` zvolí nástroj Kóta; pri výbere `D` zobrazí alebo skryje jeho živé kóty.

## Uhly {#angles}

Zvoľte nástroj **Uhol**. Kliknite na dve čiary (povrchy stien, hrany) a potom, kam má ísť oblúk. Alebo kliknite na vrchol a na jeden bod na každom ramene. Strana, na ktorú oblúk umiestnite, určí vnútorný alebo vonkajší uhol.

## Polomer a priemer {#radius-and-diameter}

Zvoľte **Polomer** alebo **Priemer**. Kliknite na oblúk alebo kružnicu (otváranie dverí, oblúk náčrtu) alebo na tri body na nej a potom, kam má ísť kóta.

## Dĺžka oblúka {#arc-length}

Zvoľte **Dĺžka oblúka**. Kliknite na oblúk alebo na jeho začiatok, bod na ňom a koniec, potom kam má ísť kóta.

![Polomer a dĺžka oblúka na otváraní dverí](img/dimensions.jpg)

## Vzhľad {#how-they-look}

Vo voľbách nástroja vyberte **Štýl** pre nové kóty; štýl alebo *Vlastnú farbu* umiestnenej kóty zmeníte vo *Vlastnostiach*. Pozri [Štýly popisov](styles).

Ako sa čísla zobrazujú (jednotka, desatinné miesta, nuly), určuje **formát čísel** pre kóty v balíku štýlov; jedna kóta môže mať vlastný vo *Vlastnostiach*. Pozri [Jednotky a formáty čísel](units#number-formats).

## Prerušenie pri značkách {#breaking-around-tags}

Štýl so zapnutým **Prerušiť kótovacie čiary pri značkách** necháva v kótovacích čiarach medzeru všade, kde cez ne leží krúžok značky alebo text poznámky, takže reťaz kót zostane čitateľná aj cez značky dverí na nej. Kóty v iných štýloch idú rovno.

Ak chcete zapísať výšku bodu, nie vzdialenosť, použite [výškovú kótu](spot-levels).

Kóty fungujú v pôdorysoch, rezoch a pohľadoch, zobrazujú sa na listoch a tlačia sa do PDF a SVG.
