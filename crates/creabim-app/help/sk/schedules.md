# Výkazy

**Výkaz** je tabuľka prvkov vytvorená z modelu: všetky dvere s označením, rozmerom a smerom otvárania; všetky steny s typom, dĺžkou a plochou. Ako ostatné pohľady je vždy aktuálny.

## Vytvorenie {#making-one}

*Súbor › Nový výkaz* (alebo *Vytvoriť › Výkaz*), potom *Dvere*, *Okná*, *Steny*, *Dosky*, *Stĺpy a nosníky*, *Nábytok*, *Zariaďovacie predmety*, *Miestnosti* alebo *Výkaz výmer* (pozri [Výkaz výmer](take-off)). Otvorí sa v 2D paneli a zobrazí sa v časti *Výkazy* v prehliadači.

## Nábytok a zariaďovacie predmety {#furniture-and-fixtures}

Výkazy *Nábytok* a *Zariaďovacie predmety* [objekty](furniture) spočítavajú, nie vypisujú. Každý riadok je jeden typ na jednom podlaží: jeho označenie (ako ho uvádzajú [značky](tags)), typ, podlažie, výška osadenia a trieda IFC a ich počet. Riadky sú zoskupené pod nadpisom pre každú kategóriu, napríklad *Elektroinštalácia*, *Osvetlenie* alebo *Kúpeľňa*, s počtom v kategórii. Súčet je dole.

- **Nábytok** je všetko, čo IFC považuje za nábytok: pohovky, postele, stoly, stoličky, šatníky, kuchynské skrinky.
- **Zariaďovacie predmety** sú všetko ostatné: sanitárna keramika, spotrebiče a [prvky elektroinštalácie, ZTI a VZT](fixtures).

Kliknutím na označenie v riadku vyberiete všetky objekty daného typu. Zaškrtnutím *Zoskupiť podľa typu* sa namiesto toho každý objekt vypíše v samostatnom riadku pod svojím typom, s podlažím a výškou.

## Práca s výkazom {#using-it}

- **Úpravy v tabuľke**: zmeňte šírku okna alebo materiál steny priamo v bunke. Model, pôdorysy aj 3D pohľad sa zmenia s ňou.
- Kliknutím na označenie v riadku vyberiete daný prvok.
- Súčty sú dole.
- Zaškrtnite *Zoskupiť podľa typu* vo *Vlastnostiach* výkazu a pre každý [typ](types) pribudne riadok nadpisu s počtom. Kliknutím na nadpis vyberiete všetky jeho riadky.
- **Exportovať CSV** a **Exportovať Excel** v hlavičke výkazu: CSV uloží túto tabuľku pre tabuľkový procesor; Excel uloží všetky výkazy do jedného zošita, s číslami ako číslami. Pozri [Výkaz výmer](take-off#to-excel).
- Vybraný riadok je označený jemným tónom a farebným pruhom, na svetlom aj tmavom papieri.

Výkaz umiestnite na list ako ktorýkoľvek pohľad. Pozri [Listy](sheets).
