# Režim úprav

Režim úprav otvorí jeden prvok tak, že sa zobrazia všetky jeho úchyty, v každom pohľade, v ktorom je: v pôdorysoch každého podlažia, ktorým prechádza, v rezoch, pohľadoch aj v 3D. Slúži na zmenu tvaru nakresleného, nie iba na posun.

## Otvorenie a zatvorenie {#opening-and-closing-it}

- Vyberte prvok a prepnite **tlačidlo režimu** oblasti z *Režimu objektov* na *Režim úprav*, alebo stlačte `Tab` nad pohľadom (ak je pod ukazovateľom viac vecí, `Tab` nimi namiesto toho prechádza);
- alebo dvakrát kliknite na vybraný prvok (alebo na niektorý jeho roh) v ľubovoľnom pohľade;
- alebo s vybraným prvkom stlačte `Enter`.

*Vlastnosti* vrstveného prvku majú aj tlačidlo **Upraviť jeho vrstvy v pohľadoch**.

Ako v Blenderi ho zatvorí iba `Tab`, tlačidlo režimu alebo *Ukončiť úpravy* v ponuke pravého tlačidla. `Esc` zruší vybrané a zostane v Režime úprav, ani kliknutie inam ho neopustí. *Vlastnosti* počas neho ukazujú *Úpravy* a stavový riadok vypisuje, čo môžete robiť.

## Body, hrany a plochy {#points-edges-and-faces}

V Režime úprav obsahuje lišta nástroja **Body**, **Hrany** a **Plochy**, pre všetky pohľady naraz. Určujú, čo kliknutie vyberie, ako v Blenderi, a ostatné druhy úchytov ustúpia. Rovnako fungujú číselné klávesy:

| Kláves | Vyberá |
| --- | --- |
| `1` | **Body**: rohy, konce, vrcholy |
| `2` | **Hrany**: strany, hrany, ostenia |
| `3` | **Plochy**: plochy, vrchy, spodky |

Opätovným stlačením toho istého klávesu vyberáte všetko. Pri *Hranách* sa zobrazí každá hrana na uchopenie; pri *Plochách* označí každú plochu bodka.

## Úprava hmoty {#editing-a-mass}

Hmota v pokoji ukazuje iba hlavné úchyty (základný bod, úchyt otáčania a vysunutie či vtiahnutie kliknutej plochy); šípky všetkých plôch, rohy, hrany a šípka zdvihu prídu v Režime úprav. Kreslí sa ako v Blenderi, tenkými tmavými drôtmi s malými bodkami v rohoch (ich veľkosti sú v *Nastavenia › Pohľad*), a hrany a plochy sa pod ukazovateľom rozsvietia.

