# Vzorový dom

CreaBIM sa otvára **Domom pri javoroch**, dvojpodlažným rodinným domom s jednopodlažným krídlom kuchyne. Je to úplný malý projekt: model, výkresy z neho vytvorené a sada výkresových listov pripravená na tlač. Nie je na ňom nič zvláštne. Je postavený tými istými nástrojmi, aké máte vy, takže ho môžete rozobrať a pozrieť sa, ako funguje, alebo ho zmeniť a sledovať, ako ho výkresy nasledujú. Späť ho dostanete cez *Súbor › Nový vzorový dom*.

## Čo je v modeli {#what-is-in-the-model}

- **Podlažia**: *Základy* (-800), *Prízemie* (±0) a *1. poschodie* (+3000). Pozri [Podlažia](levels).
- **Osi** A–E a 1–3, od ktorých merajú kóty. Pozri [Osi](grids).
- **Steny** zo [skladieb](assemblies): biela omietka alebo drevený obklad na izolácii a tvárniciach, hore rámové priečky. Steny sa napájajú v rohoch a v T-spojoch. Steny poschodia nasledujú strechu hore do štítov. Pozri [Steny](walls).
- **Dvere a okná**, ktoré zdieľajú [typy](types), s rámami. Niektoré sú prevrátené, aby sa otvárali na druhú stranu. Kuchyňu s obývačkou spája jednoduchý otvor. Pozri [Dvere a okná](doors-windows).
- **Závesová stena**: trojpoľové presklenie do záhrady v krídle kuchyne, s dverami v strednom poli. Vyreže si vlastný otvor v severnej stene. Pozri [Závesové steny](curtain-walls).
- **Podlahy**: zateplená doska na teréne, betónový strop nad prízemím s otvorom pre inštalačnú šachtu a dlažba terasy, schodov lodžie a prístupového chodníka. Pozri [Dosky](slabs).
- **Schodisko** s medzipodestou, s madlom a zábradlím. **Šachta** prereže schodiskový priestor cez strop. Pozri [Schodiská a zábradlia](stairs-railings).
- **Strechy**: bridlicová sedlová strecha so sklonom 35°, zelená plochá strecha nad krídlom a terasou a zinkový prístrešok nad vchodovými dverami. Pozri [Strechy](roofs).
- **Nosná konštrukcia**: oceľový stĺp v rohu lodžie prechádzajúci oboma podlažiami, dva stĺpiky terasy a oceľové nosníky pod balkónom a pozdĺž odkvapu. Pozri [Stĺpy a nosníky](columns-beams).
- **Základy**: betónové pásy pod stenami a pätky pod stĺpmi. Pozri [Základy](footings).
- **Nábytok a zariaďovacie predmety** z katalógu v každej miestnosti: kuchynská linka, pohovky, postele, vaňa a WC, zásuvky, vypínače, svietidlá, radiátory a kotol. Pozri [Nábytok a zariaďovacie predmety](furniture) a [Elektro, zdravotechnika a VZT](fixtures).
- **Miestnosti**: šestnásť, od 0.01 Pracovňa po 1.07 Balkón, s povrchovými úpravami a užívaním. Ich plochy nasledujú steny. Lodžia, terasa a balkón majú nakreslené obrysy. Pozri [Miestnosti](rooms).
- **Podhľady**: sadrokartón nad miestnosťami poschodia, pri ktorom končia priečky, a zavesený kazetový podhľad nad WC, skladom a technickou miestnosťou. Pozri [Podhľady](ceilings).

## Výkresy {#the-drawings}

Prehliadač projektu vypisuje pohľady. Každý vzniká z modelu, takže sa aktualizuje hneď, ako niečo zmeníte.

- **Pôdorysy** prízemia, poschodia, základov a strechy. Zobrazujú značky miestností s názvom, číslom a plochou, značky pri všetkých dverách a oknách a reťazce kót. Pozri [Kóty](dimensions) a [Značky](tags).
- **Rezy** A-A cez schodisko a B-B cez lodžiu a krídlo. Pozri [Rezy a pohľady](sections).
- **Pohľady** na všetky štyri strany, tieňované.
- **3D pohľady** z ulice a zo záhrady.
- **Výkazy** dverí, okien, stien, nosnej konštrukcie, nábytku, zariaďovacích predmetov a miestností. Pozri [Výkazy](schedules).

## Výkresové listy {#the-sheets}

Skupina *Listy* obsahuje sadu výkresov, akú by ste poslali na stavebné povolenie. Každý list má vlastné popisové pole projektu, vyplnené stavbou, investorom a stupňom dokumentácie. Pozri [Výkresové listy](sheets).

- A000 Titulný list, s pohľadom zo záhrady a zoznamom výkresov
- A101 Pôdorys prízemia a A102 Pôdorys poschodia, 1:50
- A103 Pôdorys základov, 1:50, a A104 Pôdorys strechy, 1:100
- A201 Rez A-A a A202 Rez B-B, 1:50
- A301 Pohľady, 1:100
- A501 Detaily: rez C-C v mierke 1:25 a tri [detaily](details) v mierke 1:10
- A601 a A602 Výkazy a A603 Výkaz miestností
- A901 3D pohľady

## Čo si vyskúšať {#things-to-try}

- Vyberte stenu v 3D pohľade a potiahnite jej koniec. Podlahy, kóty aj miestnosť okolo nej ju nasledujú vo všetkých pohľadoch.
- Vyberte okno a v zozname **Typ** vo *Vlastnostiach* zvoľte iný typ. Zmení sa aj jeho značka a výkaz okien.
- Vytlačte celú sadu cez *Súbor › Listy do PDF*. Pozri [Uloženie a export](start-export).
- Pravým tlačidlom kliknite na strechu v 3D pohľade a zvoľte *Skryť kategóriu v pohľade*, aby ste videli dovnútra. Pozri [Viditeľnosť a röntgen](visibility).
- Exportujte dom cez *Súbor › IFC4 (.ifc)* a otvorte ho v inom programe. Pozri [IFC](ifc).

> **Tip:** Značky miestností čítajú údaje z miestností, takže keď posuniete stenu, plocha v značke aj vo výkaze miestností sa zmení s ňou. *Vytvoriť › Kontrola dverných otvorov* vypíše každý nábytok alebo zariaďovací predmet, ktorý stojí v dverách alebo v rozsahu otvárania dverí; vzorový dom nemá žiadny.

**Ďalej:** [Začíname: prvý dom](start)
