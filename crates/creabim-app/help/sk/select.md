# Výber

Nástroj **Výber** (šípka, prvý v rade nástrojov) je to, v čom ste väčšinu času. Ním vyberáte prvky, ktoré chcete prezrieť, zmeniť, posunúť alebo odstrániť. Z ktoréhokoľvek iného nástroja sa k nemu vrátite dvojitým `Esc`.

## Výber kliknutím {#click-to-select}

- Kliknite na prvok v ľubovoľnom pohľade: 3D, pôdorys, rez, pohľad alebo pohľad na liste. Vo všetkých pohľadoch zoranžovie a *Vlastnosti* vpravo zobrazia jeho nastavenia.
- `Shift+klik` pridá prvok do výberu, alebo ho z neho znova odoberie.
- Kliknutím do prázdneho priestoru zrušíte výber. V 3D pohľade urobí to isté `Esc`, ak už niet z čoho iného ustúpiť. `Ctrl+A` vyberie všetko. `Alt+A` tiež zruší výber, pokiaľ si tieto klávesy nezabral iný program (napríklad snímanie obrazovky v chatovacej aplikácii): potom použite `Esc` alebo *Výber › Zrušiť výber*.
- Kliknutím na riadok v prehliadači projektu vyberiete tento prvok. `Shift` alebo `Ctrl` k nemu pridáva.

Stavový riadok počíta, čo je vybrané, a pri nástroji Výber končí lišta nástroja rovnakým počtom podľa druhu („Vybraných 5: 2 okná, 2 steny, 1 značka“). Keď je vybraných viac prvkov, Odstrániť, Posunúť, Kopírovať a Skryť pôsobia na všetky a *Vlastnosti* zobrazia ten vybraný naposledy.

## Najprv podržanie ukazovateľa {#hover-first}

Ešte pred kliknutím sa prvok pod ukazovateľom jemne modro rozsvieti vo všetkých otvorených pohľadoch. To je to, čo kliknutie vyberie.

## Výber toho, čo je pod tým {#picking-what-is-underneath}

Niekedy je pod ukazovateľom viac prvkov: nosník v stene, stĺp za hranou dosky. Vtedy ich malý štítok pri ukazovateli spočíta, napríklad **1/3 · Tab**.

- `Tab` presunie zvýraznenie na ďalší a `Shift+Tab` späť. Stavový riadok pomenuje zvýraznený („Nosník · Preklad (2 z 3 pod ukazovateľom)“). Kliknutím ho vyberiete.
- Alebo kliknite na štítok: krátky zoznam pomenuje všetky prvky pod ukazovateľom. Podržaním ukazovateľa nad riadkom uvidíte, ktorý to je, a kliknutím ho vyberiete.

Obyčajné kliknutie bez `Tab` vyberie to, čo vždy. V pohľade (fasáde) siaha aj za plochu, ktorú vidíte, napríklad na dosku za stenou.

![Pod ukazovateľom sú dve veci: stena svieti a Tab prejde na ďalšiu](img/tab-pick.jpg)

