# Otváranie a ukladanie

## Súbor projektu {#the-project-file}

Projekt CreaBIM je súbor `.creabim`. Obsahuje model, jeho pohľady, listy, štýly a šablóny a **históriu krokov späť**: krok späť môžete urobiť aj po zatvorení a opätovnom otvorení projektu.

- `Ctrl+S` alebo *Súbor › Uložiť* uloží. *Uložiť ako…* uloží pod iným názvom.
- `Ctrl+O` alebo *Súbor › Otvoriť…* otvorí súbor `.creabim`, `.ifc` alebo `.ifcx`.
- *Súbor › Otvoriť nedávne* vypisuje posledných desať projektov.
- Súbor pustený na okno sa otvorí.

Exporty sa zapisujú vedľa súboru projektu, s jeho názvom.

Kým má projekt neuložené zmeny, vedľa jeho názvu v hornej lište je bodka a názov okna začína znakom •. Zatvorenie CreaBIM sa potom najprv spýta: **Uložiť**, **Neukladať** alebo **Zrušiť**.

## Záznam činnosti {#the-activity-log}

CreaBIM vedie obyčajný textový záznam toho, čo ste v tejto relácii robili: každé ťahanie snímku po snímke, každú úpravu a krok späť, každú zmenu nástroja a čo sa pokazilo. Nič neopúšťa váš počítač.

Na jeho čítanie vyberte **Záznam** v ponuke druhu pohľadu oblasti. Pohľad Záznam zobrazuje záznam tak, ako sa zapisuje, s problémami červenou. **Len problémy** ponechá len pády, chyby a to, čo sa nedalo urobiť; **Kopírovať** skopíruje zobrazené na vloženie do správy; **Otvoriť priečinok** ukáže, kde sa uchováva celý záznam.

## Nahlásenie problému {#report-a-problem}

Keď sa niečo pokazí, *Pomocník › Nahlásiť problém…* (alebo tlačidlo v pohľade Záznam) uloží **jeden súbor**, ktorý priložíte k hláseniu chyby alebo e-mailu: súhrn verzie, vášho systému a toho, čo je otvorené, celý záznam činnosti a vaše nastavenia a súčasný projekt, len ak súhlasíte. CreaBIM nič nikam neposiela: kam súbor pôjde, rozhodujete vy.

## Prepojené súbory {#linked-files}

*Súbor › Importovať a prepojiť…* prevezme objekty súboru Blendera (`.blend`) alebo FreeCADu (`.FCStd`). Zostanú prepojené: so zaškrtnutým *Znova načítať prepojené súbory po uložení* ich uloženie súboru v Blenderi alebo FreeCADe tu aktualizuje do sekundy. *Znova načítať prepojený súbor* to urobí ručne. Pozri [Blender](blender) a [FreeCAD](freecad).

## Výmenné formáty {#exchange-formats}

| Formát | Čítanie | Zápis | Stránka |
| --- | --- | --- | --- |
| IFC2X3, IFC4, IFC4X3 (`.ifc`) | áno | IFC4 | [IFC](ifc) |
| IFC5 alpha (`.ifcx`) | áno | áno | [IFC](ifc) |
| Blender (`.blend`) | prepojený | cez glTF | [Blender](blender) |
| FreeCAD (`.FCStd`) | prepojený | áno | [FreeCAD](freecad) |
| glTF (`.glb`, `.gltf`) | materiály | áno | [glTF](gltf) |
| PDF | nie | listy | [PDF, SVG a DXF](dxf-svg) |
| SVG | rohové pečiatky | pohľady a listy | [PDF, SVG a DXF](dxf-svg) |
| DXF | rohové pečiatky | nie | [PDF, SVG a DXF](dxf-svg) |
| CSV | nie | výkazy | [Výkazy](schedules) |
