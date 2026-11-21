# Pozemok: terén, spevnené plochy a zeleň

Nástroje **Pozemok** (*Vytvoriť › Pozemok*) modelujú okolie budovy: terén, spevnené a štrkové plochy, stromy a kry. Autá a ľudia sú v katalógu.

## Terén {#terrain}

1. Zvoľte nástroj **Terén**.
2. Vo voľbách nástroja nastavte jeho *Hĺbku*.
3. Nakreslite obrys terénu ako dosku: `1` Mnohouholník, `2` Obdĺžnik, `3` Vyplniť miestnosť alebo `4` Z náčrtu.

Nový terén je rovný na svojom podlaží a pokrytý materiálom *Tráva* z knižnice materiálov. Na jeho tvarovanie pridajte vo *Vlastnostiach* **výškové body**. Kliknite na *Pridať výškovú kótu*, zadajte polohu X a Y a výšku nad podlažím. Povrch prechádza každým výškovým bodom a medzi nimi sa zvažuje. Rohy obrysu zostávajú na *Vrchu* terénu, pokiaľ na ne nedáte výškový bod, takže jeden bod v strede vytvorí kopec a body v rohoch naklonia celý pozemok. Nastavením *Vrstevnice každých* sa v pôdorysoch kreslia vrstevnice, napríklad každých 250 mm. Výškové body sa v pôdorysoch zobrazujú ako malé krížiky.

### Terén z bodov zamerania {#terrain-from-surveyed-points}

**Terén z bodov** na karte *Kataster* vytvorí terén zo súboru geodeta, ako to robí PilonDesigner so súborom `teren.xyz`: XYZ, TXT, PTS, CSV alebo ASC, jeden bod na riadok ako `x y z` (medzery, tabulátory, bodkočiarky alebo čiarky; úvodné číslo bodu a riadky hlavičky sa preskočia).

- Obrys vedie okolo krajných bodov a povrch prechádza každým bodom. Rozsiahle zameranie sa preriedi na približne 1500 bodov v mriežke, z každej bunky sa ponechá bod najbližší jej stredu.
- Body v **S-JTSK** (Y a X nad 50 km) sa otočia na východ a sever a ich ľavý dolný celý meter sa presunie do počiatku projektu.
- Nadmorské výšky (Bpv) sa vzťahujú k ±0,000 projektu: k jeho *Nadmorskej výške* v *Údajoch o stavbe*, ak je zadaná, inak k celému metru najnižšieho bodu. Stavový riadok uvedie, kde je počiatok a ±0,000.
- Rovnako vytvorí terén aj 3D zameranie v DXF (body, 3D krivky, TIN z 3D plôch): **Terén z výšok výkresu** vo *Vlastnostiach* referencie, zo zobrazených hladín, tam, kde je referencia umiestnená.
- Vrstevnice sa kreslia každých 0,25, 0,5 alebo 1 m podľa toho, ako veľmi terén stúpa. Vo *Vlastnostiach* sa vypíše len prvých 40 výškových bodov.

### Mapové podklady {#map-underlays}

**Mapové podklady** na karte *Kataster* otvoria v okne aplikácie stránku Pilon Map Downloader (mapy.pilondesign.eu), ako PilonDesigner. Nájdite miesto, vyberte územie a čo chcete prevziať a odošlite to tlačidlom mapy **Odoslať do CreaBIM** (aplikácia mape odpovedá ako Pilon Map Link v Reville). Situácia, ortofoto a terén sa vložia do otvoreného pôdorysu a projekt sa umiestni v S-JTSK, ako pri podkladoch z mapy nižšie. Súbory sa uložia do priečinka vedľa súboru projektu.

Aplikácia čaká na mapu len na tomto počítači a odpovedá len stránke samotnej mapy.

### Podklady z mapy (ZIP) {#a-map-package}

**Podklady z mapy (ZIP)** na karte *Kataster* vložia ZIP z Pilon Map Downloader, ako PilonDesigner: pri otvorenom pôdoryse vyberte ZIP a jeho súbory sa rozbalia do priečinka s rovnakým názvom vedľa neho.

- `situacia.dxf` (kataster, budovy, siete a vrstevnice) sa stane referenciou v pôdoryse, v metroch, tam, kam ju umiestni mapa.
- `ortofoto.jpg` so súborom georeferencie `ortofoto.jgw` sa stane obrázkovou referenciou pod modelom, umiestnenou a zväčšenou podľa súboru georeferencie. Výplň terénu ju v pôdoryse zakrýva: aby ste ju videli, zrušte v kategóriách pohľadu *Terén*.
- `teren.xyz` sa stane terénom ako pri **Teréne z bodov**, s výškami vzťahovanými k ±0,000 projektu.
- `bod_pociatku.txt` udáva polohu počiatku projektu v S-JTSK (v *Údajoch o stavbe* ako **Počiatok v S-JTSK**) a jeho ±0,000 v nadmorskej výške (Bpv), ktorá sa stane *Nadmorskou výškou* projektu, ak ju ešte nemá.

