# Steny

## Kreslenie stien {#drawing-walls}

1. Zvoľte nástroj **Stena**.
2. Nastavte voľby v lište pod nástrojmi: jej **Typ** (alebo **Hrúbku**), **Referenčnú čiaru** a **Exteriér vľavo**.
3. Kliknutím začnite, potom kliknite na každý roh. `Esc` alebo pravé tlačidlo ukončí reťazec. Kliknutím na prvý roh uzavriete slučku.

Podržaním `Shift` držíte steny v pravom uhle. Steny sa prichytávajú k rohom, stredom a iným stenám a dajú sa kresliť v pôdorysoch, v rezoch a pohľadoch (pozdĺž čiary rezu, tak vysoko, ako ich nakreslíte) a v 3D pohľade na pracovnom podlaží.

![Voľby nástroja Stena](img/wall-options.jpg)

## Oblúkové steny, kruhy a obdĺžniky {#curved-walls-circles-and-rectangles}

Lišta nástroja Stena má štyri tvary:

| Tvar | Ako |
| --- | --- |
| **Rovná** | Kliknutím začnite, kliknutím na každý roh |
| **Oblúk** | Jej začiatok, koniec, potom bod, cez ktorý prechádza |
| **Kruh** | Stred, potom bod na kružnici: dve polovičné steny uzatvárajúce okrúhlu miestnosť |
| **Obdĺžnik** | Dva protiľahlé rohy: štyri steny napojené v rohoch |

Počas kreslenia sa skutočný tvar steny zobrazuje ako náhľad v pôdorysoch aj v 3D, s dĺžkou oblúka, polomerom alebo šírkou × výškou pri ukazovateli.

Vrstvy, otvory, vrch a základ oblúkovej steny sledujú oblúk a dvere a okná v nej stoja kolmo naň. Jej konce sa napájajú na steny, ktoré stretnú, tak ako by sa napájala rovná stena pozdĺž oblúka (na pokos, na tupo alebo s prekrytím), a miestnosti, dosky a strechy, ktoré vypĺňajú miestnosť, ju obiehajú. Ťahaním **úchytu ohybu** v jej strede zmeníte krivku, alebo zadajte jej **Polomer** vo *Vlastnostiach*. Závesová stena v oblúkovej stene je pozdĺž oblúka fazetovaná, ako sa aj stavia.

## Zmena steny {#changing-a-wall}

Vyberte ju, potom:

