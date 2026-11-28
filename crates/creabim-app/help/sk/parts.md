# Vytiahnutia, ťahania po dráhe a dutiny

Keď časť budovy nie je stena, doska, strecha ani stĺp, modelujte ju ako **vytiahnutie** alebo **ťahanie po dráhe**. Oba sú bežné prvky: majú materiál a názov, v pôdorysoch a rezoch sa zobrazujú prerezané a v 3D plné a ich rozmery môžete neskôr meniť vo *Vlastnostiach*. **Dutina** má rovnaké nástroje tvaru, ale namiesto toho, aby bola plná, reže to, čo stretne.

CreaBIM nerozhoduje, čím diel je. Komín, ohnisko, atiková krycia lišta, sokel aj schodiskové stupne sa robia rovnako. Každému dáte názov, materiál a triedu IFC, ako ktorá sa exportuje.

## Vytiahnutia {#extrusions}

**Vytiahnutie** je pôdorysný obrys vytiahnutý priamo nahor. Použite ho na komínové teleso, ohnisko, pilier alebo kvetináč.

1. Zvoľte nástroj **Vytiahnutie** (*Vytvoriť › Stavebné prvky*).
2. Vo voľbách nástroja nastavte jeho *Spodok* (ako vysoko nad podlažím začína) a *Výšku*.
3. Nakreslite obrys ako pri doske: `1` Mnohouholník, `2` Obdĺžnik, `3` Vyplniť miestnosť alebo `4` Z náčrtu.

*Vlastnosti* ukazujú jeho spodok a výšku. Pod nimi je jeho **trieda IFC**. Je to *IfcBuildingElementProxy*, pokiaľ nezvolíte inú, napríklad *IfcChimney*, *IfcCovering*, *IfcMember* alebo *IfcStair*.

## Ťahania po dráhe {#sweeps}

**Ťahanie po dráhe** vedie profil prierezu pozdĺž dráhy.

1. Zvoľte nástroj **Ťahanie po dráhe**.
2. Vo voľbách nástroja zvoľte profil. **Vlastné profily nástroja** sa rozmerujú priamo tam:
   - **Atiková krycia lišta**: na vrchu steny, presahuje oba líca o svoj *Presah*.
   - **Sokel**: pozdĺž spodku steny, vystupuje z jej vonkajšieho líca.
   - **Lišta**: obyčajný obdĺžnik, na obrubník alebo tyč.
   - **Stupne**: rameno stupňov (*Stupnica*, *Výška stupňa* a počet *Stupňov*).

   Alebo zvoľte profil z **knižnice profilov**, s malým obrázkom každého: predvoľby aj vlastné profily projektu. Pozri [Profily](profiles).
3. Vo voľbách nástroja nastavte rozmery profilu.
4. Kliknutím na stenu vediete profil pozdĺž nej. Atiková krycia lišta ide po vrchu steny, sokel po jej spodku. Alebo kliknite na body dráhy kdekoľvek a dokončite opätovným kliknutím na posledný bod.

Ťahanie umiestnené na stene **sleduje stenu**. Keď sa stena posunie, predĺži alebo zmení výšku, ťahanie ide s ňou. Keď sa stena odstráni, odstráni sa aj ťahanie. Vo *Vlastnostiach* môžete prepnúť medzi vrchom a spodkom steny, odsadiť ťahanie od tejto hrany a predĺžiť ho za ktorýkoľvek koniec steny.

**Stupne ako jeden prvok**: zvoľte *Stupne* a kliknite na dva body. Dráha vedie po nábežných hranách a rameno stúpa doľava od smeru kliknutia. *Zrkadliť profil* vo *Vlastnostiach* ho otočí, aby stúpalo opačne.

*Vlastnosti* môžu zmeniť aj celkovú šírku a výšku profilu, čím ho preškálujú.

Ťahanie vytvorené s profilom z knižnice má vo *Vlastnostiach* sekciu **Profil**: zmeniť profil, otvoriť ho cez **Upraviť v pohľade**, **Aktualizovať** ho po zmene profilu, alebo **Uložiť ako nové**. Otvorený profil (drôt) sa pri ťahaní zhrubne. Počas klikania dráhy sa prierez ťahania priebežne zobrazuje.

Profilom sa môže stať ľubovoľný náčrt (v ľubovoľnej rovine, otvorený či uzavretý), doska alebo vytiahnutie: **Vytvoriť profil** v jeho *Vlastnostiach*.

## Komíny {#chimneys}

Nástroj **Komín** umiestni komín Schiedel kliknutím: v lište zvoľte systém (KERASTAR, ABSOLUT, UNI SMART, UNI ADVANCED PLUS, STABIL) a typ, jeho spodok, výšku a pootočenie tvárnice. Typ je jeden alebo dva prieduchy, s vetracou šachtou alebo bez nej.

