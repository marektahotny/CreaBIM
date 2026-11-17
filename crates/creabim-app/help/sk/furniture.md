# Nábytok a zariaďovacie predmety

Nábytok a zariaďovacie predmety sú objekty, ktoré do budovy umiestňujete, nie staviate: umývadlá, WC, vane, sprchy, kuchynské skrinky, pohovky, stoličky, stoly, postele, šatníky a písacie stoly, ako aj zásuvky, vypínače, svietidlá, vykurovacie telesá a ďalšie [prvky elektroinštalácie, ZTI a VZT](fixtures). Každý objekt je inštanciou **typu objektu**. Typ obsahuje tvar (jeho časti, každú v inom materiáli), kategóriu, triedu IFC, či stojí pri stene, voľne alebo visí zo stropu, a výšku, do ktorej sa nové objekty osadzujú. Samotný objekt pozná len svoju polohu, natočenie a výšku nad podlažím.

Na vyskúšanie otvorte [základný dom](starter-house) alebo *Súbor › Nový zariadený vzorový dom*.

## Okno Katalóg {#the-catalogue-window}

Otvorte ho cez *Vytvoriť › Katalóg…*, nástrojom pohovky na paneli nástrojov alebo **+** pri skupine *Nábytok a zariaďovacie predmety* v prehliadači.

- Hore vyberte kategóriu (Kúpeľňa, Kuchyňa, Obývanie, Stolovanie, Spálňa, Kancelária, Elektroinštalácia, Osvetlenie, Zdravotechnika, Vykurovanie a vetranie, Ostatné) alebo hľadajte podľa názvu.
- **Základné** sú objekty dodávané s CreaBIM.
- **Vaše** sú objekty, ktoré ste importovali alebo uložili. Sú dostupné v každom projekte.
- **V tomto projekte** sú vlastné typy objektov projektu vrátane tých načítaných zo súboru IFC.
- Kliknutím na obrázok objekt umiestnite. Kliknutím pravým tlačidlom na váš objekt ho z katalógu odstránite. Už umiestnené objekty zostanú.

![Okno Katalóg](img/catalogue.jpg)

## Umiestňovanie objektov {#placing-objects}

Po výbere objektu je zapnutý nástroj **Objekt**. Objekt sleduje ukazovateľ v pôdoryse aj v 3D a kliknutie ho umiestni.

- Umývadlá, WC, vane, sprchy, kuchynské skrinky, postele, pohovky a písacie stoly sa **priložia k najbližšej stene** do 600 mm. Natočia sa chrbtom k povrchu steny.
- Voľne stojace objekty, napríklad stoly, stoličky a kreslá, sa umiestnia tam, kam kliknete. Natočia sa podľa volieb nástroja.
- Svietidlá, vyústky a hlásiče dymu visia zo stropu so stredom v mieste kliknutia. Pozri [Prvky elektroinštalácie, ZTI a VZT](fixtures).
- Každý sa osadí do výšky podľa svojho typu: horná kuchynská skrinka na 1450 mm, zásuvka na 300 mm.
- **Medzerník** otočí ďalší objekt o štvrť otáčky. *Otočiť* nastaví ľubovoľný uhol, *Otočiť o 90°* pridá štvrť otáčky a *Bez stien* umiestni aj objekty pri stene tam, kam kliknete.
- Keď nie je nič vybrané, karta **Ďalší prvok** vo *Vlastnostiach* zobrazuje objekt na umiestnenie: jeho rozmery, spôsob osadenia (*na stene*, *na podlahe*, *zavesený zo stropu*), výšku **Nad podlahou** pred umiestnením a tlačidlá *Vybrať objekt…* a *Otočiť o 90°*.
- Tieň objektu ukazuje v pôdoryse aj v 3D, kam ho kliknutie umiestni.
- Ďalšími kliknutiami umiestňujete ďalšie; **Esc** ukončí.

![Umývadlo hľadá stenu, ku ktorej sa priloží](img/object-against-wall.jpg)

Pôdorysy kreslia pôdorysnú značku každého objektu. Horná skrinka nad výškou rezu sa kreslí skrytými čiarami. Rezy, pohľady a 3D zobrazujú jeho úplný tvar v jeho materiáloch.