- zadajte nové hodnoty vo *Vlastnostiach*: hrúbku, výšku, odsadenie spodku, materiál, začiatok a koniec;
- ťahajte jej štvorcové koncové úchyty, alebo okrúhle úchyty v 3D pre výšku a líca;
- vysuňte alebo vtiahnite jedno líce cez [Posun plochy](move#move-a-face-push-and-pull);
- *Prekresliť trasu* vo *Vlastnostiach* a kliknutím zadajte novú trasu.

## Referenčná čiara {#location-line}

**Referenčná čiara** je čiara, na ktorú stenu kreslíte. Môže to byť os steny, jej vonkajšie alebo vnútorné líce, alebo os či líca jej jadra (nosnej vrstvy [vrstvenej steny](assemblies)). Čiara zostáva na mieste, keď stena zhrubne, dostane inú skladbu alebo sa prevráti.

Napríklad obvodové steny kreslite na ich **vonkajšie líce**: zväčšenie hrúbky ich potom zväčší iba dovnútra a vonkajšok budovy zostane na mieste.

Zvoľte ju vo voľbách nástroja pred kreslením, alebo vo *Vlastnostiach* neskôr.

![Obvodová stena s referenčnou čiarou na vonkajšom líci vo Vlastnostiach: čiarkovaná čiara vedie po jej vonkajšom líci](img/location-line.jpg)

## Vnútri a vonku: prevrátenie {#inside-and-outside-flip}

Každá stena má vonkajšiu stranu. Počas kreslenia `Medzerník` (alebo *Exteriér vľavo*) určuje, na ktorú stranu čiary ide; ukazuje to značka v náhľade. Stenu neskôr prevrátite:

- stlačením `Medzerníka`, keď je vybraná;
- cez *Prevrátiť* vo *Vlastnostiach* alebo *Prevrátiť stenu* v ponuke pravého tlačidla.

Stena sa otočí okolo referenčnej čiary a jej dvere a okná sa otočia s ňou.

## Napojenia {#joins}

Steny, ktoré sa stretnú, sa napoja samy: roh L je na pokos (pri vrstvených stenách vrstva po vrstve), stena končiaca na inej vytvorí T a dve pretínajúce sa steny vytvoria čistý kríž, kde hrubšia prechádza. Tri a viac stien nakreslených do jedného bodu vytvoria čistý uzol. Tam, kde sa steny stretávajú s doskami a inými stenami, sa ich vrstvy napájajú podľa priority: pozri [Napojenia](joins). Posuňte alebo potiahnite stenu a steny na ňu napojené ju nasledujú.

Ak chcete ponechať koniec voľný:

- kliknite pravým tlačidlom blízko konca a zvoľte *Zakázať napojenie na konci*, alebo zrušte *Napája sa na začiatku / konci* vo *Vlastnostiach*. Koniec zostane rovný a neťahá sa;
- alebo podržte `Ctrl` či `Alt` pri ťahaní koncového úchytu a ťaháte tento koniec voľne.

## Prekrývajúce sa steny {#overlapping-walls}

Steny nakreslené na sebe vyzerajú v pôdoryse ako jedna stena, ale v 3D a vo výmerách zostávajú dvoma telesami. Po úprave stavový riadok upozorní, keď sa vybraná stena prekrýva s inou. *Správa › Prekrývajúce sa steny* vyberie všetky takéto páry; ďalším kliknutím zlúči každý pár do hrubšej steny natiahnutej cez obe (jeden krok späť). *Správa › Kópie na origináloch* nájde a odstráni prvky, ktoré ležia presne na svojej kópii.

Ak chcete stenu rozdeliť na rovnaké časti vedľa seba alebo nad sebou, použite [Rozčleniť](divide).

## Spodok a vrch {#base-and-top}

Stena začína na **spodnom podlaží** plus **odsadenie spodku**. Jej **Vrch** je buď:

- *Nepripojený*: stena má vlastnú výšku;
- *Po* podlažie, plus *Odsadenie vrchu*: výška steny sleduje toto podlažie, keď sa posunie.

**Pripojiť vrch k streche/doske** (vo *Vlastnostiach* alebo v ponuke pravého tlačidla) zariadi, že vrch sleduje spodok strechy alebo dosky nad ním, zrezaný podľa jej sklonu. Štítové steny potom čisto končia pod strechou. *Odpojiť* to zruší.

## Typy a vrstvy {#types-and-layers}

Stena môže byť inštanciou **typu steny**, ktorý drží jej skladbu, takže zmena typu zmení všetky steny tohto typu. Pozri [Typy](types) a [Skladby](assemblies). Stena s rastrom je [závesová stena](curtain-walls).

## Clonové steny {#screen-walls}

**Clonová stena** je stena z opakujúceho sa prelamovaného prvku: prelamované tvárnice, lamely alebo mriežka. Zvoľte *Vytvoriť › Pozemok › Clonová stena* a nakreslite ju ako každú stenu. Otvory sú dané typom steny, takže každá clonová stena daného typu má rovnaký vzor a zobrazujú sa v pôdorysoch, rezoch aj v 3D. Základné typy sú *Stena, prelamované tvárnice 400 × 200*, *Stena, drevené lamely* a *Stena, mriežka 150*. Clonová stena sa exportuje do IFC s otvormi vyrezanými v tvare.
