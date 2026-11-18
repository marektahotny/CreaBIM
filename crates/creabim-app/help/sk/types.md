# Typy

**Typ** je opakovane použiteľná definícia, ktorú zdieľa mnoho prvkov: „Obvodová stena 365, tehla s dutinou“, „Okno 1200 × 1300, dvojsklo“. Typ obsahuje spoločné hodnoty (skladbu, rozmery, rám, mriežku závesnej steny). Každý prvok, **inštancia**, si ponecháva vlastnú polohu a všetko, čo prepisuje.

Zmeňte typ a každá inštancia ho nasleduje, okrem hodnôt, ktoré inštancia prepisuje. Typy môžu mať steny, závesné steny, dvere, okná, dosky, strechy, stĺpy a nosníky. Nábytok a zariaďovacie predmety majú typ vždy: pozri [Nábytok a zariaďovacie predmety](furniture).

## Okno Typy {#the-types-window}

Otvorte ho cez *Materiály › Typy…*, *Vytvoriť › Typy…* alebo **⋯** pri skupine *Typy* v prehliadači.

- Typy sú zoskupené podľa druhu a pri každom je počet inštancií.
- Hodnoty typu upravíte vpravo.
- *Pridať základné* pridá základnú sadu; *Duplikovať* a *Odstrániť* sú pod zoznamom.
- *Vybrať všetky inštancie* a *Priradiť výberu* sú dole.

![Okno Typy](img/types.jpg)

## Samostatný náhľad typu {#seeing-a-type-on-its-own}

Dvakrát kliknite na typ v prehliadači alebo kliknite na **Otvoriť náhľad** v jeho *Vlastnostiach* a uvidíte ho samostatne: 3D vzorku (potiahnutím ju otáčate) s pôdorysom a rezom vedľa nej. Zmeňte hodnoty vľavo a vzorka ich nasleduje.

## Hľadanie inštancií {#finding-its-instances}

Vyberte typ a v jeho *Vlastnostiach* zaškrtnite **Zobraziť v modeli**. Každá inštancia sa zvýrazní v 3D pohľade aj v pôdoryse a oba pohľady sa na ne priblížia. Zaškrtnite **Stlmiť ostatné** a uvidíte zreteľne len ich. Samotný výber sa nemení. Zrušte zaškrtnutie alebo vyberte niečo iné a vrátite sa späť.

## Kreslenie s typom {#drawing-with-a-type}

V nástroji Stena, Dvere, Okno, Doska, Strecha, Stĺp alebo Nosník vyberte typ v ponuke **Typ** na začiatku volieb nástroja. Nové prvky sa nakreslia ako tento typ. Ak namiesto toho zadáte rozmer v lište, typ sa zruší, pretože rozmer mu už nezodpovedá.

## Typ prvku {#an-element-s-type}

Vyberte prvok. Vo *Vlastnostiach*:

- výber **Typ** zamení jeho typ (pre všetky vybrané prvky naraz);
- zmenou hodnoty ju **prepíšete**; prepísané hodnoty sú oranžové;
- *Obnoviť podľa typu* zruší prepísania, *Aktualizovať typ* ich zapíše do typu;
- *Vybrať všetky inštancie*, *Upraviť typ…*;
- *Vytvoriť z neho typ* urobí z prvku bez typu nový typ.

Výkaz môže zoskupiť riadky podľa typu: vo *Vlastnostiach* výkazu zaškrtnite *Zoskupiť podľa typu*.
