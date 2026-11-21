# Export pre ISOVER Fragment – Projektové hodnotenie

CreaBIM vie zapísať vykurovanú obálku budovy ako projektový súbor webovej
aplikácie ISOVER Fragment „Projektové hodnotenie“ (fragment.isover.sk),
v ktorej sa robí projektové energetické hodnotenie. Fragment hodnotenie
počíta sám; CreaBIM mu dodá to, čo pozná z modelu.

- V aplikácii: **Súbor › Export › Energetické hodnotenie pre ISOVER Fragment (.json)**.
  V stavovom riadku vypíše, čo v modeli chýba (U skladby, Uw okien…).
- Z príkazového riadku: `creabim fragment <model> <out.json>` (upozornenia na stderr).
- Vo Fragmente: Projektové hodnotenie › načítať z lokálneho súboru, potom
  vybrať **Kraj › Okres › Obec** (tým sa doplní teplotná oblasť,
  dennostupne a mesačné teploty) a skontrolovať redukčné faktory **bx**.

Kód: `crates/creabim-geom/src/fragment.rs` (`envelope` = výpočet obálky,
`project_json` = zápis do formátu Fragmentu). Rovnaký JSON môže čítať aj
iná aplikácia; štruktúra je opísaná nižšie.

## Formát súboru

Súbor je výpis formulára Fragmentu. Každá sekcia je objekt, v nej
podsekcia (niekedy s prázdnym menom `""`) a v nej polia podľa `id`
prvku formulára:

| typ poľa      | tvar                                                          |
|---------------|---------------------------------------------------------------|
| `text`        | `{"type":"text","value":"…"}`                                 |
| `number`      | `{"type":"number","value":"292.5"}` – číslo ako **text**      |
| `tel`         | `{"type":"tel","value":292.5}` – číslo ako číslo (tabuľky)    |
| `select-one`  | `{"type":"select-one","text":{},"index":i,"value":"…"}`       |
| `checkbox`    | `{"type":"checkbox","checked":true,"value":"on"}`             |
| `radio`       | `{"type":"radio","value":"on"}`                               |
| `textarea`    | `{"type":"textarea","value":"…"}` – často vnorený JSON ako text |

Riadky tabuliek majú číslo na konci mena poľa: plocha `_N`, otvor
v ploche `_N_M` (od 1).

Sekcie `tepelno_technicke` a `cenove_hodnotenie` sú výsledky výpočtu
Fragmentu; CreaBIM ich nezapisuje. Nezapisuje ani `vonk_okraj_podmienky`
(obec má vo Fragmente vlastný interný kód, napr. `"2587/3/560"`).

### `popis_zakazky / ""`

| pole | z CreaBIM (Údaje o stavbe) |
|---|---|
| `nazov_stavby` | názov stavby, inak názov projektu |
| `adresa` | adresa stavby |
| `investor` | stavebník (client) |
| `meno_projektanta` | zodpovedný projektant, inak hlavný architekt |
| `firma` | kancelária |

### `vnut_okraj_podmienky / ""`

`standart_podmienky` = zapnuté (20 °C, 50 %, neprerušované vykurovanie –
predvolené hodnoty Fragmentu).

### `vyhrievane_plochy`

`podlazia`:

| pole | význam | z CreaBIM |
|---|---|---|
| `druh_budovy` | 0 rodinné domy, 1 bytové domy, 2 administratívne, 3 školy, 4 nemocnice, 5 hotely a reštaurácie, 6 športové haly, 7 obchody | podľa „Využitie budovy“ (text) |
| `pocet_podlazi` | počet vykurovaných podlaží | podlažia s vykurovanou miestnosťou |
| `priemerna_vyska_podlazi` | m | objem / plocha vykurovaných miestností |
| `bud_orient` | natočenie | 0 (orientácia je pri každej ploche) |
| `ucel_budovy` | 0 nová budova, 1 rekonštrukcia | 0 |
| `VZrozmer`, `SJrozmer` | rozmer Z–V a J–S, m | vonkajšie líce obvodových stien, podľa skutočného severu |
| `Z_rozmer` | výška, m | od podlahy najnižšieho vykurovaného podlažia po najvyšší vrch steny |

