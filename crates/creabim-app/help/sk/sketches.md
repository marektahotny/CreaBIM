# Náčrty a pracovná rovina

**Náčrt** je rovinná kresba z čiar, oblúkov a kružníc na rovine. Uzavretý náčrt možno vytiahnuť do objemu alebo použiť ako obrys dosky, strechy alebo šachty.

## Kreslenie náčrtu {#drawing-a-sketch}

1. Zvoľte nástroj **Náčrt**.
2. V 3D kliknite na plochu, na ktorej chcete kresliť (alebo na zem). V pôdoryse alebo reze kliknite do výkresu.
3. Klikajte na rohy. `L`, `A`, `C` a `R` prepínajú medzi čiarami, oblúkmi (kliknite na koniec, potom na bod, cez ktorý prechádza), kružnicami (kliknite na stred, potom na bod na nej) a obdĺžnikmi (dva protiľahlé rohy, kolmo k rovine náčrtu, počas kreslenia sa zobrazuje šírka × výška). Kružnica alebo obdĺžnik je samostatný náčrt.
4. Kliknutím na prvý roh náčrt uzavriete, `Enter` alebo pravým tlačidlom ho necháte otvorený. `Esc` zruší.

`Shift` drží čiary v pravom uhle.

## Väzby {#constraints}

Vyberte náčrt a vo *Vlastnostiach* pridajte väzby: dĺžka, rovnaké, vodorovná, zvislá, rovnobežná, kolmá, totožné a polomer. Náčrt sa prispôsobí, aby ich splnil, a hodnotu väzby môžete zmeniť potiahnutím.

## Vytiahnutie {#extruding}

**Uzavretý** náčrt je plocha: má vnútri obrysu jemnú výplň v 3D, pôdorysoch aj rezoch a *Vlastnosti* uvádzajú *Uzavretý: plocha. Vytiahnutím vznikne teleso.* **Otvorený** náčrt je drôt a vytiahne sa do plochy. Uzavretý náčrt, ktorý sám seba pretína, nemá vnútro: nemá výplň a *Vytiahnuť do objemu* je nedostupné, kým sa čiary nepretínajú.

Vyberte uzavretý náčrt a stlačte `E`: stane sa objemom vytiahnutým o *Vytiahnuť o* vo *Vlastnostiach*. Otvorený náčrt sa vytiahne do plochy, ktorú môže zhrubnúť [modifikátor](masses#modifiers) Zhrubnutie.

## Pracovná rovina {#the-workplane}

**Pracovná rovina** je rovina, na ktorú sa kreslia náčrty a pomocné čiary v 3D. Vyberte ju v zozname *Pracovná rovina* v stavovom riadku:

- *Podlažie (podľa pôdorysu)*: pracovné podlažie, bežná voľba;
- *Vybrať plochu…*: kliknite na plochu v 3D;
- *Cez tri body…*: kliknite na tri body;
- *Vodorovná rovina s odsadením*: vodorovná rovina, ktorú posúvate hore a dolu.

Alebo kliknite pravým tlačidlom na plochu a zvoľte *Nastaviť pracovnú rovinu sem*.
