# Materiály

**Materiál** je to, z čoho je prvok vyrobený: betón, tehla, drevo, sklo. Má farbu, **šrafu v reze** (vzor kreslený tam, kde ho prechádza rez) a povrch (ako drsne, kovovo alebo priehľadne vyzerá v 3D). Materiály sa exportujú do IFC spolu s prvkami.

## Knižnica materiálov {#the-material-library}

CreaBIM obsahuje 52 bežných stavebných materiálov v 17 kategóriách. *Materiály › Pridať z knižnice* pridá jeden alebo celú kategóriu tlačidlom *Pridať všetky*. Karta *Knižnica* v prehliadači materiálov ich zobrazuje s popismi.

## Prehliadač materiálov {#the-material-browser}

*Materiály › Prehliadač materiálov…* vypíše materiály projektu podľa kategórií s počtom prvkov, ktoré ich používajú.

- Materiály môžete pridávať, duplikovať, premenovať a upravovať so živým náhľadom na guli.
- *Vybrať ich* vyberie prvky, ktoré materiál používajú; *Priradiť výberu* ho priradí výberu.
- *Odstrániť* sa spýta, z čoho majú byť prvky, ktoré ho ešte používajú.
- *Sadzba* mu priradí cenu za m², m³, m alebo kus pre [výkaz výmer](take-off).

![Prehliadač materiálov](img/materials.jpg)

Výber materiálu v prehliadači projektu zobrazí ten istý editor vo *Vlastnostiach*.

## Priradenie materiálu prvku {#giving-an-element-a-material}

Vyberte prvok a potom vyberte materiál vo *Vlastnostiach* alebo použite *Materiály › Priradiť materiál*. Funguje to pre steny, dosky, strechy, objemy, dvere, okná, stĺpy, nosníky, schodiská a zábradlia. Vrstvené steny, podlahy a strechy dostávajú materiály zo svojej [skladby](assemblies).

## Import materiálov {#importing-materials}

*Materiály › Importovať materiály…* načíta materiály zo súborov IFC, kariet materiálov FreeCADu (`.FCMat`), súborov glTF (`.gltf`, `.glb`) a súborov Blendera (`.blend`). Materiál, ktorého názov už projekt má, sa aktualizuje a jeho prvky ho nasledujú.
