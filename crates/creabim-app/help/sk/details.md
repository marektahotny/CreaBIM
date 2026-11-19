# Detaily

**Detail** je výkres jednej časti budovy vo väčšej mierke, napríklad päty steny, nadpražia okna alebo odkvapu. Je to samostatný pohľad, vyznačený rámčekom na pôdoryse alebo reze, ktorý detailuje. Kreslíte doň **detailné komponenty**: tepelnú izoláciu, fólie, oplechovania a ďalšie časti, ktoré model neobsahuje.

## Výrezy {#callouts}

1. Otvorte pôdorys alebo rez.
2. Zvoľte nástroj **Výrez** (v skupine Pohľady).
3. Vo voľbách nástroja vyberte mierku detailu: 1:2, 1:5, 1:10 alebo 1:20.
4. Kliknite na dva rohy rámčeka okolo časti, ktorú chcete detailovať.

Vznikne nový pohľad s názvom „Detail 1“, „Detail 2“ atď. a otvorí sa. Detail zobrazuje to, čo jeho nadradený pohľad, orezané na rámček a kreslené vo vlastnej mierke. Na nadradenom pohľade má rámček krúžok s číslom detailu. Číslo je prvé slovo názvu detailu, ktoré obsahuje číslicu: „Detail 3 Odkvap“ zobrazí 3. Ak pohľad premenujete, krúžok sa zmení s ním.

![Rez C-C základného domu s rámčekmi jeho troch detailov](img/details.jpg)

Detaily sú v prehliadači projektu vo vlastnej skupine **Detaily**. Detail funguje ako ktorýkoľvek iný pohľad: môžete ho umiestniť na [list](sheets), nastaviť jeho [viditeľnosť](visibility) a [orezať](crop-boxes) ho. Ak detail odstránite, jeho rámček z nadradeného pohľadu zmizne. Ak odstránite nadradený pohľad, detail zostane a zobrazuje stále tú istú časť.

## Detailné komponenty {#detail-components}

1. Otvorte pohľad, do ktorého chcete kresliť, obvykle detail.
2. Zvoľte nástroj **Detailný komponent** (v skupine Anotácia).
3. Vo voľbách nástroja vyberte druh komponentu a jeho šírku.
4. Klikajte na jeho body. Na ukončenie kliknite znova na posledný bod alebo stlačte Enter.

Druhy sú:

- **Rohožová izolácia**: vlnovka medzi dvoma čiarami, jedna slučka na šírku po dĺžke.
- **Fólia**: hrubá čiara pre hydroizoláciu, parozábranu alebo difúznu fóliu.
- **Oplechovanie**: tenký plný pás pre plechové oplechovanie alebo okapnicu.
- **Výplň**: drevený prvok v reze, kreslený ako obdĺžnik s krížom.
- **Opakujúci sa detail**: rad prvkov so škárami medzi nimi, napríklad tehly alebo tvárnice.
- **Oblasť**: vyplnená plocha, ktorá zakryje to, čo je pod ňou, so šrafou [materiálu](materials). Uzavriete ju kliknutím na prvý bod.
- **Lomová čiara**: cikcak tam, kde výkres končí. Potrebuje dva body.

Detailný komponent patrí pohľadu, v ktorom je nakreslený, a v inom pohľade sa nezobrazuje. Detailné komponenty sú kresba, nie časti stavby: nezobrazujú sa vo výkazoch a export IFC ich vynechá. Na ich pomenovanie pridajte [poznámky](notes) s odkazovými čiarami.

## Detaily v základnom dome {#the-details-in-the-starter-house}

[Základný dom](starter-house) má list **A501 Detaily**. Zobrazuje rez C-C cez južnú stenu v mierke 1:25 s tromi výrezmi v 1:10: päta steny, nadpražie a parapet okna a odkvap.
