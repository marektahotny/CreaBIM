# Potrubia a vzduchovody

Potrubia a vzduchovody technického zariadenia budov: voda, kanalizácia, plyn, vykurovanie, chladenie, medicinálne plyny, dymovody a vetranie. Každé je priamy úsek v **systéme**, z **typu potrubia** (rad ako meď STN EN 1057, HERZ PipeFix alebo PP-HT) a **dimenzie** v ňom.

1. Zvoľte nástroj **Potrubie** alebo tlačidlo potrubia na karte *TZB*, *Vykurovanie*, *VZT* alebo *Medicinálne plyny*, ktoré vyberie aj systém.
2. Vo voľbách nástroja zvoľte *Systém*, *Typ potrubia* a *Rozmer* a *Výšku* osi potrubia nad podlahou.
3. Kliknite na začiatok potrubia a potom na každý ďalší bod. Esc alebo pravé tlačidlo ťah ukončí.

## Systémy {#systems}

Každý systém má svoje označenie, ako ho uvádzajú výkresy (SV studená voda, TV teplá voda, C cirkulácia, K splašková kanalizácia, D dažďová kanalizácia, PV požiarna voda, ZP zemný plyn, UKP a UKS vykurovanie, CHP a CHS chladenie, O2, AIR, VAC, N2O a CO2 medicinálne plyny, DYM dymovod, VZP a VZO vetranie), svoju farbu a typ potrubia, dimenziu a výšku, ktoré dostane nové potrubie. Kanalizácia sa pri kreslení zvažuje sklonom svojho systému (2 % splašková, 1 % dažďová) a ťah pokračuje tam, kde skončilo jeho posledné potrubie.

## Zariaďovacie predmety a ich prípojky {#fixtures-and-their-connectors}

Zariaďovacie predmety, spotrebiče, vykurovacie telesá, rozdeľovače a fan-coily z katalógu majú **prípojky**, rozmiestnené ako v PilonDesigneri: umývadlo prípojku teplej a studenej vody a odpadu, WC studenej vody a kanalizácie, vykurovacie teleso prívodu a spiatočky vykurovania, fan-coil chladiacej vody a kondenzátu. Pôdorys zobrazuje každú ako malý krúžok vo farbe systému a vlastnosti objektu ich vypisujú s dimenziou a výškou.

- Začnite ťah na prípojke a nástroj prevezme jej systém a začne v jej výške.
- Ukončite ťah na prípojke rovnakého systému a potrubie k nej klesne alebo stúpne a ťah sa skončí.

Umývadlo, WC, vaňa, sprcha, drez, podlahová vpust, vonkajší výtok a vykurovacie teleso z katalógu si ponechajú vlastné 3D modely a získajú prípojky. Práčka, umývačka riadu, bidet, pisoár, kúpeľňový radiátor, rozdeľovač podlahového vykurovania a nástenné, parapetné a stropné kazetové fan-coily sú z PilonDesignera, s rozmermi a kresbou ako v ňom.

## Výpočet vodovodu {#water-supply-calculation}

**Výpočet vodovodu** na karte *TZB* nadimenzuje nakreslené potrubia studenej (SV) a teplej (TV) vody metódou PilonDesignera (výpočtové jednotky podľa STN EN 806-3, alebo STN 73 6655 / DIN 1988-300 podľa jeho nastavení).

- Sieť tvoria potrubia systému spojené tam, kde sa stretávajú ich konce alebo kde jedno končí na druhom (odbočka), až po zariaďovacie predmety, ktorých prípojky dosahujú. Začína sa na najnižšom voľnom konci potrubia bez predmetu, kde vstupuje prípojka.
- Okno zobrazuje výpočtový prietok každého systému, potrebný tlak oproti tlaku v mieste napojenia a pre každé potrubie prietok, rýchlosť, tlakovú stratu na meter a navrhnutú dimenziu. Kritická vetva je tučne. Kliknutím na potrubie ho vyberiete.
- **Použiť navrhnuté dimenzie** zmení potrubia na navrhnuté dimenzie v ich vlastnom rade a prepočíta.
- Konce potrubí nenapojené na nič, potrubia nedosiahnuteľné zo začiatku a slučky sa vypíšu, aby sa dali opraviť.

## Typy potrubí {#pipe-types}

Typy sú z katalógu PilonDesignera: systém ponúka typy pre médium, ktoré vedie. Každý má dimenzie s vonkajším a vnútorným priemerom, drsnosť pre výpočty, spoje a normy a pri radoch výrobcov (HERZ, SANHA, GAWAPLAST) aj tvarovky.

## Vlastnosti {#properties}

Vyberte potrubie a zmeňte jeho systém, typ potrubia, dimenziu, izoláciu a výšky koncov. Iný typ zachová vonkajší priemer tak blízko, ako to jeho rad dovolí, ako keď ťah mení materiál. Kanalizácia zobrazuje svoj sklon.

Potrubia sa v pôdoryse kreslia vo farbe svojho systému, prerezané, viditeľné pod rezom alebo čiarkované nad rezom pôdorysu, a v 3D ako valce (štvorhranný vzduchovod ako kváder). Do IFC sa exportujú ako IfcPipeSegment alebo IfcDuctSegment.

## Podlahové vykurovanie {#underfloor-heating}

Okno *Navrhnúť podlahové vykurovanie* navrhne miestnostiam okruhy z ich tepelnej straty a zo skladby podlahy pod nimi: systémová doska (jej rúry a raster), poter nad ňou (λE, krytie su), RλB vrstiev na potere a izolácia pod rúrami pre tepelný tok smerom dole (STN EN 1264-2, -3). Každá miestnosť sa kladie ako špirála, meander alebo dvojitý meander, prípadne sa nevykreslí.

1. Na karte *Vykurovanie* zvoľte *Okrajová zóna*, *Miesto zníženého výkonu* alebo *Diera bez potrubia*.
2. Nakreslite ju vnútri miestnosti s podlahovým kúrením mnohouholníkom (1) alebo obdĺžnikom (2).
3. V okne nastavte rozostup okrajovej zóny alebo o koľko % miesto dáva menej, zóny vypnite či zapnite a okruhy miestnosti rozdeľte zvisle alebo vodorovne.
