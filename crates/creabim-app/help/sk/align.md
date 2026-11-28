# Zarovnať, Predĺžiť a Natiahnuť

Nástroj **Zarovnať** zarovnáva prvky k referencii, podobne ako Align v Revite. Je vedľa nástroja **Výber** na začiatku radu nástrojov na každej záložke a nájdete ho aj v *Upraviť › Zarovnať k referencii…* a v ponuke pravého tlačidla 2D pohľadu.

Vždy funguje na dve kliknutia: **najprv referencia, potom to, čo sa má zmeniť**. Referencia zostáva vybraná, takže k nej môžete zarovnať viac vecí. `Esc` ju uvoľní.

Režim zvolíte v lište volieb nástroja.

## Zarovnať {#align}

1. Kliknite na referenciu: líce steny, hranu dosky, os, pomocnú čiaru, dočasnú pomocnú čiaru alebo os prvku.
2. Kliknite na hranu toho, čo sa k nej má zarovnať, alebo na jeho os.

Prvok sa natočí rovnobežne s referenciou (kratšou cestou) a prisunie sa k nej. Kliknutie na hranu dverí alebo okna posunie ich stenu.

## Predĺžiť k nemu {#extend-to-it}

Kliknite na referenciu, potom blízko konca steny, nosníka alebo osi: tento koniec sa predĺži až k referencii. Steny na ňu napojené ho nasledujú.

## Natiahnuť k nemu {#stretch-to-it}

Kliknite na referenciu, potom na hranu: k referencii sa posunie iba táto plocha a protiľahlá zostane na mieste. Pri podržaní ukazovateľa sa zvýrazní hrana, ktorú Natiahnuť vezme.

- V reze kliknite na vrch dosky pod čiarou: doska sa k nej zhrubne. Vrch steny mení jej výšku. Vrch stĺpa sa dá natiahnuť až po spodok nosníka.
- V pôdoryse sa koniec steny natiahne ako pri Predĺžiť.

Hrana musí byť rovnobežná s referenciou, s odchýlkou do 5°. Natiahnuť meria od bodu hrany najbližšieho ku kliknutiu.

![Natiahnuť v reze: horná hrana strechy krídla, zvýraznená, pred (hore) a po (dole) stiahnutí na čiaru poschodia](img/stretch.jpg)

## Osi prvkov {#centre-lines}

Prvky ponúkajú aj svoje osi, kreslené fialovo čiarkovane pri podržaní ukazovateľa nad prvkom:

- referenčná čiara steny a os nosníka, naprieč aj v jeho výške;
- obe osi stĺpa;
- stred dosky, hmoty, strechy alebo schodiska, v oboch smeroch;
- stred dverí alebo okna, naprieč stenou a v polovici výšky.

Os môže byť referenciou alebo tým, čo sa zarovnáva: kliknite na os nosníka po osi stĺpa a nosník sa vycentruje na stĺp. Cez svoju os sa dvere alebo okno posúvajú pozdĺž steny namiesto posunu steny. Natiahnuť posúva plochy, preto os ako to, čo sa má zmeniť, neberie.

## V 3D {#in-3d}

Zarovnať, Predĺžiť a Natiahnuť fungujú aj v 3D okne a v 3D pohľadoch.

1. Kliknite na referenciu: plochu, hranu, roh, os prvku, os siete alebo prázdny priestor (rovina pracovného podlažia; v 3D pohľade najnižšie podlažie).
2. Kliknite na plochu toho, čo sa má zmeniť. Pri podržaní ukazovateľa sa zobrazí oranžovo so vzdialenosťou, o ktorú sa posunie.

- **Zarovnať** posunie celý prvok, kým táto plocha nelícuje s referenciou. Zvislé prvky sa k nej najprv natočia.
- **Natiahnuť k nemu** posunie iba túto plochu, ako [Posun plochy](move#move-a-face-push-and-pull): vrch steny, spodok dosky, vrch stĺpa.
- **Predĺžiť k nemu** dovedie bližší koniec steny, nosníka alebo osi k referencii. Pri ostatných prvkoch naťahuje.

`Tab` vyberie, čo je za tým, takže dosiahnete aj plochu ukrytú medzi ostatnými. Referencia zostáva vybraná aj pri prechode medzi 3D oblasťami.

## Kde funguje {#where-it-works}

Pôdorysy, rezy, pohľady, 3D okno a 3D pohľady. Každé kliknutie je jeden krok späť.
