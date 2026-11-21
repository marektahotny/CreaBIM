# Strechy

## Kreslenie strechy {#drawing-a-roof}

1. Zvoľte nástroj **Strecha**.
2. Vo voľbách nástroja zvoľte jej tvar a **Sklon** (v stupňoch):
   - **Plochá**
   - **Pultová**: jedna rovina
   - **Sedlová**: dve roviny stretávajúce sa v hrebeni, so štítmi
   - **Valbová**: roviny na všetkých stranách
3. Nakreslite jej obrys ako pri doske: `1` Mnohouholník, `2` Obdĺžnik, `3` Vyplniť miestnosť, `4` Z náčrtu. Pri *Vyplniť miestnosť* sa strecha, ktorú by vytvoril, zobrazí pod ukazovateľom ešte pred kliknutím a medzery, ktoré tomu bránia, červeno. Obieha aj oblúkové steny.

![Vzorový dom: sedlová strecha a plochá strecha pokračujúca nad terasou](img/roofs.jpg)

## Zmena strechy {#changing-a-roof}

Vo *Vlastnostiach*:

- tvar a sklon;
- presah za obrys;
- **Odsadenie spodku**: ako vysoko nad podlažím je odkvap, napríklad 3,00 m, aby ležala na stenách podlažia;
- **Hrana odkvapu** a **Ďalšia hrana**: ktorá hrana je odkvapom pultovej strechy alebo ktoré hrany sú štíty;
- jej hrúbka, materiál alebo [skladba](assemblies) a jej otvory.

Súhrnný riadok uvádza rohy, plochu vrátane presahu a výšku strechy. *Vlastnosti* tiež hovoria, kde končí odkvap, a upozornia, keď strmý sklon a dlhý presah stiahnu odkvap pod výšku hlavy nad stenami.

## Strechy v pôdoryse {#roofs-in-plan}

Pôdorys, ktorý sa na strechu pozerá zhora, napríklad pôdorys strechy, kreslí na každej šikmej rovine šípku smerom dolu k odkvapu so sklonom vedľa nej. Šípky vychádzajú z tvaru strechy, takže sledujú zmeny jej sklonu a hrán. Plochá strecha ich nemá; jej spád napíšte do [poznámky](notes).

## Steny pod strechou {#walls-under-a-roof}

Vyberte steny a použite *Pripojiť vrch k streche/doske*: ich vrchy budú sledovať spodok strechy, zrezané podľa jej sklonu. Pozri [Steny](walls#base-and-top).

> **Poznámka:** Sedlové a valbové strechy potrebujú konvexný obrys (bez vnútorných rohov). Na obryse do L zostane strecha plochá a *Vlastnosti* to oznámia.

## Žľaby a zvody {#gutters-and-downpipes}

Nástroj **Žľab** zavesí odkvapový žľab pozdĺž strechy: klikajte po odkvape strechy roh po rohu a potom znova kliknite na posledný bod alebo stlačte Enter. Žľab drží pri streche s odstupmi z hornej lišty: *Odstup* od línie odkvapu k zadnej hrane žľabu v pôdoryse a *Pod odkvapom* od vrchu strechy pri odkvape dole k hornej hrane žľabu. Tam vyberiete aj polkruhový alebo štvorcový žľab a jeho veľkosť.

Nástroj **Zvod** osadí zvod na žľab, na ktorý kliknete. Presunutím zvod posúvate po jeho žľabe, alebo zadajte *Poloha na žľabe* vo *Vlastnostiach*. Žľab má k zvodom spád podľa *Spád* (mm na m): každý úsek klesá od najvyššieho bodu, v strede medzi dvoma zvodmi alebo na konci žľabu, k najbližšiemu zvodu.

*Vlastnosti* zvodu ukazujú pôdorysnú plochu strechy, ktorá do neho odteká, a prietok Q = r · A · C, s r a C nastavenými na žľabe (predvolene 0,03 l/s·m² a 1,0). Pôdorys strechy kreslí žľaby, zvody, ich označenia a šípky spádu.

**Tabuľka klampiarskych prvkov** (výkaz) uvádza každý prvok raz s označením (K-1, K-2…), značkou, popisom, rozvinutou šírkou, dĺžkou a príslušenstvom (háky, čelá, kotlíky, kolená, objímky) a pod ňou plochu strešných rovín, krytinu a dĺžky hrebeňa a nárožia. Umiestnite ju na výkres strechy.