Na skryté prvky, s ktorými potrebujete chvíľu pracovať, je [röntgen](visibility#x-ray) (`Alt+Z`): steny a dosky zosvetlí a to, čo je v nich, dá dopredu.

## Výber obdĺžnikom, kruhom alebo lasom {#select-with-a-box-a-circle-or-a-lasso}

S nástrojom Výber ponúka lišta nástroja tri spôsoby výberu ťahaním:

| Tvar | Ako |
| --- | --- |
| **Obdĺžnik** (`B` v 3D) | Ťahajte obdĺžnik |
| **Kruh** | Ťahajte od stredu von |
| **Laso** | Voľnou rukou obkreslite, čo chcete |

- V pôdoryse alebo reze ťahajte po prázdnom papieri.
- V 3D stlačte `B` (alebo vyberte tvar v lište nástroja), potom ťahajte.

Ťahaný **zľava doprava** (plná modrá) tvar vyberie iba to, čo leží celé v ňom. Ťahaný **sprava doľava** (čiarkovaná zelená) vyberie všetko, čoho sa dotkne. Počas ťahania sa rozsvieti, čo zachytí, a malý počet pri ukazovateli povie koľko.

Vedľa tvarov lišta nastavuje, čo ťahanie urobí s výberom: **Nový** (zachytené nahradí výber), **Pridať**, **Odčítať**, **Obrátiť** (zachytené sa prepne dnu alebo von) alebo **Prienik** (ponechá iba to, čo je vybrané aj zachytené). Podržané klávesy urobia to isté pre jedno ťahanie: `Shift` pridáva, `Ctrl` odoberá.

Ťahanie začína kdekoľvek okrem už vybraného prvku (takéto ťahanie ho posunie), v pôdorysoch, rezoch a listoch a v 3D pohľade pri obvyklom ovládaní myšou. Na liste obdĺžnik na prázdnom papieri vyberá pohľady, značky a poznámky.

![Obdĺžnik ťahaný v pôdoryse](img/box-select.jpg)

## Ponuka Výber {#the-select-menu}

Ponuka *Výber* v lište nástroja obsahuje zvyšok:

- **Vybrať všetko** (`Ctrl+A`), **Zrušiť výber** a **Obrátiť výber**: vyberie nevybrané a zruší vybrané.
- **Výber obdĺžnikom**, **Výber kruhom** a **Výber lasom**.
- **Vybrať napojené**: pridá to, čo je napojené na výber, napríklad steny, ktoré sa s ním stretávajú, a dvere a okná v ňom.
- **Vybrať náhodne**: ponechá zhruba polovicu výberu, náhodne, na preriedenie radu.
- **Všetko podľa typu**: všetky steny, všetky dvere, všetky dosky… s počtom, bez toho, aby ste najprv niečo vybrali.
- **Vybrať podobné**, ako nižšie.

## Vybrať podobné {#select-similar}

Vyberte jeden prvok (alebo niekoľko), potom naň kliknite pravým tlačidlom alebo otvorte ponuku *Upraviť* a zvoľte **Vybrať podobné**:

- **Všetky tohto typu v pohľade** (`Shift+A`): všetky prvky rovnakého [typu](types), ktoré pohľad zobrazuje. Prvok bez typu ide s prvkami svojho druhu a rozmerov (steny jednej hrúbky, okná jednej veľkosti).
- **Všetky tohto typu v projekte** (`Ctrl+Shift+A`): to isté na všetkých podlažiach.
- **Celá táto kategória v pohľade**, alebo **v projekte**: všetky steny, všetky dvere, všetky dosky… ako výber.
- **Podobné značky**: s vybranou značkou všetky značky rovnakého druhu (značky dverí, značky okien…) v tom istom pohľade.
- **Ponechať len**: pri zmiešanom výbere (napríklad steny, dvere a značky) ponechá iba jednu jeho kategóriu, napríklad iba steny. Každá položka udáva počet.

„Pohľad“ je pohľad, v ktorom bola ponuka otvorená: pôdorys, rez alebo fasáda, alebo 3D pohľad bez toho, čo je v ňom skryté.

## Výber podľa typu alebo materiálu {#selecting-by-type-or-material}

- *Vybrať všetky inštancie* vo *Vlastnostiach* vyberie všetky prvky rovnakého [typu](types).
- [Prehliadač materiálov](materials) vyberie všetky prvky z daného materiálu.
- Kliknutie na nadpis vo výkaze zoskupenom podľa typu vyberie všetky jeho riadky.

## Ponuka pravého tlačidla {#the-right-click-menu}

Kliknite pravým tlačidlom na prvok (alebo na prázdny priestor) s nástrojom Výber a uvidíte, čo sa dá urobiť priamo tam: posunúť, otočiť, kopírovať, odstrániť, skryť, prevrátiť stenu, *Skryť v pohľade*, nastaviť stred otáčania alebo pracovnú rovinu. Pozri [Kopírovanie, vkladanie, odstraňovanie a krok späť](edit).
