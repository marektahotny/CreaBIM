# Označenia a číslovanie

**Označenie** je krátky kód, ktorý identifikuje dvere, okno alebo iný prvok vo výkresoch a výkazoch: D01, W-101, C3. Zobrazujú ho jeho [značky](tags) aj [výkazy](schedules), takže sa vždy zhodujú.

## Zadanie označenia {#typing-a-mark}

Vyberte prvok a zadajte **Označenie** vo *Vlastnostiach* pod jeho názvom. Ak pole necháte prázdne, prvok dostane číslo podľa pravidiel projektu; prázdne pole ho zobrazuje sivo.

Ak už iný prvok má rovnaké označenie, *Vlastnosti* to uvedú pod poľom. **Vybrať ich** vyberie ostatné.

## Pravidlá číslovania {#numbering-rules}

**Súbor › Číslovanie označení…** nastavuje, ako sa číslujú prvky bez zadaného označenia. Každý druh prvku má vlastné pravidlo:

- **Číslovať**: číslovať tento druh automaticky. Keď je vypnuté, zobrazujú sa len zadané označenia a značka zobrazuje to, čo predtým (napríklad materiál a hrúbku steny).
- **Predpona** a **Odd.**: písmená pred číslom a čo je medzi nimi: `D` a nič dá D01, `D` a `-` dá D-01.
- **Číslice**: počet číslic s úvodnými nulami.
- **Podľa podlažia**: číslo podlažia pred poradovým číslom a číslovanie na každom podlaží od začiatku. Podlažie 1, dvere 1 je D-101; podlažie 2, dvere 3 je D-203.
- **Poradie**: *Poradie v modeli* čísluje v poradí vytvorenia prvkov, od najnižšieho podlažia. *V smere hodinových ručičiek po podlažiach* ide okolo každého podlažia od hornej strany pôdorysu.
- **Od**: prvé číslo.

**Číslo prízemia** je číslo najnižšieho podlažia na úrovni ±0 alebo nad ňou. Podlažia nad ním sa číslujú nahor a podlažia pod ním nadol, takže úroveň terénu pod prízemím je podlažie 0.

Tlačidlá **D01, W01** a **D-101, W-101** nastavia dve obvyklé schémy. Pravidlá patria projektu. **Uložiť pravidlá…** ich zapíše do súboru a **Načítať pravidlá…** ho načíta, takže kancelária môže používať rovnaké číslovanie v každom projekte.

## Prečíslovanie {#renumbering}

Pravidlá číslujú prvky živo: pridajte dvere a dvere za nimi sa posunú o jedno. Keď sú čísla ustálené, **Prečíslovať** ich zapíše do prvkov ako ich označenia, takže neskoršie zmeny ich nechajú tak a nové prvky dostanú ďalšie voľné čísla. Zvoľte jeden druh alebo všetky. S voľbou **Nahradiť aj zadané označenia** sa každý prvok očísluje nanovo podľa pravidiel.

**Označenia z názvov** sú pre modely, ktorých označenia boli zapísané v názvoch, napríklad „D-101 posuvné dvere“: každý prvok dostane ako označenie kód na začiatku svojho názvu.

## Kontrola duplicít {#checking-for-duplicates}

Spodná časť dialógu vypisuje každé označenie, ktoré má viac ako jeden prvok, s tlačidlom **Vybrať** pri každom. Objekty jedného typu zdieľajú označenie svojho typu zámerne, takže sa nepočítajú dvakrát.

## V IFC {#in-ifc}

Označenie sa zapíše ako **Tag** prvku v IFC a pri importe súboru IFC sa z neho načíta späť.