Obrys a prieduch majú rozmery výrobcu podľa dátových listov Schiedel Slovensko; Vlastnosti ich ukazujú aj so zdrojom a systém či prieduch zmenia na mieste. Komín prechádzajúci strechou dostane lemovanie a krov od neho drží 50 mm ako pri každom diele; výkaz ho uvádza v skupine Komíny v bežných metroch a s hmotnosťou.

Listy výrobcu udávajú vonkajší rozmer tvárnice a jej šachtu, nie polohu prieduchov a šachty v nej: každý prieduch je v strede štvorcovej bunky šírky tvárnice na jej konci, šachta v strede zvyšku.

## Dutiny {#voids}

**Dutina** reže každú stenu, dosku, strechu, terén, spevnenú plochu a plné vytiahnutie či ťahanie, ktoré stretne. Použite ju na ohnisko v komíne, otvor cez dosku alebo nik v stene.

1. Zvoľte nástroj **Dutina**.
2. Vo voľbách nástroja nastavte jej *Spodok* a *Výšku*.
3. Nakreslite jej obrys ako pri doske.

Dutina sa vyhodnocuje podľa toho, kde čo je. Posuňte dutinu a otvor sa posunie s ňou. Posuňte ju mimo prvku a ten už nie je rezaný. V 3D sa dutina zobrazuje ako priehľadný oranžový obrys, na ktorý sa dá kliknúť. V pôdorysoch a rezoch vidíte iba otvor, ktorý zanecháva. Plný diel zmeníte na dutinu, alebo dutinu späť na plný diel, zaškrtnutím či zrušením **Dutina** vo *Vlastnostiach*.

## Kreslenie na ploche {#drawing-on-a-face}

V 3D pohľade kreslia nástroje Vytiahnutie, Dutina, Ťahanie po dráhe, Doska a Otvor v stene na plochu pod ukazovateľom: stenu, šikmú strechu, skosenú plochu. Prvé kliknutie začne na tej ploche (alebo na pracovnej rovine, ak je nastavená) a ďalšie rohy zostávajú v tej istej rovine. Prichytávanie funguje ako v pôdoryse, ale iba na tej ploche.

| Nástroj | Nakreslený na ploche |
|---|---|
| Vytiahnutie | Vystupuje kolmo z plochy, do výšky *Výška*. |
| Dutina | Reže kolmo do plochy, do hĺbky *Hĺbka na ploche*: strešné okno, nika. |
| Ťahanie po dráhe | Jeho dráha vedie po ploche a profil z nej vystupuje: pás hore a naprieč stenou. `Enter` dokončí otvorenú dráhu; kliknutie na prvý bod ju uzavrie. |
| Doska | Iba na vodorovnej ploche: jej vrch ide na tú plochu. |
| Otvor v stene | Keď je pracovná rovina na líci steny (*Pracovná rovina › Vybrať plochu*): obdĺžnik okolo toho, čo nakreslíte. Bez nej kliknutie umiestni otvor ako predtým. |

Vytiahnutie, dutina či ťahanie si ponechá rovinu, v ktorej bolo nakreslené. Vo *Vlastnostiach* jeho *Odsadenie od plochy* a *Hĺbka* idú v smere normály plochy. Jeho rohy sa v Režime úprav posúvajú v tej rovine a posun či otočenie berie rovinu so sebou.

## Zmena tvaru {#reshaping}

Vytiahnutie, dutina a ťahanie cez body majú rovnaké úchyty ako doska. Ťahaním rohu alebo hrany zmeníte tvar. Rohy pridáte či odstránite, alebo zmeníte otvory vytiahnutia, v [Režime úprav](edit-mode) (`Enter`). Ťahanie pozdĺž steny nemá vlastné rohy: sleduje stenu.

## Výmena {#exchange}

V IFC sa vytiahnutie alebo ťahanie zapíše ako zvolená trieda, s tvarom tak, ako je postavené. Dutina sa zapíše ako *IfcOpeningElement* v každom prvku, ktorý reže. Tieto prvky si ponechajú nerezaný tvar, takže iné nástroje vidia riadny otvor. Formát IFCX nesie vlastné parametre každého dielu, takže ho CreaBIM načíta späť ako to isté upraviteľné vytiahnutie, ťahanie alebo dutinu. Pozri [IFC](ifc).

Pohľady môžu vytiahnutia a ťahania skryť cez kategóriu *Vytiahnutia a ťahania*. Dutiny sú v *Otvory, šachty a výrezy* (pozri [Viditeľnosť](visibility)).