## Zmena objektu {#changing-an-object}

Vyberte ho. Vo *Vlastnostiach*:

- **Nad svojím podlažím** ho zdvihne, napríklad hornú skrinku alebo závesné umývadlo. Pri objekte zavesenom zo stropu je tu **Pod stropom**. **Otočenie** ho otočí a **↺ 90°** pridá štvrť otáčky. V pôdoryse ho okrúhly úchop so zahnutou šípkou pred objektom otáča ukazovateľom po 5° (`Alt` pre ľubovoľný uhol).
- Výber **Typ** ho zamení za iný typ objektu, takže zo stoličky sa môže stať kreslo. Objekt má typ vždy.
- Pod nimi je **kategória** typu, spôsob **osadenia** (*Pri stene*, *Voľne stojaci* alebo *Zo stropu*), výška, do ktorej sa osadzujú **nové**, jeho **trieda IFC a preddefinovaný typ** a **materiál každej časti**. Ich zmena zmení všetky objekty daného typu.

Objekty presúvajte, otáčajte a duplikujte ako ktorýkoľvek prvok. Objekt si zachováva tvar svojho typu, takže režim plôch nástroja Presunúť a *Natiahnuť* v [Zarovnaní](align) presúvajú celý objekt. Vytlačenie bočnej plochy ho posunie a vytlačenie vrchu alebo spodku ho zdvihne alebo zníži. Zarovnanie chrbta pohovky so stenou ju teda priloží k stene.

Typy objektov sú v okne [Typy](types) v časti *Objekty*. Odstránenie typu odstráni aj jeho objekty a Ctrl+Z ich vráti.

## Vlastné objekty {#your-own-objects}

- **Z Blendera, FreeCADu alebo glTF:** v okne Katalóg *Importovať z Blendera, FreeCADu alebo glTF…* načíta súbor `.blend`, `.FCStd`, `.gltf` alebo `.glb` ako jeden objekt. Každá sieť sa stane časťou vo vlastnom materiáli. Objekt sa postaví na podlahu so stredom zadnej hrany v počiatku, preto ho modelujte otočený k −Y (predný pohľad v Blenderi). Jeho kategória, trieda IFC a osadenie sa odhadnú z názvov súboru a sietí, napríklad „sink“, „toilet“, „sofa“, „bed“ alebo „desk“. Prednosť má vlastná vlastnosť `ifc_class` v Blenderi alebo typ IFC vo FreeCADe. Po umiestnení odhad skontrolujte vo Vlastnostiach.
- **Z modelu:** vyberte objekt, niekoľko objemov alebo ľubovoľné prvky, zadajte názov a kliknite na *Uložiť výber do katalógu*. Objekt uloží svoj typ; čokoľvek iné sa uloží ako 3D tvar vo svojich farbách.

Váš katalóg je priečinok súborov `.json`, jeden na objekt, v `CreaBIM/catalogue` v priečinku nastavení (`%APPDATA%` vo Windows, inde `~/.config`). Na zdieľanie objektov ich skopírujte. Súbor projektu nesie typy umiestnených objektov, takže sa otvorí rovnako na každom počítači.

Z terminálu urobí rovnaký import `creabim catalogue-import chair.blend <priečinok>` a `creabim catalogue` vypíše základné objekty.

## IFC

Objekty sa exportujú ako IfcFurniture, IfcSanitaryTerminal, IfcElectricAppliance alebo IfcSystemFurnitureElement (zariaďovacie predmety ako IfcOutlet, IfcLightFixture, IfcSpaceHeater a ďalšie uvedené v [prvkoch TZB](fixtures)), s typmi IfcFurnitureType, IfcSanitaryTerminalType a pod. a preddefinovanými typmi ako SOFA, BED, WASHHANDBASIN, TOILETPAN, SINK alebo FRIDGE_FREEZER. Každý typ nesie svoj tvar raz, ako mapu reprezentácie *Body* s každou časťou v jej materiáli, a pôdorysnú značku ako mapu *FootPrint*. Každý objekt na tieto mapy odkazuje. Otvorenie takého súboru vráti typy aj objekty. Pozri [IFC](ifc).