Ak balík obsahuje miestne aj S-JTSK súbory, použijú sa miestne; súbory v S-JTSK sa presunú k počiatku z `bod_pociatku.txt`. Opätovné vloženie balíka pridá jeho referencie a terén znova: staré odstráňte.

### Zemné práce {#earthworks}

Nástroj **Terén** kreslí aj zemné práce, ako úpravy terénu v PilonDesigneri: v jeho lište zvoľte **Stavebná jama** alebo **Plošina**, nastavte dno (alebo výšku) nad podlažím a sklony svahov 1:n a nakreslite obrys ako dosku.

- **Stavebná jama** sa vykope po dno so svahmi stúpajúcimi k terénu a po dokončení stavby sa zasype: terén zostane pôvodný. Pôdorysy kreslia jej dno, hornú hranu svahov čiarkovane a šrafy svahov.
- **Plošina** zarovná terén na svoju výšku so svahmi dolu alebo hore k okolitému terénu. Terén mení natrvalo: 3D pohľad aj vrstevnice ju sledujú.
- Vo *Vlastnostiach*: druh, dno alebo výška, sklony svahov a hrúbka najprv zhrnutej ornice.

**Zemné práce** na karte *Kataster* otvoria výkaz zemných prác: ornica, výkop plošín, jama, ryhy pod pásovými základmi a jamy pod pätkami, premiestnenie výkopku, spätný zásyp (jama mínus podzemná časť stavby), násyp a prebytok na skládku, každá položka s orientačným kódom KROS/Cenkros. Objemy sú rozdielom terénu pred a po úprave na jeho trojuholníkovej sieti, v bunkách 0,5 m. *Kopírovať do tabuľky* ich skopíruje s bodkočiarkami pre Excel alebo Cenkros.

## Spevnené plochy {#hard-landscape}

**Spevnené plochy** sú vrstvy na teréne podobné doske, každá s vlastným materiálom a vzorom povrchu.

1. Zvoľte nástroj **Spevnené plochy**.
2. Vo voľbách nástroja vyberte, čo kresliť:
   - **Dlažba**: betónové dlaždice 600 × 400.
   - **Príjazdová cesta**: zámková dlažba kladená na väzbu.
   - **Štrk**: bodkovanie.
   - **Terasové dosky**: dosky.
   - **Asfalt**: bez vzoru.
3. Nakreslite obrys plochy ako dosku.

Voľby sú len východiskom. Vo *Vlastnostiach* zmeňte *Vrch*, *Hĺbku* a **Vzor** plochy:

- **Prvky**: ich dĺžka, šírka, posunuté rady a uhol.
- **Čiary**: ich rozostup a uhol.
- **Bodkovanie**: jeho rozostup.

Pôdorysy kreslia vzor cez plochu a 3D zobrazuje plochu v jej materiáli. Plocha nakreslená vnútri rovného terénu leží v jeho úrovni: terén jej uvoľní miesto.

## Zeleň {#planting}

1. Zvoľte nástroj **Zeleň**.
2. Vo voľbách nástroja vyberte **Strom**, **Ihličnan**, **Palmu** alebo **Ker** a jeho korunu (priemer koruny) a *Výšku*.
3. Kliknutím vysaďte. Ďalšími kliknutiami vysádzate ďalšie.

V pôdoryse je každá rastlina značkou širokou ako jej koruna: zúbkovaná koruna s kmeňom pre strom, hviezda s kruhmi pre ihličnan, listy pre palmu, menší zúbkovaný obrys pre ker. V 3D, rezoch a pohľadoch je to jednoduchý tvar: kmeň pod guľatou korunou, kužeľ, vysoký kmeň s trsom listov alebo nízky kopček. Priraďte jej materiál a zmeníte farbu koruny.

## Autá a ľudia {#cars-and-people}

*Vytvoriť › Pozemok › Autá a ľudia…* otvorí [Katalóg](furniture) v časti **Okolie**: auto, dospelý a dieťa pre mierku. Umiestnite ich ako ktorýkoľvek iný objekt.

## Úpravy tvaru {#reshaping}

Terén a spevnené plochy majú rovnaké úchopy ako doska: potiahnite roh alebo hranu, alebo ich otvorte v [režime úprav](edit-mode) (`Enter`) a pridajte či odstráňte rohy alebo zmeňte otvory plochy. Rastliny presúvajte nástrojom [Presunúť](move).

## Výmena a viditeľnosť {#exchange-and-visibility}

Terén sa exportuje do IFC ako *IfcGeographicElement* (TERRAIN), rastliny ako *IfcGeographicElement* a spevnené plochy ako *IfcCivilElement*. Formát IFCX ich načíta späť ako rovnaké upraviteľné prvky. Pohľady ich môžu skryť kategóriami *Terén a spevnené plochy* a *Zeleň* (pozri [Viditeľnosť](visibility)).
