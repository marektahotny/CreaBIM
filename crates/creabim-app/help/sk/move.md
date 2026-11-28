# Posun, otočenie, mierka, zrkadlenie a odsadenie

## Posun cez základný bod {#move-by-a-base-point}

Najpresnejší spôsob posunu, ako vo väčšine CAD programov.

1. Vyberte, čo chcete posunúť, a stlačte `M` (alebo tlačidlo Posunúť, *Upraviť › Posunúť*, alebo ponuku pravého tlačidla). Ak nie je nič vybrané, Posunúť vás najprv požiada kliknúť na to, čo sa má posunúť.
2. **Základný bod**: kliknite na bod, za ktorý budete posúvať, napríklad roh. Značka prichytenia ukáže, čo zachytíte. Zatiaľ sa nič nehýbe.
3. **Sledovanie**: výber sleduje ukazovateľ, čiarkovaná čiara ukazuje vzdialenosť.
4. **Umiestnenie**: kliknite tam, kam má ísť základný bod. Prichytáva sa k rohom, stredom, priesečníkom a osiam.

Kým sleduje ukazovateľ:

- Zadajte vzdialenosť, napríklad `2400`, `2.4 m` alebo `8'-6"`, a stlačte `Enter`: posunie sa o ňu smerom, kam ukazujete. Samotné číslo je v jednotkách projektu.
- `X`, `Y` alebo `Z` drží danú os; ďalšie stlačenie ju uvoľní. Podržaním `Shift` ho udržíte v pravom uhle.
- `Ctrl` zanechá pri každom kliknutí kópiu.
- `Esc` alebo pravé tlačidlo krok vráti a ďalšie zruší.

**Lišta Posun** v hornej časti pohľadu robí to isté myšou: štítky osí *X*, *Y* a *Z* v ich farbách (červená, zelená a modrá, ako čiary osí v pohľade), odznak **D** pre vzdialenosť, *Kopírovať*, *Dokončiť* a *Zrušiť*. Otočenie a Mierka používajú tú istú lištu. Ak prekáža, potiahnite ju za úchyt; dvojklik ju vráti. Ak ju chcete mať ako malú paletu pri ukazovateli, pozri *Nastavenia › Rozhranie › Lišta Posun, Otočenie a Mierka*.

![Posun priečky cez jej základný bod](img/move.jpg)

Posun je jeden krok späť. Dvere a okná idú so svojou stenou a steny napojené na posunutú stenu sa natiahnu, aby zostali napojené.

## Posun plochy (tlačiť a ťahať) {#move-a-face-push-and-pull}

Stlačením `F` počas Posunu (alebo kliknutím na **Plocha** na paneli Posun) posuniete namiesto celého prvku jednu jeho plochu.

- V 3D ukážte priamo na plochu. V pôdoryse alebo reze ukážte na hranu prvku: zastupuje plochu kolmú na papier.
- Kliknite, potom pohnite ukazovateľom alebo zadajte vzdialenosť. Záporné znamienko plochu vtiahne.

Čo sa zmení, závisí od plochy: koniec steny mení jej dĺžku, vrch výšku a bok hrúbku; vrch dosky jej hrúbku; ostenie dverí ich šírku; bok stĺpa jeho rozmer; plocha hmoty posúva jej body; vytiahnutie nakreslené na ploche sa prehĺbi kolmo na túto plochu z vonkajšej strany alebo rozšíri z boku. Ďalšie `F` vráti posun celých prvkov.

## Uchopenie ukazovateľom: G {#grab-with-the-pointer-g}

S ukazovateľom nad pohľadom `G` uchopí výber a ten hneď sleduje ukazovateľ, od prichyteného bodu pod ukazovateľom. Kliknutie alebo `Enter` dokončí. Klávesy osí, zadané hodnoty a `Esc` fungujú ako pri Posune.

## Ťahanie {#drag}

Stlačte vybraný prvok a ťahajte ho. Bod uchopenia aj miesto dopadu sa prichytávajú. Malý pohyb sa nepočíta, takže kliknutie nikdy nič neposunie. Pozri aj [Úchyty](handles).

## Otočenie: R {#rotate-r}

1. Vyberte, potom kliknite na *Otočiť* alebo stlačte `R` nad pohľadom.
2. Kliknite na **stred**, potom na **referenčný smer**, potom na **nový smer**. Každý sa prichytáva.
3. Alebo zadajte uhol, napríklad `30`, a stlačte `Enter`.

`R`, potom `X` alebo `Y`, nakloní hmotu okolo tejto osi. V reze R nakláňa okolo osi, pozdĺž ktorej sa pozeráte. Steny a dosky zostávajú zvislé.

S pracovnou rovinou na ploche sa Posun posúva po ploche a Otočenie otáča okolo normály plochy, takže diel nakreslený na stene sa otáča na stene.

## Mierka: S {#scale-s}

`S` nad pohľadom mení mierku výberu ukazovateľom alebo zadaným koeficientom. `X`, `Y` alebo `Z` mení mierku iba v jednej osi: stena zmení výšku, doska hrúbku.

## Duplikovať: Shift+D {#duplicate-shift-d}

`Shift+D` skopíruje výber a začne kópiu posúvať.

## Zrkadliť {#mirror}

*Upraviť › Zrkadliť* (alebo ponuka pravého tlačidla) prevráti výber cez čiaru. Kliknite na dva body na čiare. Vytvorí kópiu; `Ctrl` namiesto toho prevráti originál.

- `Shift` drží čiaru v krokoch po 15° od osí a zadaný uhol, potom `Enter`, ju nastaví.
- S pracovnou rovinou na ploche leží čiara na ploche a diel na ploche sa cez ňu prevráti a stále vystupuje z plochy.
- Steny si ponechajú vonkajšiu stranu vonku. Objekty a stĺpy sa posunú a otočia zodpovedajúco, ale ponechajú si vlastný tvar.

## Odsadenie {#offset}

*Upraviť › Odsadiť* vytvorí kópiu v zadanej vzdialenosti na jednu stranu. Pohnite ukazovateľom na požadovanú stranu a kliknite, alebo zadajte vzdialenosť a stlačte `Enter`; `Shift` zaokrúhľuje na 50 mm. `Ctrl` zmení originál namiesto vytvorenia kópie.

Uzavretý obrys (doska, strecha, podhľad, šachta, vytiahnutie) sa zväčší von alebo zmenší dnu. Čiara (stena, nosník, os, deliaca čiara miestností, základový pás, zábradlie) sa posunie do strany. Náčrt, vytiahnutie nakreslené na ploche alebo ťahanie po dráhe sa odsadí vo vlastnej rovine.

## Stred otáčania {#the-pivot}

Otočenie a mierka sa vzťahujú k **stredu otáčania**, ktorý zvolíte v stavovom riadku alebo v *Upraviť › Stred otáčania*: stredný bod, stred ohraničenia, vlastný počiatok každého prvku, aktívny prvok alebo **kurzor stredu otáčania**. `Shift+pravé tlačidlo` kdekoľvek umiestni kurzor a urobí z neho stred otáčania.

## Vytiahnuť: E {#extrude-e}

Vyberte plochu hmoty v 3D alebo uzavretý [náčrt](sketches) a stlačením `E` ho vytiahnete.
