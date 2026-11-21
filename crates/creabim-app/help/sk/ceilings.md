# Podhľady

**Podhľad** je tenká vrstva zavesená pod stropom alebo strechou nad ňou: sadrokartón alebo kazetový podhľad. Zobrazuje sa v rezoch a v 3D, má vlastný **pôdorys podhľadov**, určuje výšku miestností pod ním, visia z neho stropné svietidlá a môžu pri ňom končiť priečky.

## Umiestňovanie podhľadov {#placing-ceilings}

1. Zvoľte nástroj **Podhľad** (v *Vytvoriť › Stavebné prvky*, za Deliacou čiarou miestností).
2. Kliknite do miestnosti. Podhľad ju prekryje a **sleduje miestnosť**: keď sa steny posunú, obrys podhľadu sa zmení s nimi.

Kliknutie medzi steny tam, kde ešte nie je miestnosť, vytvorí podhľad s obrysom tamojšieho priestoru, ktorý steny nesleduje. Pre podhľad, ktorý ich sleduje, najprv umiestnite miestnosť.

Ako miestnosti a dosky vie nástroj aj nakresliť obrys podhľadu: **Mnohouholník (1)**, **Obdĺžnik (2)** alebo **Z náčrtu (4)**. Nakreslený podhľad použite pre zníženie (sadrokartónový kastlík) alebo podhľad len nad časťou miestnosti.

## Výška, hrúbka a rastr {#height-thickness-and-grid}

*Vlastnosti* obsahujú pri podhľade:

- **Výška**: jeho spodku nad podlažím. Nové podhľady visia vo výške 2500, alebo nižšie v podlaží, ktoré je na to príliš nízke.
- **Hrúbka**: smerom nahor od spodku; 12,5 pre sadrokartón.
- **Kazetový rastr**: zaškrtnite pre kazetový podhľad, s rozmerom **Kazety** (predvolene 600). Rastr sa kreslí v pôdorysoch podhľadov.
- **Materiál**, ako pri ktoromkoľvek inom prvku. Pozri [Materiály](materials).

Miestnosť pod podhľadom preberá jeho výšku, takže výkaz miestností aj výška IfcSpace sa riadia podhľadom.

## Pôdorysy podhľadov {#reflected-ceiling-plans}

Vo *Vlastnostiach* pôdorysu zaškrtnite **Pôdorys podhľadov** (alebo **Podhľady** vo voľbách pohľadu). Pôdorys potom zobrazí podhľady s rastrom a to, čo z nich visí (svietidlá a ďalšie stropné prvky), so stenami v reze ako obvykle. Nábytok, podlahy, schodiská a otváranie dverí sa vynechajú. Ak chcete mať oba, najprv pôdorys duplikujte.

## Steny končiace pri podhľade {#walls-that-stop-at-the-ceiling}

Priečka môže siahať po spodok podhľadu namiesto podlažia alebo strechy. Vyberte stenu a vo voľbách jej *Vrchu* vyberte podhľad, rovnako ako sa stena pripája k doske alebo streche (pozri [Steny](walls)). Stena potom sleduje podhľad, keď sa zmení jeho výška. V základnom dome končia priečky prvého poschodia pri podhľadoch spálne a kúpeľne.

## Prvky, ktoré z neho visia {#fixtures-that-hang-from-it}

Stropné svietidlo, hlásič dymu alebo odsávací ventilátor umiestnený nástrojom Objekt visí z podhľadu nad bodom, na ktorý kliknete, ak tam je. Ak nie je, visí z dosky alebo strechy nad ním, alebo vo výške podlažia. Pozri [Prvky elektroinštalácie, ZTI a VZT](fixtures).

## Vo výkresoch a IFC {#in-drawings-and-ifc}

Rezy kreslia podhľady ako tenké čiary v ich výške; 3D ich kreslí takmer bielou. Na ich skrytie použite kategóriu *Podhľady* vo [Viditeľnosti](visibility).

Podhľady sa exportujú ako **IfcCovering** s preddefinovaným typom *CEILING*, prepojené s priestorom pod nimi cez **IfcRelCoversSpaces**. Súbor vlastností *CreaBIM_Ceiling* uchováva rastr a miestnosť, takže po opätovnom otvorení súboru je podhľad opäť nad svojou miestnosťou. Pozri [IFC](ifc).

**Pozri aj:** [Miestnosti](rooms)
