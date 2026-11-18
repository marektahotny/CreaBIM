# Objemy, modifikátory a booleovské operácie

**Objem** je voľný 3D tvar, ktorý môžete modelovať ako v Blenderi: na skoré objemové štúdie alebo na čokoľvek, čo nie je stena, doska ani strecha.

## Vytvorenie objemu {#making-a-mass}

- Zvoľte nástroj **Objem** a kliknutím umiestnite kváder.
- Alebo vytiahnite uzavretý [náčrt](sketches) klávesom `E`.

## Úprava tvaru {#editing-its-shape}

- Kliknutím na plochu v 3D pohľade ju vyberte, potom ju `E` vytiahnite alebo `G` presuňte.
- Pomocou [Presunúť plochu](move#move-a-face-push-and-pull) plochu vytlačte alebo vtiahnite o zadanú vzdialenosť.
- Otáčajte, nakláňajte a mierkujte ho klávesmi `R` a `S` (pozri [Presunúť, otočiť a zmeniť mierku](move)).

Vybraný objem zobrazuje len hlavné úchopy: základný bod (presúva ho), úchop otáčania a šípku vytlačenia na kliknutej ploche. Šípky všetkých plôch, rohy, hrany a šípka zdvihnutia sú v [režime úprav](edit-mode) (`Tab`).

*Vlastnosti* zobrazujú jeho **Polohu**, **Otočenie (XYZ Euler)** a **Mierku** ako panel Transform v Blenderi: objem sa otáča okolo X, Y a Z, takže tvar nakreslený na stene sa otáča okolo osí steny.

Priraďte objemu triedu IFC vo *Vlastnostiach* (napríklad `IfcRoof`) a exportuje sa ako táto trieda.

## Úpravy siete {#mesh-edit}

Časť *Úpravy siete* vo *Vlastnostiach* pracuje s plochou, na ktorú ste klikli: **Vytiahnuť** ju vysunie s novými bočnými plochami (`E`), **Presunúť** ju posunie, **Zúžiť** ju zmenší a **Vsadiť** nakreslí menšiu plochu vnútri jej hrán vo vzdialenosti *Vsadiť o*. Vsadením vytvoríte ostenie, zapustený panel alebo atiku: vsaďte plochu strechy a potom ju stlačte nadol. Tieto úpravy menia sieť pod jej modifikátormi.

## Modifikátory {#modifiers}

Modifikátory menia tvar objemu bez toho, aby sa ho dotkli, takže ich môžete neskôr zmeniť alebo odstrániť. Vyberte objem, otvorte kartu **Modifikátory** vo *Vlastnostiach* (kľúč) a potom **Pridať modifikátor**. Ponuka ich zoskupuje do častí *Opakovanie*, *Fasáda*, *Tvar* a *Deformácia*, každý s ikonou; po prejdení myšou sa zobrazí, na čo slúži:

| Modifikátor | Čo robí |
| --- | --- |
| **Pole** | Kópie v rade (krokvy pergoly): počtom alebo na zadanú dĺžku; každá kópia sa môže posunúť o vlastnú veľkosť, otočiť a zväčšiť |
| **Kruhové pole** | Kópie okolo stredu vybraného v pôdoryse alebo v jeho strede: stoličky okolo stola, stĺpy okolo rotundy |
| **Pole po krivke** | Kópie pozdĺž náčrtu alebo pomocnej čiary: baluster, stĺpiky pozdĺž oblúka |
| **Zrkadlenie** | Zrkadlová kópia s rovinou v strede alebo na hrane |
| **Skosenie** | Zaoblí alebo zreže zvislé rohy: všetky, vonkajšie alebo vnútorné |
| **Škrupina** | Vyduté na zadanú hrúbku, otvorené hore, dole alebo z oboch strán |
| **Zhrubnutie** | Dá ploche hrúbku smerom dnu alebo von |
| **Rozdeliť podľa podlaží** | Podlažia vyrezané z objemu s medzerou medzi nimi: rýchla štúdia veže |
| **Panely** | Fasádne panely so škárami na zvislých obdĺžnikových plochách |
| **Lamely / rebrá** | Lamely na stranách obrátených na sever, východ, juh alebo západ, pod uhlom |
| **Zúženie** a **Skrútenie** | Zužuje ho smerom nahor alebo ho otáča, ako stúpa |
| **Vyhladenie** | Vyhladí ho; druh *Rozdelenie* zachová tvar a len pridá plochy |

![Rozdelenie objemu podľa podlaží: podlažia a medzery medzi nimi s kartou na karte Modifikátory](img/modifiers.jpg)

Každý modifikátor je karta ako v Blenderi. Jeho ikona ho zapína a vypína a zbalený zobrazuje jednoriadkový súhrn. Môžete ho premenovať a v ponuke `…` sú **Použiť** (stane sa súčasťou tvaru), **Duplikovať**, **Posunúť vyššie** a **Posunúť nižšie**; krížik ho odstráni. Rozbalený zobrazuje nastavenia v riadkoch a `?` na karte ho vysvetľuje. Karta, s ktorou pracujete, zobrazuje pomocné prvky vo všetkých pohľadoch: stred a kružnicu kruhového poľa, rovinu zrkadlenia, roviny podlaží ako priehľadné plochy. Karta *Prvok* uvádza, koľko modifikátorov objem má.

## Booleovské operácie: vyrezanie jedného tvaru z druhého {#booleans-cutting-one-shape-from-another}

Vyberte stenu, dosku alebo objem, potom v časti *Booleovské operácie* zvoľte *Odčítať…* a prvok, ktorý sa má vyrezať. Rezný prvok zostane ako priehľadný tieň, ktorý môžete vybrať a presunúť a tým zmeniť výrez. Prepnite ho na *Zjednotenie* alebo *Prienik*, alebo ho *Odstráňte*.
