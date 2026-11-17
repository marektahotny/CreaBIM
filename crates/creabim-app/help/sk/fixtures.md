# Prvky elektroinštalácie, ZTI a VZT

Prvky technického zariadenia budov sú [objekty](furniture) ako nábytok: vyberiete ich v okne Katalóg a umiestnite nástrojom Objekt. Každý má 3D tvar, značku, ktorú preň používajú výkresy TZB, obvyklú výšku osadenia a triedu IFC. Potrubia, vzduchotechnické potrubia a káble medzi nimi sa zatiaľ nemodelujú.

Celú sadu uvidíte v [základnom dome](starter-house) alebo cez *Súbor › Nový zariadený vzorový dom*.

## Čo je k dispozícii {#what-there-is}

| Kategória | Prvky | IFC |
|---|---|---|
| Elektroinštalácia | Dvojzásuvka, jednoduchá zásuvka, vypínač (jednopólový a sériový), rozvádzač, hlásič dymu | IfcOutlet, IfcSwitchingDevice, IfcElectricDistributionBoard, IfcSensor |
| Osvetlenie | Stropné svietidlo (závesné), zapustené bodové svietidlo, nástenné svietidlo | IfcLightFixture |
| Zdravotechnika | Podlahová vpust, kombinovaný kotol, vonkajší výtok | IfcWasteTerminal, IfcBoiler, IfcValve |
| Vykurovanie a vetranie | Vykurovacie teleso, stropná prívodná vyústka, stropná odvodná mriežka, split klimatizácia, odsávací ventilátor | IfcSpaceHeater, IfcAirTerminal, IfcUnitaryEquipment, IfcFan |

Umývadlá, WC, vane a sprchy sú v kategórii *Kúpeľňa* a kuchynský drez v kategórii *Kuchyňa*.

## Umiestňovanie {#placing-them}

- **Nástenné prvky** sa priložia k najbližšiemu povrchu steny do 600 mm, v obvyklej výške nad podlahou (pozri tabuľku nižšie). Vonkajší výtok umiestnite na vonkajší povrch steny kliknutím tesne za ňou.
- **Stropné prvky** (svietidlá, bodové svietidlá, vyústky, mriežky a hlásiče dymu) sa umiestnia so stredom v mieste kliknutia a visia zo stropu nad týmto bodom. Strop je spodok stropnej dosky nad nimi, a ak tam žiadna nie je, vrch podlažia (jeho výška plus konštrukčná výška).
- **Podlahová vpust** leží na podlahe v mieste kliknutia.

Nástenné prvky sa osadzujú do týchto výšok, merané po ich spodok:

| Prvok | Výška |
|---|---|
| Zásuvky | 300 mm |
| Vypínače | 1100 mm |
| Rozvádzač | 1350 mm |
| Nástenné svietidlo | 1800 mm |
| Vykurovacie teleso | 150 mm |
| Kotol | 1000 mm |
| Vonkajší výtok | 600 mm |
| Split klimatizácia | 2000 mm |
| Odsávací ventilátor | 2100 mm |

## Výšky {#heights}

Vyberte prvok. Vo *Vlastnostiach* **Nad svojím podlažím** nastavuje výšku nástenného prvku. Pri stropnom prvku je pole **Pod stropom**: 0 je v rovine stropu a 300 mm spustí závesné svietidlo o 300 mm. Presunutie stropného prvku hore či dolu alebo vytlačenie jeho hornej či spodnej plochy mení tú istú hodnotu. Keď sa posunie doska, stropné prvky pod ňou ju nasledujú.

Nastavenie typu **Osadenie** má tri možnosti: *Pri stene*, *Voľne stojaci* a *Zo stropu*. **Nové nad podlahou** (alebo *pod stropom*) je výška, do ktorej nástroj Objekt osadzuje nové prvky. Zmeňte ju napríklad na type zásuvky a každá nová zásuvka bude na 450 mm.

## Pôdorysy {#plans}

Pôdorysy kreslia štandardné značky vo veľkosti čitateľnej v mierke 1:50 aj 1:100:

- **zásuvka** je polkruh od steny s jednou čiarkou na každú zásuvku;
- **vypínač** je malý krúžok s čiarou zakončenou jednou čiarkou na každý pól;
- **svietidlo** je krúžok s krížom. Bodové svietidlo má vnútri druhý krúžok a nástenné svietidlo má držiak na stene;
- **rozvádzač** je obdĺžnik s jednou polovicou vyšrafovanou;
- **hlásič dymu** je krúžok s písmenom S;
- **vykurovacie teleso** je vyšrafovaný pás;
- **prívodná vyústka** je štvorec s uhlopriečkami a **odvodná mriežka** štvorec s priečkami;
- **ventilátor** je krúžok s tromi lopatkami, **výtok** značka ventilu a **vpust** mriežka.

Značka prvku sa kreslí celá aj vtedy, keď je nad výškou rezu. Stropné svietidlá a vyústky sa zobrazujú v pôdoryse pod nimi, ako ich zobrazujú výkresy TZB.

## Značky a výkazy {#tags-and-schedules}

- **Označiť predmety** v hlavičke pôdorysu označí každý prvok elektroinštalácie, osvetlenia, zdravotechniky a vykurovania či vetrania **označením** svojho typu: E01, E02… pre elektro, L pre osvetlenie, P pre zdravotechniku, M pre vykurovanie a vetranie, S pre zariaďovacie predmety, K pre kuchyňu a F pre nábytok. Nábytok, kuchynské skrinky a zariaďovacie predmety sa vynechajú, aby pôdorys zostal čitateľný; tie označte jednotlivo nástrojom **Značka**. Pozri [Značky](tags).
- Výkaz **Zariaďovacie predmety** spočíta každý prvok podľa typu a podlažia, pod nadpisom pre každú kategóriu, s označením, výškou osadenia a triedou IFC. Výkaz **Nábytok** urobí to isté pre nábytok. Pozri [Výkazy](schedules).

## IFC

Každý typ prvku sa exportuje ako svoj typ IFC (IfcOutletType POWEROUTLET, IfcSwitchingDeviceType TOGGLESWITCH, IfcLightFixtureType POINTSOURCE, IfcSpaceHeaterType RADIATOR, IfcAirTerminalType DIFFUSER atď.) s tvarom a pôdorysnou značkou ako mapami reprezentácie. Každý umiestnený prvok je výskyt vo svojej skutočnej výške. Spôsob osadenia typu (*Stena*, *Podlaha* alebo *Strop*) a predvolená výška sa uložia do súboru vlastností CreaBIM typu, takže po otvorení súboru stropné prvky opäť visia zo stropu. Pozri [IFC](ifc).
