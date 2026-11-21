# Profily

**Profil** je prierez, ktorý nakreslíte raz a použijete mnohokrát: atiková krycia lišta, rímsa, žľab, sokel, oceľový profil. Ťahania ho vedú po dráhe, vytiahnutia ho vytláčajú priamo nahor a stĺpy, nosníky a zábradlia ho môžu prevziať ako svoj prierez.

![Pohľad Profil v oblasti: predvoľba Atiková krycia lišta, jej vkladací bod a celkové rozmery](img/profile-view.jpg)

## Knižnica profilov {#the-profile-library}

Každý projekt má knižnicu profilov: dvanásť **predvolieb** (Atiková krycia lišta, Sokel, Lišta, Rímsa, Sokel interiérový, Žľab, Madlo, Profil I, Profil U, Uholník, Profil T a Rúra) a vlastné profily projektu. Otvoríte ju cez:

- *Správa › Profily*;
- skupinu *Profily* v prehliadači projektu;
- ponuku druhu pohľadu ktorejkoľvek [oblasti](interface#view-areas): *Profily* zobrazia knižnicu a jej editor zväčša, ako samostatný pohľad.

Predvoľbu nemožno zmeniť ani odstrániť: **Duplikujte** ju a vytvorte si vlastnú kópiu, potom meňte tú.

## Pohľad Profil {#the-profile-view}

Profil sa kreslí ako každý výkres, v **pohľade Profil**: oblasti s mriežkou, pomocnými čiarami, prichytávaním, zadávanými dĺžkami a stĺpcom priblíženia, na svetlom alebo tmavom papieri podľa témy. Jeho hlavička obsahuje:

- **zoznam profilov**, s malým obrázkom každého: zvoľte profil na úpravu;
- **+ Nový** (štvorec na začiatok), **Z výberu** (náčrty v ľubovoľnej rovine, dosky alebo vytiahnutia spojené do jedného profilu), **Duplikovať** a **Odstrániť**;
- pole **Názov**: napíšte nový názov a stlačte `Enter`. Všetko vytvorené s profilom si ho ponechá;
- nástroje: **Výber**, **Kresliť**, **Vloženie**, **Pomocná čiara**, **Posunúť**, **Otočiť**, **Zarovnať** a **Vybrať dutinu**;
- **Podklad**: obrázok, DXF výkres alebo pohľad projektu pod profilom na obkreslenie;
- **Tvar**: či je vybraný tvar uzavretý (teleso) alebo otvorený (drôt, pri ťahaní zhrubnutý);
- **Zrkadliť**: okolo vkladacieho bodu;
- **Použiť**, so štítkom počtu prvkov, ktoré ešte majú starý tvar.

Nad pohľadom Profil platia jeho vlastné klávesy: skratky aplikácie pre model ustúpia, kým `Ctrl+Z` a `Ctrl+Y` stále vracajú a opakujú. `Ctrl` + ťahanie stredným tlačidlom približuje, ako v ostatných pohľadoch.

S pohľadom Profil vpredu ukazujú *Vlastnosti* záložku **Profil**: jeho názov a rozmer, **Plochy** (vyplniť všetky dutiny), prevrátenia a X a Y vybraného rohu na zadanie.

### Kreslenie tvaru {#drawing-a-shape}

1. Zvoľte **Kresliť**.
2. Kliknite na rohy. Ukazovateľ sa prichytáva k rohom, priesečníkom pomocných čiar a čiaram podkladu a z bodu, z ktorého pracujete, a cez každý roh vedú vodorovné, zvislé a 15° pomocné čiary.
3. Pre zakrivenú stranu stlačte `A` (alebo tlačidlo **Oblúk**): kliknite na koniec oblúka, potom na bod, cez ktorý prechádza. `L` vráti rovné strany.
4. Dokončenie: kliknutím na prvý roh tvar uzavriete. Dokončí ho aj dvojklik, opätovné kliknutie na posledný roh alebo `Enter`; uzavrie sa aj sám, keď sa jeho konce stretnú. Pravé tlačidlo ho zruší.

Profil môže obsahovať viac tvarov. Kde sa tvary pretínajú alebo sú vnorené, **Vybrať dutinu** vyprázdni kliknutú plochu (otvor) a kliknutie na dutinu ju znova vyplní.

### Zmena tvaru {#changing-a-shape}

S nástrojom **Výber**:

- ťahajte roh alebo hranu (posúva sa kolmo na seba);
- kliknutím na dĺžku hrany ju zadáte. Pole dĺžky sa otvorí vedľa hrany;
- ťahaním obdĺžnika vyberiete viac (`Shift` pridáva); `A` vyberie všetko; `F` zobrazí vybrané;
- `Delete` na vybranej hrane tam uzavretý tvar otvorí (alebo otvorený skráti); na vybranom tvare ho odstráni; na rohoch či pomocných čiarach ich odstráni;
- `D` alebo prepínač v hlavičke zobrazí či skryje dĺžky hrán a celkové rozmery.

**Posunúť** (`M`) a **Otočiť** (`R`) pracujú s vybraným, od základného bodu alebo okolo stredu. Podržte `Shift` pri poslednom kliknutí, alebo najprv stlačte `Shift+D`, a zanecháte kópiu pri zachovaní originálu. **Zarovnať** zarovná hranu alebo pomocnú čiaru na inú čiaru. Pravé tlačidlo, keď nič neprebieha, otvorí rýchlu ponuku: Odstrániť, Posunúť, Otočiť, Duplikovať, Zarovnať, Zobraziť, Vybrať všetko, Vybrať dutinu a Zobraziť alebo skryť kóty.

### Vkladací bod {#the-insertion-point}

**Vkladací bod** je miesto, kde profil sedí na svojej dráhe: na hornej hrane steny pri atikovej lište, pri spodku pri sokli. Zvoľte **Vloženie** a kliknite tam, kam patrí; prichytáva sa k rohom a pomocným čiaram.

## Použitie profilu {#using-a-profile}

- **Ťahanie po dráhe** a **Vytiahnutie**: zvoľte profil vo voľbách nástroja pred kliknutím. Pozri [Vytiahnutia, ťahania po dráhe a dutiny](parts).
- **Stĺpy a nosníky**: nástroje Stĺp a Nosník berú profil z knižnice popri štvorcovom a kruhovom a *Vlastnosti* ich vypisujú vedľa vlastného prierezu prvku. Pozri [Stĺpy a nosníky](columns-beams).
- **Zábradlia**: horné madlo môže prevziať profil z knižnice. Pozri [Schodiská a zábradlia](stairs-railings).

Vybrané ťahanie alebo vytiahnutie má vo *Vlastnostiach* sekciu **Profil**: zmeniť profil, **Upraviť v pohľade** (otvorí ho v pohľade Profil), **Aktualizovať** ho na aktuálny tvar profilu alebo **Uložiť ako nové** a z vlastného tvaru urobiť nový profil projektu.

## Použiť: keď sa prvky menia {#apply-when-elements-change}

Úprava profilu nemení v modeli nič, kým nekliknete na **Použiť**. Potom každý prvok s ním vytvorený dostane nový tvar, jedným krokom späť. Dovtedy štítok na **Použiť** hovorí, koľko ich má ešte starý tvar, a každý z nich možno aktualizovať aj samostatne cez **Aktualizovať** v jeho *Vlastnostiach*.

Odstránenie profilu ponechá to, čo bolo s ním vytvorené, tak, ako je: tieto prvky si ponechajú tvar.

> **Tip:** Profil nakreslite nad naskenovaným detailom: **Podklad › Pridať obrázok…**, potom **Kalibrovať**: kliknite na dva body na ňom so známou vzdialenosťou a zadajte túto vzdialenosť.
