# Blender

CreaBIM spolupracuje s Blenderom oboma smermi: modelujte voľne v Blenderi a model načítajte, alebo pošlite model CreaBIM do Blendera na vizualizáciu.

## Z Blendera: prepojenie súboru .blend {#from-blender-link-a-blend-file}

1. *Súbor › Importovať a prepojiť…* a vyberte súbor `.blend`.
2. Jeho objekty sa načítajú s materiálmi z Blendera.

Aby sa objekty načítali ako skutočné prvky BIM:

- umiestnite ich do kolekcie s názvom *Walls*, *Slabs*, *Doors* alebo *Windows*;
- alebo dajte objektu vlastnosť `creabim_kind` alebo vlastnosť `ifc_class`, napríklad `IfcColumn`;
- alebo v CreaBIM vyberte objekt a použite *Priradiť ako…* vo *Vlastnostiach* (napríklad *Priradiť ako… › Doska*). Dostane úchopy dosky, hrúbku a materiál a zostane prepojený.

So zaškrtnutým *Súbor › Znova načítať prepojené súbory po uložení* uloženie v Blenderi aktualizuje CreaBIM do sekundy. Steny zostanú stenami a dosky doskami.

## Do Blendera: glTF {#to-blender-gltf}

*Súbor › glTF (.glb) pre Blender* zapíše každý prvok ako sieť s identifikátorom CreaBIM a materiálmi. V Blenderi použite *File › Import › glTF* alebo doplnok **CreaBIM Link** (v `tools/blender-addon` v zdrojovom kóde CreaBIM), ktorý posiela aj vaše úpravy z Blendera späť do tých istých prvkov. Pozri [glTF](gltf).

## Materiály {#materials}

*Materiály › Importovať materiály…* načíta materiály súboru `.blend` (farbu, drsnosť, kovovosť a priehľadnosť Principled BSDF).