`teple_plochy` (riadok na podlažie): `meno_i`, `pocet_i`, `dlzka_i`,
`sirka_i` (dĺžka = plocha, šírka = 1), `plocha_i`, `inTemp_i` (20);
súčty `pocet_teplovymenna_plocha`, `celkova_teplovymenna_plocha`.

`teple_objemy`: `omeno_i`, `opocet_i`, `oplocha_i`, `vyska_i`, `objem_i`,
`oinTemp_i`; súčty `pocet_vykurovany_objem`, `celkovy_vykurovany_objem`.

Plocha miestnosti = čistá podlahová plocha z CreaBIM, výška = svetlá
výška miestnosti.

### `Skladba / stavby` – plochy obálky

| pole | význam |
|---|---|
| `druh_plochy_N` | druh plochy (tabuľka nižšie), `index` = kód − 1 |
| `meno_N` | meno prvku v CreaBIM (stena, doska, strecha) |
| `pocet_N` | 1 |
| `dlzka_N`, `sirka_N` | stena: dĺžka a výška; vodorovná plocha: obdĺžnik s rovnakou plochou |
| `orient_N` | `360` = H (vodorovne, index 0), inak `0`…`315` po 45° (index 1–8: S, SV, V, JV, J, JZ, Z, SZ) |
| `plocha_N` | plocha bez otvorov, m² |
| `u_N` | U skladby, W/(m²K) – z vrstiev a λ materiálov (STN EN ISO 6946) |
| `ua_N`, `bxua_N` | U·A, bx·U·A |
| `bx_N` | 1 (nastav vo Fragmente, „redukčné faktory bx“) |
| `obal_N` | patrí do obálky |
| `tvar2D3D_N` | JSON: obdĺžnik `obdlznik_plocha_usek_AxDx_Dx_Cx` (dĺžka) × `…AyBy_By_Cy` (výška) |
| `konstrukcia_N` | JSON: `Uval`, `Rval` (R vrstiev), `hrubka` (mm), `file` (prázdne), `konstrukcia` (meno skladby), `sumarizacia` („materiál (hrúbka[cm])“, hrúbka v mm ako vo Fragmente) |
| `pocet_otvory_N`, `celkova_plocha_otvory_N`, `celkovo_bxua_otvory_N` | súčty otvorov |
| `fix_*`, `z3D_N` | ako vo vzore Fragmentu pre stenu (fix_dlzka 1, ostatné 0, 3D vypnuté) |

Súčty: `pocet_steny`, `celkova_plocha_panely`, `celkovo_bxua`,
`sumar_pocet_otvory`, `celkova_plocha_vsetky_otvory`,
`celkovo_bxua_vsetky_otvory`, `celkovo_plocha_obalky`, `celkovo_bxua_vsetko`.

#### Druh plochy (zoznam Fragmentu = riadky STN 73 0540-2)

