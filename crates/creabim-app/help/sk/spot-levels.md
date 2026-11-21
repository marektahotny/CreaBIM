# Výškové kóty

**Výšková kóta** zapisuje výšku bodu: vrch podlahy, terasy alebo stupňa. V pôdoryse je to aj **značka podlažia**, ktorá uvádza, na akom podlaží pôdorys je.

## Umiestnenie výškovej kóty {#placing-a-spot-level}

1. Zvoľte nástroj **Výšková kóta** (v skupine Anotácia).
2. Kliknite na bod:
   - V reze alebo pohľade kliknite na hranu podlahy, dosky alebo strechy. Na bode stojí malý trojuholník.
   - V pôdoryse kliknite na dosku: výšková kóta uvedie vrch dosky. Kliknutím kdekoľvek inde uvedie vlastné podlažie pôdorysu ako značku podlažia, kreslenú symbolom terča.
3. Kliknite, kam má ísť jej text.

Výšková kóta sleduje to, na čo bola umiestnená. Posuňte dosku alebo zmeňte jej podlažie a výška sa zmení.

## Čo uvádza {#what-it-reads}

Vyberte ju a vo *Vlastnostiach* zadajte vzor do poľa **Číta**. Tieto slová v zložených zátvorkách sa vyplnia:

- `{elevation}`: výška v jednotkách projektu so znamienkom: +450.
- `{m}`: v metroch na dve desatinné miesta: +0,45. `{m3}` dá tri miesta.
- `{mm}`: v milimetroch: +450.
- `{level}`: názov podlažia na úrovni bodu alebo pod ním.

`{m} ČP` uvedie „+0,45 ČP“ a `ČP {m} ({level})` uvedie „ČP +0,45 (Prízemie)“. Nová výšková kóta uvádza to isté ako naposledy vytvorená, takže vzor stačí nastaviť raz.

## Presun a štýl {#moving-and-styling}

Nástrojom Výber potiahnutím textu ju presuniete. Výšková kóta má [štýl](styles) ako kóta. Výškové kóty sú z hľadiska [viditeľnosti](visibility) kóty: pohľad, ktorý skrýva kóty, skrýva aj ich.
