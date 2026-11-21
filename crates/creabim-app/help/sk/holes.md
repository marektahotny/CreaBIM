# Otvory, otvory v stenách a šachty

Tri spôsoby, ako vyrezať otvor v budove, každý sa exportuje do IFC ako otvor.

## Prestupy v stenách {#wall-openings}

**Prestup** je otvor v stene bez dverí či okna, pre potrubie, vzduchotechniku alebo prechod. Nastavuje sa ako v PilonDesigneri.

1. Zvoľte nástroj **Prestup** (*Architektúra › Prestup*).
2. Kliknite na stenu.
3. Vo *Vlastnostiach* nastavte tvar (obdĺžnik alebo kruh, pri kruhu je šírka priemer), šírku, výšku a spodnú hranu nad podlahou.

V pôdoryse sa prestup kreslí obrysom cez hrúbku steny, krížikom a popisom za lícom steny, napr. „P 300×200 sp. 900“ alebo „P Ø150 sp. 1 050“. Prestup, ktorý pretína rovina rezu, má plné tenké čiary; prestup nad rovinou rezu tenkú čiaru s čiarkou a dvoma bodkami, pod rovinou rezu tenkú čiarkovanú. Vo *Vlastnostiach* zvolíte *V pôdoryse: Otvor* (len obrys), označenie (predvolene P) a či popis ukáže označenie a rozmer. Otvor až po podlahu vysoký aspoň 1,8 m sa kreslí ako prechod, len medzerou v stene.

## Otvory v doskách a strechách {#holes-in-slabs-and-roofs}

1. Vyberte dosku alebo strechu.
2. Kliknite na **Kresliť otvor** pod *Otvory* vo *Vlastnostiach*.
3. Nakreslite obrys otvoru, čiarami alebo oblúkmi, s bežným prichytávaním.

Otvor sa vyreže v pôdoryse aj v 3D a plocha sa aktualizuje. *Vlastnosti* vypisujú otvory dosky, každý s *Odstrániť*. Tvar otvoru zmeníte otvorením dosky v [Režime úprav](edit-mode) (`Enter`): každý otvor ukáže svoje rohy, hrany a kosoštvorce na pridanie rohov. Ťahaním vnútri otvoru ho posuniete; kliknutím doň a `Delete`, alebo pravým tlačidlom, ho odstránite. Otvor nemožno potiahnuť mimo dosky, cez jej hranu ani cez iný otvor.

![Strop prízemia s otvorom a šachtou](img/slab-holes.jpg)

## Šachty {#shafts}

**Šachta** je zvislý otvor cez všetky dosky a strechy medzi dvoma podlažiami: schodiskový priestor, výťah, stúpačka.

1. Zvoľte nástroj **Šachta**.
2. Nakreslite jej obrys ako pri doske: `1` Mnohouholník, `2` Obdĺžnik, `3` Vyplniť miestnosť alebo `4` Z náčrtu.

Predvolene vedie od tohto podlažia po najbližšie vyššie. Spodné a horné podlažie a odsadenia nastavíte vo *Vlastnostiach*. Posuňte ju a každý otvor, ktorý reže, sa posunie s ňou.

Šachtu vyberiete v každom pohľade, ktorým prechádza: v pôdoryse každého podlažia od jej spodku po vrch (kliknutím do jej obrysu, preškrtnutého, ak sa cez ňu nevidí schodisko), v rezoch a pohľadoch (kde je kreslená čiarkovane) a v 3D. Vnútri stien či dosiek ju dosiahnete cez `Tab` a [röntgen](visibility#x-ray).

Vybraná má úchyty na rohoch, hranách, spodku a vrchu. V [Režime úprav](edit-mode) môžete pridávať alebo odoberať rohy. Šachta reže iba dosky a strechy, ktoré ju obsahujú celú: potiahnutá cez hranu dosky tam nereže nič a stavový riadok to oznámi.