- Kliknutie vyberie bod, hranu (kdekoľvek pozdĺž nej) alebo plochu a zostane rozsvietená. `Shift` vyberá ďalšie, obdĺžnik vyberie body v ňom a kliknutie do prázdna výber zruší.
- `G` alebo `M` posúva vybrané, `R` otáča a `S` mení mierku okolo stredu. Hodnotu zadáte alebo os držíte ako pri [Posune](move).
- Sekcia *Úprava siete* vo *Vlastnostiach* vybranú plochu vytiahne, odsadí dnu alebo zúži (pozri [Hmoty](masses#mesh-edit)).

![Režim úprav na hmote s vybranou plochou](img/edit-mode-mass.jpg)

## Vrstvy: konce každej vrstvy {#layers-each-layer-s-ends}

Na vrstvenej stene stlačte v Režime úprav `Shift+Tab` pre **Vrstvy**: vlastné úchyty steny ustúpia jednému úchytu na každom konci každej vrstvy, tam, kde ten koniec určilo napojenie. Zobrazujú sa v pôdorysoch, rezoch, výrezoch aj v 3D (tam stupňovito po čelnej ploche, aby sa dal dosiahnuť každý). Ďalšie `Shift+Tab` sa vráti.

| Urobte toto | A |
|---|---|
| Ťahajte úchyt vrstvy pozdĺž steny | Vrstva pokračuje za napojenie alebo sa skráti; `Tab` umožní zadať o koľko |
| Podržte ukazovateľ na úchyte | Pomenuje svoju vrstvu a koniec a o koľko bol posunutý |
| Pravým tlačidlom na úchyt | *Obnoviť koniec vrstvy*, *Obnoviť všetky vrstvy*; na stene *Kresliť podľa vonkajšej plochy* alebo *vnútornej plochy*: táto čiara medzi vrstvami sa stane referenčnou čiarou steny a stena zostane na mieste |

Každá vrstva steny má aj **spodný** a **horný** úchyt v polovici dĺžky: ťahaním vrstvu predĺžite pod spodok steny alebo nad jej vrch (vonkajší plášť a izoláciu pod hranu dosky), alebo ju skrátite (povrchová úprava končiaca na potere). Pravým tlačidlom na ňom zvolíte *Obnoviť spodok vrstvy* alebo *Obnoviť vrch vrstvy*.

Na vrstvenej doske alebo streche dajú Vrstvy namiesto toho úchyt na každú hranu každej vrstvy: ťahaním kolmo na hranu vrstvu vysuniete za hranu prvku alebo ju vtiahnete, napríklad bridlicu presahujúcu za čelnú dosku. V pôdoryse sa úchyt každej vrstvy kreslí o krok ďalej, keďže by ležali na sebe.

*Vytvoriť › Kontrola napojení vrstiev* vypíše steny, kde ručne posunutá vrstva necháva v napojení medzeru alebo zasahuje do steny, na ktorú naráža. Úchyt posunutého konca alebo hrany je vyplnený. Úprava sa uchováva ako odsadenie od napojenia, takže zostane, aj keď sa stena posunie alebo natiahne. Zmena typu steny zachová úpravy vrstiev, ktoré má aj nový typ, a stavový riadok povie, koľko sa zahodilo.

## Obrysy: jeden spôsob pre všetky {#outlines-one-way-for-all-of-them}

Obrysy dosiek a ich otvory, obrysy striech a ich otvory, šachty, náčrty a voľné trasy zábradlí fungujú rovnako. Ich obrysy sú vykreslené oranžovo, s:

- **štvorcovým úchytom** na každom rohu;
- **kosoštvorcom** (◇) v strede každej hrany;
- **okrúhlym úchytom** tesne mimo každej hrany uzavretého obrysu.

| Urobte toto | A |
|---|---|
| Ťahajte roh | Posunie sa s prichytením; `Tab` umožní zadať o koľko |
| Ťahajte okrúhly úchyt hrany | Hrana sa posunie kolmo na seba |
| Ťahajte ◇ | Z hrany vyjde nový roh |
| Kliknite na roh | Vyberie sa (vyplnený oranžovo); `Shift+klik` vyberá ďalšie |
| Ťahajte obdĺžnik mimo úchytov | Vyberie všetky rohy v ňom (`Shift` pridáva) |
| `Delete` alebo `Backspace` | Odstráni vybrané rohy jedným krokom späť |
| Dvojklik na roh | Odstráni ho |
| Kliknite dovnútra otvoru | Vyberie otvor: `Delete` ho odstráni |
| Ťahajte vnútri otvoru | Posunie celý otvor |
| Pravým tlačidlom na roh, hranu alebo otvor | Jeho ponuka (nižšie) |
| `Esc` | Zruší ťahanie; potom zruší výber (`Tab` opustí Režim úprav) |

`Delete` v Režime úprav nikdy neodstráni upravovaný prvok: bez vybraného rohu iba vyzve vybrať nejaký. Uzavretý obrys si ponechá aspoň tri rohy, otvorený (trasa zábradlia, otvorený náčrt) dva.

Vybraný, ale nie v Režime úprav, ukazuje obrys iba vlastné rohy a hrany na ťahanie. Jeho otvory, kosoštvorce a výber a odstraňovanie rohov potrebujú Režim úprav, pri všetkých prvkoch rovnako.

Náčrt s oblúkmi alebo väzbami si rohy drží: môžete ich ťahať, ale nie pridávať či odoberať, a tip pri rohu to povie.

## Ponuky pravého tlačidla {#the-right-click-menus}

- **Na rohu:** *Odstrániť roh* (alebo *Odstrániť 3 rohy*, keď je vybraných viac), *Vybrať všetky rohy*.
- **Na hrane:** *Pridať roh sem* (kam ste klikli), *Vytvoriť oblúk* alebo *Vyrovnať* (obrysy dosiek) a na hrane otvoru *Odstrániť otvor*.
- **Vnútri otvoru:** *Odstrániť otvor*, *Presunúť otvor*.
- **Kdekoľvek inde:** *Vybrať všetky rohy*, *Zrušiť výber rohov*, *Ukončiť úpravy*.

## Tvary sa kontrolujú počas ťahania {#shapes-are-checked-as-you-drag}

Počas ťahania sa tvar kontroluje. Ak by bol chybný, kreslí sa **červeno** s dôvodom pri ukazovateli a v stavovom riadku a nikdy sa neuloží: pustite a zostane posledný platný tvar. Odmieta sa:

- obrys alebo otvor, ktorý by pretínal sám seba;
- otvor mimo svojej dosky či strechy, alebo cez jej hranu;
- dva prekrývajúce sa otvory.

Roh pustený na susedný sa s ním zlúči namiesto toho, aby zostala hrana nulovej dĺžky.

## Podržanie ukazovateľa, dĺžky a zadávanie {#hover-lengths-and-typing}

Podržte ukazovateľ na úchyte: zväčší sa a tip povie, čo robí ťahanie, kliknutie, `Delete` a pravé tlačidlo. Počas ťahania rohu sa pri ukazovateli zobrazujú dĺžky hrán po oboch stranách; hrana alebo otvor ukazuje, o koľko sa posunuli.

Stlačením `Tab` (alebo jednoducho písaním číslic) počas ťahania zadáte dĺžku: úchyt sa posunie o túto vzdialenosť od miesta uchopenia, smerom ťahania. `Enter` ju použije, `Esc` sa vráti k ukazovateľu.

Každé gesto je jeden krok späť s jasným názvom, napríklad *Odstrániť 2 rohy Podlahová doska* alebo *Posunúť otvor 1 v Podlahová doska*.

## Šachty {#shafts}

Rohy a hrany šachty a dva okrúhle úchyty v jej strede (jej **spodok** a **vrch**) sa zobrazia vždy, keď je vybraná; Režim úprav pridá kosoštvorce na pridávanie rohov a výber. Každá doska a strecha, ktorú reže, ju pri ťahaní nasleduje. Šachta reže iba dosku alebo strechu, ktorá ju obsahuje celú: potiahnite ju cez hranu a stavový riadok upozorní, že tú dosku nereže. Pozri [Šachty](holes#shafts).

## Ostatné prvky {#other-elements}

Steny, dvere, okná, otvory v stenách, stĺpy, nosníky, schodiská a ďalšie zobrazujú v Režime úprav svoje bežné [úchyty](handles), v každom pohľade.