| kód | skratka | Fragment | CreaBIM priradí |
|---|---|---|---|
| 1 | VSS | vonkajšia stena a šikmá strecha nad obytným priestorom so sklonom > 45° | obvodová stena pri vykurovanej miestnosti; strecha strmšia ako 45° |
| 2 | PSS | plochá a šikmá strecha ≤ 45° | strecha nad vykurovanou miestnosťou bez stropu nad ňou |
| 3 | SVP | strop nad vonkajším prostredím | podlaha vyššieho podlažia, pod ktorou nie je miestnosť |
| 4 | SNP | strop pod nevykurovaným priestorom | doska vyššieho podlažia, nad ktorou nie je miestnosť (podkrovie) |
| 5 | VSH | vnútorná stena s vodorovným tepelným tokom | stena medzi vykurovanou a nevykurovanou miestnosťou (garáž) |
| 6 | VDH | vnútorný strop, tok zdola nahor | doska pod nevykurovanou miestnosťou nad vykurovanou |
| 7 | VHD | vnútorný strop, tok zhora nadol | podlaha nad nevykurovanou miestnosťou |
| 8 | SZ1 | stena k zemine, hĺbka do 0,5 m | časť obvodovej steny pod terénom |
| 9 | SZ2 | stena k zemine, hĺbka 0,5–2,0 m | 〃 |
| 10 | SZ3 | stena k zemine, hĺbka nad 2,0 m | 〃 |
| 11 | PV1 | podlaha na teréne do 0,5 m pod terénom a do 2,0 m od vnútorného povrchu vonkajšej steny | pás podlahy najnižšieho podlažia do 2,0 m od vnútorného líca obvodových stien (pod stenami tiež) |
| 12 | PVT | podlaha na teréne – ostatné | zvyšok podlahy; celá podlaha, ak je hlbšie ako 0,5 m pod terénom |

Hĺbka terénu = najnižší bod terénu pri stavbe (ak je terén v modeli),
inak ±0,000.

#### Otvory `_N_M`

| pole | význam |
|---|---|
| `druh_otvoru_N_M` | 1 OK okno, 2 SO strešné okno, 3 PD presklené dvere, 4 DV plné dvere, 5 PR poklop, 6 PS presklená stena; `index` = kód − 1. CreaBIM: okno → 1, dvere so solárnym faktorom g → 3, ostatné dvere → 4 |
| `omeno_N_M` | meno otvoru |
| `opocet_N_M` | 1 (každý otvor zvlášť) |
| `osirka_N_M`, `ovyska_N_M`, `oplocha_N_M` | m, m² |
| `tvar2D_N_M` | JSON: `pole: [{x, y}]` – ľavý dolný roh od ľavého konca plochy pri pohľade zvonku a od spodku plochy |
| `ou_N_M` | Uw z vlastností okna/dverí (údaj výrobcu); 0, ak chýba |
| `oua_N_M`, `obx_N_M`, `obxua_N_M` | U·A, bx (1), bx·U·A |
| `osolar_N_M` | počítať solárne zisky (okná a presklené dvere) |
| `okonstrukcia_N_M` | prázdne (Ug, Uf, Ψg CreaBIM zatiaľ nemá; Fragment ich tu môže mať) |

`osklon_N_M` (podiel zasklenia %) CreaBIM nezapisuje – nechá sa
predvolená hodnota Fragmentu.

## Ako CreaBIM počíta obálku

- **Vykurované** sú všetky miestnosti okrem garáže a vonkajších priestorov
  (lódžia, terasa, balkón).
- **Obvodové steny**: steny, ktoré má výkres situácie za obvodové, a len
  tá ich časť, ktorá susedí s vykurovanou miestnosťou. Dĺžka na vonkajšom
  líci od rohu po roh, výška od podlahy po podlahu ďalšieho podlažia
  (najvyššie podlažie: jeho výška). Orientácia podľa smeru von a
  skutočného severu.
- **Podlaha**: doska, na ktorej vykurovaná miestnosť stojí; jej obrys, ak
  na nej stoja len vykurované miestnosti, inak plochy miestností.
- **Strop / strecha**: doska nasledujúceho podlažia nad miestnosťou, inak
  strecha nad ňou (plocha miestností / cos sklonu).
- **U** skladby: z vrstiev a λ materiálov, odpory pri povrchu podľa
  STN EN ISO 6946 (rovnaké ako vo vlastnostiach skladby v CreaBIM).

## Čo ešte nie je

- výber obce (Fragment má vlastné kódy obcí),
- Ug / Uf / Ψg a podiel zasklenia okien,
- rekuperácia: Fragment má `pomerRekuperacia` a `ucinnostRekuperacia`
  vo výpočtovej časti; účinnosť jednotiek nie je v katalógu CreaBIM,
- tepelné mosty (Fragment: paušálne ΔU 0,05 alebo presne).
