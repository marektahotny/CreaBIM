# IFC

**IFC** (Industry Foundation Classes) je otvorený formát súborov, ktorým si programy BIM vymieňajú modely. Súbor IFC uvádza nielen aké tvary obsahuje, ale aj čo sú: toto je stena na prízemí, z tehly, s týmito dverami. CreaBIM používa pre svoje prvky identifikátory IFC, takže model, ktorý odíde a vráti sa, si zachová svoju identitu.

## Export {#exporting}

- *Súbor › IFC4 (.ifc)*: obvyklý súbor IFC pre Revit, ArchiCAD, Vectorworks, Bonsai (BlenderBIM), FreeCAD, Solibri a prehliadače IFC.
- *Súbor › IFC5 (.ifcx, alpha)*: nové IFC založené na JSON. Jeho schéma je stále vo verzii alpha, takže iné nástroje ho ešte nemusia vedieť načítať.

Čo sa exportuje: podlažia, steny s osou umiestnenia a vrstvami, dosky, strechy, dvere a okná so smerom a otváraním, otvory a šachty, stĺpy, nosníky, schodiská, zábradlia, závesné steny so stĺpikmi a panelmi, typy, materiály s farbami, nábytok a zariaďovacie predmety s typmi a objemy s triedou IFC. Pomocné čiary, náčrty a orezové kvádre sa neexportujú.

### Ako sa exportujú steny, dosky a strechy {#how-walls-slabs-and-roofs-go-out}

Pred zápisom sa export spýta, ako exportovať steny, dosky a strechy, a uvedie, čo pri každej voľbe uvidí druhý program. Voľba sa zapamätá.

| Voľba | Druhý program dostane |
| --- | --- |
| **Ako modelované prvky** (odporúčané) | Každý jeden IfcWall alebo IfcSlab so súborom vrstiev a jednoduchým telesom. Do CreaBIM sa vráti tak, ako odišiel. |
| **Spojené tvary, ako sú nakreslené** | Ich telesá orezané tam, kde steny prechádzajú doskami, ako ich zobrazuje CreaBIM |
| **Vrstvy ako samostatné prvky** | Nosná časť zostane IfcWall alebo IfcSlab; povrchové úpravy, tepelné izolácie a fólie sa stanú prvkami IfcCovering (obklad, podlaha, krytina), ktoré ju pokrývajú. Pre profesie a výmery po vrstvách. |

Názov stavby, pozemku a budovy zo *Správa › Údaje o stavbe* pomenuje projekt, pozemok a budovu v IFC.

## Otvorenie súboru IFC {#opening-an-ifc-file}

*Súbor › Otvoriť…* načíta súbory IFC2X3, IFC4 a IFC4X3 a `.ifcx`.

- **Steny** sa stanú natívnymi upraviteľnými stenami, keď je ich teleso priamy zvislý hranol, či už ho súbor zapisuje ako vytiahnutie alebo ako fazetovanú B-rep či sadu plôch (ako FreeCAD a mnohé ďalšie nástroje). B-rep stena už môže mať vyrezané otvory dverí a okien: načítajú sa späť ako otvory. Stena nakreslená okolo miestnosti po osi z lomenej čiary, prípadne s jedným vytiahnutím na vrstvu, sa stane jednou stenou na každý priamy úsek, so stretnutím v rohoch.
- **Dosky** sa stanú natívnymi, keď sú rovné zvislé hranoly, s otvormi a s vrchom v akejkoľvek výške: mimo podlažia leží doska na podlaží pod ňou s odsadením vrchu.
- **Dvere a okná** sa stanú natívnymi v natívnej stene: cez vzťahy otvorov v súbore (`IfcRelVoidsElement`, `IfcRelFillsElement`), alebo ak ich súbor nemá, v stene, v ktorej leží ich teleso. Prevezmú rozmer otvoru vyrezaného v stene, ak existuje, takže sa zobrazia vo výkazoch dverí a okien a dostanú označenia. Otvor, ktorý nič nevypĺňa, sa stane otvorom v stene.
- **Stĺpy a nosníky** sa stanú natívnymi, keď sú priame hranoly: zvislý s obdĺžnikovým alebo kruhovým prierezom, vodorovný prút s obdĺžnikovým prierezom. Ich typy sa vrátia. Stĺpy, nosníky, strechy, schodiská a zábradlia zapísané CreaBIM sa vrátia presne tak, ako boli.
- Vrátia sa aj typy, skladby, materiály a osové systémy. Nábytok a zariaďovacie predmety s IfcFurnitureType, IfcSanitaryTerminalType, IfcElectricApplianceType alebo IfcSystemFurnitureElementType, ktoré majú mapu telesa, sa načítajú ako [objekty](furniture) daného typu.
- Všetko ostatné s tvarom (štítová stena, šikmá strešná doska, schodiská a zábradlia z iných nástrojov, terén…) sa načíta ako sieť, ktorá si ponechá triedu IFC. Otvory v sieti sa do nej vyrežú a jej rohy sa spoja, takže je uzavretá.
- Jednotky, umiestnenia a identifikátory IFC sa zachovajú a pre každé podlažie sa vytvorí pôdorys.

## Správa o importe {#the-import-report}

Keď má import čo oznámiť, otvorí sa **Správa o importe**: koľko prvkov každej triedy IFC sa načítalo natívne a koľko ako siete, poznámky zoskupené podľa druhu s počtom a pod nimi každý prvok, ktorý sa nedal načítať natívne, podľa triedy, názvu a GlobalId, s dôvodom, napríklad:

> IfcWall 'Wall-Gable-E' (0NBfRAfdjALxRLSMXhQcSA) is an IfcFacetedBrep that has 2 sloped faces (a raked or gable top, or a battered face); imported as a mesh

*Kopírovať správu* ju celú skopíruje ako text na opravu modelu v programe, ktorý ho zapísal, alebo na odoslanie s otázkou. *Súbor › Správa o importe* ju otvorí znova. `creabim import-ifc` vypíše tie isté poznámky a zoznam.

## Len materiály {#materials-only}

*Materiály › Importovať materiály…* načíta len materiály súboru IFC s ich kategóriami a povrchovými štýlmi.
